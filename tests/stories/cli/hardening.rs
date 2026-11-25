use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use cucumber::{then, when};
use serde_json::Value;
use specforge::cli::{run, CliOutput};
use specforge::graph::{self, ArtifactId, GraphSnapshot, VerificationMethod};
use specforge::ledger::{EvidenceKind, EvidenceLink, EvidencePayload, EvidenceRecord, EvidenceRelation, ProducerMeta};
use specforge::ledger_fs::FsLedger;
use specforge::trace::Source;
use specforge::verify::{EvaluationResult, RunState, VerificationOutcome};

use super::path_arg;
use crate::Forge;

fn run_binary(args: &[&str], epoch: Option<u64>) -> CliOutput {
    let mut cmd: Command = Command::new(env!("CARGO_BIN_EXE_specforge"));
    cmd.args(args);
    match epoch {
        Some(value) => {
            cmd.env("SOURCE_DATE_EPOCH", value.to_string());
        }
        None => {
            cmd.env_remove("SOURCE_DATE_EPOCH");
        }
    }
    let output: Output = cmd.output().expect("specforge binary executes");
    let exit_code: u8 = u8::try_from(output.status.code().expect("process exited with status")).expect("exit code fits in u8");
    let stdout: String = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let stderr: String = String::from_utf8(output.stderr).expect("utf-8 stderr");
    CliOutput { exit_code, stdout, stderr }
}

fn parse_iso_utc_epoch(timestamp: &str) -> u64 {
    let bytes: &[u8] = timestamp.as_bytes();
    assert_eq!(bytes.len(), 20, "expected YYYY-MM-DDTHH:MM:SSZ, got {timestamp}");
    assert_eq!(bytes[4], b'-', "invalid timestamp {timestamp}");
    assert_eq!(bytes[7], b'-', "invalid timestamp {timestamp}");
    assert_eq!(bytes[10], b'T', "invalid timestamp {timestamp}");
    assert_eq!(bytes[13], b':', "invalid timestamp {timestamp}");
    assert_eq!(bytes[16], b':', "invalid timestamp {timestamp}");
    assert_eq!(bytes[19], b'Z', "invalid timestamp {timestamp}");
    let year: i64 = timestamp[0..4].parse::<i64>().expect("year parses");
    let month: i64 = timestamp[5..7].parse::<i64>().expect("month parses");
    let day: i64 = timestamp[8..10].parse::<i64>().expect("day parses");
    let hour: u64 = timestamp[11..13].parse::<u64>().expect("hour parses");
    let minute: u64 = timestamp[14..16].parse::<u64>().expect("minute parses");
    let second: u64 = timestamp[17..19].parse::<u64>().expect("second parses");
    assert!((1..=12).contains(&month), "month out of range in {timestamp}");
    assert!((1..=31).contains(&day), "day out of range in {timestamp}");
    assert!(hour < 24 && minute < 60 && second < 60, "time out of range in {timestamp}");
    let adj_year: i64 = year - if month <= 2 { 1 } else { 0 };
    let era: i64 = adj_year.div_euclid(400);
    let yoe: i64 = adj_year.rem_euclid(400);
    let adj_month: i64 = if month > 2 { month - 3 } else { month + 9 };
    let doy: i64 = (153 * adj_month + 2) / 5 + day - 1;
    let doe: i64 = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days: i64 = era * 146_097 + doe - 719_468;
    assert!(days >= 0, "timestamp before unix epoch: {timestamp}");
    (days as u64) * 86_400 + hour * 3_600 + minute * 60 + second
}

#[when(expr = "the specforge binary runs with {string}")]
fn binary_runs_raw(world: &mut Forge, raw: String) {
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    world.cli.last_output = Some(run_binary(&tokens, None));
}

#[when("the CLI checks malformed flags, inline values, missing values, and extra arguments")]
fn cli_checks_malformed_args(world: &mut Forge) {
    let safe: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    let cases: Vec<(&str, Vec<String>)> = vec![
        ("does not accept a value", vec!["compile".to_string(), safe.clone(), "--json=true".to_string()]),
        (
            "flag --ledger requires a value",
            vec!["assess".to_string(), safe, "--ledger".to_string(), "--json".to_string()],
        ),
        ("unknown flag for assess", vec!["assess".to_string(), "--bogus".to_string()]),
        ("unknown flag for verify", vec!["verify".to_string(), "--bogus".to_string()]),
        ("unknown flag for impact", vec!["impact".to_string(), "--bogus".to_string()]),
        ("unexpected positional argument for impact", vec!["impact".to_string(), "extra-dir".to_string()]),
        ("unknown flag for argument", vec!["argument".to_string(), "--bogus".to_string()]),
        ("unknown flag for recommend", vec!["recommend".to_string(), "--bogus".to_string()]),
        ("unknown flag for ledger", vec!["ledger".to_string(), ledger, "--bogus".to_string()]),
    ];
    let mut results: Vec<(String, CliOutput)> = Vec::with_capacity(cases.len());
    for (needle, args) in cases {
        results.push((needle.to_string(), run(args)));
    }
    world.cli.malformed_checks = results;
}

#[then("every subcommand rejects malformed CLI invocations with exit 2 and diagnostic stderr")]
fn assert_malformed_checks(world: &mut Forge) {
    assert!(!world.cli.malformed_checks.is_empty(), "malformed checks were executed");
    for (needle, output) in &world.cli.malformed_checks {
        assert_eq!(output.exit_code, 2, "expected exit 2 for \"{needle}\", got {output:?}");
        assert!(
            output.stderr.contains(needle.as_str()),
            "expected stderr to contain \"{needle}\", got:\n{}",
            output.stderr
        );
        assert!(
            output.stderr.contains("Usage:"),
            "expected stderr to include Usage text for \"{needle}\", got:\n{}",
            output.stderr
        );
    }
}

#[when("the CLI computes human-readable impact from baseline to property-edit against the ledger")]
fn cli_impact_human_property_edit(world: &mut Forge) {
    let from: String = path_arg(&world.cli.safe_dir, "baseline dir exists");
    let to: String = path_arg(&world.cli.property_edit_dir, "property-edit dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["impact", "--from", &from, "--to", &to, "--ledger", &ledger]));
}

#[when(expr = "the ledger is seeded with colliding record {string} and a supports link")]
fn seed_colliding_record_and_link(world: &mut Forge, record_id: String) {
    let safe_dir: &PathBuf = world.cli.safe_dir.as_ref().expect("safe spec dir exists");
    let ledger_dir: &PathBuf = world.cli.ledger_dir.as_ref().expect("ledger dir exists");
    let text: String = fs::read_to_string(safe_dir.join("01-message.md")).expect("safe spec readable");
    let snapshot: GraphSnapshot = graph::compile_graph(&[Source {
        path: "01-message.md".to_string(),
        text,
    }])
    .expect("safe spec compiles");
    let mut fs_ledger: FsLedger = FsLedger::open(ledger_dir.clone()).expect("fs ledger opens");
    let payload: EvidencePayload = EvidencePayload {
        id: record_id.clone(),
        kind: EvidenceKind::PropertyEvaluation,
        producer: ProducerMeta {
            name: "specforge-test-engine".to_string(),
            version: "0.1.0".to_string(),
        },
        adapter: ProducerMeta {
            name: "specforge-test-adapter".to_string(),
            version: "0.1.0".to_string(),
        },
        timestamp: "2026-11-25T12:00:00Z".to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        source_revisions: snapshot.sources.clone(),
        dependencies: snapshot.dependency_pins_for_claim("CLM-MSG-001"),
        run_id: "RUN-SEED-2".to_string(),
        activity_id: "ACT-SEED-2".to_string(),
        method: VerificationMethod::PropertyBased,
        property_id: Some(ArtifactId::parse("PROP-MSG-001").expect("valid prop id")),
        claim_ids: vec![ArtifactId::parse("CLM-MSG-001").expect("valid claim id")],
        scope: "single-broker".to_string(),
        assumptions: vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption id")],
        run_state: RunState::Completed,
        outcome: VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        counterexample_id: None,
        replay: None,
        artifact_digests: BTreeMap::new(),
        limitations: Vec::new(),
    };
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("record seals");
    fs_ledger.append_record(record).expect("record appends");
    let link: EvidenceLink =
        EvidenceLink::seal(record_id, "CLM-MSG-001".to_string(), EvidenceRelation::Supports, "seeded support".to_string()).expect("link seals");
    fs_ledger.append_link(link).expect("link appends");
}

#[when("the CLI runs ledger list on the ledger directory")]
fn cli_ledger_list_human(world: &mut Forge) {
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["ledger", &ledger, "list"]));
}

#[when(expr = "the specforge binary verifies {string} for {string} at epoch {int} into the ledger")]
fn binary_verify_at_epoch(world: &mut Forge, property: String, claim: String, epoch: u64) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    let output: CliOutput = run_binary(
        &[
            "verify",
            &specs,
            "--property",
            &property,
            "--claim",
            &claim,
            "--seed",
            "7",
            "--budget",
            "16",
            "--ledger",
            &ledger,
            "--json",
        ],
        Some(epoch),
    );
    world.cli.last_output = Some(output);
}

#[then(expr = "the recorded ledger timestamp for {string} equals {string}")]
fn assert_recorded_timestamp_equals(world: &mut Forge, evidence_id: String, expected: String) {
    let ledger_dir: &PathBuf = world.cli.ledger_dir.as_ref().expect("ledger dir exists");
    let reopened: FsLedger = FsLedger::open_read_only(ledger_dir.clone()).expect("ledger reopens");
    let record: &EvidenceRecord = reopened.ledger().get(&evidence_id).expect("evidence record exists");
    assert_eq!(record.timestamp, expected);
}

#[then(expr = "the recorded ledger timestamp for {string} matches the system clock in UTC")]
fn assert_recorded_timestamp_matches_clock(world: &mut Forge, evidence_id: String) {
    let ledger_dir: &PathBuf = world.cli.ledger_dir.as_ref().expect("ledger dir exists");
    let reopened: FsLedger = FsLedger::open_read_only(ledger_dir.clone()).expect("ledger reopens");
    let record: &EvidenceRecord = reopened.ledger().get(&evidence_id).expect("evidence record exists");
    let parsed_epoch: u64 = parse_iso_utc_epoch(&record.timestamp);
    let now_epoch: u64 = SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock after unix epoch").as_secs();
    let delta: u64 = now_epoch.abs_diff(parsed_epoch);
    assert!(
        delta <= 15,
        "recorded timestamp {} ({parsed_epoch}) drifted {delta}s from now ({now_epoch})",
        record.timestamp
    );
}

#[then("the specforge binary records exact UTC timestamps for epochs 4102448523, 4107575742, and 7275011696")]
fn assert_epoch_boundary_timestamps(world: &mut Forge) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger_dir: PathBuf = world.cli.ledger_dir.clone().expect("ledger dir exists");
    let ledger_arg: String = ledger_dir.to_str().expect("utf-8 path").to_string();
    let cases: [(u64, &str); 3] = [
        (4_102_448_523, "2100-01-01T01:02:03Z"),
        (4_107_575_742, "2100-03-01T09:15:42Z"),
        (7_275_011_696, "2200-07-15T12:34:56Z"),
    ];
    for (epoch, expected) in cases {
        let output: CliOutput = run_binary(
            &[
                "verify",
                &specs,
                "--property",
                "PROP-MSG-001",
                "--claim",
                "CLM-MSG-001",
                "--seed",
                "7",
                "--budget",
                "16",
                "--ledger",
                &ledger_arg,
                "--json",
            ],
            Some(epoch),
        );
        assert_eq!(output.exit_code, 0, "verify at epoch {epoch} failed: {}", output.stderr);
        let parsed: Value = serde_json::from_str::<Value>(&output.stdout).expect("valid verify JSON");
        let evidence_id: &str = parsed.get("evidence_id").and_then(Value::as_str).expect("evidence_id present");
        let reopened: FsLedger = FsLedger::open_read_only(ledger_dir.clone()).expect("ledger reopens");
        let record: &EvidenceRecord = reopened.ledger().get(evidence_id).expect("record exists");
        assert_eq!(record.timestamp, expected, "unexpected UTC timestamp for epoch {epoch}");
    }
}

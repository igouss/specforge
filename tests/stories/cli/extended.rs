use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use cucumber::{given, when};
use specforge::cli::run;
use specforge::graph::{self, ArtifactId, GraphSnapshot, VerificationMethod};
use specforge::ledger::{EvidenceKind, EvidencePayload, EvidenceRecord, ProducerMeta};
use specforge::ledger_fs::FsLedger;
use specforge::trace::Source;
use specforge::verify::{EvaluationResult, RunState, VerificationOutcome};

use super::{fresh_workspace, path_arg, safe_variant, write_spec_dir, CliState};
use crate::{fixture_text, Forge};

fn ensure_defective_dir(world: &mut Forge) -> String {
    if let Some(dir) = world.cli.defective_dir.as_ref() {
        return dir.to_str().expect("utf-8 path").to_string();
    }
    let root: PathBuf = world.cli.workspace.clone().expect("workspace exists");
    let defective: PathBuf = write_spec_dir(&root, "defective", "01-bad.md", &fixture_text("sdv-defective.md"));
    let path: String = defective.to_str().expect("utf-8 path").to_string();
    world.cli.defective_dir = Some(defective);
    path
}

fn missing_ledger_arg(world: &Forge) -> String {
    world
        .cli
        .workspace
        .as_ref()
        .expect("workspace exists")
        .join("missing-ledger")
        .to_str()
        .expect("utf-8 path")
        .to_string()
}

#[given("a CLI workspace with a safe assurance specification directory and an empty ledger directory")]
fn workspace_safe_assurance_and_ledger(world: &mut Forge) {
    let root: PathBuf = fresh_workspace();
    let safe: PathBuf = write_spec_dir(&root, "safe-assurance", "01-assurance.md", &safe_variant("sdv-assurance.md"));
    let ledger: PathBuf = root.join("ledger");
    fs::create_dir_all(&ledger).expect("ledger dir created");
    world.cli = CliState {
        workspace: Some(root),
        safe_dir: Some(safe),
        ledger_dir: Some(ledger),
        ..CliState::default()
    };
}

#[when(expr = "the CLI builds an argument for claim {string} on the safe directory against the ledger")]
fn cli_argument_markdown(world: &mut Forge, claim: String) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["argument", &specs, "--ledger", &ledger, "--claim", &claim]));
}

#[when(expr = "the CLI builds an argument for claim {string} on the safe directory against the ledger as JSON")]
fn cli_argument_json(world: &mut Forge, claim: String) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["argument", &specs, "--ledger", &ledger, "--claim", &claim, "--json"]));
}

#[when(expr = "the CLI builds an argument for claim {string} with subclaims {string} on the safe directory against the ledger as JSON")]
fn cli_argument_subclaims_json(world: &mut Forge, claim: String, subclaims: String) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    let mut args: Vec<String> = vec!["argument".to_string(), specs, "--ledger".to_string(), ledger, "--claim".to_string(), claim];
    for raw in subclaims.split(',') {
        let trimmed: &str = raw.trim();
        if !trimmed.is_empty() {
            args.push("--subclaim".to_string());
            args.push(trimmed.to_string());
        }
    }
    args.push("--json".to_string());
    world.cli.last_output = Some(run(args));
}

#[when(expr = "the CLI builds an argument for claim {string} on a defective specification directory against the ledger")]
fn cli_argument_defective(world: &mut Forge, claim: String) {
    let specs: String = ensure_defective_dir(world);
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["argument", &specs, "--ledger", &ledger, "--claim", &claim]));
}

#[when(expr = "the CLI builds an argument for claim {string} on the safe directory against a nonexistent ledger")]
fn cli_argument_missing_ledger(world: &mut Forge, claim: String) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = missing_ledger_arg(world);
    world.cli.last_output = Some(run(["argument", &specs, "--ledger", &ledger, "--claim", &claim]));
}

#[when("the CLI runs recommend on the safe directory against the ledger")]
fn cli_recommend_safe(world: &mut Forge) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["recommend", &specs, "--ledger", &ledger]));
}

#[when("the CLI runs recommend on the safe directory against the ledger as JSON")]
fn cli_recommend_safe_json(world: &mut Forge) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["recommend", &specs, "--ledger", &ledger, "--json"]));
}

#[when("the CLI runs recommend on the property-edit directory against the ledger as JSON")]
fn cli_recommend_property_edit_json(world: &mut Forge) {
    let specs: String = path_arg(&world.cli.property_edit_dir, "property-edit spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["recommend", &specs, "--ledger", &ledger, "--json"]));
}

#[when("the CLI runs recommend on a defective specification directory against the ledger")]
fn cli_recommend_defective(world: &mut Forge) {
    let specs: String = ensure_defective_dir(world);
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["recommend", &specs, "--ledger", &ledger]));
}

#[when("the CLI runs recommend on the safe directory against a nonexistent ledger")]
fn cli_recommend_missing_ledger(world: &mut Forge) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = missing_ledger_arg(world);
    world.cli.last_output = Some(run(["recommend", &specs, "--ledger", &ledger]));
}

#[when("the CLI runs recommend on the violating directory against the ledger as JSON")]
fn cli_recommend_violating_json(world: &mut Forge) {
    let specs: String = path_arg(&world.cli.violating_dir, "violating spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["recommend", &specs, "--ledger", &ledger, "--json"]));
}

fn seal_blobless_record(snapshot: &GraphSnapshot, id: &str, run_id: &str, property_id: &str, claim_id: &str, result: EvaluationResult) -> EvidenceRecord {
    let payload: EvidencePayload = EvidencePayload {
        id: id.to_string(),
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
        dependencies: snapshot.dependency_pins_for_claim(claim_id),
        run_id: run_id.to_string(),
        activity_id: "ACT-BLOBLESS-1".to_string(),
        method: VerificationMethod::PropertyBased,
        property_id: Some(ArtifactId::parse(property_id).expect("valid prop id")),
        claim_ids: vec![ArtifactId::parse(claim_id).expect("valid claim id")],
        scope: "single-broker".to_string(),
        assumptions: vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption id")],
        run_state: RunState::Completed,
        outcome: VerificationOutcome::Evaluation(result),
        counterexample_id: None,
        replay: None,
        artifact_digests: BTreeMap::new(),
        limitations: vec!["blobless evidence".to_string()],
    };
    EvidenceRecord::seal(payload).expect("record seals")
}

#[given(expr = "a CLI workspace with a violating assurance directory, a hydrated counterexample for {string}, and blobless mixed evidence for {string}")]
fn workspace_violating_assurance_mixed_ledger(world: &mut Forge, hydrated_claim: String, blobless_claim: String) {
    let root: PathBuf = fresh_workspace();
    let content: String = fixture_text("sdv-assurance.md");
    let violating: PathBuf = write_spec_dir(&root, "violating-assurance", "01-assurance.md", &content);
    let snapshot: GraphSnapshot = graph::compile_graph(&[Source {
        path: "01-assurance.md".to_string(),
        text: content,
    }])
    .expect("violating assurance spec compiles");
    let ledger_dir: PathBuf = root.join("ledger");
    fs::create_dir_all(&ledger_dir).expect("ledger dir created");
    let specs_arg: String = violating.to_str().expect("utf-8 path").to_string();
    let ledger_arg: String = ledger_dir.to_str().expect("utf-8 path").to_string();
    let verify_out: specforge::cli::CliOutput = run([
        "verify",
        &specs_arg,
        "--property",
        "PROP-MSG-001",
        "--claim",
        &hydrated_claim,
        "--seed",
        "42",
        "--budget",
        "128",
        "--ledger",
        &ledger_arg,
        "--json",
    ]);
    assert_eq!(verify_out.exit_code, 1, "expected violated verify for {hydrated_claim}");
    let mut fs_ledger: FsLedger = FsLedger::open(ledger_dir.clone()).expect("fs ledger opens");
    let inconclusive_record: EvidenceRecord = seal_blobless_record(
        &snapshot,
        "EV-INCONC-2",
        "RUN-INCONC-2",
        "PROP-MSG-002",
        &blobless_claim,
        EvaluationResult::Inconclusive,
    );
    let violated_record: EvidenceRecord = seal_blobless_record(
        &snapshot,
        "EV-BLOBLESS-2",
        "RUN-BLOBLESS-2",
        "PROP-MSG-002",
        &blobless_claim,
        EvaluationResult::Violated,
    );
    fs_ledger.append_record(inconclusive_record).expect("inconclusive record appends");
    fs_ledger.append_record(violated_record).expect("violated record appends");
    world.cli = CliState {
        workspace: Some(root),
        violating_dir: Some(violating),
        ledger_dir: Some(ledger_dir),
        ..CliState::default()
    };
}

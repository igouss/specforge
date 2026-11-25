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

#[given("a CLI workspace with a violating specification directory and a blobless violated ledger")]
fn workspace_violating_and_blobless_ledger(world: &mut Forge) {
    let root: PathBuf = fresh_workspace();
    let content: String = fixture_text("sdv-message.md");
    let violating: PathBuf = write_spec_dir(&root, "violating", "01-message.md", &content);
    let snapshot: GraphSnapshot = graph::compile_graph(&[Source {
        path: "01-message.md".to_string(),
        text: content,
    }])
    .expect("violating spec compiles");
    let ledger_dir: PathBuf = root.join("ledger");
    let mut fs_ledger: FsLedger = FsLedger::open(ledger_dir.clone()).expect("fs ledger opens");
    let payload: EvidencePayload = EvidencePayload {
        id: "EV-BLOBLESS-1".to_string(),
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
        run_id: "RUN-BLOBLESS-1".to_string(),
        activity_id: "ACT-BLOBLESS-1".to_string(),
        method: VerificationMethod::PropertyBased,
        property_id: Some(ArtifactId::parse("PROP-MSG-001").expect("valid prop id")),
        claim_ids: vec![ArtifactId::parse("CLM-MSG-001").expect("valid claim id")],
        scope: "single-broker".to_string(),
        assumptions: vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption id")],
        run_state: RunState::Completed,
        outcome: VerificationOutcome::Evaluation(EvaluationResult::Violated),
        counterexample_id: None,
        replay: None,
        artifact_digests: BTreeMap::new(),
        limitations: vec!["blobless violated evidence".to_string()],
    };
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("record seals");
    fs_ledger.append_record(record).expect("record appends");
    world.cli = CliState {
        workspace: Some(root),
        violating_dir: Some(violating),
        ledger_dir: Some(ledger_dir),
        ..CliState::default()
    };
}

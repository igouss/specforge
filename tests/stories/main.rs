#![allow(clippy::expect_used, clippy::panic)]

mod adapters;
mod cli;
mod evidence;
mod exploration;
mod external_assurance;
mod graph;
mod ledger;
mod standing;
mod stories;
mod trace;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};

use specforge::assess::{Assessment, Pending};
use specforge::assurance::{ClaimAssessment, ImpactSet};
use specforge::graph::{ContentHash, GraphDefect, GraphSnapshot};
use specforge::ledger::{EvidenceLedger, EvidenceRecord, LedgerError};
use specforge::ledger_fs::FsLedger;
use specforge::model::GeneratedScenario;
use specforge::stories::{Mode, Outcome, Project};
use specforge::trace::{Defect, Trace};
use specforge::verify::{AdapterError, Counterexample, FindingRecord, PropertyEvaluation, RunEnvelope, VerificationDecision};

static STARTED: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Default, cucumber::World)]
#[world(init = Self::started)]
pub struct Forge {
    documents: Vec<String>,
    compiled: Option<Result<Trace, Vec<Defect>>>,
    bound: Vec<String>,
    twice: Vec<String>,
    pending: Vec<Pending>,
    monitored: Vec<String>,
    assessment: Option<Assessment>,
    project: Option<Project>,
    outcome: Option<Outcome>,
    ran: usize,
    graph_documents: Vec<String>,
    graph_compiled: Option<Result<GraphSnapshot, Vec<GraphDefect>>>,
    baseline_snapshot: Option<GraphSnapshot>,
    revised_snapshot: Option<GraphSnapshot>,
    evidence_ledger: EvidenceLedger,
    last_append: Option<Result<(), LedgerError>>,
    fs_ledger_dir: Option<PathBuf>,
    fs_ledger: Option<FsLedger>,
    fs_reload: Option<Result<FsLedger, LedgerError>>,
    blob_digests: std::collections::BTreeMap<String, ContentHash>,
    claim_assessment: Option<ClaimAssessment>,
    impact: Option<ImpactSet>,
    generated_scenarios: Vec<GeneratedScenario>,
    evaluation: Option<PropertyEvaluation>,
    finding: Option<FindingRecord>,
    counterexample: Option<Counterexample>,
    original_counterexample_snapshot: Option<Counterexample>,
    decision: Option<VerificationDecision>,
    snapshot_digest_before_decision: Option<ContentHash>,
    envelope: Option<RunEnvelope>,
    replay_envelope: Option<RunEnvelope>,
    replay_result: Option<Result<RunEnvelope, AdapterError>>,
    cucumber_record: Option<EvidenceRecord>,
    exploration: exploration::ExplorationState,
    cli: cli::CliState,
    external_assurance: external_assurance::ExternalAssuranceState,
}

impl Forge {
    fn started() -> Self {
        STARTED.fetch_add(1, Ordering::SeqCst);
        Self::default()
    }
}

fn witnessed(outcome: &Outcome) -> bool {
    let started: usize = STARTED.load(Ordering::SeqCst);
    match outcome {
        Outcome::Ran(tally) => tally.ran == started && tally.passed == tally.ran,
        Outcome::Drawn(_) => started == 0,
        Outcome::Unread(_) | Outcome::Defective(_) | Outcome::Unfit(_) | Outcome::Stale(_) => false,
    }
}

pub fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

pub fn fixture(name: &str) -> PathBuf {
    root().join("tests/fixtures").join(name)
}

pub fn fixture_text(name: &str) -> String {
    fs::read_to_string(fixture(name)).expect("the fixture is read")
}

pub fn unescaped(text: &str) -> String {
    text.replace("\\\"", "\"")
}

pub fn listed(text: &str, separator: &str) -> Vec<String> {
    unescaped(text).split(separator).map(str::to_string).collect()
}

fn main() -> ExitCode {
    let project: Project = Project {
        specifications: root().join("specs"),
        pending: root().join("specs/pending.md"),
        view: root().join("docs/views/trace.md"),
        scratch: Path::new(env!("CARGO_TARGET_TMPDIR")).join("stories"),
        monitored: Vec::new(),
    };
    let mode: Mode = match std::env::var("SPECFORGE_VIEW").is_ok_and(|value: String| value == "draw") {
        true => Mode::Draw,
        false => Mode::Verify,
    };
    let runtime: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread().build().expect("a runtime");
    let outcome: Outcome = runtime.block_on(specforge::stories::stories::<Forge>(&project, mode));
    println!("{outcome}");
    match outcome.passed() && witnessed(&outcome) {
        true => ExitCode::SUCCESS,
        false => ExitCode::FAILURE,
    }
}

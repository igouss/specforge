use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use super::format::{format_argument, format_assess, format_compile, format_impact, format_ledger_list, format_ledger_verify, format_recommend, format_verify};
use super::parse::LedgerAction;
use super::recommend::{append_refuted_fallbacks, hydrate_corpus};
use super::{utc_timestamp_now, CliOutput, EXIT_CHECK_FAILED, EXIT_OK};
use crate::argument::{self, AssuranceArgument};
use crate::assurance::{self, ClaimAssessment, ImpactSet};
use crate::corpus::CounterexampleRegistry;
use crate::graph::{self, canonical_json, ArtifactId, ContentHash, GraphDefect, GraphSnapshot, VerificationClaim};
use crate::ledger::{EvidenceKind, EvidenceRecord, LedgerError};
use crate::ledger_fs::FsLedger;
use crate::planner::{self, ExplorationRecommendation};
use crate::proptest_adapter::ProptestAdapter;
use crate::trace::Source;
use crate::verify::{AdapterError, ClaimStatus, EvaluationResult, FormalOutcome, RunEnvelope, VerificationAdapter, VerificationOutcome, VerificationRequest};

const DEFAULT_MAX_SEQUENCE_LEN: usize = 12;

enum CompileError {
    Io(String),
    Defects(Vec<String>),
}

fn load_markdown_sources(dir: &Path) -> Result<Vec<Source>, String> {
    let entries: fs::ReadDir = fs::read_dir(dir).map_err(|error: std::io::Error| format!("{}: {error}", dir.display()))?;
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries {
        let path: PathBuf = entry.map_err(|error: std::io::Error| format!("{}: {error}", dir.display()))?.path();
        if path.is_dir() {
            continue;
        }
        if path.extension().is_some_and(|ext: &OsStr| ext == "md") {
            let name: String = path.file_name().map_or_else(String::new, |name: &OsStr| name.to_string_lossy().into_owned());
            files.push((name, path));
        }
    }
    files.sort_by(|left: &(String, PathBuf), right: &(String, PathBuf)| left.0.cmp(&right.0));
    files
        .into_iter()
        .map(|(name, path): (String, PathBuf)| {
            let text: String = fs::read_to_string(&path).map_err(|error: std::io::Error| format!("{}: {error}", path.display()))?;
            Ok(Source { path: name, text })
        })
        .collect()
}

fn compile_dir(dir: &Path) -> Result<GraphSnapshot, CompileError> {
    let sources: Vec<Source> = load_markdown_sources(dir).map_err(CompileError::Io)?;
    graph::compile_graph(&sources)
        .map_err(|defects: Vec<GraphDefect>| CompileError::Defects(defects.into_iter().map(|defect: GraphDefect| defect.to_string()).collect()))
}

fn compile_error_output(error: CompileError, json: bool) -> CliOutput {
    match error {
        CompileError::Io(message) => CliOutput::error(message, json),
        CompileError::Defects(defects) => CliOutput::defects(defects, json),
    }
}

fn open_existing_ledger(dir: &Path) -> Result<FsLedger, String> {
    if !dir.exists() {
        return Err(format!("ledger directory does not exist: {}", dir.display()));
    }
    if !dir.is_dir() {
        return Err(format!("ledger path is not a directory: {}", dir.display()));
    }
    FsLedger::open_read_only(dir.to_path_buf()).map_err(|error: LedgerError| error.to_string())
}

fn is_claim_supported(status: ClaimStatus) -> bool {
    match status {
        ClaimStatus::Supported => true,
        ClaimStatus::Refuted | ClaimStatus::Inconclusive | ClaimStatus::RequiresReview | ClaimStatus::Stale | ClaimStatus::Invalid => false,
    }
}

fn verify_exit_code(outcome: &VerificationOutcome) -> u8 {
    match outcome {
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied) => EXIT_OK,
        VerificationOutcome::Evaluation(
            EvaluationResult::Violated | EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated | EvaluationResult::InvalidEvaluation,
        ) => EXIT_CHECK_FAILED,
        VerificationOutcome::Formal(
            FormalOutcome::Proved
            | FormalOutcome::Refuted
            | FormalOutcome::BoundedSuccess { .. }
            | FormalOutcome::BoundedFailure { .. }
            | FormalOutcome::Unknown
            | FormalOutcome::Timeout
            | FormalOutcome::Unsupported,
        ) => EXIT_CHECK_FAILED,
    }
}

pub(crate) fn run_compile(specs_dir: &Path, json: bool) -> CliOutput {
    let snapshot: GraphSnapshot = match compile_dir(specs_dir) {
        Ok(snapshot) => snapshot,
        Err(error) => return compile_error_output(error, json),
    };
    format_compile(&snapshot, json)
}

pub(crate) fn run_assess(specs_dir: &Path, ledger_dir: &Path, claim_filter: Option<&str>, json: bool) -> CliOutput {
    let snapshot: GraphSnapshot = match compile_dir(specs_dir) {
        Ok(snapshot) => snapshot,
        Err(error) => return compile_error_output(error, json),
    };
    let fs_ledger: FsLedger = match open_existing_ledger(ledger_dir) {
        Ok(ledger) => ledger,
        Err(message) => return CliOutput::error(message, json),
    };
    let targets: Vec<&VerificationClaim> = match claim_filter {
        Some(claim_id) => match snapshot.claim(claim_id) {
            Some(claim) => vec![claim],
            None => return CliOutput::error(format!("unknown claim {claim_id}"), json),
        },
        None => snapshot.claims.iter().collect(),
    };
    let assessments: Vec<ClaimAssessment> = targets
        .into_iter()
        .map(|claim: &VerificationClaim| assurance::assess_claim(claim, &snapshot, fs_ledger.ledger()))
        .collect();
    let passed: bool = !assessments.is_empty() && assessments.iter().all(|item: &ClaimAssessment| is_claim_supported(item.status));
    let exit_code: u8 = if passed { EXIT_OK } else { EXIT_CHECK_FAILED };
    format_assess(&snapshot, &assessments, passed, exit_code, json)
}

fn next_evidence_ids(ledger: &FsLedger) -> (String, String) {
    let count: usize = ledger.ledger().records().len();
    let start: usize = count + 1;
    let index: usize = (start..=start.saturating_add(count))
        .find(|candidate: &usize| ledger.ledger().get(&format!("EV-CLI-{candidate}")).is_none())
        .unwrap_or(start);
    (format!("EV-CLI-{index}"), format!("RUN-CLI-{index}"))
}

fn persist_envelope(ledger: &mut FsLedger, snapshot: &GraphSnapshot, envelope: &RunEnvelope, evidence_id: String) -> Result<Option<ContentHash>, String> {
    let mut artifact_digests: BTreeMap<String, ContentHash> = BTreeMap::new();
    let mut primary_digest: Option<ContentHash> = None;
    if let Some(counterexample) = envelope.minimized_counterexample.as_ref().or(envelope.original_counterexample.as_ref()) {
        let encoded: String = canonical_json(counterexample)?;
        let digest: ContentHash = ledger.write_blob(encoded.as_bytes()).map_err(|error: LedgerError| error.to_string())?;
        artifact_digests.insert("counterexample".to_string(), digest.clone());
        primary_digest = Some(digest);
    }
    if envelope.minimized_counterexample.is_some() {
        if let Some(original) = envelope.original_counterexample.as_ref() {
            let encoded: String = canonical_json(original)?;
            let digest: ContentHash = ledger.write_blob(encoded.as_bytes()).map_err(|error: LedgerError| error.to_string())?;
            artifact_digests.insert("original_counterexample".to_string(), digest);
        }
    }
    let record: EvidenceRecord = EvidenceRecord::from_envelope(
        evidence_id,
        EvidenceKind::PropertyEvaluation,
        utc_timestamp_now(),
        snapshot,
        envelope,
        artifact_digests,
    )
    .map_err(|error: LedgerError| error.to_string())?;
    ledger.append_record(record).map_err(|error: LedgerError| error.to_string())?;
    Ok(primary_digest)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_verify(specs_dir: &Path, property: &str, claims: &[String], seed: u64, budget: usize, ledger_dir: Option<&Path>, json: bool) -> CliOutput {
    let snapshot: GraphSnapshot = match compile_dir(specs_dir) {
        Ok(snapshot) => snapshot,
        Err(error) => return compile_error_output(error, json),
    };
    let Some(property_id): Option<ArtifactId> = ArtifactId::parse(property) else {
        return CliOutput::error(format!("invalid property id \"{property}\""), json);
    };
    let mut claim_ids: Vec<ArtifactId> = Vec::with_capacity(claims.len());
    for raw in claims {
        let Some(parsed): Option<ArtifactId> = ArtifactId::parse(raw) else {
            return CliOutput::error(format!("invalid claim id \"{raw}\""), json);
        };
        claim_ids.push(parsed);
    }
    let adapter: ProptestAdapter = ProptestAdapter::for_machine(DEFAULT_MAX_SEQUENCE_LEN);
    let validation_request: VerificationRequest = VerificationRequest {
        run_id: format!("RUN-CLI-{property_id}-{seed}-{budget}"),
        activity_id: "ACT-CLI-PBT".to_string(),
        property_id: property_id.clone(),
        claim_ids: claim_ids.clone(),
        budget,
        seed: Some(seed),
    };
    if let Err(error) = adapter.validate(&snapshot, &validation_request) {
        return CliOutput::error(error.to_string(), json);
    }
    let mut opened_ledger: Option<FsLedger> = match ledger_dir {
        Some(dir) => match FsLedger::open(dir.to_path_buf()) {
            Ok(ledger) => Some(ledger),
            Err(error) => return CliOutput::error(error.to_string(), json),
        },
        None => None,
    };
    let (evidence_id, run_id): (Option<String>, String) = match opened_ledger.as_ref() {
        Some(ledger) => {
            let (ev_id, r_id): (String, String) = next_evidence_ids(ledger);
            (Some(ev_id), r_id)
        }
        None => (None, validation_request.run_id),
    };
    let request: VerificationRequest = VerificationRequest {
        run_id,
        activity_id: validation_request.activity_id,
        property_id,
        claim_ids,
        budget,
        seed: Some(seed),
    };
    let envelope: RunEnvelope = match adapter.execute(&snapshot, &request) {
        Ok(envelope) => envelope,
        Err(error) => {
            let message: String = match error {
                AdapterError::Unsupported(why) | AdapterError::InvalidRequest(why) | AdapterError::NotReplayable(why) | AdapterError::Execution(why) => why,
            };
            return CliOutput::error(message, json);
        }
    };
    let counterexample_blob_digest: Option<ContentHash> = match (opened_ledger.as_mut(), evidence_id.as_ref()) {
        (Some(ledger), Some(ev_id)) => match persist_envelope(ledger, &snapshot, &envelope, ev_id.clone()) {
            Ok(digest) => digest,
            Err(message) => return CliOutput::error(message, json),
        },
        _ => None,
    };
    let exit_code: u8 = verify_exit_code(&envelope.outcome);
    format_verify(
        &envelope,
        seed,
        budget,
        evidence_id.as_deref(),
        counterexample_blob_digest.as_ref(),
        exit_code,
        json,
    )
}

pub(crate) fn run_ledger(ledger_dir: &Path, action: LedgerAction, json: bool) -> CliOutput {
    let fs_ledger: FsLedger = match open_existing_ledger(ledger_dir) {
        Ok(ledger) => ledger,
        Err(message) => return CliOutput::error(message, json),
    };
    match action {
        LedgerAction::Verify => {
            let mut blobs: BTreeSet<&ContentHash> = BTreeSet::new();
            for record in fs_ledger.ledger().records() {
                for digest in record.artifact_digests.values() {
                    blobs.insert(digest);
                }
            }
            format_ledger_verify(fs_ledger.ledger().records().len(), fs_ledger.ledger().links().len(), blobs.len(), json)
        }
        LedgerAction::Export => match fs_ledger.ledger().export_json() {
            Ok(exported) => CliOutput::ok(exported),
            Err(error) => CliOutput::error(error.to_string(), json),
        },
        LedgerAction::List => format_ledger_list(fs_ledger.ledger().records(), fs_ledger.ledger().links(), json),
    }
}

pub(crate) fn run_impact(from_dir: &Path, to_dir: &Path, ledger_dir: &Path, json: bool) -> CliOutput {
    let previous: GraphSnapshot = match compile_dir(from_dir) {
        Ok(snapshot) => snapshot,
        Err(error) => return compile_error_output(error, json),
    };
    let current: GraphSnapshot = match compile_dir(to_dir) {
        Ok(snapshot) => snapshot,
        Err(error) => return compile_error_output(error, json),
    };
    let fs_ledger: FsLedger = match open_existing_ledger(ledger_dir) {
        Ok(ledger) => ledger,
        Err(message) => return CliOutput::error(message, json),
    };
    let impact: ImpactSet = assurance::impact_set(&previous, &current, fs_ledger.ledger());
    let clean: bool = impact.stale_evidence_ids.is_empty() && impact.affected_claim_ids.is_empty();
    let exit_code: u8 = if clean { EXIT_OK } else { EXIT_CHECK_FAILED };
    format_impact(&previous, &current, &impact, clean, exit_code, json)
}

pub(crate) fn run_argument(specs_dir: &Path, ledger_dir: &Path, claim: &str, subclaims: &[String], json: bool) -> CliOutput {
    let snapshot: GraphSnapshot = match compile_dir(specs_dir) {
        Ok(snapshot) => snapshot,
        Err(error) => return compile_error_output(error, json),
    };
    let fs_ledger: FsLedger = match open_existing_ledger(ledger_dir) {
        Ok(ledger) => ledger,
        Err(message) => return CliOutput::error(message, json),
    };
    let built: AssuranceArgument = match argument::build_assurance_argument(&snapshot, fs_ledger.ledger(), claim, subclaims) {
        Ok(built) => built,
        Err(message) => return CliOutput::error(message, json),
    };
    let exit_code: u8 = if is_claim_supported(built.status) { EXIT_OK } else { EXIT_CHECK_FAILED };
    format_argument(&built, exit_code, json)
}

pub(crate) fn run_recommend(specs_dir: &Path, ledger_dir: &Path, json: bool) -> CliOutput {
    let snapshot: GraphSnapshot = match compile_dir(specs_dir) {
        Ok(snapshot) => snapshot,
        Err(error) => return compile_error_output(error, json),
    };
    let fs_ledger: FsLedger = match open_existing_ledger(ledger_dir) {
        Ok(ledger) => ledger,
        Err(message) => return CliOutput::error(message, json),
    };
    let assessments: Vec<ClaimAssessment> = snapshot
        .claims
        .iter()
        .map(|claim: &VerificationClaim| assurance::assess_claim(claim, &snapshot, fs_ledger.ledger()))
        .collect();
    let corpus: CounterexampleRegistry = match hydrate_corpus(&snapshot, &fs_ledger) {
        Ok(corpus) => corpus,
        Err(message) => return CliOutput::error(message, json),
    };
    let mut recommendations: Vec<ExplorationRecommendation> = planner::recommend_activities(&snapshot, fs_ledger.ledger(), &[], &assessments, &corpus);
    append_refuted_fallbacks(&snapshot, &assessments, &mut recommendations);
    let clean: bool = recommendations.is_empty();
    let exit_code: u8 = if clean { EXIT_OK } else { EXIT_CHECK_FAILED };
    format_recommend(&snapshot, &recommendations, clean, exit_code, json)
}

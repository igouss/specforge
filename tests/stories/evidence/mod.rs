mod assurance;
mod fs_ledger;
mod validation;

use std::collections::BTreeMap;

use cucumber::{given, then, when};
use specforge::assurance::{assess_claim, impact_set, ClaimAssessment, EvidenceEvaluationEntry, ImpactSet};
use specforge::graph::{ArtifactId, ContentHash, DependencyPin, GraphSnapshot, VerificationClaim, VerificationMethod};
use specforge::ledger::{EvidenceKind, EvidenceLedger, EvidenceLink, EvidencePayload, EvidenceRecord, EvidenceRelation, LedgerError, ProducerMeta};
use specforge::ledger_fs::FsLedger;
use specforge::verify::{EvaluationResult, FormalOutcome, RunState, VerificationOutcome};

use crate::graph::{compile_fixture, current_snapshot};
use crate::Forge;

#[derive(Debug, Default)]
pub struct EvidenceState {
    pub last_record_seal: Option<Result<EvidenceRecord, LedgerError>>,
    pub last_link_seal: Option<Result<EvidenceLink, LedgerError>>,
    pub last_exported_json: Option<String>,
    pub last_json_import: Option<Result<EvidenceLedger, String>>,
    pub fs_read_only: Option<FsLedger>,
    pub fs_blob_write: Option<Result<ContentHash, LedgerError>>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_payload_with_scope(
    snapshot: &GraphSnapshot,
    id: &str,
    property_id: &str,
    claim_id: &str,
    scope: &str,
    method: VerificationMethod,
    outcome: VerificationOutcome,
    artifact_digests: BTreeMap<String, ContentHash>,
) -> EvidencePayload {
    let prop_id: ArtifactId = ArtifactId::parse(property_id).expect("valid property id");
    let clm_id: ArtifactId = ArtifactId::parse(claim_id).expect("valid claim id");
    let dependencies: Vec<DependencyPin> = snapshot.dependency_pins_for_claim(claim_id);
    EvidencePayload {
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
        timestamp: "2026-11-23T20:00:00Z".to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        source_revisions: snapshot.sources.clone(),
        dependencies,
        run_id: format!("RUN-{id}"),
        activity_id: "ACT-TEST-001".to_string(),
        method,
        property_id: Some(prop_id),
        claim_ids: vec![clm_id],
        scope: scope.to_string(),
        assumptions: vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption id")],
        run_state: RunState::Completed,
        outcome,
        counterexample_id: None,
        replay: None,
        artifact_digests,
        limitations: vec!["finite test run".to_string()],
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_record_with_scope(
    snapshot: &GraphSnapshot,
    id: &str,
    property_id: &str,
    claim_id: &str,
    scope: &str,
    method: VerificationMethod,
    outcome: VerificationOutcome,
    artifact_digests: BTreeMap<String, ContentHash>,
) -> EvidenceRecord {
    let payload: EvidencePayload = build_payload_with_scope(snapshot, id, property_id, claim_id, scope, method, outcome, artifact_digests);
    EvidenceRecord::seal(payload).expect("evidence record seals")
}

pub(super) fn build_record(
    snapshot: &GraphSnapshot,
    id: &str,
    property_id: &str,
    claim_id: &str,
    method: VerificationMethod,
    outcome: VerificationOutcome,
    artifact_digests: BTreeMap<String, ContentHash>,
) -> EvidenceRecord {
    build_record_with_scope(snapshot, id, property_id, claim_id, "single-broker", method, outcome, artifact_digests)
}

pub(super) fn parse_relation(relation: &str) -> EvidenceRelation {
    match relation {
        "supports" => EvidenceRelation::Supports,
        "contradicts" => EvidenceRelation::Contradicts,
        "corroborates" => EvidenceRelation::Corroborates,
        "derived_from" => EvidenceRelation::DerivedFrom,
        "supersedes" => EvidenceRelation::Supersedes,
        "invalidates" => EvidenceRelation::Invalidates,
        other => panic!("unknown relation {other}"),
    }
}

#[given("an empty evidence ledger")]
fn empty_ledger(world: &mut Forge) {
    world.evidence_ledger = EvidenceLedger::new();
}

#[given(expr = "a satisfied evidence record {string} is appended for property {string} and claim {string}")]
#[when(expr = "a satisfied evidence record {string} is appended for property {string} and claim {string}")]
fn append_satisfied(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    let record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a satisfied evidence record {string} in scope {string} is appended for property {string} and claim {string}")]
fn append_satisfied_in_scope(world: &mut Forge, id: String, scope: String, property_id: String, claim_id: String) {
    let record: EvidenceRecord = build_record_with_scope(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        &scope,
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a violated evidence record {string} is appended for property {string} and claim {string}")]
#[when(expr = "a violated evidence record {string} is appended for property {string} and claim {string}")]
fn append_violated(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    let record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Violated),
        BTreeMap::new(),
    );
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "an inconclusive evidence record {string} is appended for property {string} and claim {string}")]
fn append_inconclusive(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    let record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Inconclusive),
        BTreeMap::new(),
    );
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a bounded-success formal evidence record {string} with bound {string} is appended for property {string} and claim {string}")]
fn append_bounded_formal(world: &mut Forge, id: String, bound: String, property_id: String, claim_id: String) {
    let record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        VerificationMethod::Formal,
        VerificationOutcome::Formal(FormalOutcome::BoundedSuccess { bound }),
        BTreeMap::new(),
    );
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[when(expr = "a duplicate evidence record {string} is appended")]
fn append_duplicate(world: &mut Forge, id: String) {
    let record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        "PROP-MSG-001",
        "CLM-MSG-001",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    world.last_append = Some(world.evidence_ledger.append_record(record));
}

#[when(expr = "a tampered evidence record {string} is appended")]
fn append_tampered(world: &mut Forge, id: String) {
    let mut record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        "PROP-MSG-001",
        "CLM-MSG-001",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    record.scope = "tampered-scope".to_string();
    world.last_append = Some(world.evidence_ledger.append_record(record));
}

#[then("the append is refused as duplicate")]
fn refused_duplicate(world: &mut Forge) {
    assert!(
        matches!(world.last_append, Some(Err(LedgerError::DuplicateRecord(_)))),
        "{:?}",
        world.last_append
    );
}

#[then("the append is refused as invalid digest")]
fn refused_invalid_digest(world: &mut Forge) {
    assert!(matches!(world.last_append, Some(Err(LedgerError::InvalidDigest(_)))), "{:?}", world.last_append);
}

#[when(expr = "a link {string} is appended from {string} to {string}")]
fn append_link(world: &mut Forge, relation: String, from_id: String, to_id: String) {
    let relation: EvidenceRelation = parse_relation(&relation);
    let link: EvidenceLink = EvidenceLink::seal(from_id, to_id, relation, "recorded in story".to_string()).expect("link seals");
    world.evidence_ledger.append_link(link).expect("link appends");
}

#[then(regex = r"^the ledger(?: still)? holds (\d+) evidence records?$")]
fn ledger_count(world: &mut Forge, expected: usize) {
    assert_eq!(world.evidence_ledger.records().len(), expected);
}

#[then("ledger integrity verification passes")]
fn ledger_integrity(world: &mut Forge) {
    world.evidence_ledger.verify_integrity().expect("integrity passes");
}

#[then("exporting the ledger as JSON is deterministic")]
fn ledger_export_deterministic(world: &mut Forge) {
    let first: String = world.evidence_ledger.export_json().expect("first export");
    let second: String = world.evidence_ledger.export_json().expect("second export");
    assert_eq!(first, second);
    world.evidence_state.last_exported_json = Some(first);
}

#[then(regex = r#"^record "([^"]+)"(?: still)? has evaluation result "([^"]+)"$"#)]
fn record_eval_result(world: &mut Forge, id: String, expected: String) {
    let record: &EvidenceRecord = world.evidence_ledger.get(&id).expect("record exists");
    match &record.outcome {
        VerificationOutcome::Evaluation(result) => assert_eq!(result.to_string(), expected),
        other => panic!("expected evaluation outcome, got {other:?}"),
    }
}

#[when(expr = "the snapshot changes to {string}")]
fn snapshot_changes_to(world: &mut Forge, name: String) {
    let previous: GraphSnapshot = current_snapshot(world).clone();
    let current: GraphSnapshot = compile_fixture(&name).expect("new snapshot compiles");
    world.impact = Some(impact_set(&previous, &current, &world.evidence_ledger));
    world.baseline_snapshot = Some(previous);
    world.graph_compiled = Some(Ok(current));
}

#[when(expr = "claim {string} is assessed against the current snapshot")]
fn assess_claim_step(world: &mut Forge, claim_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let claim: &VerificationClaim = snapshot.claim(&claim_id).expect("claim exists");
    world.claim_assessment = Some(assess_claim(claim, snapshot, &world.evidence_ledger));
}

#[then(expr = "the claim status is {string}")]
fn claim_status_is(world: &mut Forge, expected: String) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    assert_eq!(assessment.status.to_string(), expected, "{:?}", assessment.rationale);
}

#[then(expr = "evidence {string} has freshness {string}")]
fn evidence_freshness_is(world: &mut Forge, id: String, expected: String) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    let entry: &EvidenceEvaluationEntry = assessment
        .evaluated
        .iter()
        .find(|entry: &&EvidenceEvaluationEntry| entry.evidence_id == id)
        .expect("evidence in assessment");
    assert_eq!(entry.freshness.to_string(), expected);
}

#[then(expr = "the impact set names stale evidence {string} and affected claim {string}")]
fn impact_set_names(world: &mut Forge, evidence_id: String, claim_id: String) {
    let impact: &ImpactSet = world.impact.as_ref().expect("impact computed");
    assert!(impact.stale_evidence_ids.contains(&evidence_id), "{impact:?}");
    assert!(impact.affected_claim_ids.iter().any(|id: &ArtifactId| id.as_str() == claim_id), "{impact:?}");
}

#[then(expr = "the claim assessment reports {int} contradiction")]
fn claim_contradiction_count(world: &mut Forge, expected: usize) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    assert_eq!(assessment.contradictions.len(), expected);
}

#[then(expr = "the claim assessment rationale mentions {string}")]
fn claim_rationale_mentions(world: &mut Forge, needle: String) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    assert!(
        assessment.rationale.iter().any(|line: &String| line.contains(&needle)),
        "{:?}",
        assessment.rationale
    );
}

#[then("the claim assessment is not an unbounded proof")]
fn claim_not_unbounded_proof(world: &mut Forge) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    assert!(!assessment.unbounded_proof);
}

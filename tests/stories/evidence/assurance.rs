use std::collections::BTreeMap;

use cucumber::{given, then};
use specforge::assurance::{ClaimAssessment, ImpactSet};
use specforge::graph::{ArtifactId, ContentHash, DependencyPin, GraphSnapshot, VerificationMethod};
use specforge::ledger::{EvidencePayload, EvidenceRecord};
use specforge::verify::{EvaluationResult, RunState, VerificationOutcome};

use super::build_payload_with_scope;
use crate::graph::current_snapshot;
use crate::{listed, Forge};

fn pins_for_artifacts(snapshot: &GraphSnapshot, artifact_ids: &[String]) -> Vec<DependencyPin> {
    artifact_ids
        .iter()
        .map(|artifact_id: &String| {
            let digest: ContentHash = snapshot.artifact_digest(artifact_id).expect("artifact digest exists").clone();
            DependencyPin {
                artifact_id: artifact_id.clone(),
                digest,
            }
        })
        .collect()
}

fn standalone_payload(
    snapshot: &GraphSnapshot,
    id: &str,
    property_id: &str,
    dependencies: Vec<DependencyPin>,
    outcome: VerificationOutcome,
) -> EvidencePayload {
    let mut payload: EvidencePayload = build_payload_with_scope(
        snapshot,
        id,
        property_id,
        "CLM-MSG-001",
        "single-broker",
        VerificationMethod::PropertyBased,
        outcome,
        BTreeMap::new(),
    );
    payload.claim_ids.clear();
    payload.dependencies = dependencies;
    payload
}

#[given(expr = "a satisfied evidence record {string} without source revisions is appended for property {string} and claim {string}")]
fn append_satisfied_without_sources(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    let mut payload: EvidencePayload = build_payload_with_scope(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        "single-broker",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    payload.source_revisions.clear();
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("record seals");
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a satisfied evidence record {string} without a property is appended for claim {string}")]
fn append_satisfied_without_property(world: &mut Forge, id: String, claim_id: String) {
    let mut payload: EvidencePayload = build_payload_with_scope(
        current_snapshot(world),
        &id,
        "PROP-MSG-001",
        &claim_id,
        "single-broker",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    payload.property_id = None;
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("record seals");
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a failed violated evidence record {string} is appended for property {string} and claim {string}")]
fn append_failed_violated(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    let mut payload: EvidencePayload = build_payload_with_scope(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        "single-broker",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Violated),
        BTreeMap::new(),
    );
    payload.run_state = RunState::Failed;
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("record seals");
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a standalone satisfied evidence record {string} with property pins only is appended for property {string}")]
fn append_standalone_satisfied_prop_pins(world: &mut Forge, id: String, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let pins: Vec<DependencyPin> = snapshot.dependency_pins_for_property(&property_id);
    let payload: EvidencePayload = standalone_payload(snapshot, &id, &property_id, pins, VerificationOutcome::Evaluation(EvaluationResult::Satisfied));
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("record seals");
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a standalone violated evidence record {string} with property pins only is appended for property {string}")]
fn append_standalone_violated_prop_pins(world: &mut Forge, id: String, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let pins: Vec<DependencyPin> = snapshot.dependency_pins_for_property(&property_id);
    let payload: EvidencePayload = standalone_payload(snapshot, &id, &property_id, pins, VerificationOutcome::Evaluation(EvaluationResult::Violated));
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("record seals");
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a standalone satisfied evidence record {string} with claim pins for {string} is appended for property {string}")]
fn append_standalone_satisfied_claim_pins(world: &mut Forge, id: String, claim_id: String, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let pins: Vec<DependencyPin> = snapshot.dependency_pins_for_claim(&claim_id);
    let payload: EvidencePayload = standalone_payload(snapshot, &id, &property_id, pins, VerificationOutcome::Evaluation(EvaluationResult::Satisfied));
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("record seals");
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a satisfied evidence record {string} pinning only {string} is appended for property {string} and claim {string}")]
fn append_satisfied_pinning_only(world: &mut Forge, id: String, artifact_id: String, property_id: String, claim_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let mut payload: EvidencePayload = build_payload_with_scope(
        snapshot,
        &id,
        &property_id,
        &claim_id,
        "single-broker",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    payload.dependencies = pins_for_artifacts(snapshot, &[artifact_id]);
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("record seals");
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a standalone satisfied evidence record {string} pinning {string} is appended for property {string}")]
fn append_standalone_pinning(world: &mut Forge, id: String, artifacts_raw: String, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let ids: Vec<String> = listed(&artifacts_raw, ", ");
    let pins: Vec<DependencyPin> = pins_for_artifacts(snapshot, &ids);
    let payload: EvidencePayload = standalone_payload(snapshot, &id, &property_id, pins, VerificationOutcome::Evaluation(EvaluationResult::Satisfied));
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("record seals");
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[then(expr = "the claim assessment rationale does not mention {string}")]
fn claim_rationale_omits(world: &mut Forge, needle: String) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    assert!(
        !assessment.rationale.iter().any(|line: &String| line.contains(&needle)),
        "unexpected {needle:?} in {:?}",
        assessment.rationale
    );
}

#[then(expr = "the impact set names affected claim {string} with {int} stale evidence records")]
fn impact_set_claim_and_stale_count(world: &mut Forge, claim_id: String, expected_stale: usize) {
    let impact: &ImpactSet = world.impact.as_ref().expect("impact computed");
    assert_eq!(impact.stale_evidence_ids.len(), expected_stale, "{impact:?}");
    assert!(impact.affected_claim_ids.iter().any(|id: &ArtifactId| id.as_str() == claim_id), "{impact:?}");
}

#[then(expr = "the impact set names stale evidence {string} without affecting claim {string}")]
fn impact_set_stale_without_claim(world: &mut Forge, evidence_id: String, claim_id: String) {
    let impact: &ImpactSet = world.impact.as_ref().expect("impact computed");
    assert!(impact.stale_evidence_ids.contains(&evidence_id), "{impact:?}");
    assert!(!impact.affected_claim_ids.iter().any(|id: &ArtifactId| id.as_str() == claim_id), "{impact:?}");
}

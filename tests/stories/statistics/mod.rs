mod steps;

use std::collections::BTreeMap;

use specforge::graph::{ArtifactId, DependencyPin, GraphSnapshot, SourceRevision, VerificationMethod};
use specforge::ledger::{EvidenceKind, EvidencePayload, EvidenceRecord, ProducerMeta};
use specforge::statistics::{
    compute_qualified_estimate, ConfidenceLevel, QualifiedEstimate, SamplingModel, StatisticalError, StatisticalMethod, StatisticalSample,
};
use specforge::verify::{RunState, VerificationOutcome};

use crate::graph::current_snapshot;
use crate::Forge;

#[derive(Debug, Default)]
pub struct StatisticsState {
    pub last_result: Option<Result<QualifiedEstimate, StatisticalError>>,
}

pub(super) fn confidence_from_pct(pct: u8) -> ConfidenceLevel {
    match pct {
        90 => ConfidenceLevel::Ninety,
        95 => ConfidenceLevel::NinetyFive,
        99 => ConfidenceLevel::NinetyNine,
        other => panic!("unsupported confidence level percentage {other}"),
    }
}

pub(super) fn default_model(method: StatisticalMethod) -> SamplingModel {
    SamplingModel {
        hypothesis: "Acknowledged messages are not redelivered under random broker workloads".to_string(),
        sampling_population: "Single-broker randomized message delivery traces".to_string(),
        sampling_process: "Independent seeded trials drawn uniformly over bounded schedules".to_string(),
        independence_assumptions: vec![
            "Trials use independent pseudo-random seeds".to_string(),
            "Broker state is reset cleanly between trials".to_string(),
        ],
        scope: "single-broker".to_string(),
        method,
        limitations: vec!["Covers bounded message lengths under simulated schedules only".to_string()],
    }
}

pub(super) fn wilson_95_model() -> SamplingModel {
    default_model(StatisticalMethod::WilsonScore {
        confidence: ConfidenceLevel::NinetyFive,
    })
}

pub(super) fn sample_for(claim_id: &str, evidence_raw: &str, successes: usize, total: usize) -> StatisticalSample {
    let evidence_ids: Vec<String> = evidence_raw.split(',').map(|item: &str| item.trim().to_string()).collect();
    StatisticalSample {
        claim_id: ArtifactId::parse(claim_id).expect("valid claim id"),
        sample_size: total,
        observed_successes: successes,
        observed_failures: total.saturating_sub(successes),
        evidence_ids,
    }
}

pub(super) fn run_estimate(world: &mut Forge, model: &SamplingModel, sample: &StatisticalSample) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let result: Result<QualifiedEstimate, StatisticalError> = compute_qualified_estimate(snapshot, &world.evidence_ledger, model, sample);
    world.statistics.last_result = Some(result);
}

pub(super) fn require_estimate(world: &Forge) -> &QualifiedEstimate {
    match &world.statistics.last_result {
        Some(Ok(estimate)) => estimate,
        Some(Err(err)) => panic!("expected qualified estimate, got error: {err}"),
        None => panic!("no statistical estimate computed"),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn append_custom_record(
    world: &mut Forge,
    id: &str,
    property_id: &str,
    claim_id: &str,
    kind: EvidenceKind,
    method: VerificationMethod,
    run_state: RunState,
    outcome: VerificationOutcome,
    assumptions: Vec<ArtifactId>,
    source_revisions: Vec<SourceRevision>,
) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let prop_id: ArtifactId = ArtifactId::parse(property_id).expect("valid property id");
    let clm_id: ArtifactId = ArtifactId::parse(claim_id).expect("valid claim id");
    let dependencies: Vec<DependencyPin> = snapshot.dependency_pins_for_claim(claim_id);
    let payload: EvidencePayload = EvidencePayload {
        id: id.to_string(),
        kind,
        producer: ProducerMeta {
            name: "specforge-stat-engine".to_string(),
            version: "0.1.0".to_string(),
        },
        adapter: ProducerMeta {
            name: "specforge-stat-adapter".to_string(),
            version: "0.1.0".to_string(),
        },
        timestamp: "2026-11-24T12:00:00Z".to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        source_revisions,
        dependencies,
        run_id: format!("RUN-{id}"),
        activity_id: "ACT-STAT-001".to_string(),
        method,
        property_id: Some(prop_id),
        claim_ids: vec![clm_id],
        scope: "single-broker".to_string(),
        assumptions,
        run_state,
        outcome,
        counterexample_id: None,
        replay: None,
        artifact_digests: BTreeMap::new(),
        limitations: vec!["finite sampled execution".to_string()],
    };
    let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("evidence record seals");
    world.evidence_ledger.append_record(record).expect("record appends");
}

#[allow(clippy::too_many_arguments)]
pub(super) fn append_record(
    world: &mut Forge,
    id: &str,
    property_id: &str,
    claim_id: &str,
    kind: EvidenceKind,
    method: VerificationMethod,
    run_state: RunState,
    outcome: VerificationOutcome,
) {
    let revisions: Vec<SourceRevision> = current_snapshot(world).sources.clone();
    let assumptions: Vec<ArtifactId> = vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption id")];
    append_custom_record(world, id, property_id, claim_id, kind, method, run_state, outcome, assumptions, revisions);
}

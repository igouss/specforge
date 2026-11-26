use std::collections::BTreeMap;

use cucumber::{then, when};
use specforge::assurance::{assess_claim, ClaimAssessment};
use specforge::corpus::{replay_corpus, CorpusReplaySummary, CounterexampleRegistry};
use specforge::graph::{ArtifactId, GraphSnapshot, VerificationClaim, VerificationMethod};
use specforge::ledger::{EvidenceKind, EvidenceLedger, EvidencePayload, EvidenceRecord, ProducerMeta};
use specforge::planner::{recommend_activities, ExplorationRecommendation};
use specforge::proptest_adapter::ProptestAdapter;
use specforge::verify::{ClaimStatus, Counterexample, EvaluationResult, RunState, VerificationOutcome};

use super::planner::build_active_counterexample;
use super::safe_runner;
use crate::graph::current_snapshot;
use crate::Forge;

#[when(expr = "multiple assessments with different statuses for {string} are planned")]
fn plan_multi_status_assessments(world: &mut Forge, claim_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let claim: &VerificationClaim = snapshot.claim(&claim_id).expect("claim exists");
    let empty_ledger: EvidenceLedger = EvidenceLedger::new();
    let mut inconclusive: ClaimAssessment = assess_claim(claim, &snapshot, &empty_ledger);
    inconclusive.status = ClaimStatus::Inconclusive;
    inconclusive.rationale = vec!["z-inconclusive".to_string()];
    let mut stale: ClaimAssessment = inconclusive.clone();
    stale.status = ClaimStatus::Stale;
    stale.rationale = vec!["a-stale".to_string()];
    let assessments: Vec<ClaimAssessment> = vec![stale, inconclusive];
    let empty_registry: CounterexampleRegistry = CounterexampleRegistry::new();
    world.exploration.recommendations = recommend_activities(&snapshot, &empty_ledger, &[], &assessments, &empty_registry);
}

#[then(expr = "recommendation {int} has id {string} and reason {string}")]
fn recommendation_at_index(world: &mut Forge, one_based: usize, expected_id: String, expected_reason: String) {
    let rec: &ExplorationRecommendation = world.exploration.recommendations.get(one_based - 1).expect("recommendation exists at index");
    assert_eq!(rec.id, expected_id, "unexpected recommendation id in {:?}", world.exploration.recommendations);
    assert_eq!(
        rec.reason.to_string(),
        expected_reason,
        "unexpected recommendation reason in {:?}",
        world.exploration.recommendations
    );
}

fn seal_replay_evidence(
    snapshot: &GraphSnapshot,
    id: &str,
    activity_id: &str,
    property_id: &str,
    counterexample_id: Option<String>,
    result: EvaluationResult,
) -> EvidenceRecord {
    let payload: EvidencePayload = EvidencePayload {
        id: id.to_string(),
        kind: EvidenceKind::Replay,
        producer: ProducerMeta {
            name: "specforge-test-engine".to_string(),
            version: "0.1.0".to_string(),
        },
        adapter: ProducerMeta {
            name: "specforge-test-adapter".to_string(),
            version: "0.1.0".to_string(),
        },
        timestamp: "2026-11-24T06:50:00Z".to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        source_revisions: snapshot.sources.clone(),
        dependencies: snapshot.dependency_pins_for_property(property_id),
        run_id: format!("RUN-{id}"),
        activity_id: activity_id.to_string(),
        method: VerificationMethod::PropertyBased,
        property_id: Some(ArtifactId::parse(property_id).expect("valid property id")),
        claim_ids: Vec::new(),
        scope: "single-broker".to_string(),
        assumptions: vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption id")],
        run_state: RunState::Completed,
        outcome: VerificationOutcome::Evaluation(result),
        counterexample_id,
        replay: None,
        artifact_digests: BTreeMap::new(),
        limitations: Vec::new(),
    };
    EvidenceRecord::seal(payload).expect("replay evidence seals")
}

#[when("a registered counterexample has a failing replay followed by a passing replay and is planned")]
fn plan_conflicting_replays(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let mut registry: CounterexampleRegistry = CounterexampleRegistry::new();
    let ce: Counterexample = build_active_counterexample(&snapshot, "RUN-PLAN-CONF");
    registry.register_original(ce).expect("ce registered");
    let mut ledger: EvidenceLedger = EvidenceLedger::new();
    let buggy_adapter: ProptestAdapter = ProptestAdapter::for_machine(12);
    let _fail: CorpusReplaySummary =
        replay_corpus(&registry, &snapshot, &buggy_adapter, &mut ledger, "RUN-CONF-FAIL", "2026-11-24T06:51:00Z").expect("failing replay executes");
    let safe_adapter: ProptestAdapter = ProptestAdapter::with_runner(safe_runner, 12);
    let _pass: CorpusReplaySummary =
        replay_corpus(&registry, &snapshot, &safe_adapter, &mut ledger, "RUN-CONF-PASS", "2026-11-24T06:52:00Z").expect("passing replay executes");
    world.exploration.recommendations = recommend_activities(&snapshot, &ledger, &[], &[], &registry);
}

#[when("a registered counterexample has only an inconclusive replay and is planned")]
fn plan_inconclusive_replay(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let mut registry: CounterexampleRegistry = CounterexampleRegistry::new();
    let ce: Counterexample = build_active_counterexample(&snapshot, "RUN-PLAN-INC");
    let activity: String = format!("replay-{}", ce.id);
    let ce_id: String = ce.id.clone();
    registry.register_original(ce).expect("ce registered");
    let mut ledger: EvidenceLedger = EvidenceLedger::new();
    let record: EvidenceRecord = seal_replay_evidence(&snapshot, "EV-REP-INC", &activity, "PROP-MSG-001", Some(ce_id), EvaluationResult::Inconclusive);
    ledger.append_record(record).expect("inconclusive replay appends");
    world.exploration.recommendations = recommend_activities(&snapshot, &ledger, &[], &[], &registry);
}

#[when(expr = "a registered counterexample has only a passing replay for a different counterexample on {string} and is planned")]
fn plan_unrelated_ce_replay(world: &mut Forge, property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let mut registry: CounterexampleRegistry = CounterexampleRegistry::new();
    let ce: Counterexample = build_active_counterexample(&snapshot, "RUN-PLAN-UNREL");
    registry.register_original(ce).expect("ce registered");
    let mut ledger: EvidenceLedger = EvidenceLedger::new();
    let record: EvidenceRecord = seal_replay_evidence(
        &snapshot,
        "EV-REP-OTHER",
        "replay-CE-OTHER-999",
        &property_id,
        Some("CE-OTHER-999".to_string()),
        EvaluationResult::Satisfied,
    );
    ledger.append_record(record).expect("unrelated replay appends");
    world.exploration.recommendations = recommend_activities(&snapshot, &ledger, &[], &[], &registry);
}

use std::collections::BTreeMap;

use cucumber::{then, when};
use specforge::assurance::{assess_claim, ClaimAssessment};
use specforge::corpus::{replay_corpus, CorpusReplaySummary};
use specforge::coverage::{compute_coverage, CoverageRecord};
use specforge::graph::{ArtifactId, ContentHash, GraphSnapshot, Property, StateMachine, VerificationClaim, VerificationMethod};
use specforge::ledger::{EvidenceKind, EvidenceLedger, EvidencePayload, EvidenceRecord, ProducerMeta};
use specforge::model::{evaluate_property, execute_machine};
use specforge::planner::{recommend_activities, ExplorationRecommendation};
use specforge::proptest_adapter::ProptestAdapter;
use specforge::verify::{Counterexample, EvaluationResult, ExecutionTrace, FindingRecord, PropertyEvaluation, ReplayDescriptor, RunState, VerificationOutcome};

use super::safe_runner;
use crate::graph::current_snapshot;
use crate::Forge;

fn method_label(method: VerificationMethod) -> &'static str {
    match method {
        VerificationMethod::ExampleBased => "example_based",
        VerificationMethod::ModelBased => "model_based",
        VerificationMethod::PropertyBased => "property_based",
        VerificationMethod::DeterministicSimulation => "deterministic_simulation",
        VerificationMethod::Formal => "formal",
    }
}

pub(super) fn build_active_counterexample(snapshot: &GraphSnapshot, run_id: &str) -> Counterexample {
    let machine: &StateMachine = snapshot.state_machine("SM-MSG-001").expect("machine exists");
    let property: &Property = snapshot.property("PROP-MSG-001").expect("property exists");
    let digest: &ContentHash = snapshot.artifact_digest("PROP-MSG-001").expect("digest exists");
    let bad_events: Vec<String> = vec!["deliver".to_string(), "ack".to_string(), "redeliver".to_string()];
    let bad_trace: ExecutionTrace = execute_machine(machine, &bad_events);
    let replay: ReplayDescriptor = ReplayDescriptor {
        engine: "proptest".to_string(),
        engine_version: "1".to_string(),
        seed: Some(99),
        replay_token: Some("chacha-seed:99".to_string()),
        events: bad_events,
        determinism: "deterministic replay".to_string(),
        replayable: true,
    };
    let (_, _, ce): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) =
        evaluate_property(property, digest, &bad_trace, run_id, &snapshot.snapshot_id, replay);
    ce.expect("violation produces ce")
}

#[when("coverage gaps, stale claim assessment, and an active counterexample are planned")]
fn plan_gaps_stale_and_ce(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let machine: &StateMachine = snapshot.state_machine("SM-MSG-001").expect("machine exists");
    let safe_events: Vec<String> = vec!["deliver".to_string(), "ack".to_string()];
    let safe_trace: ExecutionTrace = execute_machine(machine, &safe_events);
    let coverage: CoverageRecord = compute_coverage(&snapshot, "SM-MSG-001", &[safe_trace]).expect("coverage computed");

    let claim: &VerificationClaim = snapshot.claim("CLM-MSG-001").expect("claim exists");
    let assessment: ClaimAssessment = assess_claim(claim, &snapshot, &world.evidence_ledger);

    let ce: Counterexample = build_active_counterexample(&snapshot, "RUN-PLAN-CE");
    world.exploration.registry.register_original(ce).expect("ce registered");

    world.exploration.snapshot_digest_before_plan = Some(snapshot.snapshot_id.clone());
    world.exploration.ledger_count_before_plan = Some(world.evidence_ledger.records().len());
    world.exploration.ledger_json_before_plan = Some(world.evidence_ledger.export_json().expect("ledger exports"));

    let coverage_slice: Vec<CoverageRecord> = vec![coverage];
    let assessment_slice: Vec<ClaimAssessment> = vec![assessment];
    world.exploration.recommendations = recommend_activities(
        &snapshot,
        &world.evidence_ledger,
        &coverage_slice,
        &assessment_slice,
        &world.exploration.registry,
    );
}

#[then(expr = "the planner recommends {string} via {string} targeting {string} naming {string}")]
fn planner_recommends_full(world: &mut Forge, reason: String, method: String, target: String, item: String) {
    let matched: bool = world.exploration.recommendations.iter().any(|rec: &ExplorationRecommendation| {
        rec.reason.to_string() == reason
            && method_label(rec.method) == method
            && rec.target_id.as_str() == target
            && rec.items.iter().any(|entry: &String| entry == &item)
            && !rec.rationale.is_empty()
    });
    assert!(
        matched,
        "missing recommendation ({reason}, {method}, {target}, {item}) in {:?}",
        world.exploration.recommendations
    );
}

#[then(expr = "the planner recommends {string} via {string} targeting {string}")]
fn planner_recommends_method(world: &mut Forge, reason: String, method: String, target: String) {
    let matched: bool = world.exploration.recommendations.iter().any(|rec: &ExplorationRecommendation| {
        rec.reason.to_string() == reason && method_label(rec.method) == method && rec.target_id.as_str() == target && !rec.rationale.is_empty()
    });
    assert!(
        matched,
        "missing recommendation ({reason}, {method}, {target}) in {:?}",
        world.exploration.recommendations
    );
}

#[then(expr = "the planner recommends {string} targeting {string} naming {string}")]
fn planner_recommends_naming(world: &mut Forge, reason: String, target: String, item: String) {
    let matched: bool = world.exploration.recommendations.iter().any(|rec: &ExplorationRecommendation| {
        rec.reason.to_string() == reason
            && rec.target_id.as_str() == target
            && rec.items.iter().any(|entry: &String| entry == &item)
            && !rec.rationale.is_empty()
    });
    assert!(
        matched,
        "missing recommendation ({reason}, {target}, {item}) in {:?}",
        world.exploration.recommendations
    );
}

#[then(expr = "the planner recommends {string} targeting {string}")]
fn planner_recommends_target(world: &mut Forge, reason: String, target: String) {
    let matched: bool = world
        .exploration
        .recommendations
        .iter()
        .any(|rec: &ExplorationRecommendation| rec.reason.to_string() == reason && rec.target_id.as_str() == target && !rec.rationale.is_empty());
    assert!(
        matched,
        "missing recommendation ({reason}, {target}) in {:?}",
        world.exploration.recommendations
    );
}

#[then(expr = "the planner emits no {string} recommendation")]
fn planner_emits_no(world: &mut Forge, reason: String) {
    let found: bool = world
        .exploration
        .recommendations
        .iter()
        .any(|rec: &ExplorationRecommendation| rec.reason.to_string() == reason);
    assert!(!found, "unexpected recommendation {reason} in {:?}", world.exploration.recommendations);
}

#[then("planning leaves the snapshot digest and ledger unchanged")]
fn planning_read_only(world: &mut Forge) {
    let before_digest: &ContentHash = world.exploration.snapshot_digest_before_plan.as_ref().expect("before digest saved");
    assert_eq!(&current_snapshot(world).snapshot_id, before_digest);
    assert_eq!(Some(world.evidence_ledger.records().len()), world.exploration.ledger_count_before_plan);
    let after_json: String = world.evidence_ledger.export_json().expect("ledger exports");
    assert_eq!(Some(after_json), world.exploration.ledger_json_before_plan);
}

#[when("an unverified claim is planned")]
fn plan_unverified(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let claim: &VerificationClaim = snapshot.claim("CLM-MSG-001").expect("claim exists");
    let assessment: ClaimAssessment = assess_claim(claim, &snapshot, &world.evidence_ledger);
    world.exploration.recommendations = recommend_activities(&snapshot, &world.evidence_ledger, &[], &[assessment], &world.exploration.registry);
}

fn seal_claim_evidence(snapshot: &GraphSnapshot, id: &str, claim_id: &str, result: EvaluationResult) -> EvidenceRecord {
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
        timestamp: "2026-11-24T06:45:00Z".to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        source_revisions: snapshot.sources.clone(),
        dependencies: snapshot.dependency_pins_for_claim(claim_id),
        run_id: format!("RUN-{id}"),
        activity_id: "ACT-PLAN-001".to_string(),
        method: VerificationMethod::PropertyBased,
        property_id: Some(ArtifactId::parse("PROP-MSG-001").expect("valid property id")),
        claim_ids: vec![ArtifactId::parse(claim_id).expect("valid claim id")],
        scope: "single-broker".to_string(),
        assumptions: vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption id")],
        run_state: RunState::Completed,
        outcome: VerificationOutcome::Evaluation(result),
        counterexample_id: None,
        replay: None,
        artifact_digests: BTreeMap::new(),
        limitations: Vec::new(),
    };
    EvidenceRecord::seal(payload).expect("evidence seals")
}

#[when(expr = "contradictory current evidence exists for {string} and is planned")]
fn plan_contradictory(world: &mut Forge, claim_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let mut ledger: EvidenceLedger = EvidenceLedger::new();
    ledger
        .append_record(seal_claim_evidence(&snapshot, "EV-CON-1", &claim_id, EvaluationResult::Satisfied))
        .expect("ev1 appended");
    ledger
        .append_record(seal_claim_evidence(&snapshot, "EV-CON-2", &claim_id, EvaluationResult::Violated))
        .expect("ev2 appended");
    let claim: &VerificationClaim = snapshot.claim(&claim_id).expect("claim exists");
    let assessment: ClaimAssessment = assess_claim(claim, &snapshot, &ledger);
    world.evidence_ledger = ledger;
    let assessments: Vec<ClaimAssessment> = vec![assessment];
    world.exploration.recommendations = recommend_activities(&snapshot, &world.evidence_ledger, &[], &assessments, &world.exploration.registry);
    world.exploration.second_recommendations = recommend_activities(&snapshot, &world.evidence_ledger, &[], &assessments, &world.exploration.registry);
}

#[then("planning is deterministic across repeated calls")]
fn planning_deterministic(world: &mut Forge) {
    assert_eq!(world.exploration.recommendations, world.exploration.second_recommendations);
}

#[when("a registered counterexample is resolved by a current passing replay and planned")]
fn plan_resolved_counterexample(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let ce: Counterexample = build_active_counterexample(&snapshot, "RUN-PLAN-RES");
    world.exploration.registry.register_original(ce).expect("ce registered");
    let adapter: ProptestAdapter = ProptestAdapter::with_runner(safe_runner, 12);
    let _summary: CorpusReplaySummary = replay_corpus(
        &world.exploration.registry,
        &snapshot,
        &adapter,
        &mut world.evidence_ledger,
        "RUN-RES-PASS",
        "2026-11-24T06:48:00Z",
    )
    .expect("passing replay succeeds");
    world.exploration.recommendations = recommend_activities(&snapshot, &world.evidence_ledger, &[], &[], &world.exploration.registry);
}

#[when("a subsequent failing replay occurs for that counterexample and is planned")]
fn plan_regressed_counterexample(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let adapter: ProptestAdapter = ProptestAdapter::for_machine(12);
    let _summary: CorpusReplaySummary = replay_corpus(
        &world.exploration.registry,
        &snapshot,
        &adapter,
        &mut world.evidence_ledger,
        "RUN-RES-REG",
        "2026-11-24T06:49:00Z",
    )
    .expect("failing replay executes");
    world.exploration.recommendations = recommend_activities(&snapshot, &world.evidence_ledger, &[], &[], &world.exploration.registry);
}

#[when("a stale coverage record with a mismatched model digest is planned")]
fn plan_stale_coverage(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let machine: &StateMachine = snapshot.state_machine("SM-MSG-001").expect("machine exists");
    let safe_events: Vec<String> = vec!["deliver".to_string(), "ack".to_string()];
    let safe_trace: ExecutionTrace = execute_machine(machine, &safe_events);
    let mut coverage: CoverageRecord = compute_coverage(&snapshot, "SM-MSG-001", &[safe_trace]).expect("coverage computed");
    coverage.model_digest = ContentHash::of_bytes(b"stale-machine-digest");
    world.exploration.recommendations = recommend_activities(&snapshot, &world.evidence_ledger, &[coverage], &[], &world.exploration.registry);
}

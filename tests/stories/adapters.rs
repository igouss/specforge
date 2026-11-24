use std::cell::Cell;
use std::collections::BTreeMap;

use cucumber::{given, then, when};
use specforge::assurance::{assess_claim, ClaimAssessment};
use specforge::graph::{ArtifactId, ContentHash, GraphSnapshot, Property, StateMachine, VerificationClaim};
use specforge::ledger::{EvidenceKind, EvidenceRecord};
use specforge::model::{evaluate_property, execute_machine, generate_scenarios, GeneratedScenario};
use specforge::proptest_adapter::ProptestAdapter;
use specforge::stories::{stories_with_evidence, Outcome, Project};
use specforge::verify::{
    AdapterError, Counterexample, ExecutionTrace, FindingRecord, PropertyEvaluation, ReplayDescriptor, RootCause, RunEnvelope, TraceEvent, VerificationAdapter,
    VerificationDecision, VerificationOutcome, VerificationRequest,
};

use crate::graph::current_snapshot;
use crate::ledger::{Ledger, RAN};
use crate::{listed, Forge};

fn safe_broker_runner(machine: &StateMachine, events: &[String]) -> ExecutionTrace {
    let mut current: String = machine.initial.clone();
    let mut trace_events: Vec<TraceEvent> = Vec::new();
    for (index, event_name) in events.iter().enumerate() {
        let next: String = match (current.as_str(), event_name.as_str()) {
            ("queued", "deliver") => "delivered".to_string(),
            ("delivered", "ack") => "acknowledged".to_string(),
            _ => current.clone(),
        };
        trace_events.push(TraceEvent {
            step: index + 1,
            event: event_name.clone(),
            from_state: current,
            to_state: next.clone(),
            inputs: BTreeMap::new(),
            fault: None,
            virtual_time: Some((index + 1) as u64),
        });
        current = next;
    }
    ExecutionTrace {
        initial_state: machine.initial.clone(),
        events: trace_events,
    }
}

fn buggy_broker_runner(machine: &StateMachine, events: &[String]) -> ExecutionTrace {
    execute_machine(machine, events)
}

#[when(expr = "scenarios are generated from state machine {string}")]
fn gen_scenarios(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    world.generated_scenarios = generate_scenarios(snapshot, &machine_id).expect("scenarios generated");
}

#[then(expr = "at least {int} scenarios are generated from {string}")]
fn at_least_scenarios(world: &mut Forge, min_count: usize, machine_id: String) {
    assert!(world.generated_scenarios.len() >= min_count, "{:?}", world.generated_scenarios);
    for scenario in &world.generated_scenarios {
        assert_eq!(scenario.model_id.as_str(), machine_id);
    }
}

#[then("every generated scenario records its strategy and source model digest")]
fn scenarios_record_provenance(world: &mut Forge) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    for scenario in &world.generated_scenarios {
        assert!(!scenario.strategy.is_empty());
        assert_eq!(Some(&scenario.model_digest), snapshot.artifact_digest(scenario.model_id.as_str()));
    }
}

#[then(expr = "the generated scenarios cover transitions {string}")]
fn scenarios_cover_transitions(world: &mut Forge, names: String) {
    let covered: Vec<String> = world
        .generated_scenarios
        .iter()
        .flat_map(|scenario: &GeneratedScenario| scenario.covered_transitions.clone())
        .collect();
    for expected in listed(&names, ", ") {
        assert!(covered.contains(&expected), "missing transition {expected} in {covered:?}");
    }
}

fn eval_on_events(world: &mut Forge, property_id: &str, events: &[&str]) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let property: &Property = snapshot.property(property_id).expect("property exists");
    let digest: &ContentHash = snapshot.artifact_digest(property_id).expect("digest exists");
    let machine: &StateMachine = snapshot.state_machine(property.subject.as_str()).expect("machine exists");
    let event_list: Vec<String> = events.iter().map(|s: &&str| (*s).to_string()).collect();
    let trace: ExecutionTrace = execute_machine(machine, &event_list);
    let replay: ReplayDescriptor = ReplayDescriptor {
        engine: "specforge-model".to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        seed: None,
        replay_token: None,
        events: event_list,
        determinism: "deterministic state-machine trace".to_string(),
        replayable: true,
    };
    let snapshot_id: ContentHash = snapshot.snapshot_id.clone();
    let (evaluation, finding, counterexample): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) =
        evaluate_property(property, digest, &trace, "RUN-EVAL-001", &snapshot_id, replay);
    world.evaluation = Some(evaluation);
    world.finding = finding;
    world.counterexample = counterexample;
    world.snapshot_digest_before_decision = Some(snapshot_id);
}

#[when(expr = "property {string} is evaluated on the safe message trace")]
fn eval_safe_trace(world: &mut Forge, property_id: String) {
    eval_on_events(world, &property_id, &["deliver", "ack"]);
}

#[when(expr = "property {string} is evaluated on the redelivery violation trace")]
fn eval_violation_trace(world: &mut Forge, property_id: String) {
    eval_on_events(world, &property_id, &["deliver", "ack", "redeliver"]);
}

#[then(expr = "the evaluation result is {string}")]
fn eval_result_is(world: &mut Forge, expected: String) {
    let evaluation: &PropertyEvaluation = world.evaluation.as_ref().expect("evaluated");
    assert_eq!(evaluation.result.to_string(), expected);
}

#[then("no finding is created")]
fn no_finding(world: &mut Forge) {
    assert!(world.finding.is_none());
}

#[then(expr = "a finding is created with root cause {string}")]
fn finding_root_cause(world: &mut Forge, expected: String) {
    let finding: &FindingRecord = world.finding.as_ref().expect("finding created");
    assert_eq!(finding.root_cause.to_string(), expected);
}

#[then(expr = "a counterexample is captured at step {int} referencing {string}")]
fn counterexample_captured(world: &mut Forge, step_index: usize, property_id: String) {
    let ce: &Counterexample = world.counterexample.as_ref().expect("counterexample captured");
    assert_eq!(ce.violation.step, step_index);
    assert_eq!(ce.property_id.as_str(), property_id);
}

#[when(expr = "a verification decision disposes that finding as {string} by {string}")]
fn dispose_finding(world: &mut Forge, diagnosis: String, approver: String) {
    let finding: &FindingRecord = world.finding.as_ref().expect("finding exists");
    let root_cause: RootCause = match diagnosis.as_str() {
        "implementation" => RootCause::Implementation,
        "specification" => RootCause::Specification,
        "behavioral_model" => RootCause::BehavioralModel,
        "environment_or_assumption" => RootCause::EnvironmentOrAssumption,
        "verification" => RootCause::Verification,
        "expected_behavior" => RootCause::ExpectedBehavior,
        _ => RootCause::Unresolved,
    };
    let decision: VerificationDecision = VerificationDecision::dispose(
        "DEC-001".to_string(),
        finding,
        Vec::new(),
        root_cause,
        "broker failed to persist acknowledgement before redelivery".to_string(),
        "fix_implementation".to_string(),
        approver,
        None,
    )
    .expect("decision is valid");
    world.decision = Some(decision);
}

#[then(expr = "the decision records diagnosis {string} and approver {string}")]
fn decision_records(world: &mut Forge, diagnosis: String, approver: String) {
    let decision: &VerificationDecision = world.decision.as_ref().expect("decision created");
    assert_eq!(decision.diagnosis.to_string(), diagnosis);
    assert_eq!(decision.approver, approver);
}

#[then("the snapshot digest is unchanged")]
fn snapshot_digest_unchanged(world: &mut Forge) {
    let before: &ContentHash = world.snapshot_digest_before_decision.as_ref().expect("before recorded");
    assert_eq!(&current_snapshot(world).snapshot_id, before);
}

fn run_proptest(world: &mut Forge, property_id: &str, claim_id: &str, seed: u64, budget: usize, safe: bool) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let adapter: ProptestAdapter = if safe {
        ProptestAdapter::with_runner(safe_broker_runner, 8)
    } else {
        ProptestAdapter::with_runner(buggy_broker_runner, 12)
    };
    let request: VerificationRequest = VerificationRequest {
        run_id: format!("RUN-PBT-{seed}-{budget}"),
        activity_id: "ACT-PBT-001".to_string(),
        property_id: ArtifactId::parse(property_id).expect("valid property id"),
        claim_ids: vec![ArtifactId::parse(claim_id).expect("valid claim id")],
        budget,
        seed: Some(seed),
    };
    let envelope: RunEnvelope = adapter.execute(&snapshot, &request).expect("proptest adapter executes");
    let record: EvidenceRecord = EvidenceRecord::from_envelope(
        format!("EV-PBT-{}", world.evidence_ledger.records().len() + 1),
        EvidenceKind::PropertyEvaluation,
        "2026-11-23T20:05:00Z".to_string(),
        &snapshot,
        &envelope,
        BTreeMap::new(),
    )
    .expect("evidence sealed");
    world.evidence_ledger.append_record(record).expect("evidence appended");
    world.original_counterexample_snapshot = envelope.original_counterexample.clone();
    world.envelope = Some(envelope);
}

#[when(expr = "the proptest adapter verifies {string} for claim {string} with seed {int} against the safe message broker")]
fn proptest_safe(world: &mut Forge, property_id: String, claim_id: String, seed: u64) {
    run_proptest(world, &property_id, &claim_id, seed, 32, true);
}

#[given(expr = "the proptest adapter verifies {string} for claim {string} with seed {int} against the buggy redelivering broker")]
#[when(expr = "the proptest adapter verifies {string} for claim {string} with seed {int} against the buggy redelivering broker")]
fn proptest_buggy(world: &mut Forge, property_id: String, claim_id: String, seed: u64) {
    run_proptest(world, &property_id, &claim_id, seed, 128, false);
}

#[when(expr = "the proptest adapter runs {string} for claim {string} with zero case budget")]
fn proptest_zero_budget(world: &mut Forge, property_id: String, claim_id: String) {
    run_proptest(world, &property_id, &claim_id, 42, 0, true);
}

#[then(expr = "the adapter run result is {string}")]
fn adapter_run_result(world: &mut Forge, expected: String) {
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("adapter ran");
    match &envelope.outcome {
        VerificationOutcome::Evaluation(result) => assert_eq!(result.to_string(), expected),
        other => panic!("expected evaluation outcome, got {other:?}"),
    }
}

#[then(expr = "assessing claim {string} against the current snapshot yields {string}")]
fn assess_yields(world: &mut Forge, claim_id: String, expected: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let claim: &VerificationClaim = snapshot.claim(&claim_id).expect("claim exists");
    let assessment: ClaimAssessment = assess_claim(claim, snapshot, &world.evidence_ledger);
    assert_eq!(assessment.status.to_string(), expected, "{:?}", assessment.rationale);
    world.claim_assessment = Some(assessment);
}

#[then("a minimized counterexample is linked to its original counterexample")]
fn minimized_linked(world: &mut Forge) {
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("adapter ran");
    let original: &Counterexample = envelope.original_counterexample.as_ref().expect("original counterexample");
    let minimized: &Counterexample = envelope.minimized_counterexample.as_ref().expect("minimized counterexample");
    assert_eq!(minimized.original_id.as_deref(), Some(original.id.as_str()));
    assert!(minimized.minimization.is_some());
}

#[then("the minimized counterexample is no longer than the original")]
fn minimized_no_longer(world: &mut Forge) {
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("adapter ran");
    let original: &Counterexample = envelope.original_counterexample.as_ref().expect("original counterexample");
    let minimized: &Counterexample = envelope.minimized_counterexample.as_ref().expect("minimized counterexample");
    assert!(minimized.replay.events.len() <= original.replay.events.len());
}

#[then(expr = "the counterexample replay descriptor is replayable with seed {int}")]
fn replay_descriptor_seed(world: &mut Forge, seed: u64) {
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("adapter ran");
    let minimized: &Counterexample = envelope.minimized_counterexample.as_ref().expect("minimized counterexample");
    assert!(minimized.replay.replayable);
    assert_eq!(minimized.replay.seed, Some(seed));
}

#[when("the minimized counterexample is replayed against the buggy redelivering broker")]
fn replay_minimized(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let minimized: Counterexample = world
        .envelope
        .as_ref()
        .expect("adapter ran")
        .minimized_counterexample
        .clone()
        .expect("minimized counterexample");
    let adapter: ProptestAdapter = ProptestAdapter::with_runner(buggy_broker_runner, 12);
    let replay_envelope: RunEnvelope = adapter.replay(&snapshot, &minimized, "RUN-REPLAY-001").expect("replay executes");
    let record: EvidenceRecord = EvidenceRecord::from_envelope(
        format!("EV-REPLAY-{}", world.evidence_ledger.records().len() + 1),
        EvidenceKind::Replay,
        "2026-11-23T20:06:00Z".to_string(),
        &snapshot,
        &replay_envelope,
        BTreeMap::new(),
    )
    .expect("replay evidence sealed");
    world.evidence_ledger.append_record(record).expect("replay evidence appended");
    world.replay_envelope = Some(replay_envelope);
}

#[when("the counterexample is stripped of replayability and replayed")]
fn replay_non_replayable(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let mut counterexample: Counterexample = world
        .envelope
        .as_ref()
        .expect("adapter ran")
        .minimized_counterexample
        .clone()
        .expect("minimized counterexample");
    counterexample.replay.replayable = false;
    let adapter: ProptestAdapter = ProptestAdapter::with_runner(buggy_broker_runner, 12);
    world.replay_result = Some(adapter.replay(&snapshot, &counterexample, "RUN-REPLAY-BAD-001"));
}

#[then("replay is rejected as not replayable")]
fn replay_rejected_not_replayable(world: &mut Forge) {
    assert!(
        matches!(world.replay_result, Some(Err(AdapterError::NotReplayable(_)))),
        "{:?}",
        world.replay_result
    );
}

#[then(expr = "the replay reproduces {string}")]
fn replay_reproduces(world: &mut Forge, expected: String) {
    let envelope: &RunEnvelope = world.replay_envelope.as_ref().expect("replay ran");
    match &envelope.outcome {
        VerificationOutcome::Evaluation(result) => assert_eq!(result.to_string(), expected),
        other => panic!("expected evaluation outcome, got {other:?}"),
    }
}

#[then("the original counterexample is unchanged")]
fn original_ce_unchanged(world: &mut Forge) {
    let before: &Counterexample = world.original_counterexample_snapshot.as_ref().expect("before saved");
    let after: &Counterexample = world
        .envelope
        .as_ref()
        .expect("envelope exists")
        .original_counterexample
        .as_ref()
        .expect("original exists");
    assert_eq!(before, after);
}

#[when("the cucumber adapter verifies the project and records evidence")]
async fn cucumber_adapter_verifies(world: &mut Forge) {
    let project: Project = world.project.clone().expect("project given");
    let (outcome, record): (Outcome, EvidenceRecord) = RAN
        .scope(Cell::new(0), async {
            stories_with_evidence::<Ledger>(
                &project,
                "EV-CUCUMBER-001".to_string(),
                "RUN-CUCUMBER-001".to_string(),
                "2026-11-23T20:10:00Z".to_string(),
                Vec::new(),
                &mut world.evidence_ledger,
            )
            .await
        })
        .await
        .expect("cucumber adapter runs and records evidence");
    world.outcome = Some(outcome);
    world.cucumber_record = Some(record);
}

#[then(expr = "the cucumber evidence record has evaluation result {string}")]
fn cucumber_evidence_result(world: &mut Forge, expected: String) {
    let record: &EvidenceRecord = world.cucumber_record.as_ref().expect("cucumber record exists");
    match &record.outcome {
        VerificationOutcome::Evaluation(result) => assert_eq!(result.to_string(), expected),
        other => panic!("expected evaluation outcome, got {other:?}"),
    }
}

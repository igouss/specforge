use std::collections::BTreeMap;

use cucumber::{then, when};
use specforge::graph::{ContentHash, GraphSnapshot, Property, StateMachine, Transition};
use specforge::model::{evaluate_property, execute_machine, generate_scenarios, GeneratedScenario};
use specforge::verify::{
    AdapterError, Counterexample, ExecutionTrace, FindingRecord, PropertyEvaluation, ReplayDescriptor, RootCause, TraceEvent, VerificationDecision,
};

use crate::graph::current_snapshot;
use crate::{listed, Forge};

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

#[then(expr = "the generated scenarios are numbered {string}")]
fn scenarios_numbered(world: &mut Forge, expected: String) {
    let actual: Vec<String> = world
        .generated_scenarios
        .iter()
        .map(|scenario: &GeneratedScenario| scenario.id.clone())
        .collect();
    assert_eq!(actual, listed(&expected, ", "));
}

#[then(expr = "every generated scenario follows valid transitions of {string}")]
fn scenarios_follow_transitions(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let machine: &StateMachine = snapshot.state_machine(&machine_id).expect("machine exists");
    assert!(!world.generated_scenarios.is_empty());
    for scenario in &world.generated_scenarios {
        assert_eq!(scenario.covered_states.first(), Some(&machine.initial));
        assert_eq!(scenario.covered_states.len(), scenario.events.len() + 1);
        assert_eq!(scenario.covered_transitions.len(), scenario.events.len());
        for (index, event_name) in scenario.events.iter().enumerate() {
            let from: &str = &scenario.covered_states[index];
            let to: &str = &scenario.covered_states[index + 1];
            let transition_name: &str = &scenario.covered_transitions[index];
            let matched: bool = machine.transitions.iter().any(|transition: &Transition| {
                transition.from == from && transition.to == to && transition.name == transition_name && transition.event == *event_name
            });
            assert!(
                matched,
                "scenario {} step {} ({from} --{transition_name}/{event_name}--> {to}) is not a valid transition of {machine_id}",
                scenario.id,
                index + 1
            );
        }
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

fn eval_on_trace(world: &mut Forge, property_id: &str, trace: &ExecutionTrace, event_list: Vec<String>) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let property: &Property = snapshot.property(property_id).expect("property exists");
    let digest: &ContentHash = snapshot.artifact_digest(property_id).expect("digest exists");
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
        evaluate_property(property, digest, trace, "RUN-EVAL-001", &snapshot_id, replay);
    world.evaluation = Some(evaluation);
    world.finding = finding;
    world.counterexample = counterexample;
    world.snapshot_digest_before_decision = Some(snapshot_id);
}

fn eval_on_events(world: &mut Forge, property_id: &str, events: &[&str]) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let property: &Property = snapshot.property(property_id).expect("property exists");
    let machine: &StateMachine = snapshot.state_machine(property.subject.as_str()).expect("machine exists");
    let event_list: Vec<String> = events.iter().map(|s: &&str| (*s).to_string()).collect();
    let trace: ExecutionTrace = execute_machine(machine, &event_list);
    eval_on_trace(world, property_id, &trace, event_list);
}

#[when(expr = "property {string} is evaluated on the safe message trace")]
fn eval_safe_trace(world: &mut Forge, property_id: String) {
    eval_on_events(world, &property_id, &["deliver", "ack"]);
}

#[when(expr = "property {string} is evaluated on the redelivery violation trace")]
fn eval_violation_trace(world: &mut Forge, property_id: String) {
    eval_on_events(world, &property_id, &["deliver", "ack", "redeliver"]);
}

#[when(expr = "property {string} is evaluated on events {string}")]
fn eval_custom_events(world: &mut Forge, property_id: String, names: String) {
    let owned: Vec<String> = listed(&names, ", ");
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    eval_on_events(world, &property_id, &refs);
}

#[when(expr = "property {string} is evaluated on a trace with input {string} set to {string}")]
fn eval_with_input(world: &mut Forge, property_id: String, key: String, value: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let property: &Property = snapshot.property(&property_id).expect("property exists");
    let machine: &StateMachine = snapshot.state_machine(property.subject.as_str()).expect("machine exists");
    let event_list: Vec<String> = vec!["deliver".to_string()];
    let mut trace: ExecutionTrace = execute_machine(machine, &event_list);
    if let Some(first) = trace.events.first_mut() {
        first.inputs.insert(key, value);
    }
    eval_on_trace(world, &property_id, &trace, event_list);
}

#[when(expr = "property {string} is evaluated on a discontinuous trace")]
fn eval_discontinuous_trace(world: &mut Forge, property_id: String) {
    let trace: ExecutionTrace = ExecutionTrace {
        initial_state: "queued".to_string(),
        events: vec![TraceEvent {
            step: 1,
            event: "ack".to_string(),
            from_state: "delivered".to_string(),
            to_state: "acknowledged".to_string(),
            inputs: BTreeMap::new(),
            fault: None,
            virtual_time: Some(1),
        }],
    };
    eval_on_trace(world, &property_id, &trace, vec!["ack".to_string()]);
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

#[then("the counterexample trace records 1-based steps and virtual times")]
fn counterexample_trace_times(world: &mut Forge) {
    let ce: &Counterexample = world.counterexample.as_ref().expect("counterexample captured");
    assert!(!ce.events.is_empty());
    for (index, event) in ce.events.iter().enumerate() {
        let expected_step: usize = index + 1;
        assert_eq!(event.step, expected_step);
        assert_eq!(event.virtual_time, Some(expected_step as u64));
    }
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

fn parse_root_cause(label: &str) -> RootCause {
    match label {
        "implementation" => RootCause::Implementation,
        "specification" => RootCause::Specification,
        "behavioral_model" => RootCause::BehavioralModel,
        "environment_or_assumption" => RootCause::EnvironmentOrAssumption,
        "verification" => RootCause::Verification,
        "expected_behavior" => RootCause::ExpectedBehavior,
        _ => RootCause::Unresolved,
    }
}

#[then(expr = "disposing that finding as {string} with authorized change {string} is rejected")]
fn dispose_with_change_rejected(world: &mut Forge, diagnosis: String, change: String) {
    let finding: &FindingRecord = world.finding.as_ref().expect("finding exists");
    let result: Result<VerificationDecision, String> = VerificationDecision::dispose(
        "DEC-REJ-1".to_string(),
        finding,
        Vec::new(),
        parse_root_cause(&diagnosis),
        "needs diagnosis before change".to_string(),
        "defer".to_string(),
        "alice".to_string(),
        Some(change),
    );
    let err: String = result.expect_err("unresolved authorized change rejected");
    assert!(err.contains("unresolved"), "unexpected error: {err}");
}

#[then(expr = "disposing that finding as {string} with authorized change {string} succeeds")]
fn dispose_with_change_succeeds(world: &mut Forge, diagnosis: String, change: String) {
    let finding: &FindingRecord = world.finding.as_ref().expect("finding exists");
    let decision: VerificationDecision = VerificationDecision::dispose(
        "DEC-OK-1".to_string(),
        finding,
        Vec::new(),
        parse_root_cause(&diagnosis),
        "root cause confirmed".to_string(),
        "fix_implementation".to_string(),
        "alice".to_string(),
        Some(change.clone()),
    )
    .expect("resolved diagnosis allows authorized change");
    assert_eq!(decision.authorized_change.as_deref(), Some(change.as_str()));
}

#[then(expr = "disposing that finding as {string} without an authorized change succeeds")]
fn dispose_without_change_succeeds(world: &mut Forge, diagnosis: String) {
    let finding: &FindingRecord = world.finding.as_ref().expect("finding exists");
    let decision: VerificationDecision = VerificationDecision::dispose(
        "DEC-OK-2".to_string(),
        finding,
        Vec::new(),
        parse_root_cause(&diagnosis),
        "triage opened".to_string(),
        "investigate".to_string(),
        "alice".to_string(),
        None,
    )
    .expect("unresolved diagnosis without change succeeds");
    assert!(decision.authorized_change.is_none());
}

#[then("adapter error variants format human-readable diagnostics")]
fn adapter_error_display_checked(_world: &mut Forge) {
    assert!(AdapterError::Unsupported("shrinking".to_string()).to_string().contains("unsupported"));
    assert!(AdapterError::InvalidRequest("empty id".to_string()).to_string().contains("invalid"));
    assert!(AdapterError::NotReplayable("no seed".to_string()).to_string().contains("not replayable"));
    assert!(AdapterError::Execution("crashed".to_string()).to_string().contains("execution failed"));
}

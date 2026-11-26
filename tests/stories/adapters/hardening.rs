use std::cell::Cell;
use std::collections::BTreeMap;

use cucumber::{then, when};
use specforge::graph::{ArtifactId, ContentHash, GraphSnapshot, Property, StateMachine};
use specforge::model::{evaluate_property, execute_machine};
use specforge::proptest_adapter::ProptestAdapter;
use specforge::verify::{
    AdapterError, Counterexample, ExecutionTrace, MinimizationMeta, ReplayDescriptor, RunEnvelope, TraceEvent, VerificationAdapter, VerificationRequest,
};

use super::buggy_broker_runner;
use crate::graph::current_snapshot;
use crate::Forge;

fn make_request(property_id: &str, claim_id: &str, seed: u64, budget: usize) -> VerificationRequest {
    VerificationRequest {
        run_id: format!("RUN-PBT-HARDEN-{seed}-{budget}"),
        activity_id: "ACT-PBT-001".to_string(),
        property_id: ArtifactId::parse(property_id).expect("valid property id"),
        claim_ids: vec![ArtifactId::parse(claim_id).expect("valid claim id")],
        budget,
        seed: Some(seed),
    }
}

fn synthetic_trace(initial_state: &str, step_pairs: &[(String, String, String)]) -> ExecutionTrace {
    let mut events: Vec<TraceEvent> = Vec::with_capacity(step_pairs.len());
    for (index, (from_state, event, to_state)) in step_pairs.iter().enumerate() {
        events.push(TraceEvent {
            step: index + 1,
            event: event.clone(),
            from_state: from_state.clone(),
            to_state: to_state.clone(),
            inputs: BTreeMap::new(),
            fault: None,
            virtual_time: Some((index + 1) as u64),
        });
    }
    ExecutionTrace {
        initial_state: initial_state.to_string(),
        events,
    }
}

fn trace_reached_redelivered(trace: &ExecutionTrace) -> bool {
    trace.events.iter().any(|step: &TraceEvent| step.to_state == "redelivered")
}

#[then("shrinking completes within the attempt budget")]
fn shrinking_within_budget(world: &mut Forge) {
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("adapter ran");
    let original: &Counterexample = envelope.original_counterexample.as_ref().expect("original counterexample");
    let minimized: &Counterexample = envelope.minimized_counterexample.as_ref().expect("minimized counterexample");
    let meta: &MinimizationMeta = minimized.minimization.as_ref().expect("minimization metadata");
    assert!(meta.attempts > 0 && meta.attempts <= 256, "unexpected shrink attempts: {}", meta.attempts);
    assert!(
        minimized.replay.events.len() < original.replay.events.len(),
        "expected shrinking to reduce length ({} vs {})",
        minimized.replay.events.len(),
        original.replay.events.len()
    );
}

#[when(expr = "the proptest adapter re-runs {string} for claim {string} with seed {int} against the buggy redelivering broker")]
fn proptest_rerun_buggy(world: &mut Forge, property_id: String, claim_id: String, seed: u64) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let adapter: ProptestAdapter = ProptestAdapter::with_runner(buggy_broker_runner, 12);
    let request: VerificationRequest = make_request(&property_id, &claim_id, seed, 128);
    let envelope: RunEnvelope = adapter.execute(&snapshot, &request).expect("proptest rerun executes");
    world.replay_envelope = Some(envelope);
}

#[then("the two proptest runs produced identical original counterexamples")]
fn proptest_runs_identical(world: &mut Forge) {
    let first: &Counterexample = world
        .envelope
        .as_ref()
        .expect("first run")
        .original_counterexample
        .as_ref()
        .expect("first original");
    let second: &Counterexample = world
        .replay_envelope
        .as_ref()
        .expect("second run")
        .original_counterexample
        .as_ref()
        .expect("second original");
    assert_eq!(first.replay.events, second.replay.events);
    assert_eq!(first.events, second.events);
}

#[then("the two proptest runs produced distinct original counterexamples")]
fn proptest_runs_distinct(world: &mut Forge) {
    let first: &Counterexample = world
        .envelope
        .as_ref()
        .expect("first run")
        .original_counterexample
        .as_ref()
        .expect("first original");
    let second: &Counterexample = world
        .replay_envelope
        .as_ref()
        .expect("second run")
        .original_counterexample
        .as_ref()
        .expect("second original");
    assert_ne!(first.replay.events, second.replay.events);
}

fn validate_request(world: &mut Forge, request: VerificationRequest) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let adapter: ProptestAdapter = ProptestAdapter::for_machine(8);
    world.replay_result = match adapter.validate(&snapshot, &request) {
        Err(err) => Some(Err(err)),
        Ok(()) => None,
    };
}

#[when("the proptest adapter validates a request with empty run id")]
fn validate_empty_run_id(world: &mut Forge) {
    let mut request: VerificationRequest = make_request("PROP-MSG-001", "CLM-MSG-001", 42, 8);
    request.run_id = "   ".to_string();
    validate_request(world, request);
}

#[when("the proptest adapter validates a request with empty activity id")]
fn validate_empty_activity_id(world: &mut Forge) {
    let mut request: VerificationRequest = make_request("PROP-MSG-001", "CLM-MSG-001", 42, 8);
    request.activity_id = " ".to_string();
    validate_request(world, request);
}

#[when(expr = "the proptest adapter validates a request referencing unknown claim {string}")]
fn validate_unknown_claim(world: &mut Forge, claim_id: String) {
    let request: VerificationRequest = make_request("PROP-MSG-001", &claim_id, 42, 8);
    validate_request(world, request);
}

#[when(expr = "the proptest adapter validates a request referencing unknown property {string}")]
fn validate_unknown_property(world: &mut Forge, property_id: String) {
    let request: VerificationRequest = make_request(&property_id, "CLM-MSG-001", 42, 8);
    validate_request(world, request);
}

#[then("the proptest request is rejected as invalid")]
fn proptest_request_rejected(world: &mut Forge) {
    assert!(
        matches!(world.replay_result, Some(Err(AdapterError::InvalidRequest(_)))),
        "{:?}",
        world.replay_result
    );
}

fn run_shrink_with_runner<F>(world: &mut Forge, property_id: &str, runner: F)
where
    F: Fn(&StateMachine, &[String]) -> ExecutionTrace + 'static,
{
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let adapter: ProptestAdapter = ProptestAdapter::with_runner(runner, 12);
    let request: VerificationRequest = make_request(property_id, "CLM-MSG-001", 42, 128);
    let envelope: RunEnvelope = adapter.execute(&snapshot, &request).expect("shrink run executes");
    world.envelope = Some(envelope);
}

#[when(expr = "the proptest adapter shrinks {string} against a runner forging a wrong initial state on shorter traces")]
fn shrink_wrong_initial_state(world: &mut Forge, property_id: String) {
    let original_found: Cell<bool> = Cell::new(false);
    run_shrink_with_runner(world, &property_id, move |machine: &StateMachine, events: &[String]| {
        let trace: ExecutionTrace = execute_machine(machine, events);
        if original_found.get() && !trace_reached_redelivered(&trace) {
            let steps: Vec<(String, String, String)> = events
                .iter()
                .map(|event: &String| ("redelivered".to_string(), event.clone(), "redelivered".to_string()))
                .collect();
            return synthetic_trace("redelivered", &steps);
        }
        if trace_reached_redelivered(&trace) {
            original_found.set(true);
        }
        trace
    });
}

#[when(expr = "the proptest adapter shrinks {string} against a runner forging a wrong trace length on shorter traces")]
fn shrink_wrong_trace_length(world: &mut Forge, property_id: String) {
    let original_found: Cell<bool> = Cell::new(false);
    run_shrink_with_runner(world, &property_id, move |machine: &StateMachine, events: &[String]| {
        let trace: ExecutionTrace = execute_machine(machine, events);
        if original_found.get() && !trace_reached_redelivered(&trace) {
            let mut steps: Vec<(String, String, String)> = Vec::new();
            let mut current: String = machine.initial.clone();
            for event in events {
                steps.push((current, event.clone(), "redelivered".to_string()));
                current = "redelivered".to_string();
            }
            steps.push((current, "redeliver".to_string(), "redelivered".to_string()));
            return synthetic_trace(&machine.initial, &steps);
        }
        if trace_reached_redelivered(&trace) {
            original_found.set(true);
        }
        trace
    });
}

#[when(expr = "the proptest adapter shrinks {string} against a runner forging a wrong event name on shorter traces")]
fn shrink_wrong_event_name(world: &mut Forge, property_id: String) {
    let original_found: Cell<bool> = Cell::new(false);
    run_shrink_with_runner(world, &property_id, move |machine: &StateMachine, events: &[String]| {
        let trace: ExecutionTrace = execute_machine(machine, events);
        if original_found.get() && !trace_reached_redelivered(&trace) {
            let mut steps: Vec<(String, String, String)> = Vec::new();
            let mut current: String = machine.initial.clone();
            for _ in events {
                steps.push((current, "forged_event".to_string(), "redelivered".to_string()));
                current = "redelivered".to_string();
            }
            return synthetic_trace(&machine.initial, &steps);
        }
        if trace_reached_redelivered(&trace) {
            original_found.set(true);
        }
        trace
    });
}

#[when(expr = "the proptest adapter shrinks {string} against a flaky runner whose shrunk violation stops reproducing")]
fn shrink_flaky_runner(world: &mut Forge, property_id: String) {
    let original_found: Cell<bool> = Cell::new(false);
    let flaked: Cell<bool> = Cell::new(false);
    run_shrink_with_runner(world, &property_id, move |machine: &StateMachine, events: &[String]| {
        let trace: ExecutionTrace = execute_machine(machine, events);
        if original_found.get() && events == ["deliver", "ack", "redeliver"] {
            if !flaked.get() {
                flaked.set(true);
                return trace;
            }
            let steps: Vec<(String, String, String)> = events
                .iter()
                .map(|event: &String| (machine.initial.clone(), event.clone(), machine.initial.clone()))
                .collect();
            return synthetic_trace(&machine.initial, &steps);
        }
        if trace_reached_redelivered(&trace) {
            original_found.set(true);
        }
        trace
    });
}

#[then(expr = "shrinking preserves a causally consistent minimized trace of length {int}")]
fn shrinking_preserves_consistent(world: &mut Forge, expected_len: usize) {
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("adapter ran");
    let minimized: &Counterexample = envelope.minimized_counterexample.as_ref().expect("minimized counterexample");
    assert_eq!(minimized.initial_state, "queued");
    assert_eq!(minimized.events.len(), expected_len);
    assert_eq!(minimized.replay.events.len(), expected_len);
    for (step, expected_event) in minimized.events.iter().zip(minimized.replay.events.iter()) {
        assert_eq!(&step.event, expected_event);
    }
    assert_eq!(minimized.replay.events, vec!["deliver", "ack", "redeliver"]);
}

#[then("shrinking falls back to the original counterexample")]
fn shrinking_falls_back(world: &mut Forge) {
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("adapter ran");
    let original: &Counterexample = envelope.original_counterexample.as_ref().expect("original counterexample");
    let minimized: &Counterexample = envelope.minimized_counterexample.as_ref().expect("minimized counterexample");
    let meta: &MinimizationMeta = minimized.minimization.as_ref().expect("minimization metadata");
    assert_eq!(minimized.replay.events, original.replay.events);
    assert_eq!(minimized.events, original.events);
    assert_eq!(meta.minimized_len, meta.original_len);
}

#[when(expr = "a step-zero counterexample for {string} with empty replay events is replayed")]
fn replay_step_zero_empty_events(world: &mut Forge, property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let property: &Property = snapshot.property(&property_id).expect("property exists");
    let digest: &ContentHash = snapshot.artifact_digest(property.id.as_str()).expect("property digest");
    let trace: ExecutionTrace = ExecutionTrace {
        initial_state: "queued".to_string(),
        events: Vec::new(),
    };
    let replay: ReplayDescriptor = ReplayDescriptor {
        engine: "proptest".to_string(),
        engine_version: "1".to_string(),
        seed: Some(42),
        replay_token: Some("chacha-seed:42".to_string()),
        events: Vec::new(),
        determinism: "deterministic initial-state violation".to_string(),
        replayable: true,
    };
    let (_, _, counterexample) = evaluate_property(property, digest, &trace, "RUN-STEP0", &snapshot.snapshot_id, replay);
    let ce: Counterexample = counterexample.expect("step-zero counterexample");
    assert_eq!(ce.violation.step, 0);
    assert!(ce.replay.events.is_empty());
    let adapter: ProptestAdapter = ProptestAdapter::for_machine(8);
    let envelope: RunEnvelope = adapter.replay(&snapshot, &ce, "RUN-REPLAY-STEP0").expect("step-zero replay succeeds");
    world.replay_envelope = Some(envelope);
}

#[when("the counterexample replay events are cleared while keeping replayable true and replayed")]
fn replay_positive_step_empty_events(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let mut counterexample: Counterexample = world
        .envelope
        .as_ref()
        .expect("adapter ran")
        .minimized_counterexample
        .clone()
        .expect("minimized counterexample");
    assert!(counterexample.violation.step > 0);
    counterexample.replay.replayable = true;
    counterexample.replay.events.clear();
    let adapter: ProptestAdapter = ProptestAdapter::with_runner(buggy_broker_runner, 12);
    world.replay_result = Some(adapter.replay(&snapshot, &counterexample, "RUN-REPLAY-NOEVENTS-001"));
}

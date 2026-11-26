use std::collections::BTreeMap;

use cucumber::{then, when};
use specforge::corpus::CorpusError;
use specforge::graph::{ContentHash, GraphSnapshot, Property, StateMachine};
use specforge::model::{evaluate_property, execute_machine};
use specforge::verify::{Counterexample, ExecutionTrace, FindingRecord, MinimizationMeta, PropertyEvaluation, ReplayDescriptor, TraceEvent};

use super::ensure_original_registered;
use crate::graph::current_snapshot;
use crate::Forge;

pub(super) fn make_replay(events: Vec<String>, replayable: bool, seed: Option<u64>, token: Option<String>) -> ReplayDescriptor {
    ReplayDescriptor {
        engine: "proptest".to_string(),
        engine_version: "1".to_string(),
        seed,
        replay_token: token,
        events,
        determinism: "deterministic test replay".to_string(),
        replayable,
    }
}

pub(super) fn counterexample_for_events(
    snapshot: &GraphSnapshot,
    property_id: &str,
    ce_id: &str,
    run_id: &str,
    events: &[&str],
    replay: ReplayDescriptor,
) -> Counterexample {
    let property: &Property = snapshot.property(property_id).expect("property exists");
    let digest: &ContentHash = snapshot.artifact_digest(property.id.as_str()).expect("property digest");
    let machine: &StateMachine = snapshot.state_machine(property.subject.as_str()).expect("machine exists");
    let event_names: Vec<String> = events.iter().map(|name: &&str| (*name).to_string()).collect();
    let trace: ExecutionTrace = execute_machine(machine, &event_names);
    let (_, _, counterexample): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) =
        evaluate_property(property, digest, &trace, run_id, &snapshot.snapshot_id, replay);
    let mut ce: Counterexample = counterexample.expect("trace produces counterexample");
    ce.id = ce_id.to_string();
    ce
}

pub(super) fn minimized_from_original(
    snapshot: &GraphSnapshot,
    original: &Counterexample,
    min_id: &str,
    events: &[&str],
    replay: ReplayDescriptor,
) -> Counterexample {
    let mut ce: Counterexample = counterexample_for_events(snapshot, original.property_id.as_str(), min_id, &original.run_id, events, replay);
    ce.original_id = Some(original.id.clone());
    ce.minimization = Some(MinimizationMeta {
        method: "proptest-value-tree-shrink".to_string(),
        version: "1".to_string(),
        attempts: 1,
        original_len: original.events.len(),
        minimized_len: ce.events.len(),
    });
    ce
}

#[when(expr = "two distinct original counterexamples are registered for {string}")]
fn register_two_distinct_originals(world: &mut Forge, property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let first: Counterexample = counterexample_for_events(
        &snapshot,
        &property_id,
        "CE-ORIG-001",
        "RUN-ORIG-001",
        &events,
        make_replay(event_vec.clone(), true, Some(42), Some("chacha-seed:42".to_string())),
    );
    let second: Counterexample = counterexample_for_events(
        &snapshot,
        &property_id,
        "CE-ORIG-002",
        "RUN-ORIG-002",
        &events,
        make_replay(event_vec, true, Some(99), Some("chacha-seed:99".to_string())),
    );
    world.exploration.original_snapshot = Some(first.clone());
    world.exploration.registry.register_original(first).expect("first original registers");
    world.exploration.registry.register_original(second).expect("second original registers");
}

#[when(expr = "a duplicate original counterexample is registered for {string}")]
fn register_duplicate_original(world: &mut Forge, property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let dup: Counterexample = counterexample_for_events(
        &snapshot,
        &property_id,
        "CE-ORIG-001",
        "RUN-ORIG-DUP",
        &events,
        make_replay(event_vec, true, Some(42), Some("chacha-seed:42".to_string())),
    );
    world.exploration.minimized_registration = Some(world.exploration.registry.register_original(dup));
}

#[when("an original counterexample with an empty id is registered")]
fn register_empty_id_original(world: &mut Forge) {
    let mut bad: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    bad.id = "   ".to_string();
    world.exploration.minimized_registration = Some(world.exploration.registry.register_original(bad));
}

#[when("an original counterexample carrying an original_id is registered")]
fn register_original_with_original_id(world: &mut Forge) {
    let mut bad: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    bad.id = "CE-ORIG-WITH-LINK".to_string();
    bad.original_id = Some("CE-ORIG-001".to_string());
    world.exploration.minimized_registration = Some(world.exploration.registry.register_original(bad));
}

#[when("an original counterexample carrying minimization metadata is registered")]
fn register_original_with_minimization_meta(world: &mut Forge) {
    let mut bad: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    bad.id = "CE-ORIG-WITH-META".to_string();
    bad.minimization = Some(MinimizationMeta {
        method: "manual".to_string(),
        version: "1".to_string(),
        attempts: 1,
        original_len: bad.events.len(),
        minimized_len: bad.events.len(),
    });
    world.exploration.minimized_registration = Some(world.exploration.registry.register_original(bad));
}

#[when("a forged minimized counterexample with a wrong destination state is registered")]
fn register_forged_wrong_to_state(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = ensure_original_registered(world);
    let property: &Property = snapshot.property(original.property_id.as_str()).expect("property exists");
    let digest: &ContentHash = snapshot.artifact_digest(property.id.as_str()).expect("property digest");
    let forged_events: Vec<TraceEvent> = vec![TraceEvent {
        step: 1,
        event: "deliver".to_string(),
        from_state: "queued".to_string(),
        to_state: "redelivered".to_string(),
        inputs: BTreeMap::new(),
        fault: None,
        virtual_time: Some(1),
    }];
    let forged_trace: ExecutionTrace = ExecutionTrace {
        initial_state: "queued".to_string(),
        events: forged_events.clone(),
    };
    let mut forged_replay: ReplayDescriptor = original.replay.clone();
    forged_replay.events = vec!["deliver".to_string()];
    let (eval, _, _): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) =
        evaluate_property(property, digest, &forged_trace, &original.run_id, &snapshot.snapshot_id, forged_replay.clone());
    let mut forged: Counterexample = original.clone();
    forged.id = format!("{}-wrong-to", original.id);
    forged.original_id = Some(original.id.clone());
    forged.minimization = Some(MinimizationMeta {
        method: "forged".to_string(),
        version: "1".to_string(),
        attempts: 1,
        original_len: original.events.len(),
        minimized_len: 1,
    });
    forged.events = forged_events;
    forged.replay = forged_replay;
    forged.violation = eval.violation.expect("forged trace produces violation");
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, forged));
}

#[when("a minimized counterexample matching the updated property digest is registered against the original")]
fn register_minimized_updated_digest(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = ensure_original_registered(world);
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let mut min_ce: Counterexample = minimized_from_original(
        &snapshot,
        &original,
        "CE-MIN-NEW-DIGEST",
        &events,
        make_replay(event_vec, true, Some(42), Some("chacha-seed:42".to_string())),
    );
    min_ce.original_id = Some(original.id.clone());
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[then("minimized registration succeeds")]
fn minimized_registration_ok(world: &mut Forge) {
    assert_eq!(world.exploration.minimized_registration, Some(Ok(())));
}

#[then("corpus registration fails with a duplicate id error")]
fn registration_duplicate_id(world: &mut Forge) {
    match &world.exploration.minimized_registration {
        Some(Err(err @ CorpusError::DuplicateId(_))) => {
            assert!(!err.to_string().trim().is_empty(), "CorpusError Display must be non-empty");
        }
        other => panic!("expected DuplicateId, got {other:?}"),
    }
}

#[then("corpus registration fails with an empty id error")]
fn registration_empty_id(world: &mut Forge) {
    match &world.exploration.minimized_registration {
        Some(Err(err @ CorpusError::EmptyId)) => {
            assert!(!err.to_string().trim().is_empty(), "CorpusError Display must be non-empty");
        }
        other => panic!("expected EmptyId, got {other:?}"),
    }
}

#[then("corpus registration fails with a model mismatch error")]
fn registration_model_mismatch(world: &mut Forge) {
    match &world.exploration.minimized_registration {
        Some(Err(err @ CorpusError::ModelMismatch(_))) => {
            assert!(!err.to_string().trim().is_empty(), "CorpusError Display must be non-empty");
        }
        other => panic!("expected ModelMismatch, got {other:?}"),
    }
}

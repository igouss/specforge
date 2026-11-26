use std::collections::BTreeMap;

use cucumber::when;
use specforge::corpus::{replay_corpus, CounterexampleRegistry};
use specforge::graph::GraphSnapshot;
use specforge::ledger::{EvidenceKind, EvidenceRecord};
use specforge::proptest_adapter::ProptestAdapter;
use specforge::verify::{Counterexample, RunEnvelope, VerificationAdapter};

use super::validation::{counterexample_for_events, make_replay, minimized_from_original};
use crate::graph::current_snapshot;
use crate::Forge;

#[when(expr = "a 3-event original counterexample for {string} is registered")]
fn register_three_event_original(world: &mut Forge, property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let original: Counterexample = counterexample_for_events(
        &snapshot,
        &property_id,
        "CE-THREE-ORIG",
        "RUN-THREE-ORIG",
        &events,
        make_replay(event_vec, true, Some(42), Some("chacha-seed:42".to_string())),
    );
    world.exploration.original_snapshot = Some(original.clone());
    world.exploration.registry.register_original(original).expect("3-event original registers");
}

#[when(expr = "an equal-length 3-event minimized counterexample for {string} is registered")]
fn register_equal_length_minimized(world: &mut Forge, _property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let min_ce: Counterexample = minimized_from_original(
        &snapshot,
        &original,
        "CE-THREE-MIN-EQ",
        &events,
        make_replay(event_vec, true, Some(42), Some("chacha-seed:42".to_string())),
    );
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when(expr = "a 4-event minimized counterexample longer than the original for {string} is registered")]
fn register_longer_minimized(world: &mut Forge, _property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let events: [&str; 4] = ["tick", "deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let min_ce: Counterexample = minimized_from_original(
        &snapshot,
        &original,
        "CE-THREE-MIN-LONGER",
        &events,
        make_replay(event_vec, true, Some(42), Some("chacha-seed:42".to_string())),
    );
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when(expr = "a minimized counterexample missing minimization metadata for {string} is registered")]
fn register_missing_meta_minimized(world: &mut Forge, _property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let mut min_ce: Counterexample = minimized_from_original(
        &snapshot,
        &original,
        "CE-THREE-MIN-NOMETA",
        &events,
        make_replay(event_vec, true, Some(42), Some("chacha-seed:42".to_string())),
    );
    min_ce.minimization = None;
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when(expr = "a minimized counterexample with an empty minimization method for {string} is registered")]
fn register_empty_method_minimized(world: &mut Forge, _property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let mut min_ce: Counterexample = minimized_from_original(
        &snapshot,
        &original,
        "CE-THREE-MIN-EMPTYMETHOD",
        &events,
        make_replay(event_vec, true, Some(42), Some("chacha-seed:42".to_string())),
    );
    if let Some(meta) = min_ce.minimization.as_mut() {
        meta.method = "   ".to_string();
    }
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when(expr = "a minimized counterexample with wrong original length metadata for {string} is registered")]
fn register_wrong_orig_len_minimized(world: &mut Forge, _property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let mut min_ce: Counterexample = minimized_from_original(
        &snapshot,
        &original,
        "CE-THREE-MIN-BADORIGLEN",
        &events,
        make_replay(event_vec, true, Some(42), Some("chacha-seed:42".to_string())),
    );
    if let Some(meta) = min_ce.minimization.as_mut() {
        meta.original_len = 99;
    }
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when(expr = "a minimized counterexample with wrong minimized length metadata for {string} is registered")]
fn register_wrong_min_len_minimized(world: &mut Forge, _property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let mut min_ce: Counterexample = minimized_from_original(
        &snapshot,
        &original,
        "CE-THREE-MIN-BADMINLEN",
        &events,
        make_replay(event_vec, true, Some(42), Some("chacha-seed:42".to_string())),
    );
    if let Some(meta) = min_ce.minimization.as_mut() {
        meta.minimized_len = 1;
    }
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when(expr = "a step-zero original counterexample for {string} is registered")]
fn register_step_zero_original(world: &mut Forge, property_id: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = counterexample_for_events(
        &snapshot,
        &property_id,
        "CE-STEP0-ORIG",
        "RUN-STEP0-ORIG",
        &[],
        make_replay(Vec::new(), true, Some(42), Some("chacha-seed:42".to_string())),
    );
    world.exploration.original_snapshot = Some(original.clone());
    world.exploration.registry.register_original(original).expect("step-zero original registers");
}

#[when("a replayable step-zero minimized counterexample with seed and no token is registered")]
fn register_step_zero_min_seed_only(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let min_ce: Counterexample = minimized_from_original(&snapshot, &original, "CE-STEP0-MIN-SEED", &[], make_replay(Vec::new(), true, Some(42), None));
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when("a replayable step-zero minimized counterexample with neither seed nor token is registered")]
fn register_step_zero_min_no_source(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let min_ce: Counterexample = minimized_from_original(&snapshot, &original, "CE-STEP0-MIN-NONE", &[], make_replay(Vec::new(), true, None, None));
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when("a non-replayable minimized counterexample with empty replay events is registered")]
fn register_non_replayable_min_empty_events(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let min_ce: Counterexample = minimized_from_original(&snapshot, &original, "CE-NONREPLAY-EMPTY", &events, make_replay(Vec::new(), false, None, None));
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when("a non-replayable minimized counterexample with matching replay events is registered")]
fn register_non_replayable_min_matching_events(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let min_ce: Counterexample = minimized_from_original(&snapshot, &original, "CE-NONREPLAY-MATCH", &events, make_replay(event_vec, false, None, None));
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when("a non-replayable minimized counterexample with mismatched replay events is registered")]
fn register_non_replayable_min_mismatched_events(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = world.exploration.original_snapshot.clone().expect("original exists");
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let min_ce: Counterexample = minimized_from_original(
        &snapshot,
        &original,
        "CE-NONREPLAY-MISMATCH",
        &events,
        make_replay(vec!["deliver".to_string()], false, None, None),
    );
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, min_ce));
}

#[when("two replayable originals are replayed with a second-entry evidence id collision")]
fn replay_two_originals_second_id_collision(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let events: [&str; 3] = ["deliver", "ack", "redeliver"];
    let event_vec: Vec<String> = events.iter().map(|e: &&str| (*e).to_string()).collect();
    let first: Counterexample = counterexample_for_events(
        &snapshot,
        "PROP-MSG-001",
        "CE-COL-1",
        "RUN-COL-1",
        &events,
        make_replay(event_vec.clone(), true, Some(42), Some("chacha-seed:42".to_string())),
    );
    let second: Counterexample = counterexample_for_events(
        &snapshot,
        "PROP-MSG-001",
        "CE-COL-2",
        "RUN-COL-2",
        &events,
        make_replay(event_vec, true, Some(99), Some("chacha-seed:99".to_string())),
    );
    world.exploration.registry = CounterexampleRegistry::new();
    world.exploration.registry.register_original(first.clone()).expect("first registers");
    world.exploration.registry.register_original(second).expect("second registers");

    let adapter: ProptestAdapter = ProptestAdapter::for_machine(12);
    let pre_envelope: RunEnvelope = adapter.replay(&snapshot, &first, "RUN-COLLIDE-002").expect("seed collision envelope");
    let collision_record: EvidenceRecord = EvidenceRecord::from_envelope(
        "EV-RUN-COLLIDE-002".to_string(),
        EvidenceKind::Replay,
        "2026-11-24T06:41:00Z".to_string(),
        &snapshot,
        &pre_envelope,
        BTreeMap::new(),
    )
    .expect("collision record seals");
    world.evidence_ledger.append_record(collision_record).expect("collision record appended");
    world.exploration.ledger_count_before_replay = Some(world.evidence_ledger.records().len());

    world.exploration.replay_error = Some(replay_corpus(
        &world.exploration.registry,
        &snapshot,
        &adapter,
        &mut world.evidence_ledger,
        "RUN-COLLIDE",
        "2026-11-24T06:43:00Z",
    ));
}

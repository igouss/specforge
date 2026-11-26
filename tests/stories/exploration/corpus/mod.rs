mod minimization;
mod validation;

use std::collections::BTreeMap;

use cucumber::{given, then, when};
use specforge::corpus::{replay_corpus, CorpusError, CorpusReplayEntry, CorpusReplaySummary, CounterexampleRegistry};
use specforge::graph::{ContentHash, GraphSnapshot, Property, StateMachine};
use specforge::ledger::{EvidenceKind, EvidenceRecord};
use specforge::model::{evaluate_property, execute_machine};
use specforge::proptest_adapter::ProptestAdapter;
use specforge::verify::{
    AdapterCapabilities, AdapterError, Counterexample, ExecutionTrace, FindingRecord, MinimizationMeta, PropertyEvaluation, ReplayDescriptor, RunEnvelope,
    TraceEvent, VerificationAdapter, VerificationRequest,
};

use super::safe_runner;
use crate::graph::current_snapshot;
use crate::Forge;

struct MismatchedEnvelopeAdapter {
    inner: ProptestAdapter,
}

impl VerificationAdapter for MismatchedEnvelopeAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        self.inner.capabilities()
    }

    fn validate(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<(), AdapterError> {
        self.inner.validate(snapshot, request)
    }

    fn execute(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<RunEnvelope, AdapterError> {
        self.inner.execute(snapshot, request)
    }

    fn replay(&self, snapshot: &GraphSnapshot, counterexample: &Counterexample, run_id: &str) -> Result<RunEnvelope, AdapterError> {
        let mut envelope: RunEnvelope = self.inner.replay(snapshot, counterexample, run_id)?;
        envelope.run_id = "RUN-TAMPERED-MISMATCH".to_string();
        Ok(envelope)
    }
}

#[given("an empty counterexample registry")]
fn empty_registry(world: &mut Forge) {
    world.exploration.registry = CounterexampleRegistry::new();
    world.exploration.original_snapshot = None;
    world.exploration.minimized_id = None;
    world.exploration.minimized_registration = None;
    world.exploration.replay_summary = None;
    world.exploration.replay_error = None;
}

#[when("the discovered original and minimized counterexamples are registered in the corpus")]
fn register_discovered_corpus(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("proptest envelope exists");
    let original: Counterexample = envelope.original_counterexample.clone().expect("original exists");
    let minimized: Counterexample = envelope.minimized_counterexample.clone().expect("minimized exists");
    world.exploration.original_snapshot = Some(original.clone());
    world.exploration.minimized_id = Some(minimized.id.clone());
    world.exploration.registry.register_original(original).expect("original registers");
    world
        .exploration
        .registry
        .register_minimized(&snapshot, minimized)
        .expect("minimized registers");
}

#[then(expr = "the registry holds {int} original and {int} minimized counterexample")]
fn registry_counts(world: &mut Forge, originals: usize, minimized: usize) {
    let orig_list: Vec<&Counterexample> = world.exploration.registry.originals();
    assert_eq!(orig_list.len(), originals);
    let min_count: usize = world.exploration.registry.entries().len() - orig_list.len();
    assert_eq!(min_count, minimized);
}

fn replay_with_runner(world: &mut Forge, safe: bool) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    world.exploration.replay_counter += 1;
    let prefix: String = format!("RUN-CORPUS-{}", world.exploration.replay_counter);
    let adapter: ProptestAdapter = if safe {
        ProptestAdapter::with_runner(safe_runner, 12)
    } else {
        ProptestAdapter::for_machine(12)
    };
    let summary: CorpusReplaySummary = replay_corpus(
        &world.exploration.registry,
        &snapshot,
        &adapter,
        &mut world.evidence_ledger,
        &prefix,
        "2026-11-24T06:40:00Z",
    )
    .expect("corpus replays");
    world.exploration.replay_summary = Some(summary);
}

#[when("the corpus is replayed against the buggy redelivering broker")]
fn replay_corpus_buggy(world: &mut Forge) {
    replay_with_runner(world, false);
}

#[when("the corpus is replayed against the safe message broker")]
fn replay_corpus_safe(world: &mut Forge) {
    replay_with_runner(world, true);
}

#[then(expr = "the corpus replay entry {int} has outcome {string} and emitted replay evidence")]
fn replay_entry_with_evidence(world: &mut Forge, one_based: usize, expected: String) {
    let summary: &CorpusReplaySummary = world.exploration.replay_summary.as_ref().expect("summary exists");
    let entry: &CorpusReplayEntry = summary.entries.get(one_based - 1).expect("entry exists");
    assert_eq!(entry.outcome.to_string(), expected);
    let evidence_id: &str = entry.evidence_id.as_deref().expect("evidence emitted");
    let record: &EvidenceRecord = world.evidence_ledger.get(evidence_id).expect("evidence in ledger");
    assert_eq!(record.kind, EvidenceKind::Replay);
}

#[then(expr = "the corpus replay entry {int} targeted the minimized counterexample")]
fn replay_entry_targeted_minimized(world: &mut Forge, one_based: usize) {
    let summary: &CorpusReplaySummary = world.exploration.replay_summary.as_ref().expect("summary exists");
    let entry: &CorpusReplayEntry = summary.entries.get(one_based - 1).expect("entry exists");
    let expected_id: &str = world.exploration.minimized_id.as_deref().expect("minimized id saved");
    assert_eq!(entry.counterexample_id, expected_id);
}

#[then("the registered original counterexample is unchanged")]
fn registered_original_unchanged(world: &mut Forge) {
    let before: &Counterexample = world.exploration.original_snapshot.as_ref().expect("original snapshot saved");
    let after: &Counterexample = world.exploration.registry.get(&before.id).expect("original still registered");
    assert_eq!(before, after);
}

pub(super) fn ensure_original_registered(world: &mut Forge) -> Counterexample {
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("envelope exists");
    let original: Counterexample = envelope.original_counterexample.clone().expect("original exists");
    if world.exploration.registry.get(&original.id).is_none() {
        world.exploration.registry.register_original(original.clone()).expect("original registers");
    }
    original
}

#[when("a forged minimized counterexample skipping a transition is registered")]
fn register_forged_shortcut(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = ensure_original_registered(world);
    let property: &Property = snapshot.property(original.property_id.as_str()).expect("property exists");
    let digest: &ContentHash = snapshot.artifact_digest(property.id.as_str()).expect("property digest");
    let forged_events: Vec<TraceEvent> = vec![TraceEvent {
        step: 1,
        event: "redeliver".to_string(),
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
    forged_replay.events = vec!["redeliver".to_string()];
    let (eval, _, _): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) =
        evaluate_property(property, digest, &forged_trace, &original.run_id, &snapshot.snapshot_id, forged_replay.clone());
    let mut forged: Counterexample = original.clone();
    forged.id = format!("{}-shortcut", original.id);
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

#[when("a forged minimized counterexample that no longer violates is registered")]
fn register_forged_safe(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let original: Counterexample = ensure_original_registered(world);
    let machine: &StateMachine = snapshot.state_machine("SM-MSG-001").expect("machine exists");
    let safe_events: Vec<String> = vec!["deliver".to_string(), "ack".to_string()];
    let safe_trace: ExecutionTrace = execute_machine(machine, &safe_events);
    let mut forged: Counterexample = original.clone();
    forged.id = format!("{}-safe", original.id);
    forged.original_id = Some(original.id.clone());
    forged.minimization = Some(MinimizationMeta {
        method: "forged".to_string(),
        version: "1".to_string(),
        attempts: 1,
        original_len: original.events.len(),
        minimized_len: safe_events.len(),
    });
    forged.events = safe_trace.events;
    forged.replay.events = safe_events;
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, forged));
}

#[when("a forged minimized counterexample with mismatched replay events is registered")]
fn register_forged_mismatched_replay(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let _original: Counterexample = ensure_original_registered(world);
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("envelope exists");
    let mut forged: Counterexample = envelope.minimized_counterexample.clone().expect("minimized exists");
    forged.id = format!("{}-mismatch-replay", forged.id);
    forged.replay.events = vec!["deliver".to_string()];
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, forged));
}

#[when("a minimized counterexample chaining onto another minimized counterexample is registered")]
fn register_chained_minimized(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let _original: Counterexample = ensure_original_registered(world);
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("envelope exists");
    let valid_min: Counterexample = envelope.minimized_counterexample.clone().expect("minimized exists");
    if world.exploration.registry.get(&valid_min.id).is_none() {
        world
            .exploration
            .registry
            .register_minimized(&snapshot, valid_min.clone())
            .expect("valid minimized registers");
    }
    let mut chained: Counterexample = valid_min.clone();
    chained.id = format!("{}-chained", valid_min.id);
    chained.original_id = Some(valid_min.id.clone());
    world.exploration.minimized_registration = Some(world.exploration.registry.register_minimized(&snapshot, chained));
}

#[then("minimized registration fails with an invalid minimized error")]
fn minimized_registration_invalid(world: &mut Forge) {
    match &world.exploration.minimized_registration {
        Some(Err(err @ CorpusError::InvalidMinimized(_))) => {
            assert!(!err.to_string().trim().is_empty(), "CorpusError Display must be non-empty");
        }
        other => panic!("expected InvalidMinimized, got {other:?}"),
    }
}

#[when("a non-replayable counterexample is registered and the corpus is replayed")]
fn register_non_replayable_and_replay(world: &mut Forge) {
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("envelope exists");
    let mut ce: Counterexample = envelope.original_counterexample.clone().expect("original exists");
    ce.replay.replayable = false;
    world.exploration.registry.register_original(ce).expect("non-replayable registers");
    replay_with_runner(world, true);
}

#[then(expr = "the corpus replay entry {int} has outcome {string} and no replay evidence")]
fn replay_entry_without_evidence(world: &mut Forge, one_based: usize, expected: String) {
    let summary: &CorpusReplaySummary = world.exploration.replay_summary.as_ref().expect("summary exists");
    let entry: &CorpusReplayEntry = summary.entries.get(one_based - 1).expect("entry exists");
    assert_eq!(entry.outcome.to_string(), expected);
    assert!(entry.evidence_id.is_none());
}

#[when("a replayable counterexample is replayed through a mismatched-envelope adapter")]
fn replay_through_mismatched_adapter(world: &mut Forge) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let envelope: &RunEnvelope = world.envelope.as_ref().expect("envelope exists");
    let mut ce: Counterexample = envelope.original_counterexample.clone().expect("original exists");
    ce.id = format!("{}-mismatched-run", ce.id);
    world.exploration.registry = CounterexampleRegistry::new();
    world.exploration.registry.register_original(ce).expect("ce registers");
    world.exploration.ledger_count_before_replay = Some(world.evidence_ledger.records().len());
    let adapter: MismatchedEnvelopeAdapter = MismatchedEnvelopeAdapter {
        inner: ProptestAdapter::for_machine(12),
    };
    world.exploration.replay_error = Some(replay_corpus(
        &world.exploration.registry,
        &snapshot,
        &adapter,
        &mut world.evidence_ledger,
        "RUN-MISMATCH",
        "2026-11-24T06:42:00Z",
    ));
}

#[then("corpus replay fails and leaves the ledger unchanged")]
fn corpus_replay_fails_atomically(world: &mut Forge) {
    match &world.exploration.replay_error {
        Some(Err(err @ CorpusError::ReplayFailed(_))) => {
            assert!(!err.to_string().trim().is_empty(), "CorpusError Display must be non-empty");
        }
        other => panic!("expected ReplayFailed, got {other:?}"),
    }
    assert_eq!(Some(world.evidence_ledger.records().len()), world.exploration.ledger_count_before_replay);
}

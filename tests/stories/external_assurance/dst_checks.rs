use std::collections::BTreeMap;

use cucumber::{then, when};
use specforge::dst::{normalize_dst_report, DstReplayMetadata, DstRunReport};
use specforge::graph::{self, ContentHash, GraphSnapshot};
use specforge::ledger::{EvidenceKind, EvidenceRecord};
use specforge::trace::Source;
use specforge::verify::{
    AdapterError, Counterexample, EvaluationResult, ExecutionTrace, ReplayDescriptor, RunEnvelope, RunState, TraceEvent, VerificationOutcome,
    VerificationRequest,
};

use super::dst::{base_dst_report, build_dst_trace, deterministic_guarantee, dst_request};
use crate::graph::current_snapshot;
use crate::{fixture_text, listed, Forge};

fn run_state_text(state: RunState) -> &'static str {
    match state {
        RunState::Requested => "requested",
        RunState::Started => "started",
        RunState::Completed => "completed",
        RunState::Failed => "failed",
        RunState::Cancelled => "cancelled",
        RunState::Inconclusive => "inconclusive",
    }
}

fn eval_outcome_text(outcome: &VerificationOutcome) -> String {
    match outcome {
        VerificationOutcome::Evaluation(result) => result.to_string(),
        VerificationOutcome::Formal(formal) => format!("formal:{formal:?}"),
    }
}

#[when(expr = "a DST report with an empty engine version is normalized for {string}")]
fn normalize_empty_version_dst(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = dst_request("RUN-DST-BADVER-1", &property_id, None);
    let mut report: DstRunReport = base_dst_report(RunState::Completed, build_dst_trace(false), deterministic_guarantee());
    report.engine_version = "   ".to_string();
    world.external_assurance.dst_result = Some(normalize_dst_report(snapshot, &request, &report));
}

#[when(expr = "a DST report with an empty initial state is normalized for {string}")]
fn normalize_empty_initial_state_dst(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = dst_request("RUN-DST-EMPTYINIT-1", &property_id, None);
    let trace: ExecutionTrace = ExecutionTrace {
        initial_state: "   ".to_string(),
        events: Vec::new(),
    };
    let report: DstRunReport = base_dst_report(RunState::Completed, trace, deterministic_guarantee());
    world.external_assurance.dst_result = Some(normalize_dst_report(snapshot, &request, &report));
}

#[when(expr = "a DST report with an unknown initial state is normalized for {string}")]
fn normalize_unknown_initial_state_dst(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = dst_request("RUN-DST-BADINIT-1", &property_id, None);
    let trace: ExecutionTrace = ExecutionTrace {
        initial_state: "ghost_state".to_string(),
        events: Vec::new(),
    };
    let report: DstRunReport = base_dst_report(RunState::Completed, trace, deterministic_guarantee());
    world.external_assurance.dst_result = Some(normalize_dst_report(snapshot, &request, &report));
}

#[when(expr = "a DST report with an empty trace event name is normalized for {string}")]
fn normalize_empty_event_dst(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = dst_request("RUN-DST-EMPTYEV-1", &property_id, None);
    let trace: ExecutionTrace = ExecutionTrace {
        initial_state: "queued".to_string(),
        events: vec![TraceEvent {
            step: 1,
            event: " ".to_string(),
            from_state: "queued".to_string(),
            to_state: "delivered".to_string(),
            inputs: BTreeMap::new(),
            fault: None,
            virtual_time: Some(10),
        }],
    };
    let report: DstRunReport = base_dst_report(RunState::Completed, trace, deterministic_guarantee());
    world.external_assurance.dst_result = Some(normalize_dst_report(snapshot, &request, &report));
}

#[when(expr = "a DST report with a decreasing virtual clock is normalized for {string}")]
fn normalize_decreasing_clock_dst(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = dst_request("RUN-DST-BADCLOCK-1", &property_id, None);
    let mut trace: ExecutionTrace = build_dst_trace(false);
    trace.events[0].virtual_time = Some(30);
    trace.events[1].virtual_time = Some(20);
    let report: DstRunReport = base_dst_report(RunState::Completed, trace, deterministic_guarantee());
    world.external_assurance.dst_result = Some(normalize_dst_report(snapshot, &request, &report));
}

#[when("a completed DST report evaluates an input property with no matching input")]
fn normalize_inconclusive_input_dst(world: &mut Forge) {
    let base: String = fixture_text("sdv-assurance.md");
    let modified: String = base.replace("never(state == redelivered)", "always(input.mode == strict)");
    let snapshot: GraphSnapshot = graph::compile_graph(&[Source {
        path: "input-prop.md".to_string(),
        text: modified,
    }])
    .expect("modified snapshot compiles");
    let request: VerificationRequest = dst_request("RUN-DST-INPUT-1", "PROP-MSG-001", None);
    let report: DstRunReport = base_dst_report(RunState::Completed, build_dst_trace(false), deterministic_guarantee());
    world.external_assurance.dst_result = Some(normalize_dst_report(&snapshot, &request, &report));
}

#[then(expr = "the DST envelope has run state {string} and outcome {string}")]
fn dst_envelope_state_outcome(world: &mut Forge, expected_state: String, expected_outcome: String) {
    let (envelope, _digest): &(RunEnvelope, ContentHash) = world
        .external_assurance
        .dst_result
        .as_ref()
        .expect("dst result recorded")
        .as_ref()
        .expect("dst normalization succeeded");
    assert_eq!(run_state_text(envelope.run_state), expected_state);
    assert_eq!(eval_outcome_text(&envelope.outcome), expected_outcome);
}

#[then("the DST replay descriptor is replayable with a deterministic metadata digest")]
fn dst_replayable_with_digest(world: &mut Forge) {
    let (envelope, digest): &(RunEnvelope, ContentHash) = world
        .external_assurance
        .dst_result
        .as_ref()
        .expect("dst result recorded")
        .as_ref()
        .expect("dst normalization succeeded");
    let replay: &ReplayDescriptor = envelope.replay.as_ref().expect("replay descriptor present");
    assert!(replay.replayable, "expected replayable descriptor: {replay:?}");
    let metadata: &DstReplayMetadata = world.external_assurance.dst_replay_metadata.as_ref().expect("metadata saved");
    let recomputed: ContentHash = ContentHash::of_canonical_json(metadata).expect("metadata hashes");
    assert_eq!(*digest, recomputed);
}

#[then("the DST counterexample preserves fault injection and virtual time annotations")]
fn dst_ce_preserves_annotations(world: &mut Forge) {
    let (envelope, _digest): &(RunEnvelope, ContentHash) = world
        .external_assurance
        .dst_result
        .as_ref()
        .expect("dst result recorded")
        .as_ref()
        .expect("dst normalization succeeded");
    let ce: &Counterexample = envelope.original_counterexample.as_ref().expect("counterexample recorded");
    assert_eq!(ce.events.len(), 3);
    assert_eq!(ce.events[0].virtual_time, Some(10));
    assert_eq!(ce.events[2].virtual_time, Some(40));
    assert_eq!(ce.events[2].fault.as_deref(), Some("disk_fsync_stall"));
}

#[then("the replayed DST envelope reproduces the violation")]
fn dst_replay_reproduces(world: &mut Forge) {
    let envelope: &RunEnvelope = world.external_assurance.dst_replay_envelope.as_ref().expect("replay recorded");
    assert_eq!(envelope.run_state, RunState::Completed);
    assert_eq!(envelope.outcome, VerificationOutcome::Evaluation(EvaluationResult::Violated));
    assert!(envelope.original_counterexample.is_some());
}

#[then("the DST envelope seals into the evidence ledger with its replay metadata digest")]
fn dst_seals_into_ledger(world: &mut Forge) {
    world.external_assurance.evidence_counter += 1;
    let id: String = format!("EV-DST-{}", world.external_assurance.evidence_counter);
    let (envelope, digest): (RunEnvelope, ContentHash) = world
        .external_assurance
        .dst_result
        .as_ref()
        .expect("dst result recorded")
        .as_ref()
        .expect("dst normalization succeeded")
        .clone();
    let mut artifacts: BTreeMap<String, ContentHash> = BTreeMap::new();
    artifacts.insert("dst_replay_metadata".to_string(), digest.clone());
    let record: EvidenceRecord = {
        let snapshot: &GraphSnapshot = current_snapshot(world);
        EvidenceRecord::from_envelope(
            id.clone(),
            EvidenceKind::PropertyEvaluation,
            "2026-11-24T08:00:00Z".to_string(),
            snapshot,
            &envelope,
            artifacts,
        )
        .expect("dst envelope seals")
    };
    assert_eq!(record.artifact_digests.get("dst_replay_metadata"), Some(&digest));
    world.evidence_ledger.append_record(record).expect("dst record appends");
    assert!(world.evidence_ledger.get(&id).is_some());
}

#[then("the DST replay descriptor is not replayable")]
fn dst_not_replayable(world: &mut Forge) {
    let (envelope, _digest): &(RunEnvelope, ContentHash) = world
        .external_assurance
        .dst_result
        .as_ref()
        .expect("dst result recorded")
        .as_ref()
        .expect("dst normalization succeeded");
    let replay: &ReplayDescriptor = envelope.replay.as_ref().expect("replay descriptor present");
    assert!(!replay.replayable, "expected non-replayable descriptor: {replay:?}");
}

#[then(expr = "the DST limitations name missing guarantees {string}")]
fn dst_limitations_name_missing(world: &mut Forge, expected: String) {
    let (envelope, _digest): &(RunEnvelope, ContentHash) = world
        .external_assurance
        .dst_result
        .as_ref()
        .expect("dst result recorded")
        .as_ref()
        .expect("dst normalization succeeded");
    let joined: String = envelope.limitations.join(" | ");
    for guarantee in listed(&expected, ", ") {
        assert!(joined.contains(&guarantee), "expected {guarantee} in limitations: {joined}");
    }
}

#[then(expr = "the DST limitations mention {string}")]
fn dst_limitations_mention(world: &mut Forge, needle: String) {
    let (envelope, _digest): &(RunEnvelope, ContentHash) = world
        .external_assurance
        .dst_result
        .as_ref()
        .expect("dst result recorded")
        .as_ref()
        .expect("dst normalization succeeded");
    let joined: String = envelope.limitations.join(" | ");
    assert!(joined.contains(&needle), "expected \"{needle}\" in limitations: {joined}");
}

#[then("DST normalization fails with an invalid request error")]
fn dst_fails_invalid_request(world: &mut Forge) {
    let result: &Result<(RunEnvelope, ContentHash), AdapterError> = world.external_assurance.dst_result.as_ref().expect("dst result recorded");
    assert!(matches!(result, Err(AdapterError::InvalidRequest(_))), "got {result:?}");
}

use std::collections::BTreeMap;

use cucumber::{then, when};
use specforge::assurance::{assess_claim, ClaimAssessment};
use specforge::formal::{normalize_formal_report, FormalBounds, FormalCheckReport};
use specforge::graph::{ArtifactId, ContentHash, GraphSnapshot, VerificationClaim};
use specforge::ledger::{EvidenceKind, EvidenceRecord};
use specforge::verify::{
    AdapterError, ClaimStatus, Counterexample, ExecutionTrace, FormalOutcome, ReplayDescriptor, RunEnvelope, RunState, TraceEvent, VerificationOutcome,
    VerificationRequest,
};

use super::formal::{base_formal_report, formal_counterexample_trace, formal_request};
use crate::graph::current_snapshot;
use crate::{listed, Forge};

fn formal_outcome_label(outcome: &VerificationOutcome) -> String {
    match outcome {
        VerificationOutcome::Evaluation(result) => format!("evaluation:{result}"),
        VerificationOutcome::Formal(formal) => match formal {
            FormalOutcome::Proved => "proved".to_string(),
            FormalOutcome::Refuted => "refuted".to_string(),
            FormalOutcome::BoundedSuccess { bound } => format!("bounded_success:{bound}"),
            FormalOutcome::BoundedFailure { bound } => format!("bounded_failure:{bound}"),
            FormalOutcome::Unknown => "unknown".to_string(),
            FormalOutcome::Timeout => "timeout".to_string(),
            FormalOutcome::Unsupported => "unsupported".to_string(),
        },
    }
}

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

#[when(expr = "a formal report with empty bounded success is normalized for {string}")]
fn normalize_empty_bound_formal(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = formal_request("RUN-FORMAL-EMPTYBOUND", &property_id, None);
    let report: FormalCheckReport = base_formal_report(request.property_id.clone(), FormalOutcome::BoundedSuccess { bound: "  ".to_string() });
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a proved formal report with undeclared assumption {string} is normalized for {string} and claim {string}")]
fn normalize_inconsistent_assumption_proved(world: &mut Forge, assumption_id: String, property_id: String, claim_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = formal_request("RUN-FORMAL-BADASM", &property_id, Some(&claim_id));
    let mut report: FormalCheckReport = base_formal_report(request.property_id.clone(), FormalOutcome::Proved);
    report.assumptions.push(ArtifactId::parse(&assumption_id).expect("valid extra assumption"));
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a bounded success formal report with undeclared assumption {string} is normalized for {string} and claim {string}")]
fn normalize_inconsistent_assumption_bounded(world: &mut Forge, assumption_id: String, property_id: String, claim_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = formal_request("RUN-FORMAL-BADASM-B", &property_id, Some(&claim_id));
    let mut report: FormalCheckReport = base_formal_report(
        request.property_id.clone(),
        FormalOutcome::BoundedSuccess {
            bound: "depth <= 8".to_string(),
        },
    );
    report.assumptions.push(ArtifactId::parse(&assumption_id).expect("valid extra assumption"));
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a bounded success formal report with zero max_states is normalized for {string}")]
fn normalize_zero_states_bounded(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = formal_request("RUN-FORMAL-ZEROSTATES", &property_id, None);
    let mut report: FormalCheckReport = base_formal_report(
        request.property_id.clone(),
        FormalOutcome::BoundedSuccess {
            bound: "states == 0".to_string(),
        },
    );
    report.bounds = Some(FormalBounds {
        max_depth: Some(4),
        max_states: Some(0),
        timeout_seconds: Some(10),
        description: "degenerate state bound".to_string(),
    });
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a proved formal report with a bounds description is normalized for {string}")]
fn normalize_proved_with_bounds_desc(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = formal_request("RUN-FORMAL-PROVED-BND", &property_id, None);
    let mut report: FormalCheckReport = base_formal_report(request.property_id.clone(), FormalOutcome::Proved);
    report.bounds = Some(FormalBounds {
        max_depth: None,
        max_states: None,
        timeout_seconds: None,
        description: "bounded to 3 replicas".to_string(),
    });
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a proved formal report without proof artifacts is normalized for {string}")]
fn normalize_proved_without_artifacts(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = formal_request("RUN-FORMAL-NOART", &property_id, None);
    let mut report: FormalCheckReport = base_formal_report(request.property_id.clone(), FormalOutcome::Proved);
    report.artifacts = Vec::new();
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a proved formal report with a counterexample trace is normalized for {string}")]
fn normalize_proved_with_ce(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = formal_request("RUN-FORMAL-PROVED-CE", &property_id, None);
    let mut report: FormalCheckReport = base_formal_report(request.property_id.clone(), FormalOutcome::Proved);
    report.counterexample_trace = Some(formal_counterexample_trace());
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a bounded success formal report with a counterexample trace is normalized for {string}")]
fn normalize_bounded_success_with_ce(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = formal_request("RUN-FORMAL-BSUCCESS-CE", &property_id, None);
    let mut report: FormalCheckReport = base_formal_report(
        request.property_id.clone(),
        FormalOutcome::BoundedSuccess {
            bound: "depth <= 10".to_string(),
        },
    );
    report.counterexample_trace = Some(formal_counterexample_trace());
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a refuted formal report with a discontinuous counterexample trace is normalized for {string}")]
fn normalize_refuted_discontinuous_trace(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = formal_request("RUN-FORMAL-BADTRACE", &property_id, None);
    let mut report: FormalCheckReport = base_formal_report(request.property_id.clone(), FormalOutcome::Refuted);
    report.counterexample_trace = Some(ExecutionTrace {
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
    });
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a formal report referencing property {string} is normalized for request {string}")]
fn normalize_mismatched_property_formal(world: &mut Forge, report_prop: String, request_prop: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = formal_request("RUN-FORMAL-MISMATCH", &request_prop, None);
    let report: FormalCheckReport = base_formal_report(ArtifactId::parse(&report_prop).expect("valid property id"), FormalOutcome::Proved);
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[then(expr = "the formal envelope has run state {string} and formal outcome {string}")]
fn formal_envelope_state_outcome(world: &mut Forge, expected_state: String, expected_outcome: String) {
    let (envelope, _digests): &(RunEnvelope, BTreeMap<String, ContentHash>) = world
        .external_assurance
        .formal_result
        .as_ref()
        .expect("formal result recorded")
        .as_ref()
        .expect("formal normalization succeeded");
    assert_eq!(run_state_text(envelope.run_state), expected_state);
    assert_eq!(formal_outcome_label(&envelope.outcome), expected_outcome);
}

#[then(expr = "the formal artifact digests include {string}")]
fn formal_artifact_digests_include(world: &mut Forge, names: String) {
    let (_envelope, digests): &(RunEnvelope, BTreeMap<String, ContentHash>) = world
        .external_assurance
        .formal_result
        .as_ref()
        .expect("formal result recorded")
        .as_ref()
        .expect("formal normalization succeeded");
    for name in listed(&names, ", ") {
        assert!(digests.contains_key(&name), "missing artifact digest {name} in {digests:?}");
    }
}

#[then(expr = "the formal limitations retain bound {string}")]
fn formal_limitations_retain_bound(world: &mut Forge, bound: String) {
    let (envelope, _digests): &(RunEnvelope, BTreeMap<String, ContentHash>) = world
        .external_assurance
        .formal_result
        .as_ref()
        .expect("formal result recorded")
        .as_ref()
        .expect("formal normalization succeeded");
    let joined: String = envelope.limitations.join(" | ");
    assert!(joined.contains(&bound), "expected bound {bound} in limitations: {joined}");
}

#[then("the formal envelope records a finding and counterexample")]
fn formal_records_finding_and_ce(world: &mut Forge) {
    let (envelope, _digests): &(RunEnvelope, BTreeMap<String, ContentHash>) = world
        .external_assurance
        .formal_result
        .as_ref()
        .expect("formal result recorded")
        .as_ref()
        .expect("formal normalization succeeded");
    assert!(envelope.finding.is_some(), "expected formal finding");
    assert!(envelope.original_counterexample.is_some(), "expected formal counterexample");
}

#[then("the formal counterexample is not replayable and discloses a replay limitation")]
fn formal_ce_not_replayable(world: &mut Forge) {
    let (envelope, _digests): &(RunEnvelope, BTreeMap<String, ContentHash>) = world
        .external_assurance
        .formal_result
        .as_ref()
        .expect("formal result recorded")
        .as_ref()
        .expect("formal normalization succeeded");
    let replay: &ReplayDescriptor = envelope.replay.as_ref().expect("replay descriptor present");
    assert!(!replay.replayable, "expected formal replay descriptor to be non-replayable");
    let ce: &Counterexample = envelope.original_counterexample.as_ref().expect("counterexample present");
    assert!(!ce.replay.replayable, "expected counterexample replay descriptor to be non-replayable");
    let joined: String = envelope.limitations.join(" | ");
    assert!(
        joined.contains("formal counterexample replay is not declared as supported by this adapter"),
        "missing replay limitation in: {joined}"
    );
}

fn seal_and_assess_formal(world: &mut Forge, claim_id: &str) -> ClaimAssessment {
    world.external_assurance.evidence_counter += 1;
    let id: String = format!("EV-FORMAL-{}", world.external_assurance.evidence_counter);
    let (envelope, digests): (RunEnvelope, BTreeMap<String, ContentHash>) = world
        .external_assurance
        .formal_result
        .as_ref()
        .expect("formal result recorded")
        .as_ref()
        .expect("formal normalization succeeded")
        .clone();
    let record: EvidenceRecord = {
        let snapshot: &GraphSnapshot = current_snapshot(world);
        EvidenceRecord::from_envelope(id, EvidenceKind::FormalCheck, "2026-11-24T08:05:00Z".to_string(), snapshot, &envelope, digests)
            .expect("formal envelope seals")
    };
    world.evidence_ledger.append_record(record).expect("formal record appends");
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let claim: &VerificationClaim = snapshot.claim(claim_id).expect("claim exists");
    assess_claim(claim, snapshot, &world.evidence_ledger)
}

#[then(expr = "the formal envelope seals into the evidence ledger and supports {string} as unbounded proof")]
fn formal_supports_unbounded(world: &mut Forge, claim_id: String) {
    let assessment: ClaimAssessment = seal_and_assess_formal(world, &claim_id);
    assert_eq!(assessment.status, ClaimStatus::Supported);
    assert!(assessment.unbounded_proof);
}

#[then(expr = "the formal envelope seals into the evidence ledger and supports {string} without unbounded proof")]
fn formal_supports_bounded(world: &mut Forge, claim_id: String) {
    let assessment: ClaimAssessment = seal_and_assess_formal(world, &claim_id);
    assert_eq!(assessment.status, ClaimStatus::Supported);
    assert!(!assessment.unbounded_proof);
}

#[then("formal normalization fails with an invalid request error")]
fn formal_fails_invalid_request(world: &mut Forge) {
    let result: &Result<(RunEnvelope, BTreeMap<String, ContentHash>), AdapterError> =
        world.external_assurance.formal_result.as_ref().expect("formal result recorded");
    assert!(matches!(result, Err(AdapterError::InvalidRequest(_))), "got {result:?}");
}

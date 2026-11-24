mod validate;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use self::validate::{
    format_bounds_limitation, validate_counterexample_trace, validate_formal_inputs, validate_positive_assumptions, ValidatedFormalInputs,
    FORMAL_PROPERTY_REF_KEY,
};
use crate::graph::{ArtifactId, ContentHash, GraphSnapshot, Property, VerificationMethod};
use crate::verify::{
    AdapterCapabilities, AdapterError, Counterexample, ExecutionTrace, FindingRecord, FormalOutcome, ReplayDescriptor, RootCause, RunEnvelope, RunState,
    TraceEvent, VerificationOutcome, VerificationRequest, ViolationDetail,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalPropertyRef {
    pub property_id: ArtifactId,
    pub external_tool: String,
    pub module: String,
    pub symbol: String,
    pub digest: Option<ContentHash>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalBounds {
    pub max_depth: Option<usize>,
    pub max_states: Option<usize>,
    pub timeout_seconds: Option<u64>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalArtifactRef {
    pub name: String,
    pub kind: String,
    pub digest: ContentHash,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalCheckReport {
    pub adapter_id: String,
    pub adapter_version: String,
    pub engine_name: String,
    pub engine_version: String,
    pub property_ref: FormalPropertyRef,
    pub outcome: FormalOutcome,
    pub bounds: Option<FormalBounds>,
    #[serde(default)]
    pub assumptions: Vec<ArtifactId>,
    #[serde(default)]
    pub artifacts: Vec<FormalArtifactRef>,
    pub counterexample_trace: Option<ExecutionTrace>,
    pub raw_status: String,
    #[serde(default)]
    pub limitations: Vec<String>,
}

/// When a formal verification engine integrates with SpecForge, `FormalAdapter` shall declare its capabilities and return structured formal check reports.
pub trait FormalAdapter {
    fn capabilities(&self) -> AdapterCapabilities;
    fn check_formal(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<FormalCheckReport, AdapterError>;
}

fn attach_property_ref_digest(report: &FormalCheckReport, artifact_digests: &mut BTreeMap<String, ContentHash>) -> Result<(), AdapterError> {
    let digest: ContentHash = match &report.property_ref.digest {
        Some(declared) => declared.clone(),
        None => ContentHash::of_canonical_json(&report.property_ref).map_err(AdapterError::Execution)?,
    };
    artifact_digests.insert(FORMAL_PROPERTY_REF_KEY.to_string(), digest);
    Ok(())
}

fn build_formal_refutation(
    snapshot: &GraphSnapshot,
    request: &VerificationRequest,
    report: &FormalCheckReport,
    property: &Property,
    property_digest: &ContentHash,
    summary: String,
    limitations: &mut Vec<String>,
) -> Result<(FindingRecord, Option<Counterexample>, Option<ReplayDescriptor>), AdapterError> {
    let finding_id: String = format!("FND-{}-{}", request.run_id, property.id);
    let Some(trace): Option<&ExecutionTrace> = report.counterexample_trace.as_ref() else {
        let finding: FindingRecord = FindingRecord {
            id: finding_id,
            property_id: property.id.clone(),
            run_id: request.run_id.clone(),
            root_cause: RootCause::Unresolved,
            counterexample_id: None,
            summary,
        };
        return Ok((finding, None, None));
    };
    validate_counterexample_trace(trace)?;
    let counterexample_id: String = format!("CE-{}-{}", request.run_id, property.id);
    let events: Vec<String> = trace.events.iter().map(|event: &TraceEvent| event.event.clone()).collect();
    let replay: ReplayDescriptor = ReplayDescriptor {
        engine: report.engine_name.clone(),
        engine_version: report.engine_version.clone(),
        seed: None,
        replay_token: Some(format!(
            "formal:{}:{}::{}",
            report.property_ref.external_tool, report.property_ref.module, report.property_ref.symbol
        )),
        events,
        determinism: "formal counterexample witness trace".to_string(),
        replayable: false,
    };
    limitations.push("formal counterexample replay is not declared as supported by this adapter".to_string());
    let (step, observed): (usize, String) = trace.events.last().map_or_else(
        || (0, format!("initial_state == {}", trace.initial_state)),
        |last: &TraceEvent| (last.step, format!("state == {} after event {}", last.to_state, last.event)),
    );
    let violation: ViolationDetail = ViolationDetail {
        step,
        observed,
        expected: property.predicate.expression.clone(),
    };
    let counterexample: Counterexample = Counterexample {
        id: counterexample_id.clone(),
        property_id: property.id.clone(),
        property_digest: property_digest.clone(),
        finding_id: finding_id.clone(),
        run_id: request.run_id.clone(),
        snapshot_id: snapshot.snapshot_id.clone(),
        initial_state: trace.initial_state.clone(),
        events: trace.events.clone(),
        violation,
        replay: replay.clone(),
        original_id: None,
        minimization: None,
    };
    let finding: FindingRecord = FindingRecord {
        id: finding_id,
        property_id: property.id.clone(),
        run_id: request.run_id.clone(),
        root_cause: RootCause::Unresolved,
        counterexample_id: Some(counterexample_id),
        summary,
    };
    Ok((finding, Some(counterexample), Some(replay)))
}

/// When a formal check report is normalized against a snapshot and request, `normalize_formal_report` shall validate references, assumptions, bounds, traces, and artifacts, preserve all seven formal outcomes losslessly, and collect artifact digests.
pub fn normalize_formal_report(
    snapshot: &GraphSnapshot,
    request: &VerificationRequest,
    report: &FormalCheckReport,
) -> Result<(RunEnvelope, BTreeMap<String, ContentHash>), AdapterError> {
    let mut validated: ValidatedFormalInputs<'_> = validate_formal_inputs(snapshot, request, report)?;
    let property: &Property = validated.property;
    let mut limitations: Vec<String> = report.limitations.clone();

    let (run_state, finding, counterexample, replay): (RunState, Option<FindingRecord>, Option<Counterexample>, Option<ReplayDescriptor>) =
        match &report.outcome {
            FormalOutcome::Proved => {
                if report.counterexample_trace.is_some() {
                    return Err(AdapterError::InvalidRequest(
                        "proved formal report must not attach a counterexample trace".to_string(),
                    ));
                }
                validate_positive_assumptions(report, property, &validated.claims, "proved")?;
                if let Some(bounds) = &report.bounds {
                    if bounds.max_depth.is_some() || bounds.max_states.is_some() || !bounds.description.trim().is_empty() {
                        return Err(AdapterError::InvalidRequest(
                            "formal report declaring depth, state, or descriptive bounds cannot report unbounded Proved".to_string(),
                        ));
                    }
                }
                if report.artifacts.is_empty() {
                    return Err(AdapterError::InvalidRequest(
                        "proved formal report must attach at least one proof artifact digest".to_string(),
                    ));
                }
                attach_property_ref_digest(report, &mut validated.artifact_digests)?;
                (RunState::Completed, None, None, None)
            }
            FormalOutcome::Refuted => {
                let summary: String = format!(
                    "formal engine {} ({}::{}) refuted property {}",
                    report.property_ref.external_tool, report.property_ref.module, report.property_ref.symbol, property.id
                );
                let (finding, ce, replay): (FindingRecord, Option<Counterexample>, Option<ReplayDescriptor>) =
                    build_formal_refutation(snapshot, request, report, property, &validated.property_digest, summary, &mut limitations)?;
                (RunState::Completed, Some(finding), ce, replay)
            }
            FormalOutcome::BoundedSuccess { bound } => {
                if bound.trim().is_empty() {
                    return Err(AdapterError::InvalidRequest("bounded_success outcome requires a non-empty bound".to_string()));
                }
                if report.counterexample_trace.is_some() {
                    return Err(AdapterError::InvalidRequest(
                        "bounded_success formal report must not attach a counterexample trace".to_string(),
                    ));
                }
                validate_positive_assumptions(report, property, &validated.claims, "bounded_success")?;
                if let Some(bounds) = &report.bounds {
                    if bounds.max_states == Some(0) || bounds.timeout_seconds == Some(0) {
                        return Err(AdapterError::InvalidRequest(
                            "bounded_success formal report must not declare zero max_states or zero timeout_seconds".to_string(),
                        ));
                    }
                }
                attach_property_ref_digest(report, &mut validated.artifact_digests)?;
                limitations.push(format_bounds_limitation(bound, report.bounds.as_ref(), true));
                (RunState::Completed, None, None, None)
            }
            FormalOutcome::BoundedFailure { bound } => {
                if bound.trim().is_empty() {
                    return Err(AdapterError::InvalidRequest("bounded_failure outcome requires a non-empty bound".to_string()));
                }
                limitations.push(format_bounds_limitation(bound, report.bounds.as_ref(), false));
                let summary: String = format!(
                    "formal engine {} ({}::{}) refuted property {} within bound \"{bound}\"",
                    report.property_ref.external_tool, report.property_ref.module, report.property_ref.symbol, property.id
                );
                let (finding, ce, replay): (FindingRecord, Option<Counterexample>, Option<ReplayDescriptor>) =
                    build_formal_refutation(snapshot, request, report, property, &validated.property_digest, summary, &mut limitations)?;
                (RunState::Completed, Some(finding), ce, replay)
            }
            FormalOutcome::Unknown => {
                limitations.push(format!(
                    "formal tool {} returned unknown for {}::{}; result is inconclusive",
                    report.property_ref.external_tool, report.property_ref.module, report.property_ref.symbol
                ));
                (RunState::Inconclusive, None, None, None)
            }
            FormalOutcome::Timeout => {
                let timeout_note: String = report.bounds.as_ref().and_then(|bounds: &FormalBounds| bounds.timeout_seconds).map_or_else(
                    || "formal solver timed out before completing verification; result is inconclusive".to_string(),
                    |seconds: u64| format!("formal solver timed out after {seconds}s; result is inconclusive"),
                );
                limitations.push(timeout_note);
                (RunState::Inconclusive, None, None, None)
            }
            FormalOutcome::Unsupported => {
                limitations.push(format!(
                    "formal tool {} reported {}::{} as unsupported; result is inconclusive",
                    report.property_ref.external_tool, report.property_ref.module, report.property_ref.symbol
                ));
                (RunState::Inconclusive, None, None, None)
            }
        };

    let envelope: RunEnvelope = RunEnvelope {
        run_id: request.run_id.clone(),
        activity_id: request.activity_id.clone(),
        adapter_id: report.adapter_id.clone(),
        adapter_version: report.adapter_version.clone(),
        engine_name: report.engine_name.clone(),
        engine_version: report.engine_version.clone(),
        snapshot_id: snapshot.snapshot_id.clone(),
        source_revisions: snapshot.sources.clone(),
        property_id: property.id.clone(),
        claim_ids: request.claim_ids.clone(),
        method: VerificationMethod::Formal,
        scope: property.scope.clone(),
        assumptions: report.assumptions.clone(),
        run_state,
        outcome: VerificationOutcome::Formal(report.outcome.clone()),
        evaluation: None,
        finding,
        original_counterexample: counterexample,
        minimized_counterexample: None,
        replay,
        limitations,
        raw_status: report.raw_status.clone(),
    };

    Ok((envelope, validated.artifact_digests))
}

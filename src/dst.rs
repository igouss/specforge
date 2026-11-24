use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::graph::{ContentHash, GraphSnapshot, Property, StateMachine, VerificationMethod};
use crate::model::evaluate_property;
use crate::verify::{
    AdapterCapabilities, AdapterError, Counterexample, EvaluationResult, ExecutionTrace, FindingRecord, PropertyEvaluation, ReplayDescriptor, RunEnvelope,
    RunState, TraceEvent, VerificationOutcome, VerificationRequest,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterminismGuarantee {
    pub deterministic_rng: bool,
    pub deterministic_scheduler: bool,
    pub virtual_clock: bool,
    pub deterministic_faults: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl DeterminismGuarantee {
    /// When all four simulation determinism controls hold, `is_fully_deterministic` shall return true.
    pub fn is_fully_deterministic(&self) -> bool {
        self.deterministic_rng && self.deterministic_scheduler && self.virtual_clock && self.deterministic_faults
    }

    /// When any determinism control is absent, `missing_guarantees` shall list the missing guarantee names in deterministic order.
    pub fn missing_guarantees(&self) -> Vec<&'static str> {
        let mut missing: Vec<&'static str> = Vec::new();
        if !self.deterministic_rng {
            missing.push("deterministic_rng");
        }
        if !self.deterministic_scheduler {
            missing.push("deterministic_scheduler");
        }
        if !self.virtual_clock {
            missing.push("virtual_clock");
        }
        if !self.deterministic_faults {
            missing.push("deterministic_faults");
        }
        missing
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerDecision {
    pub step: usize,
    pub actor: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaultInjection {
    pub step: usize,
    pub target: String,
    pub fault_kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DstReplayMetadata {
    pub seed: Option<u64>,
    #[serde(default)]
    pub scheduler_decisions: Vec<SchedulerDecision>,
    #[serde(default)]
    pub fault_injections: Vec<FaultInjection>,
    #[serde(default)]
    pub runtime_config: BTreeMap<String, String>,
    pub determinism: DeterminismGuarantee,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DstRunReport {
    pub adapter_id: String,
    pub adapter_version: String,
    pub engine_name: String,
    pub engine_version: String,
    pub run_state: RunState,
    pub trace: ExecutionTrace,
    pub replay_metadata: DstReplayMetadata,
    pub raw_status: String,
    #[serde(default)]
    pub limitations: Vec<String>,
}

/// When a deterministic simulation testing engine integrates with SpecForge, `DstAdapter` shall declare its capabilities, execute simulations, and replay DST counterexamples into structured reports.
pub trait DstAdapter {
    fn capabilities(&self) -> AdapterCapabilities;
    fn execute_dst(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<DstRunReport, AdapterError>;
    fn replay_dst(&self, snapshot: &GraphSnapshot, counterexample: &Counterexample, run_id: &str) -> Result<DstRunReport, AdapterError>;
}

fn validate_dst_inputs<'a>(
    snapshot: &'a GraphSnapshot,
    request: &VerificationRequest,
    report: &DstRunReport,
) -> Result<(&'a Property, ContentHash), AdapterError> {
    if request.run_id.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("run_id must not be empty".to_string()));
    }
    if request.activity_id.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("activity_id must not be empty".to_string()));
    }
    if report.adapter_id.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("adapter_id must not be empty".to_string()));
    }
    if report.adapter_version.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("adapter_version must not be empty".to_string()));
    }
    if report.engine_name.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("engine_name must not be empty".to_string()));
    }
    if report.engine_version.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("engine_version must not be empty".to_string()));
    }
    if report.raw_status.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("raw_status must not be empty".to_string()));
    }
    for claim_id in &request.claim_ids {
        if snapshot.claim(claim_id.as_str()).is_none() {
            return Err(AdapterError::InvalidRequest(format!("unknown claim {claim_id}")));
        }
    }
    let property: &Property = snapshot
        .property(request.property_id.as_str())
        .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown property {}", request.property_id)))?;
    let property_digest: ContentHash = snapshot
        .artifact_digest(property.id.as_str())
        .cloned()
        .ok_or_else(|| AdapterError::InvalidRequest(format!("missing digest for property {}", property.id)))?;
    let machine: &StateMachine = snapshot
        .state_machine(property.subject.as_str())
        .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown subject state machine {}", property.subject)))?;
    if report.trace.initial_state.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("trace initial_state must not be empty".to_string()));
    }
    if !machine.states.contains(&report.trace.initial_state) {
        return Err(AdapterError::InvalidRequest(format!(
            "trace initial_state \"{}\" is not declared in state machine {}",
            report.trace.initial_state, machine.id
        )));
    }
    let mut last_virtual_time: Option<u64> = None;
    for event in &report.trace.events {
        if event.event.trim().is_empty() {
            return Err(AdapterError::InvalidRequest(format!(
                "trace event at step {} must have a non-empty event name",
                event.step
            )));
        }
        if event.from_state.trim().is_empty() {
            return Err(AdapterError::InvalidRequest(format!(
                "trace event at step {} must have a non-empty from_state",
                event.step
            )));
        }
        if event.to_state.trim().is_empty() {
            return Err(AdapterError::InvalidRequest(format!(
                "trace event at step {} must have a non-empty to_state",
                event.step
            )));
        }
        if report.replay_metadata.determinism.virtual_clock {
            if let Some(vt) = event.virtual_time {
                if let Some(previous) = last_virtual_time {
                    if vt < previous {
                        return Err(AdapterError::InvalidRequest(format!(
                            "virtual_time decreased from {previous} to {vt} at step {}",
                            event.step
                        )));
                    }
                }
                last_virtual_time = Some(vt);
            }
        }
    }
    Ok((property, property_digest))
}

fn run_state_label(state: RunState) -> &'static str {
    match state {
        RunState::Requested => "requested",
        RunState::Started => "started",
        RunState::Completed => "completed",
        RunState::Failed => "failed",
        RunState::Cancelled => "cancelled",
        RunState::Inconclusive => "inconclusive",
    }
}

/// When a DST run report is normalized against a snapshot and request, `normalize_dst_report` shall validate inputs, digest replay metadata, evaluate the property on the trace, enforce EARS-032 replayability, and prevent incomplete runs from reporting satisfied.
pub fn normalize_dst_report(
    snapshot: &GraphSnapshot,
    request: &VerificationRequest,
    report: &DstRunReport,
) -> Result<(RunEnvelope, ContentHash), AdapterError> {
    let (property, property_digest): (&Property, ContentHash) = validate_dst_inputs(snapshot, request, report)?;
    let metadata_digest: ContentHash = ContentHash::of_canonical_json(&report.replay_metadata).map_err(AdapterError::Execution)?;

    let replay_events: Vec<String> = report.trace.events.iter().map(|event: &TraceEvent| event.event.clone()).collect();
    let missing_guarantees: Vec<&'static str> = report.replay_metadata.determinism.missing_guarantees();
    let has_replay_controls: bool = report.replay_metadata.seed.is_some() || !replay_events.is_empty();
    let replayable: bool = report.replay_metadata.determinism.is_fully_deterministic() && has_replay_controls;

    let determinism_summary: String = if missing_guarantees.is_empty() {
        "fully_deterministic(rng, scheduler, virtual_clock, faults)".to_string()
    } else {
        format!("non_deterministic(missing: {})", missing_guarantees.join(", "))
    };

    let replay: ReplayDescriptor = ReplayDescriptor {
        engine: report.engine_name.clone(),
        engine_version: report.engine_version.clone(),
        seed: report.replay_metadata.seed,
        replay_token: Some(format!("dst-metadata:{metadata_digest}")),
        events: replay_events,
        determinism: determinism_summary,
        replayable,
    };

    let mut limitations: Vec<String> = report.limitations.clone();
    if !missing_guarantees.is_empty() {
        limitations.push(format!(
            "simulation is not replayable: missing determinism guarantees: {}",
            missing_guarantees.join(", ")
        ));
    } else if !has_replay_controls {
        limitations.push("simulation is not replayable: neither seed nor trace events were recorded".to_string());
    }

    let (mut evaluation, finding, counterexample): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) = evaluate_property(
        property,
        &property_digest,
        &report.trace,
        &request.run_id,
        &snapshot.snapshot_id,
        replay.clone(),
    );

    let (run_state, outcome, final_finding, final_counterexample): (RunState, VerificationOutcome, Option<FindingRecord>, Option<Counterexample>) =
        match report.run_state {
            RunState::Completed => match evaluation.result {
                EvaluationResult::Satisfied => {
                    limitations.push("finite deterministic simulation trace; does not prove universal correctness".to_string());
                    (RunState::Completed, VerificationOutcome::Evaluation(EvaluationResult::Satisfied), None, None)
                }
                EvaluationResult::Violated => (
                    RunState::Completed,
                    VerificationOutcome::Evaluation(EvaluationResult::Violated),
                    finding,
                    counterexample,
                ),
                EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated => {
                    limitations.push(format!(
                        "property evaluation over simulation trace was {}; cannot establish satisfaction",
                        evaluation.result
                    ));
                    (RunState::Inconclusive, VerificationOutcome::Evaluation(evaluation.result), None, None)
                }
                EvaluationResult::InvalidEvaluation => {
                    limitations.push("property evaluator rejected the simulation predicate or trace as invalid".to_string());
                    (
                        RunState::Failed,
                        VerificationOutcome::Evaluation(EvaluationResult::InvalidEvaluation),
                        None,
                        None,
                    )
                }
            },
            RunState::Inconclusive | RunState::Failed | RunState::Cancelled | RunState::Requested | RunState::Started => {
                let state_name: &str = run_state_label(report.run_state);
                match evaluation.result {
                    EvaluationResult::Violated => (
                        report.run_state,
                        VerificationOutcome::Evaluation(EvaluationResult::Violated),
                        finding,
                        counterexample,
                    ),
                    EvaluationResult::InvalidEvaluation => {
                        limitations.push("property evaluator rejected the simulation predicate or trace as invalid".to_string());
                        (
                            report.run_state,
                            VerificationOutcome::Evaluation(EvaluationResult::InvalidEvaluation),
                            None,
                            None,
                        )
                    }
                    EvaluationResult::Satisfied => {
                        evaluation.result = EvaluationResult::Inconclusive;
                        limitations.push(format!("simulation run state was {state_name}; partial trace cannot establish satisfaction"));
                        (report.run_state, VerificationOutcome::Evaluation(EvaluationResult::Inconclusive), None, None)
                    }
                    EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated => {
                        limitations.push(format!("simulation run state was {state_name}; evaluation did not complete"));
                        (report.run_state, VerificationOutcome::Evaluation(evaluation.result), None, None)
                    }
                }
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
        method: VerificationMethod::DeterministicSimulation,
        scope: property.scope.clone(),
        assumptions: property.assumptions.clone(),
        run_state,
        outcome,
        evaluation: Some(evaluation),
        finding: final_finding,
        original_counterexample: final_counterexample,
        minimized_counterexample: None,
        replay: Some(replay),
        limitations,
        raw_status: report.raw_status.clone(),
    };

    Ok((envelope, metadata_digest))
}

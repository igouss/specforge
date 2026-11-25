use std::collections::BTreeMap;

use specforge::graph::{ArtifactId, ContentHash, GraphSnapshot, Property, StateMachine, VerificationClaim, VerificationMethod};
use specforge::ledger::{EvidenceKind, EvidencePayload, EvidenceRecord, ProducerMeta};
use specforge::model::evaluate_property;
use specforge::orchestrator::OrchestratorError;
use specforge::verify::{
    AdapterCapabilities, AdapterError, Counterexample, EvaluationResult, ExecutionTrace, FindingRecord, PropertyEvaluation, ReplayDescriptor, RunEnvelope,
    RunState, TraceEvent, VerificationAdapter, VerificationOutcome, VerificationRequest,
};

use crate::graph::current_snapshot;
use crate::Forge;

pub(super) fn safe_trace(machine: &StateMachine, events: &[String]) -> ExecutionTrace {
    let mut cur: String = machine.initial.clone();
    let trace_events: Vec<TraceEvent> = events
        .iter()
        .enumerate()
        .map(|(i, ev): (usize, &String)| {
            let next: String = match (cur.as_str(), ev.as_str()) {
                ("queued", "deliver") => "delivered".to_string(),
                ("delivered", "ack") => "acknowledged".to_string(),
                _ => cur.clone(),
            };
            let item: TraceEvent = TraceEvent {
                step: i + 1,
                event: ev.clone(),
                from_state: cur.clone(),
                to_state: next.clone(),
                inputs: BTreeMap::new(),
                fault: None,
                virtual_time: Some((i + 1) as u64),
            };
            cur = next;
            item
        })
        .collect();
    ExecutionTrace {
        initial_state: machine.initial.clone(),
        events: trace_events,
    }
}

#[derive(Clone, Copy)]
pub(super) enum StubFault {
    None,
    WrongRunId,
    WrongMethod,
    WrongActivity,
    WrongClaims,
    CompletedInconclusive,
}

pub(super) struct StubAdapter {
    pub(super) method: VerificationMethod,
    pub(super) fault: StubFault,
}

impl VerificationAdapter for StubAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            adapter_id: "specforge-stub-adapter".to_string(),
            adapter_version: "1.0.0".to_string(),
            engine_name: "specforge-dst-sim".to_string(),
            engine_version: "1.0.0".to_string(),
            methods: vec![self.method],
            supports_replay: true,
            supports_shrinking: false,
            supports_coverage: false,
        }
    }

    fn validate(&self, snapshot: &GraphSnapshot, req: &VerificationRequest) -> Result<(), AdapterError> {
        snapshot
            .property(req.property_id.as_str())
            .map(|_: &Property| ())
            .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown property {}", req.property_id)))
    }

    fn execute(&self, snapshot: &GraphSnapshot, req: &VerificationRequest) -> Result<RunEnvelope, AdapterError> {
        let prop: &Property = snapshot
            .property(req.property_id.as_str())
            .ok_or_else(|| AdapterError::InvalidRequest("unknown property".to_string()))?;
        let digest: &ContentHash = snapshot
            .artifact_digest(prop.id.as_str())
            .ok_or_else(|| AdapterError::InvalidRequest("missing digest".to_string()))?;
        let sm: &StateMachine = snapshot
            .state_machine(prop.subject.as_str())
            .ok_or_else(|| AdapterError::InvalidRequest("unknown machine".to_string()))?;
        let events: Vec<String> = vec!["deliver".to_string(), "ack".to_string()];
        let trace: ExecutionTrace = safe_trace(sm, &events);
        let seed: u64 = req.seed.unwrap_or(0);
        let replay: ReplayDescriptor = ReplayDescriptor {
            engine: "specforge-dst-sim".to_string(),
            engine_version: "1.0.0".to_string(),
            seed: Some(seed),
            replay_token: Some(format!("dst-seed:{seed}")),
            events,
            determinism: "deterministic simulation seed".to_string(),
            replayable: true,
        };
        let (eval, finding, ce): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) =
            evaluate_property(prop, digest, &trace, &req.run_id, &snapshot.snapshot_id, replay.clone());
        let run_id: String = match self.fault {
            StubFault::WrongRunId => "RUN-WRONG-ID".to_string(),
            StubFault::None | StubFault::WrongMethod | StubFault::WrongActivity | StubFault::WrongClaims | StubFault::CompletedInconclusive => {
                req.run_id.clone()
            }
        };
        let activity_id: String = match self.fault {
            StubFault::WrongActivity => "ACT-WRONG-ID".to_string(),
            StubFault::None | StubFault::WrongRunId | StubFault::WrongMethod | StubFault::WrongClaims | StubFault::CompletedInconclusive => {
                req.activity_id.clone()
            }
        };
        let method: VerificationMethod = match self.fault {
            StubFault::WrongMethod => VerificationMethod::Formal,
            StubFault::None | StubFault::WrongRunId | StubFault::WrongActivity | StubFault::WrongClaims | StubFault::CompletedInconclusive => self.method,
        };
        let claim_ids: Vec<ArtifactId> = match self.fault {
            StubFault::WrongClaims => Vec::new(),
            StubFault::None | StubFault::WrongRunId | StubFault::WrongMethod | StubFault::WrongActivity | StubFault::CompletedInconclusive => {
                req.claim_ids.clone()
            }
        };
        let outcome: VerificationOutcome = match self.fault {
            StubFault::CompletedInconclusive => VerificationOutcome::Evaluation(EvaluationResult::Inconclusive),
            StubFault::None | StubFault::WrongRunId | StubFault::WrongMethod | StubFault::WrongActivity | StubFault::WrongClaims => {
                VerificationOutcome::Evaluation(eval.result)
            }
        };
        Ok(RunEnvelope {
            run_id,
            activity_id,
            adapter_id: "specforge-stub-adapter".to_string(),
            adapter_version: "1.0.0".to_string(),
            engine_name: "specforge-dst-sim".to_string(),
            engine_version: "1.0.0".to_string(),
            snapshot_id: snapshot.snapshot_id.clone(),
            source_revisions: snapshot.sources.clone(),
            property_id: prop.id.clone(),
            claim_ids,
            method,
            scope: prop.scope.clone(),
            assumptions: prop.assumptions.clone(),
            run_state: RunState::Completed,
            outcome,
            evaluation: Some(eval),
            finding,
            original_counterexample: ce,
            minimized_counterexample: None,
            replay: Some(replay),
            limitations: vec!["finite deterministic simulation run".to_string()],
            raw_status: "dst:satisfied".to_string(),
        })
    }

    fn replay(&self, _snapshot: &GraphSnapshot, _ce: &Counterexample, _run_id: &str) -> Result<RunEnvelope, AdapterError> {
        Err(AdapterError::Unsupported("replay unused".to_string()))
    }
}

pub(super) fn state_label(state: RunState) -> &'static str {
    match state {
        RunState::Requested => "requested",
        RunState::Started => "started",
        RunState::Completed => "completed",
        RunState::Failed => "failed",
        RunState::Cancelled => "cancelled",
        RunState::Inconclusive => "inconclusive",
    }
}

pub(super) fn assert_invalid(result: Result<(), OrchestratorError>) {
    assert!(matches!(result, Err(OrchestratorError::InvalidTransition { .. })), "{result:?}");
}

pub(super) fn append_pair(world: &mut Forge, prefix: &str, prop_id: &str, claim_id: &str, replay_token: Option<String>, events: Vec<String>) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let claim: &VerificationClaim = snapshot.claim(claim_id).expect("claim exists");
    let replay: ReplayDescriptor = ReplayDescriptor {
        engine: "custom-sim".to_string(),
        engine_version: "1.0.0".to_string(),
        seed: None,
        replay_token,
        events,
        determinism: "deterministic replay".to_string(),
        replayable: true,
    };
    for idx in 1..=2 {
        let id: String = format!("{prefix}-{idx}");
        let payload: EvidencePayload = EvidencePayload {
            id: id.clone(),
            kind: EvidenceKind::PropertyEvaluation,
            producer: ProducerMeta {
                name: "custom-sim".to_string(),
                version: "1.0.0".to_string(),
            },
            adapter: ProducerMeta {
                name: "custom-adapter".to_string(),
                version: "1.0.0".to_string(),
            },
            timestamp: "2026-06-01T12:15:00Z".to_string(),
            snapshot_id: snapshot.snapshot_id.clone(),
            source_revisions: snapshot.sources.clone(),
            dependencies: snapshot.dependency_pins_for_claim(claim.id.as_str()),
            run_id: format!("RUN-{id}"),
            activity_id: "ACT-CUSTOM-1".to_string(),
            method: VerificationMethod::DeterministicSimulation,
            property_id: Some(ArtifactId::parse(prop_id).expect("valid prop id")),
            claim_ids: vec![claim.id.clone()],
            scope: claim.scope.clone(),
            assumptions: claim.assumptions.clone(),
            run_state: RunState::Completed,
            outcome: VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
            counterexample_id: None,
            replay: Some(replay.clone()),
            artifact_digests: BTreeMap::new(),
            limitations: vec!["finite run".to_string()],
        };
        let record: EvidenceRecord = EvidenceRecord::seal(payload).expect("seals");
        world.evidence_ledger.append_record(record).expect("appends");
    }
}

use std::collections::BTreeMap;

use cucumber::when;
use specforge::dst::{normalize_dst_report, DeterminismGuarantee, DstAdapter, DstReplayMetadata, DstRunReport, FaultInjection, SchedulerDecision};
use specforge::graph::{ArtifactId, ContentHash, GraphSnapshot, VerificationMethod};
use specforge::verify::{AdapterCapabilities, AdapterError, Counterexample, ExecutionTrace, RunEnvelope, RunState, TraceEvent, VerificationRequest};

use crate::graph::current_snapshot;
use crate::Forge;

struct MockDstAdapter {
    inject_redelivery_fault: bool,
}

pub(super) fn deterministic_guarantee() -> DeterminismGuarantee {
    DeterminismGuarantee {
        deterministic_rng: true,
        deterministic_scheduler: true,
        virtual_clock: true,
        deterministic_faults: true,
        notes: vec!["discrete-event virtual clock runtime".to_string()],
    }
}

pub(super) fn build_dst_trace(inject_fault: bool) -> ExecutionTrace {
    let mut events: Vec<TraceEvent> = vec![
        TraceEvent {
            step: 1,
            event: "deliver".to_string(),
            from_state: "queued".to_string(),
            to_state: "delivered".to_string(),
            inputs: BTreeMap::new(),
            fault: None,
            virtual_time: Some(10),
        },
        TraceEvent {
            step: 2,
            event: "ack".to_string(),
            from_state: "delivered".to_string(),
            to_state: "acknowledged".to_string(),
            inputs: BTreeMap::new(),
            fault: None,
            virtual_time: Some(25),
        },
    ];
    if inject_fault {
        events.push(TraceEvent {
            step: 3,
            event: "redeliver".to_string(),
            from_state: "acknowledged".to_string(),
            to_state: "redelivered".to_string(),
            inputs: BTreeMap::new(),
            fault: Some("disk_fsync_stall".to_string()),
            virtual_time: Some(40),
        });
    }
    ExecutionTrace {
        initial_state: "queued".to_string(),
        events,
    }
}

pub(super) fn build_replay_metadata(seed: Option<u64>, inject_fault: bool, determinism: DeterminismGuarantee) -> DstReplayMetadata {
    let mut runtime_config: BTreeMap<String, String> = BTreeMap::new();
    runtime_config.insert("tick_ns".to_string(), "1000".to_string());
    runtime_config.insert("node_count".to_string(), "1".to_string());
    let fault_injections: Vec<FaultInjection> = if inject_fault {
        vec![FaultInjection {
            step: 3,
            target: "broker-storage".to_string(),
            fault_kind: "disk_fsync_stall".to_string(),
            detail: "delayed ack flush triggered duplicate redelivery".to_string(),
        }]
    } else {
        Vec::new()
    };
    DstReplayMetadata {
        seed,
        scheduler_decisions: vec![
            SchedulerDecision {
                step: 1,
                actor: "broker".to_string(),
                action: "deliver".to_string(),
            },
            SchedulerDecision {
                step: 2,
                actor: "consumer".to_string(),
                action: "ack".to_string(),
            },
        ],
        fault_injections,
        runtime_config,
        determinism,
    }
}

impl DstAdapter for MockDstAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            adapter_id: "mock-dst-adapter".to_string(),
            adapter_version: "0.2.0".to_string(),
            engine_name: "sim-kernel".to_string(),
            engine_version: "1.4.0".to_string(),
            methods: vec![VerificationMethod::DeterministicSimulation],
            supports_replay: true,
            supports_shrinking: false,
            supports_coverage: true,
        }
    }

    fn execute_dst(&self, _snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<DstRunReport, AdapterError> {
        let caps: AdapterCapabilities = self.capabilities();
        Ok(DstRunReport {
            adapter_id: caps.adapter_id,
            adapter_version: caps.adapter_version,
            engine_name: caps.engine_name,
            engine_version: caps.engine_version,
            run_state: RunState::Completed,
            trace: build_dst_trace(self.inject_redelivery_fault),
            replay_metadata: build_replay_metadata(request.seed, self.inject_redelivery_fault, deterministic_guarantee()),
            raw_status: if self.inject_redelivery_fault {
                "dst_completed:violation".to_string()
            } else {
                "dst_completed:ok".to_string()
            },
            limitations: Vec::new(),
        })
    }

    fn replay_dst(&self, _snapshot: &GraphSnapshot, counterexample: &Counterexample, _run_id: &str) -> Result<DstRunReport, AdapterError> {
        if !counterexample.replay.replayable {
            return Err(AdapterError::NotReplayable(format!("counterexample {} is not replayable", counterexample.id)));
        }
        let caps: AdapterCapabilities = self.capabilities();
        let has_fault: bool = counterexample.events.iter().any(|event: &TraceEvent| event.fault.is_some());
        Ok(DstRunReport {
            adapter_id: caps.adapter_id,
            adapter_version: caps.adapter_version,
            engine_name: caps.engine_name,
            engine_version: caps.engine_version,
            run_state: RunState::Completed,
            trace: ExecutionTrace {
                initial_state: counterexample.initial_state.clone(),
                events: counterexample.events.clone(),
            },
            replay_metadata: build_replay_metadata(counterexample.replay.seed, has_fault, deterministic_guarantee()),
            raw_status: "dst_replay:completed".to_string(),
            limitations: vec![format!("replayed DST counterexample {}", counterexample.id)],
        })
    }
}

pub(super) fn dst_request(run_id: &str, property_id: &str, claim_id: Option<&str>) -> VerificationRequest {
    let claim_ids: Vec<ArtifactId> = claim_id
        .map(|id: &str| vec![ArtifactId::parse(id).expect("valid claim id")])
        .unwrap_or_default();
    VerificationRequest {
        run_id: run_id.to_string(),
        activity_id: "ACT-DST-001".to_string(),
        property_id: ArtifactId::parse(property_id).expect("valid property id"),
        claim_ids,
        budget: 16,
        seed: Some(77),
    }
}

pub(super) fn base_dst_report(run_state: RunState, trace: ExecutionTrace, determinism: DeterminismGuarantee) -> DstRunReport {
    DstRunReport {
        adapter_id: "mock-dst-adapter".to_string(),
        adapter_version: "0.2.0".to_string(),
        engine_name: "sim-kernel".to_string(),
        engine_version: "1.4.0".to_string(),
        run_state,
        trace,
        replay_metadata: build_replay_metadata(Some(77), false, determinism),
        raw_status: "dst_completed:ok".to_string(),
        limitations: Vec::new(),
    }
}

#[when(expr = "a deterministic DST adapter executes a violating fault-injected simulation for {string} and claim {string}")]
fn execute_violating_dst(world: &mut Forge, property_id: String, claim_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let adapter: MockDstAdapter = MockDstAdapter { inject_redelivery_fault: true };
    let request: VerificationRequest = dst_request("RUN-DST-VIOL-1", &property_id, Some(&claim_id));
    let report: DstRunReport = adapter.execute_dst(snapshot, &request).expect("dst execution succeeds");
    let normalized: Result<(RunEnvelope, ContentHash), AdapterError> = normalize_dst_report(snapshot, &request, &report);
    world.external_assurance.dst_replay_metadata = Some(report.replay_metadata);
    world.external_assurance.dst_result = Some(normalized);
}

#[when(expr = "a deterministic DST adapter executes a safe simulation for {string} and claim {string}")]
fn execute_safe_dst(world: &mut Forge, property_id: String, claim_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let adapter: MockDstAdapter = MockDstAdapter {
        inject_redelivery_fault: false,
    };
    let request: VerificationRequest = dst_request("RUN-DST-SAFE-1", &property_id, Some(&claim_id));
    let report: DstRunReport = adapter.execute_dst(snapshot, &request).expect("dst execution succeeds");
    let normalized: Result<(RunEnvelope, ContentHash), AdapterError> = normalize_dst_report(snapshot, &request, &report);
    world.external_assurance.dst_replay_metadata = Some(report.replay_metadata);
    world.external_assurance.dst_result = Some(normalized);
}

#[when("the deterministic DST adapter replays the DST counterexample")]
fn replay_dst_ce(world: &mut Forge) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let ce: Counterexample = world
        .external_assurance
        .dst_result
        .as_ref()
        .expect("dst result recorded")
        .as_ref()
        .expect("dst normalized")
        .0
        .original_counterexample
        .clone()
        .expect("dst counterexample recorded");
    let adapter: MockDstAdapter = MockDstAdapter { inject_redelivery_fault: true };
    let report: DstRunReport = adapter.replay_dst(snapshot, &ce, "RUN-DST-REPLAY-1").expect("dst replay succeeds");
    let request: VerificationRequest = dst_request("RUN-DST-REPLAY-1", ce.property_id.as_str(), None);
    let (envelope, _digest): (RunEnvelope, ContentHash) = normalize_dst_report(snapshot, &request, &report).expect("replay normalizes");
    world.external_assurance.dst_replay_envelope = Some(envelope);
}

#[when(expr = "a non-deterministic DST report missing virtual clock and deterministic faults is normalized for {string}")]
fn normalize_nondeterministic_dst(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = dst_request("RUN-DST-NONDET-1", &property_id, None);
    let determinism: DeterminismGuarantee = DeterminismGuarantee {
        deterministic_rng: true,
        deterministic_scheduler: true,
        virtual_clock: false,
        deterministic_faults: false,
        notes: vec!["wall-clock timer used".to_string()],
    };
    let report: DstRunReport = base_dst_report(RunState::Completed, build_dst_trace(false), determinism);
    world.external_assurance.dst_result = Some(normalize_dst_report(snapshot, &request, &report));
}

#[when(expr = "a non-deterministic DST report missing deterministic RNG and scheduler is normalized for {string}")]
fn normalize_nondeterministic_rng_sched_dst(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = dst_request("RUN-DST-NONDET-2", &property_id, None);
    let determinism: DeterminismGuarantee = DeterminismGuarantee {
        deterministic_rng: false,
        deterministic_scheduler: false,
        virtual_clock: true,
        deterministic_faults: true,
        notes: vec!["unseeded rng and os threads".to_string()],
    };
    let report: DstRunReport = base_dst_report(RunState::Completed, build_dst_trace(false), determinism);
    world.external_assurance.dst_result = Some(normalize_dst_report(snapshot, &request, &report));
}

#[when(expr = "a non-deterministic DST report missing only deterministic faults is normalized for {string}")]
fn normalize_nondeterministic_faults_only_dst(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = dst_request("RUN-DST-NONDET-3", &property_id, None);
    let determinism: DeterminismGuarantee = DeterminismGuarantee {
        deterministic_rng: true,
        deterministic_scheduler: true,
        virtual_clock: true,
        deterministic_faults: false,
        notes: vec!["external fault injector".to_string()],
    };
    let report: DstRunReport = base_dst_report(RunState::Completed, build_dst_trace(false), determinism);
    world.external_assurance.dst_result = Some(normalize_dst_report(snapshot, &request, &report));
}

#[when(expr = "a deterministic DST report without a seed but with recorded trace events is normalized for {string}")]
fn normalize_seedless_with_events_dst(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = dst_request("RUN-DST-SEEDLESS-1", &property_id, None);
    let mut report: DstRunReport = base_dst_report(RunState::Completed, build_dst_trace(false), deterministic_guarantee());
    report.replay_metadata.seed = None;
    let normalized: Result<(RunEnvelope, ContentHash), AdapterError> = normalize_dst_report(snapshot, &request, &report);
    world.external_assurance.dst_replay_metadata = Some(report.replay_metadata);
    world.external_assurance.dst_result = Some(normalized);
}

#[when(expr = "a deterministic DST report with neither seed nor trace events is normalized for {string}")]
fn normalize_no_replay_controls_dst(world: &mut Forge, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = dst_request("RUN-DST-NOCONTROL-1", &property_id, None);
    let empty_trace: ExecutionTrace = ExecutionTrace {
        initial_state: "queued".to_string(),
        events: Vec::new(),
    };
    let mut report: DstRunReport = base_dst_report(RunState::Completed, empty_trace, deterministic_guarantee());
    report.replay_metadata.seed = None;
    world.external_assurance.dst_result = Some(normalize_dst_report(snapshot, &request, &report));
}

#[when(expr = "a safe-trace DST report with run state {string} is normalized for {string}")]
fn normalize_incomplete_dst(world: &mut Forge, state_name: String, property_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let run_state: RunState = match state_name.as_str() {
        "inconclusive" => RunState::Inconclusive,
        "failed" => RunState::Failed,
        "cancelled" => RunState::Cancelled,
        other => panic!("unexpected run state {other}"),
    };
    let request: VerificationRequest = dst_request("RUN-DST-INCOMPLETE-1", &property_id, None);
    let mut report: DstRunReport = base_dst_report(run_state, build_dst_trace(false), deterministic_guarantee());
    report.raw_status = format!("dst:{state_name}");
    world.external_assurance.dst_result = Some(normalize_dst_report(snapshot, &request, &report));
}

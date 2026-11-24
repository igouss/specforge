use std::collections::BTreeMap;

use cucumber::when;
use specforge::formal::{normalize_formal_report, FormalAdapter, FormalArtifactRef, FormalBounds, FormalCheckReport, FormalPropertyRef};
use specforge::graph::{ArtifactId, ContentHash, GraphSnapshot, Property, VerificationMethod};
use specforge::verify::{AdapterCapabilities, AdapterError, ExecutionTrace, FormalOutcome, TraceEvent, VerificationRequest};

use crate::graph::current_snapshot;
use crate::Forge;

struct MockFormalAdapter {
    outcome: FormalOutcome,
    bounds: Option<FormalBounds>,
    include_counterexample: bool,
}

pub(super) fn formal_counterexample_trace() -> ExecutionTrace {
    ExecutionTrace {
        initial_state: "queued".to_string(),
        events: vec![
            TraceEvent {
                step: 1,
                event: "deliver".to_string(),
                from_state: "queued".to_string(),
                to_state: "delivered".to_string(),
                inputs: BTreeMap::new(),
                fault: None,
                virtual_time: Some(1),
            },
            TraceEvent {
                step: 2,
                event: "ack".to_string(),
                from_state: "delivered".to_string(),
                to_state: "acknowledged".to_string(),
                inputs: BTreeMap::new(),
                fault: None,
                virtual_time: Some(2),
            },
            TraceEvent {
                step: 3,
                event: "redeliver".to_string(),
                from_state: "acknowledged".to_string(),
                to_state: "redelivered".to_string(),
                inputs: BTreeMap::new(),
                fault: None,
                virtual_time: Some(3),
            },
        ],
    }
}

pub(super) fn standard_artifacts() -> Vec<FormalArtifactRef> {
    vec![
        FormalArtifactRef {
            name: "proof-script".to_string(),
            kind: "tla_module".to_string(),
            digest: ContentHash::of_bytes(b"MODULE BrokerSafety"),
            uri: Some("formal/BrokerSafety.tla".to_string()),
        },
        FormalArtifactRef {
            name: "proof-log".to_string(),
            kind: "solver_log".to_string(),
            digest: ContentHash::of_bytes(b"TLC completed"),
            uri: None,
        },
    ]
}

impl FormalAdapter for MockFormalAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            adapter_id: "mock-formal-adapter".to_string(),
            adapter_version: "0.3.0".to_string(),
            engine_name: "tlc-model-checker".to_string(),
            engine_version: "2.18.0".to_string(),
            methods: vec![VerificationMethod::Formal],
            supports_replay: false,
            supports_shrinking: false,
            supports_coverage: false,
        }
    }

    fn check_formal(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<FormalCheckReport, AdapterError> {
        let caps: AdapterCapabilities = self.capabilities();
        let digest: Option<ContentHash> = snapshot.artifact_digest(request.property_id.as_str()).cloned();
        let assumptions: Vec<ArtifactId> = snapshot
            .property(request.property_id.as_str())
            .map_or_else(Vec::new, |prop: &Property| prop.assumptions.clone());
        Ok(FormalCheckReport {
            adapter_id: caps.adapter_id,
            adapter_version: caps.adapter_version,
            engine_name: caps.engine_name,
            engine_version: caps.engine_version,
            property_ref: FormalPropertyRef {
                property_id: request.property_id.clone(),
                external_tool: "TLC".to_string(),
                module: "BrokerSafety".to_string(),
                symbol: "NoRedeliveryAfterAck".to_string(),
                digest,
            },
            outcome: self.outcome.clone(),
            bounds: self.bounds.clone(),
            assumptions,
            artifacts: standard_artifacts(),
            counterexample_trace: self.include_counterexample.then(formal_counterexample_trace),
            raw_status: format!("formal:{:?}", self.outcome),
            limitations: Vec::new(),
        })
    }
}

pub(super) fn formal_request(run_id: &str, property_id: &str, claim_id: Option<&str>) -> VerificationRequest {
    let claim_ids: Vec<ArtifactId> = claim_id
        .map(|id: &str| vec![ArtifactId::parse(id).expect("valid claim id")])
        .unwrap_or_default();
    VerificationRequest {
        run_id: run_id.to_string(),
        activity_id: "ACT-FORMAL-001".to_string(),
        property_id: ArtifactId::parse(property_id).expect("valid property id"),
        claim_ids,
        budget: 1,
        seed: None,
    }
}

pub(super) fn base_formal_report(property_id: ArtifactId, outcome: FormalOutcome) -> FormalCheckReport {
    FormalCheckReport {
        adapter_id: "mock-formal-adapter".to_string(),
        adapter_version: "0.3.0".to_string(),
        engine_name: "tlc-model-checker".to_string(),
        engine_version: "2.18.0".to_string(),
        property_ref: FormalPropertyRef {
            property_id,
            external_tool: "TLC".to_string(),
            module: "BrokerSafety".to_string(),
            symbol: "NoRedeliveryAfterAck".to_string(),
            digest: None,
        },
        outcome,
        bounds: None,
        assumptions: vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption")],
        artifacts: standard_artifacts(),
        counterexample_trace: None,
        raw_status: "formal_completed".to_string(),
        limitations: Vec::new(),
    }
}

#[when(expr = "a formal adapter checks {string} for claim {string} with outcome {string}")]
fn check_formal_outcome(world: &mut Forge, property_id: String, claim_id: String, outcome_name: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let (outcome, bounds, include_ce): (FormalOutcome, Option<FormalBounds>, bool) = match outcome_name.as_str() {
        "proved" => (FormalOutcome::Proved, None, false),
        "refuted" => (FormalOutcome::Refuted, None, true),
        "unknown" => (FormalOutcome::Unknown, None, false),
        "timeout" => (
            FormalOutcome::Timeout,
            Some(FormalBounds {
                max_depth: None,
                max_states: None,
                timeout_seconds: Some(60),
                description: "solver wall-time limit".to_string(),
            }),
            false,
        ),
        "unsupported" => (FormalOutcome::Unsupported, None, false),
        other => panic!("unexpected formal outcome {other}"),
    };
    let adapter: MockFormalAdapter = MockFormalAdapter {
        outcome,
        bounds,
        include_counterexample: include_ce,
    };
    let request: VerificationRequest = formal_request(&format!("RUN-FORMAL-{outcome_name}"), &property_id, Some(&claim_id));
    let report: FormalCheckReport = adapter.check_formal(snapshot, &request).expect("formal check succeeds");
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a formal adapter checks {string} for claim {string} with bounded success {string}")]
fn check_formal_bounded_success(world: &mut Forge, property_id: String, claim_id: String, bound: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let adapter: MockFormalAdapter = MockFormalAdapter {
        outcome: FormalOutcome::BoundedSuccess { bound: bound.clone() },
        bounds: Some(FormalBounds {
            max_depth: Some(12),
            max_states: Some(10000),
            timeout_seconds: Some(30),
            description: format!("bounded model check {bound}"),
        }),
        include_counterexample: false,
    };
    let request: VerificationRequest = formal_request("RUN-FORMAL-BSUCCESS", &property_id, Some(&claim_id));
    let report: FormalCheckReport = adapter.check_formal(snapshot, &request).expect("formal check succeeds");
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

#[when(expr = "a formal adapter checks {string} for claim {string} with bounded failure {string}")]
fn check_formal_bounded_failure(world: &mut Forge, property_id: String, claim_id: String, bound: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let adapter: MockFormalAdapter = MockFormalAdapter {
        outcome: FormalOutcome::BoundedFailure { bound: bound.clone() },
        bounds: Some(FormalBounds {
            max_depth: Some(8),
            max_states: Some(5000),
            timeout_seconds: Some(30),
            description: format!("bounded refutation {bound}"),
        }),
        include_counterexample: true,
    };
    let request: VerificationRequest = formal_request("RUN-FORMAL-BFAIL", &property_id, Some(&claim_id));
    let report: FormalCheckReport = adapter.check_formal(snapshot, &request).expect("formal check succeeds");
    world.external_assurance.formal_result = Some(normalize_formal_report(snapshot, &request, &report));
}

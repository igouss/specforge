//! Empirical `sf-vre` spike: 3-node distributed ledger DST over `DstAdapter` (`SF-016`),
//! `GraphSnapshot` (`SF-007`), `EvidenceLedger` (`SF-008`), `assess_claim` (`SF-009`),
//! `CounterexampleRegistry` (`SF-013`), and `build_assurance_argument` (`SF-018`).

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

use specforge::argument::{build_assurance_argument, export_argument_markdown, AssuranceArgument};
use specforge::assurance::{assess_claim, ClaimAssessment};
use specforge::corpus::CounterexampleRegistry;
use specforge::dst::{normalize_dst_report, DeterminismGuarantee, DstAdapter, DstReplayMetadata, DstRunReport, FaultInjection, SchedulerDecision};
use specforge::graph::{compile_graph, ArtifactId, ContentHash, GraphSnapshot, VerificationClaim, VerificationMethod};
use specforge::ledger::{EvidenceKind, EvidenceLedger, EvidenceRecord};
use specforge::trace::Source;
use specforge::verify::{
    AdapterCapabilities, AdapterError, ClaimStatus, Counterexample, EvaluationResult, ExecutionTrace, FindingRecord, RootCause, RunEnvelope, RunState,
    TraceEvent, VerificationOutcome, VerificationRequest,
};

const SPEC_MD: &str = r#"# Distributed Ledger Conservation Specification
### REQ-001 @ v1 — Ledger conserves total credits
> While transfers, partitions, and restarts occur across the 3-node ledger, total credits shall remain 100.

```specforge
kind: assumption
id: ASM-LEDGER-001
statement: Three-node cluster starts with Alice=100, Bob=0, and crash-stop storage WAL.
scope: 3-node-cluster-transfer-partition-restart
```
```specforge
kind: state_machine
id: SM-LEDGER-001
requirements: [REQ-001]
initial: ready
states: [ready, transferring, partitioned, restarting, recovered]
transitions:
  - { name: debit_and_credit, from: ready, to: transferring, event: transfer_60 }
  - { name: split_cluster, from: transferring, to: partitioned, event: network_partition }
  - { name: crash_coordinator, from: partitioned, to: restarting, event: node_restart }
  - { name: replay_wal, from: restarting, to: recovered, event: wal_recover }
```
```specforge
kind: property
id: PROP-LEDGER-001
name: credit_conservation
property_kind: state_invariant
subject: SM-LEDGER-001
predicate: { language: trace, expression: "always(input.total == 100)" }
scope: 3-node-cluster-transfer-partition-restart
assumptions: [ASM-LEDGER-001]
requirements: [REQ-001]
methods: [deterministic_simulation]
```
```specforge
kind: claim
id: CLM-LEDGER-001
proposition: Total credits remain 100 across partition and coordinator restart.
scope: 3-node-cluster-transfer-partition-restart
assumptions: [ASM-LEDGER-001]
properties: [PROP-LEDGER-001]
requirements: [REQ-001]
```
"#;

#[derive(Default)]
struct LedgerCluster {
    alice_node0: i64,
    bob_node1: i64,
    applied_node1: BTreeSet<u64>,
    wal_node2: Vec<(u64, i64)>,
    partitioned: bool,
    clock: u64,
    rng: u64,
    events: Vec<TraceEvent>,
    decisions: Vec<SchedulerDecision>,
    faults: Vec<FaultInjection>,
}

impl LedgerCluster {
    fn new(seed: u64) -> LedgerCluster {
        LedgerCluster {
            alice_node0: 100,
            rng: seed ^ 0x9E37_79B9_7F4A_7C15,
            ..LedgerCluster::default()
        }
    }

    fn credit_bob(&mut self, tx_id: u64, amount: i64, idempotent: bool) {
        if !idempotent || self.applied_node1.insert(tx_id) {
            self.bob_node1 += amount;
        }
    }

    fn deliver_ack(&mut self, tx_id: u64) {
        if !self.partitioned {
            self.wal_node2.retain(|(id, _): &(u64, i64)| *id != tx_id);
        }
    }

    fn record(&mut self, actor: &str, event: &str, from: &str, to: &str, fault: Option<(&str, &str, &str)>) {
        self.rng = self.rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        self.clock += 5 + ((self.rng >> 32) % 10);
        let step: usize = self.events.len() + 1;
        let total: i64 = self.alice_node0 + self.bob_node1;
        let inputs: BTreeMap<String, String> = BTreeMap::from([
            ("alice".to_string(), self.alice_node0.to_string()),
            ("bob".to_string(), self.bob_node1.to_string()),
            ("total".to_string(), total.to_string()),
        ]);
        self.decisions.push(SchedulerDecision {
            step,
            actor: actor.to_string(),
            action: event.to_string(),
        });
        let fault_kind: Option<String> = fault.map(|(target, kind, detail): (&str, &str, &str)| {
            self.faults.push(FaultInjection {
                step,
                target: target.to_string(),
                fault_kind: kind.to_string(),
                detail: detail.to_string(),
            });
            kind.to_string()
        });
        self.events.push(TraceEvent {
            step,
            event: event.to_string(),
            from_state: from.to_string(),
            to_state: to.to_string(),
            inputs,
            fault: fault_kind,
            virtual_time: Some(self.clock),
        });
    }
}

struct LedgerDstAdapter {
    idempotent_recovery: bool,
}

impl LedgerDstAdapter {
    fn build_report(&self, seed: u64, raw_status: &str) -> DstRunReport {
        let mut c: LedgerCluster = LedgerCluster::new(seed);
        c.wal_node2.push((1, 60));
        c.alice_node0 -= 60;
        c.credit_bob(1, 60, self.idempotent_recovery);
        c.record("node-0", "transfer_60", "ready", "transferring", None);
        c.partitioned = true;
        c.deliver_ack(1);
        c.record(
            "network",
            "network_partition",
            "transferring",
            "partitioned",
            Some(("node-1<->node-2", "network_partition", "dropped commit ack for tx 1")),
        );
        c.record(
            "node-2",
            "node_restart",
            "partitioned",
            "restarting",
            Some(("node-2", "node_restart", "coordinator restarted with unacknowledged WAL")),
        );
        c.partitioned = false;
        let pending: Vec<(u64, i64)> = c.wal_node2.clone();
        for (tx_id, amount) in pending {
            c.credit_bob(tx_id, amount, self.idempotent_recovery);
            c.deliver_ack(tx_id);
        }
        c.record("node-2", "wal_recover", "restarting", "recovered", None);

        let caps: AdapterCapabilities = self.capabilities();
        let runtime_config: BTreeMap<String, String> = BTreeMap::from([
            ("nodes".to_string(), "3".to_string()),
            ("idempotent_recovery".to_string(), self.idempotent_recovery.to_string()),
        ]);
        DstRunReport {
            adapter_id: caps.adapter_id,
            adapter_version: caps.adapter_version,
            engine_name: caps.engine_name,
            engine_version: caps.engine_version,
            run_state: RunState::Completed,
            trace: ExecutionTrace {
                initial_state: "ready".to_string(),
                events: c.events,
            },
            replay_metadata: DstReplayMetadata {
                seed: Some(seed),
                scheduler_decisions: c.decisions,
                fault_injections: c.faults,
                runtime_config,
                determinism: DeterminismGuarantee {
                    deterministic_rng: true,
                    deterministic_scheduler: true,
                    virtual_clock: true,
                    deterministic_faults: true,
                    notes: vec!["single-threaded seeded 3-node ledger simulator".to_string()],
                },
            },
            raw_status: raw_status.to_string(),
            limitations: Vec::new(),
        }
    }
}

impl DstAdapter for LedgerDstAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            adapter_id: "ledger-dst-spike".to_string(),
            adapter_version: "0.1.0".to_string(),
            engine_name: "mini-ledger-sim".to_string(),
            engine_version: "0.1.0".to_string(),
            methods: vec![VerificationMethod::DeterministicSimulation],
            supports_replay: true,
            supports_shrinking: false,
            supports_coverage: false,
        }
    }

    fn execute_dst(&self, _snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<DstRunReport, AdapterError> {
        let seed: u64 = request.seed.ok_or_else(|| AdapterError::InvalidRequest("seed is required".to_string()))?;
        Ok(self.build_report(seed, "dst_completed"))
    }

    fn replay_dst(&self, _snapshot: &GraphSnapshot, ce: &Counterexample, _run_id: &str) -> Result<DstRunReport, AdapterError> {
        if !ce.replay.replayable {
            return Err(AdapterError::NotReplayable(format!("counterexample {} is not replayable", ce.id)));
        }
        let seed: u64 = ce.replay.seed.ok_or_else(|| AdapterError::NotReplayable("missing replay seed".to_string()))?;
        Ok(self.build_report(seed, "dst_replay_completed"))
    }
}

fn str_err<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

fn check(ok: bool, msg: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn make_req(run_id: &str, property_id: &ArtifactId, claim_ids: Vec<ArtifactId>, seed: Option<u64>) -> VerificationRequest {
    VerificationRequest {
        run_id: run_id.to_string(),
        activity_id: "ACT-DST-001".to_string(),
        property_id: property_id.clone(),
        claim_ids,
        budget: 1,
        seed,
    }
}

fn run_spike() -> Result<(), String> {
    let source: Source = Source {
        path: "specs/ledger.md".to_string(),
        text: SPEC_MD.to_string(),
    };
    let snapshot: GraphSnapshot = compile_graph(&[source]).map_err(|defects: Vec<specforge::graph::GraphDefect>| format!("graph defects: {defects:?}"))?;
    let prop_id: ArtifactId = ArtifactId::parse("PROP-LEDGER-001").ok_or("invalid property id")?;
    let claim_id: ArtifactId = ArtifactId::parse("CLM-LEDGER-001").ok_or("invalid claim id")?;
    let claim: &VerificationClaim = snapshot.claim(claim_id.as_str()).ok_or("missing CLM-LEDGER-001")?;

    let safe_adapter: LedgerDstAdapter = LedgerDstAdapter { idempotent_recovery: true };
    let safe_req: VerificationRequest = make_req("RUN-SAFE-1", &prop_id, vec![claim_id.clone()], Some(42));
    let safe_rep: DstRunReport = safe_adapter.execute_dst(&snapshot, &safe_req).map_err(str_err)?;
    let (safe_env, safe_meta): (RunEnvelope, ContentHash) = normalize_dst_report(&snapshot, &safe_req, &safe_rep).map_err(str_err)?;
    check(
        safe_env.run_state == RunState::Completed && safe_env.outcome == VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        "safe run must complete and satisfy",
    )?;
    let safe_rec: EvidenceRecord = EvidenceRecord::from_envelope(
        "EV-LEDGER-SAFE-1".to_string(),
        EvidenceKind::PropertyEvaluation,
        "2026-11-24T00:00:00Z".to_string(),
        &snapshot,
        &safe_env,
        BTreeMap::from([("dst_replay_metadata".to_string(), safe_meta)]),
    )
    .map_err(str_err)?;
    let mut ledger: EvidenceLedger = EvidenceLedger::new();
    ledger.append_record(safe_rec).map_err(str_err)?;
    let assessment: ClaimAssessment = assess_claim(claim, &snapshot, &ledger);
    check(
        assessment.status == ClaimStatus::Supported && !assessment.unbounded_proof,
        "safe claim must be bounded Supported",
    )?;
    let argument: AssuranceArgument = build_assurance_argument(&snapshot, &ledger, claim_id.as_str(), &[])?;
    let markdown: String = export_argument_markdown(&argument);
    check(
        markdown.contains("Status: `supported`") && argument.defeaters.is_empty(),
        "argument export must be supported",
    )?;

    let buggy_adapter: LedgerDstAdapter = LedgerDstAdapter { idempotent_recovery: false };
    let buggy_req: VerificationRequest = make_req("RUN-BUGGY-1", &prop_id, vec![claim_id], Some(42));
    let buggy_rep: DstRunReport = buggy_adapter.execute_dst(&snapshot, &buggy_req).map_err(str_err)?;
    let (buggy_env, buggy_meta): (RunEnvelope, ContentHash) = normalize_dst_report(&snapshot, &buggy_req, &buggy_rep).map_err(str_err)?;
    check(
        buggy_env.run_state == RunState::Completed && buggy_env.outcome == VerificationOutcome::Evaluation(EvaluationResult::Violated),
        "buggy run must complete and violate",
    )?;
    let finding: &FindingRecord = buggy_env.finding.as_ref().ok_or("missing buggy finding")?;
    let ce: &Counterexample = buggy_env.original_counterexample.as_ref().ok_or("missing counterexample")?;
    check(
        finding.root_cause == RootCause::Unresolved && ce.replay.replayable && ce.violation.observed == "input.total == 160",
        "invalid buggy finding/ce",
    )?;
    let mut registry: CounterexampleRegistry = CounterexampleRegistry::new();
    registry.register_original(ce.clone()).map_err(str_err)?;
    check(registry.get(&ce.id).is_some(), "counterexample must be registered")?;

    let replay_rep: DstRunReport = buggy_adapter.replay_dst(&snapshot, ce, "RUN-BUGGY-REPLAY-1").map_err(str_err)?;
    let replay_req: VerificationRequest = make_req("RUN-BUGGY-REPLAY-1", &prop_id, Vec::new(), ce.replay.seed);
    let (replay_env, replay_meta): (RunEnvelope, ContentHash) = normalize_dst_report(&snapshot, &replay_req, &replay_rep).map_err(str_err)?;
    let replay_ce: &Counterexample = replay_env.original_counterexample.as_ref().ok_or("missing replay counterexample")?;
    check(
        replay_env.run_state == RunState::Completed
            && replay_env.outcome == VerificationOutcome::Evaluation(EvaluationResult::Violated)
            && replay_ce.violation == ce.violation
            && replay_ce.events == ce.events
            && replay_meta == buggy_meta,
        "replay must deterministically reproduce violation",
    )?;

    println!("snapshot: {} ({} artifacts)", snapshot.snapshot_id, snapshot.artifact_count());
    println!(
        "safe:   outcome={:?} claim={} unbounded_proof={}",
        safe_env.outcome, assessment.status, argument.unbounded_proof
    );
    println!(
        "buggy:  outcome={:?} finding={} root_cause={} observed=\"{}\"",
        buggy_env.outcome, finding.id, finding.root_cause, ce.violation.observed
    );
    println!(
        "replay: registered={} reproduced={} metadata_match={}",
        registry.originals().len(),
        replay_ce.violation.observed,
        replay_meta == buggy_meta
    );
    println!("\n{markdown}");
    Ok(())
}

fn main() -> ExitCode {
    match run_spike() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ledger_dst_spike failed: {error}");
            ExitCode::FAILURE
        }
    }
}

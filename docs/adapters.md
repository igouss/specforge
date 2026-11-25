# Adding a Verification Engine Adapter

SpecForge keeps external verification engines behind the `VerificationAdapter` trait in [src/verify.rs](../src/verify.rs). The canonical Specification Graph (`GraphSnapshot`) and Evidence Ledger (`EvidenceRecord`, `EvidenceLedger`) never adopt an external tool's internal schema.

## Trait contract

Implement `specforge::verify::VerificationAdapter`:

```rust
use specforge::graph::GraphSnapshot;
use specforge::verify::{
    AdapterCapabilities, AdapterError, Counterexample, RunEnvelope, VerificationAdapter, VerificationRequest,
};

pub struct MyAdapter;

impl VerificationAdapter for MyAdapter {
    fn capabilities(&self) -> AdapterCapabilities { /* declare methods, replay, shrinking, coverage */ }
    fn validate(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<(), AdapterError> { /* check preconditions */ }
    fn execute(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<RunEnvelope, AdapterError> { /* run engine & normalize */ }
    fn replay(&self, snapshot: &GraphSnapshot, counterexample: &Counterexample, run_id: &str) -> Result<RunEnvelope, AdapterError> { /* optional replay */ }
}
```

## Adapter rules

1. **Preserve raw status and engine versions.** Populate `adapter_id`, `adapter_version`, `engine_name`, `engine_version`, and `raw_status` on every `RunEnvelope`.
2. **Never turn inconclusive into satisfied.** A zero/exhausted budget, timeout, or rejected run must map to `RunState::Inconclusive` and `EvaluationResult::Inconclusive` (or `FormalOutcome::Unknown` / `Timeout`).
3. **Keep formal and empirical outcomes distinct.** Emit `VerificationOutcome::Formal(FormalOutcome::BoundedSuccess { bound })` for bounded model checks rather than `FormalOutcome::Proved` or `EvaluationResult::Satisfied`.
4. **Preserve counterexample lineage.** When an engine shrinks a failing case, keep the original `Counterexample` intact and link the minimized one through `original_id` and `minimization: Some(MinimizationMeta { .. })`. Only set `replay.replayable = true` when the engine supplies genuine replay controls.
5. **Seal evidence immutably.** Convert the `RunEnvelope` into an `EvidenceRecord` with `EvidenceRecord::from_envelope` and append it to an `EvidenceLedger` or `FsLedger`.

## Specialized pure adapter contracts

### Deterministic Simulation Testing (`DstAdapter`)

Deterministic simulation engines (for example, discrete-event simulators with virtual clocks and fault injection) implement `specforge::dst::DstAdapter` in [src/dst.rs](../src/dst.rs) and normalize their structured `DstRunReport` via `normalize_dst_report`:

- Capture `DeterminismGuarantee` (`deterministic_rng`, `deterministic_scheduler`, `virtual_clock`, `deterministic_faults`), `SchedulerDecision`s, `FaultInjection`s, and `runtime_config` inside `DstReplayMetadata`.
- `normalize_dst_report` hashes `DstReplayMetadata` into a canonical SHA-256 `ContentHash`, evaluates the property via `model::evaluate_property` while preserving `fault` and `virtual_time` annotations, and enforces EARS-032: `replay.replayable` is `true` only when all four determinism guarantees hold and a seed or trace events were recorded. Missing guarantees are named explicitly in `limitations`.
- Incomplete run states (`Inconclusive`, `Failed`, `Cancelled`, `Requested`, `Started`) can never be normalized as `EvaluationResult::Satisfied`.

### Formal Verification (`FormalAdapter`)

Model checkers, SMT solvers, and theorem provers implement `specforge::formal::FormalAdapter` in [src/formal/mod.rs](../src/formal/mod.rs) and normalize their `FormalCheckReport` via `normalize_formal_report`:

- Bind each checked property through `FormalPropertyRef` (`property_id`, `external_tool`, `module`, `symbol`, optional `digest`), declare explicit `FormalBounds` when bounded, list snapshot `assumptions`, and attach SHA-256 `FormalArtifactRef`s (proof scripts, solver logs, counterexample witnesses).
- `normalize_formal_report` preserves all seven `FormalOutcome` variants (`Proved`, `Refuted`, `BoundedSuccess`, `BoundedFailure`, `Unknown`, `Timeout`, `Unsupported`) losslessly, rejects empty bounds on bounded outcomes, rejects `Proved` when assumptions are inconsistent with the property or claims (or when depth/state bounds are declared), retains declared bounds in `limitations`, and maps `Unknown`, `Timeout`, and `Unsupported` to `RunState::Inconclusive`.
- Returned artifact digests (`BTreeMap<String, ContentHash>`) seal directly into `EvidenceRecord::from_envelope` with `EvidenceKind::FormalCheck`.

## Reference implementations

- **PBT / state-machine adapter**: [src/proptest_adapter/mod.rs](../src/proptest_adapter/mod.rs) (`feature = "proptest-adapter"`).
- **Example-based Cucumber adapter**: `stories_with_evidence` / `record_stories_evidence` in [src/stories.rs](../src/stories.rs) (`feature = "cucumber"`).
- **Pure state-machine generator and trace evaluator**: [src/model.rs](../src/model.rs) (no I/O, `wasm32`-clean).
- **Pure DST adapter contract**: [src/dst.rs](../src/dst.rs) (no I/O, `wasm32`-clean).
- **Pure formal verification adapter contract**: [src/formal/mod.rs](../src/formal/mod.rs) (no I/O, `wasm32`-clean).

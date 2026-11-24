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

## Reference implementations

- **PBT / state-machine adapter**: [src/proptest_adapter.rs](../src/proptest_adapter.rs) (`feature = "proptest-adapter"`).
- **Example-based Cucumber adapter**: `stories_with_evidence` / `record_stories_evidence` in [src/stories.rs](../src/stories.rs) (`feature = "cucumber"`).
- **Pure state-machine generator and trace evaluator**: [src/model.rs](../src/model.rs) (no I/O, `wasm32`-clean).

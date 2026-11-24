# 0003. Three-node ledger DST spike confirms D6 and the `DstAdapter` boundary

Date: 2026-11-24

## Context

Bead `sf-vre` recorded two open questions left over from [ADR 0001](0001-start-as-the-trace-not-the-engine.md) and the archived design conversation in [sources/chatgpt-export/conversation.md](../../sources/chatgpt-export/conversation.md):

1. **Missing deliverables from the original export** (EARS requirements, specification language/meta-model, and engine architecture). This half of `sf-vre` was superseded and fulfilled by the Specification-Driven Verification handoff in [sources/verification-requirements/](../../sources/verification-requirements/README.md), [ADR 0002](0002-specification-driven-verification.md), and the implemented `SF-007`–`SF-018` specifications.
2. **Empirical test of assumed default D6** ("The design export's simulation engine, state machines, fault models, environments, execution planner and YAML ontology are not built. Reverses if a consumer exists whose stories need nodes, a network or injected faults that it cannot write on its own harness."): build the export's canonical 3-node distributed ledger scenario (Alice transfers 60 of 100 credits to Bob, network partition drops the commit ack, coordinator restarts and replays its WAL, conservation invariant `total == 100`) directly as consumer code over SpecForge's pure-core `DstAdapter` (`SF-016`) and verify whether any simulation-engine concept is missing from SpecForge.

## Experiment (`examples/ledger_dst_spike.rs`)

[examples/ledger_dst_spike.rs](../../examples/ledger_dst_spike.rs) implements the full end-to-end flow with zero new crate dependencies:

- **Inline specification (`SPEC_MD`, 44 lines, lines 20–63):** declares `REQ-001`, `ASM-LEDGER-001`, `SM-LEDGER-001`, `PROP-LEDGER-001` (`always(input.total == 100)`), and `CLM-LEDGER-001`, compiled via `graph::compile_graph` (`SF-007`) into a content-hashed `GraphSnapshot`.
- **3-node deterministic simulator (`LedgerCluster`, 70 lines, lines 65–134):** models `node-0` (Alice's shard, 100 credits), `node-1` (Bob's shard + applied transaction set), `node-2` (coordinator WAL), deterministic seeded LCG + monotonic virtual clock, `network_partition` dropping commit acks, and `node_restart` triggering WAL recovery.
- **`DstAdapter` glue (`LedgerDstAdapter` + `impl DstAdapter`, 96 lines, lines 136–231):** declares `DeterminismGuarantee` with all four controls enabled (`deterministic_rng`, `deterministic_scheduler`, `virtual_clock`, `deterministic_faults`), records `SchedulerDecision` and `FaultInjection` metadata, and implements both `execute_dst` and seed-driven `replay_dst`.
- **End-to-end verification driver (123 lines, lines 233–355):** exercises both modes through `normalize_dst_report` (`SF-016`), `EvidenceRecord::from_envelope` / `EvidenceLedger` (`SF-008`), `assess_claim` (`SF-009`), `CounterexampleRegistry` (`SF-013`), and `build_assurance_argument` / `export_argument_markdown` (`SF-018`).

### Line and byte counts

| Component | Location | Lines |
|---|---|---:|
| 3-node simulator (`LedgerCluster`) | `examples/ledger_dst_spike.rs:65-134` | 70 |
| `DstAdapter` glue (`LedgerDstAdapter` + `impl DstAdapter`) | `examples/ledger_dst_spike.rs:136-231` | 96 |
| **Combined simulator + `DstAdapter` glue** | `examples/ledger_dst_spike.rs:65-231` | **167** |
| Complete self-contained example (with spec + driver) | `examples/ledger_dst_spike.rs` | 355 (14,640 bytes) |

### Observed outcomes (`cargo run --offline --example ledger_dst_spike`)

1. **Safe ledger (idempotent WAL recovery across partition + restart):** total remains `100` at every step -> `EvaluationResult::Satisfied` -> sealed into `EvidenceLedger` -> `assess_claim` returns `ClaimStatus::Supported` with `unbounded_proof = false` -> `build_assurance_argument` exports a clean Markdown assurance argument disclosing the finite-trace limitation and zero defeaters.
2. **Buggy ledger (non-idempotent WAL replay after partition + restart):** total reaches `160` -> `EvaluationResult::Violated` -> `FindingRecord` with `RootCause::Unresolved` + replayable `Counterexample` (`observed = "input.total == 160"`, `expected = "input.total == 100"`) -> registered in `CounterexampleRegistry` -> `replay_dst` deterministically re-executes from the recorded seed and reproduces the identical trace, violation, and canonical `DstReplayMetadata` SHA-256 digest (`metadata_match = true`).

## Decision

**Assumed default D6 stands.**

A multi-node distributed ledger with network partitions, node restarts, WAL recovery, monotonic virtual time, seeded scheduling, and fault injection fits cleanly in **167 lines** of consumer code behind `DstAdapter` (`SF-016`). **Zero new simulation-engine concepts** (nodes, network topology, fault-model runtime, or YAML ontology) are required inside SpecForge.

## What it preserved

- SpecForge's pure `wasm32`-clean core owns specification compilation (`SF-007`), property evaluation, replayability and determinism checks (`SF-016`), immutable evidence sealing (`SF-008`), claim assessment (`SF-009`), append-only counterexample registration (`SF-013`), and Claim–Argument–Evidence export (`SF-018`).
- Domain-specific simulation mechanics (nodes, shards, partitions, WALs, clocks, fault injectors) stay in consumer code or dedicated external engines (such as Turmoil or FoundationDB-style harnesses) behind `DstAdapter`.
- Incomplete or finite simulation runs never produce false greens: DST satisfaction explicitly carries its finite-trace limitation and sets `unbounded_proof = false`.

## What would reverse it

- Multiple independent consumers needing to share an identical reusable multi-node network/fault-injection harness where duplicating simulator code becomes measurably costlier than a separate adapter/runtime crate — even then, that runtime would live as an external crate implementing `DstAdapter`, not inside SpecForge's core.

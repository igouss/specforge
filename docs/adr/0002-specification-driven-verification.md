# 0002. Specification-driven verification extends the trace as pure core plus feature-gated adapters

Date: 2026-11-23

## Operator decision

2026-11-23, confirming the Phase 0 mapping of the archived [Specification-Driven Verification handoff](../../sources/verification-requirements/README.md):

1. Archive the handoff package in `sources/verification-requirements/` and build Phase 1 + Phase 2 (the first vertical slice end-to-end).
2. Extend `specs/*.md` with typed fenced blocks so Markdown stays the single authoring format.
3. Power the first executable vertical slice with a pure Rust state-machine + trace property evaluator, a `proptest` PBT adapter, and a wrapper over the existing Cucumber runner.
4. Keep the Evidence Ledger domain, canonical JSON serialization and SHA-256 content digests in the pure `wasm32`-clean core, with append-only JSONL and content-addressed blob files behind a feature flag.

This reconciles `ADR-SDV-001` through `ADR-SDV-008` with [ADR 0001](0001-start-as-the-trace-not-the-engine.md) and supersedes assumed defaults `D3` (Markdown now also carries typed `specforge` fenced blocks) and the evidence-deferral clause of `sf-raj`.

## Weakest part at the time

SpecForge compiled Markdown EARS+Gherkin specifications into a `Trace`, assessed step bindings, drew a committed table, and ran Cucumber stories. It had no content hash of a specification snapshot, no typed representation of properties, behavioral models, assumptions, verification claims or plans, no immutable evidence ledger with dependency freshness, and no adapter contract for PBT/MBT engines or replayable counterexamples.

## Forces in conflict

- **Single source of truth vs. richer graph entities.** Adding a second authoring format (separate YAML trees) splits intent across files; extending `specs/*.md` with explicitly tagged fenced blocks (` ```specforge `) preserves backward compatibility with `SF-001`–`SF-006` while keeping prose YAML examples ignored.
- **Pure `wasm32` core vs. external engines and persistence.** SpecForge's core does no I/O and compiles to `wasm32-unknown-unknown`. External test engines (`cucumber`, `proptest`) and filesystem storage belong behind Cargo features while the canonical graph snapshot, content hashing, property evaluator, evidence ledger semantics, freshness engine, and adapter traits stay pure.
- **Reuse engines vs. canonical semantics.** Following both ADR 0001 and `ADR-SDV-003`, SpecForge does not build a DST runtime, theorem prover, or general-purpose PBT library; it provides thin adapters over existing tools (`cucumber`, `proptest`) and owns the canonical graph, evidence ledger, counterexample lineage, and claim assessment.

## Move taken

Add five pure-core modules (all `wasm32`-compatible, no I/O) and two feature-gated modules:

- `graph` (core): compiles `Trace` + ` ```specforge ` YAML blocks (`assumption`, `state_machine`, `property`, `claim`, `plan`) into an immutable `GraphSnapshot` with deterministic SHA-256 content digests per document, per artifact, and for the whole snapshot.
- `verify` (core): distinct result vocabularies (`EvaluationResult`, `FormalOutcome`, `ClaimStatus`, `RunState`, `RootCause`), `ExecutionTrace`, `PropertyEvaluation`, `FindingRecord`, `Counterexample`, `ReplayDescriptor`, `VerificationDecision`, and the `Adapter` trait boundary.
- `model` (core): pure Rust state-machine scenario generator (with state/transition coverage) and trace-checkable safety/invariant property evaluator.
- `ledger` (core): immutable `EvidenceRecord`, `EvidenceLink`, canonical JSON serialization, SHA-256 payload integrity verification, and in-memory append-only `EvidenceLedger`.
- `assurance` (core): `assess_claim` (validity, provenance, freshness, contradictions, inconclusive handling, explicit rationale — never an unqualified percentage) and `impact_set` (surgical dependency-based stale detection).
- `ledger_fs` (`feature = "ledger-fs"`): append-only JSONL ledger + content-addressed `blobs/<sha256>` persistence with load-time integrity validation.
- `proptest_adapter` (`feature = "proptest-adapter"`): PBT adapter over `proptest` capturing seed/replay descriptors, original and shrunk counterexample lineage, and deterministic replay.
- `stories` (`feature = "cucumber"`): existing runner preserved unchanged, plus an adapter helper recording a run as an immutable `EvidenceRecord`.

## What it preserved

- Full backward compatibility with `SF-001`–`SF-006` and existing consumers (`compile`, `assess`, `drawn`, `stories`).
- Strict separation of defects, findings, execution outcomes, formal outcomes, evidence facts, claim assessments, and engineering decisions — never a false green (`inconclusive` is never `satisfied` or `supported`).
- Zero I/O in the default pure core (`just wasm` stays green), doc-comments-only (`just comments`), no `unwrap`/`expect` in `src/`, and EARS+Gherkin stories as the test suite (`SF-007`–`SF-011`).

## What would reverse it

- A consumer whose specifications outgrow inline ` ```specforge ` blocks turns companion YAML files into a second parser input behind `graph::compile`.
- Multi-process concurrent ledger writers turn the single-file JSONL ledger behind `ledger-fs` into a locked or database-backed store behind the same `EvidenceLedger` contract.

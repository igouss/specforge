# Verification Requirements Traceability Matrix

Fulfills Deliverables 6 and 8 of [IMPLEMENTATION_HANDOFF.md](../sources/verification-requirements/planning/IMPLEMENTATION_HANDOFF.md), tracing every requirement group in [REQUIREMENTS.md](../sources/verification-requirements/requirements/REQUIREMENTS.md) (`SFV-FR-001`–`115`, `SFV-NFR-001`–`012`), every EARS statement in [EARS_REQUIREMENTS.md](../sources/verification-requirements/specifications/EARS_REQUIREMENTS.md) (`EARS-001`–`043`), and all 15 scenarios in [ACCEPTANCE_FEATURES.feature](../sources/verification-requirements/specifications/ACCEPTANCE_FEATURES.feature) to SpecForge specs (`SF-001`–`SF-020`), `src/` modules/symbols, and explicit out-of-scope boundaries.

> `SF-001`–`SF-018` live in `specs/01-trace.md`–`specs/09-external-assurance.md` (including `VerificationDecision::dispose` in `SF-010` / `src/verify.rs`); `SF-019` (plan orchestration, run lifecycle, and evidence correlation in `src/orchestrator/` + `src/assurance/correlation.rs`) and `SF-020` (qualified statistical estimation in `src/statistics/`) complete `SF-001`–`SF-020`.

## 1. Functional Requirements (`SFV-FR-001`–`115`)

| Group & IDs | Scope | SpecForge Specs | `src/` Modules & Key Symbols | Status |
|---|---|---|---|---|
| **5.1 Graph** `SFV-FR-001`–`004` | Typed `Property`, stable ID, safety/liveness kinds, links to reqs/models/scenarios/claims | `SF-007`, `SF-010` | `graph/{mod,compile,validate}.rs` (`Property`, `PropertyKind`, `ArtifactId`, `compile_graph`), `model.rs` (`GeneratedScenario`) | Done |
| **5.1 Graph** `SFV-FR-005`–`008` | Distinct domain entities, immutable hashed `GraphSnapshot`, explicit `Assumption` & scope/bounds | `SF-001`–`SF-009`, `SF-016`, `SF-017` | `trace.rs`, `graph/{mod,digest}.rs` (`GraphSnapshot`, `SourceRevision`, `Assumption`, `ContentHash`), `dst.rs`, `formal/mod.rs` (`FormalBounds`) | Done |
| **5.2 Planning** `SFV-FR-010`–`012` | `VerificationPlan`, multi-method selection, separate engine results per claim | `SF-007`–`SF-009`, `SF-011`, `SF-016`–`SF-019` | `graph/mod.rs` (`VerificationPlan`, `PlanActivity`, `VerificationMethod`), `orchestrator/{mod,run}.rs`, `ledger/mod.rs`, `assurance/mod.rs`, `argument/mod.rs` | Done |
| **5.2 Planning** `SFV-FR-013`–`015` | Plan activity selection + gap/staleness recommendations, snapshot pinning, 6 distinct `RunState`s | `SF-008`, `SF-011`, `SF-014`, `SF-015`, `SF-019` | `orchestrator/{mod,run}.rs`, `planner.rs` (`recommend_activities`), `cli/{commands,recommend}.rs` (`run_recommend`), `verify.rs` (`RunEnvelope`, `RunState`) | Done |
| **5.3 MBT** `SFV-FR-020`–`024` | MBT adapter contract, state/transition/req traceability, strategy, replay metadata, 4 separate coverage dimensions | `SF-010`, `SF-012` | `model.rs` (`generate_scenarios`, `GeneratedScenario`), `coverage.rs` (`MbtAdapter`, `ImportedScenario`, `validate_imported_scenario`, `compute_coverage`, `CoverageRecord`, `TransitionPair`) | Done |
| **5.4 PBT** `SFV-FR-030`–`032`, `034` | PBT adapter, generator version/seed/budget/shrink metadata, input vs path distinction, shrunk-to-original link | `SF-010`, `SF-011`, `SF-013`, `SF-015` | `verify.rs` (`VerificationAdapter`, `ReplayDescriptor`, `MinimizationMeta`, `Counterexample`), `proptest_adapter/{mod,shrink}.rs` (`ProptestAdapter`), `cli/commands.rs` (`run_verify`) | Done |
| **5.4 PBT** `SFV-FR-033` | Structured type-level value generators bound to graph constraints | — | Event-sequence generation in `proptest_adapter/`; rich typed value strategies deferred (Section 5) | Deferred |
| **5.5 DST** `SFV-FR-040`–`044` | Pure DST adapter contract, seed/scheduler/virtual-time/fault capture, determinism guarantees, trace links, replay | `SF-016`, `SF-008`, `SF-013` | `dst.rs` (`DstAdapter`, `DstRunReport`, `DstReplayMetadata`, `DeterminismGuarantee`, `SchedulerDecision`, `FaultInjection`, `normalize_dst_report`), `examples/ledger_dst_spike.rs` | Done |
| **5.6 Formal** `SFV-FR-050`–`054` | Pure formal adapter contract, all 7 `FormalOutcome`s, bounds/assumptions/artifacts, external property refs, no probability collapse | `SF-008`, `SF-009`, `SF-017`, `SF-020` | `formal/{mod,validate}.rs` (`FormalAdapter`, `FormalCheckReport`, `FormalPropertyRef`, `FormalBounds`, `FormalArtifactRef`, `normalize_formal_report`), `verify.rs` (`FormalOutcome`), `statistics/{mod,validate}.rs` | Done |
| **5.7 Evaluation** `SFV-FR-060`–`065` | Trace-checkable evaluator, 5 `EvaluationResult`s, `FindingRecord`, `RootCause` taxonomy defaulting to `Unresolved` | `SF-008`–`SF-011`, `SF-016` | `model.rs` (`evaluate_property`), `verify.rs` (`EvaluationResult`, `PropertyEvaluation`, `FindingRecord`, `RootCause`, `ExecutionTrace`) | Done |
| **5.8 Counterexamples** `SFV-FR-070`–`077` | First-class witness with full context, original preservation, minimization categories, causal shrink validation, replayability, read-only recommendations | `SF-010`, `SF-011`, `SF-013`–`SF-016` | `verify.rs` (`Counterexample`, `ViolationDetail`, `MinimizationMeta`, `VerificationDecision::dispose`), `corpus/{mod,replay}.rs` (`CounterexampleRegistry`, `validate_minimized`, `replay_corpus`), `planner.rs` | Done |
| **5.9 Ledger** `SFV-FR-080`–`086` | Append-only ledger, SHA-256 sealed `EvidenceRecord`s, 6 typed `EvidenceRelation` links, negative/inconclusive retention, canonical JSON export, content-addressed blobs | `SF-008`, `SF-009`, `SF-015` | `ledger/{mod,record}.rs` (`EvidenceLedger`, `EvidenceRecord`, `EvidenceLink`, `EvidenceRelation`, `ProducerMeta`, `export_json`), `ledger_fs.rs` (`FsLedger`), `cli/commands.rs` (`run_ledger`) | Done |
| **5.10 Assurance** `SFV-FR-090`–`096` | `VerificationClaim`, separate evidence/assessment/decision types, validity/freshness/scope/correlation checks, contradictions/gaps, CAE arguments, qualified `ClaimStatus` | `SF-007`–`SF-009`, `SF-015`, `SF-017`–`SF-019` | `graph/mod.rs` (`VerificationClaim`), `assurance/mod.rs` (`assess_claim`, `ClaimAssessment`, `Contradiction`), `assurance/correlation.rs` (`CorrelationNote`), `argument/{mod,export}.rs` (`build_assurance_argument`, `AssuranceArgument`, `ArgumentDefeater`), `cli/commands.rs` (`run_assess`, `run_argument`) | Done |
| **5.10 Statistics** `SFV-FR-097`–`099` | Qualified statistical estimates with model, assumptions, sample size, uncertainty interval; refuses naive pass-rate percentages | `SF-020` | `statistics/{mod,validate}.rs` (`compute_qualified_estimate`, `SamplingModel`, `StatisticalMethod`, `QualifiedEstimate`) | Done |
| **5.11 Freshness** `SFV-FR-100`–`104` | Artifact `DependencyPin`s, surgical staleness, `Current`/`Stale`/`Invalidated`/`Superseded`, cross-snapshot `ImpactSet`, history retention | `SF-007`–`SF-009`, `SF-015` | `graph/mod.rs` (`DependencyPin`), `assurance/freshness.rs` (`freshness_of`, `impact_set`), `assurance/mod.rs` (`EvidenceFreshness`, `ImpactSet`), `cli/commands.rs` (`run_impact`) | Done |
| **5.12 Feedback** `SFV-FR-110`–`115` | Gap/staleness/contradiction/counterexample recommendations, explicit `VerificationDecision` approval boundary, regression replay | `SF-010`, `SF-013`–`SF-015` | `planner.rs` (`recommend_activities`, `ExplorationRecommendation`, `RecommendationReason`), `verify.rs` (`VerificationDecision::dispose`), `corpus/replay.rs` (`replay_corpus`), `cli/{commands,recommend}.rs` (`run_recommend`) | Done |

## 2. Non-Functional Requirements (`SFV-NFR-001`–`012`)

| ID | Requirement | SpecForge Spec / Gate | Backing Modules & Tooling | Status |
|---|---|---|---|---|
| `SFV-NFR-001` | Backward compatibility with `specs/*.md` | `SF-001`–`SF-007` | `trace.rs` (`compile`), `graph/compile.rs` (`compile_graph`) | Done |
| `SFV-NFR-002` | Consistent stable ID conventions (`PREFIX-NNN`) | `SF-001`, `SF-007` | `trace.rs` (`RequirementId`), `graph/mod.rs` (`ArtifactId`) | Done |
| `SFV-NFR-003` | Engine-independent pure core (`wasm32`-clean) | `just wasm`, [ADR 0002](adr/0002-specification-driven-verification.md) | Default features build pure core (`wasm32-unknown-unknown`) | Done |
| `SFV-NFR-004` | Isolated per-run execution (no cross-run state leak) | `SF-011`, `SF-015`, `SF-019` | `proptest_adapter/mod.rs` (fresh `TestRng`), `orchestrator/{mod,run}.rs` | Done |
| `SFV-NFR-005` | Progress and terminal run-state visibility | `SF-011`, `SF-019` | `verify.rs` (`RunState`), `orchestrator/run.rs`; async daemon deferred (Section 5) | Scoped |
| `SFV-NFR-006` | Validate imported tool outputs against contracts | `SF-012`, `SF-016`, `SF-017` | `coverage.rs`, `dst.rs`, `formal/validate.rs` | Done |
| `SFV-NFR-007` | Record tool versions and configuration | `SF-008`, `SF-011`, `SF-016`, `SF-017` | `verify.rs` (`RunEnvelope`), `ledger/record.rs` (`ProducerMeta`), `dst.rs` | Done |
| `SFV-NFR-008` | Incremental/surgical recomputation on change | `SF-009`, `SF-015` | `assurance/freshness.rs` (`freshness_of`, `impact_set`) | Done |
| `SFV-NFR-009` | Machine-readable diagnostics (`--json`, typed errors) | `SF-007`, `SF-008`, `SF-015` | `graph/mod.rs` (`GraphDefect`), `ledger/mod.rs` (`LedgerError`), `cli/mod.rs` | Done |
| `SFV-NFR-010` | Trace redaction and security/retention policies | — | Consumer adapter/storage responsibility (Section 5) | Deferred |
| `SFV-NFR-011` | Offline/local execution | `just isolated check`, `SF-015` | Pure core + local `FsLedger` + bubblewrap gate | Done |
| `SFV-NFR-012` | Deterministic canonical serialization & SHA-256 | `SF-007`, `SF-008`, `SF-018` | `graph/digest.rs` (`canonical_json`, `ContentHash`), `argument/export.rs` | Done |

## 3. EARS Requirements (`EARS-001`–`043`)

| EARS ID | Summary | Specs | Primary `src/` Symbols |
|---|---|---|---|
| `EARS-001` | Preserve Specification Graph as canonical typed model | `SF-001`–`SF-007` | `graph::compile_graph`, `GraphSnapshot` |
| `EARS-002` | Bind every run to immutable snapshot & source revisions | `SF-007`, `SF-008`, `SF-011` | `RunEnvelope`, `EvidenceRecord::from_envelope` |
| `EARS-003` | Preserve evidence records as append-only artifacts | `SF-008` | `EvidenceLedger`, `FsLedger` |
| `EARS-004` | Distinguish evidence, assessments, claims, findings, counterexamples, decisions | `SF-007`–`SF-010` | `EvidenceRecord`, `ClaimAssessment`, `VerificationClaim`, `FindingRecord`, `Counterexample`, `VerificationDecision` |
| `EARS-010` | Record snapshot, revisions, properties, assumptions, config per run | `SF-008`, `SF-011`, `SF-019` | `VerificationRequest`, `RunEnvelope`, `orchestrator` |
| `EARS-011` | Property violation creates/associates finding with context | `SF-010`, `SF-011`, `SF-016` | `model::evaluate_property`, `FindingRecord` |
| `EARS-012` | Preserve original counterexample linked to property & run | `SF-010`, `SF-011`, `SF-013` | `Counterexample`, `CounterexampleRegistry::register_original` |
| `EARS-013` | Minimized counterexample preserves original & shrink metadata | `SF-011`, `SF-013` | `proptest_adapter::shrink`, `MinimizationMeta`, `validate_minimized` |
| `EARS-014` | Source dependency change identifies stale evidence & claims | `SF-009`, `SF-015` | `assurance::freshness_of`, `assurance::impact_set` |
| `EARS-015` | Inconclusive result preserved as inconclusive, never satisfied | `SF-009`, `SF-011`, `SF-015`–`SF-017` | `EvaluationResult::Inconclusive`, `FormalOutcome::{Unknown, Timeout, Unsupported}` |
| `EARS-016` | Validate external adapter output before acceptance | `SF-012`, `SF-016`, `SF-017` | `validate_imported_scenario`, `normalize_dst_report`, `normalize_formal_report` |
| `EARS-017` | Surface contradictory evidence and require explicit disposition | `SF-009`, `SF-018` | `assess_claim`, `Contradiction`, `ClaimStatus::RequiresReview` |
| `EARS-018` | Record verification decision and resulting revision relationship | `SF-010` | `verify::VerificationDecision::dispose` |
| `EARS-020` | Expose active run state and progress information | `SF-011`, `SF-019` | `RunState`, `orchestrator` (live async daemon deferred) |
| `EARS-021` | Disclose stale dependencies on affected claims | `SF-009`, `SF-015`, `SF-018` | `EvidenceFreshness::Stale`, `ClaimStatus::Stale`, `ArgumentDefeater` |
| `EARS-022` | Never mark claim `supported` with unresolved contradictions | `SF-009`, `SF-018` | `assess_claim`, `build_assurance_argument` |
| `EARS-023` | Display declared bounds alongside bounded formal results | `SF-017`, `SF-018` | `FormalOutcome::{BoundedSuccess, BoundedFailure}`, `FormalBounds`, `export_argument_markdown` |
| `EARS-030` | Unresolved counterexample never auto-mutates specifications | `SF-010`, `SF-014` | `RootCause::Unresolved`, `VerificationDecision::dispose`, `planner::recommend_activities` |
| `EARS-031` | Absent/invalid statistical assumptions refuse numeric confidence | `SF-020` | `statistics::{compute_qualified_estimate, SamplingModel}` |
| `EARS-032` | Insufficient replay metadata sets `replayable = false` | `SF-011`, `SF-013`, `SF-016` | `ReplayDescriptor::replayable`, `normalize_dst_report` |
| `EARS-033` | Reject corrupt/tampered evidence with diagnostics | `SF-008`, `SF-009`, `SF-015` | `EvidenceRecord::verify_digest`, `EvidenceLedger::verify_integrity`, `FsLedger::open_read_only` |
| `EARS-034` | Finite campaign without violations never claims universal proof | `SF-009`, `SF-011`, `SF-016`, `SF-018` | `ClaimAssessment::unbounded_proof`, `AssuranceArgument::unbounded_proof` |
| `EARS-040` | Retain deterministic replay descriptor when supported | `SF-011`, `SF-012`, `SF-016` | `ReplayDescriptor`, `DstReplayMetadata` |
| `EARS-041` | Retain shrink metadata and original link when supported | `SF-011`, `SF-013` | `MinimizationMeta`, `Counterexample::original_id` |
| `EARS-042` | Retain references & SHA-256 digests for formal proof artifacts | `SF-017`, `SF-008` | `FormalArtifactRef`, `EvidenceRecord::artifact_digests` |
| `EARS-043` | Compute qualified statistical estimate when defensible model set | `SF-020` | `statistics::{compute_qualified_estimate, StatisticalMethod, QualifiedEstimate}` |

## 4. Acceptance Scenarios ([ACCEPTANCE_FEATURES.feature](../sources/verification-requirements/specifications/ACCEPTANCE_FEATURES.feature))

| # | Tag | Scenario | Specs | Backing Modules |
|---|---|---|---|---|
| 1 | `@P0` | Define a property linked to a requirement and behavioral model | `SF-007` | `graph/{mod,compile,validate}.rs` |
| 2 | `@P0` | Create a verification plan | `SF-007`, `SF-019` | `graph/mod.rs`, `orchestrator/{mod,run}.rs` |
| 3 | `@P0` | Execute a property check and record immutable evidence | `SF-008`, `SF-011`, `SF-015` | `proptest_adapter/`, `ledger/`, `ledger_fs.rs`, `cli/` |
| 4 | `@P0` | Preserve a property violation as a counterexample | `SF-010`, `SF-011`, `SF-015` | `model.rs`, `verify.rs`, `proptest_adapter/` |
| 5 | `@P0` | Do not treat incomplete exploration as a pass | `SF-009`, `SF-011`, `SF-015` | `proptest_adapter/mod.rs`, `assurance/mod.rs`, `cli/` |
| 6 | `@P0` | Preserve evidence history | `SF-008` | `ledger/mod.rs`, `ledger_fs.rs` |
| 7 | `@P0` | Identify stale evidence after a source change | `SF-009`, `SF-015` | `assurance/freshness.rs`, `cli/commands.rs` |
| 8 | `@P1` | Generate a scenario from a behavioral model | `SF-010`, `SF-012` | `model.rs`, `coverage.rs` |
| 9 | `@P1` | Preserve generated input and shrink metadata | `SF-011`, `SF-013` | `proptest_adapter/shrink.rs`, `corpus/mod.rs` |
| 10 | `@P1` | Replay a captured counterexample | `SF-011`, `SF-013`, `SF-016` | `proptest_adapter/mod.rs`, `corpus/replay.rs`, `dst.rs` |
| 11 | `@P1` | Record a diagnosis without silently changing specifications | `SF-010`, `SF-014` | `verify.rs` (`VerificationDecision::dispose`), `planner.rs` |
| 12 | `@P1` | Recommend exploration for a coverage gap | `SF-014`, `SF-015` | `coverage.rs`, `planner.rs`, `cli/{commands,recommend}.rs` |
| 13 | `@P2` | Preserve bounded formal verification results | `SF-017`, `SF-018` | `formal/`, `assurance/mod.rs`, `argument/` |
| 14 | `@P2` | Surface contradictory evidence | `SF-009`, `SF-018` | `assurance/mod.rs`, `argument/mod.rs` |
| 15 | `@P3` | Compute a qualified statistical estimate | `SF-020` | `statistics/{mod,validate}.rs` |

## 5. Deferred / Out-of-Scope Boundaries

Per `IMPLEMENTATION_HANDOFF.md` Deliverable 8, [ADR 0002](adr/0002-specification-driven-verification.md), and [ADR 0003](adr/0003-ledger-dst-spike-confirms-adapter-boundary.md):

1. **Concrete external tool bridges (TLA+/TLC, Apalache, Kani, Coq/Lean, Turmoil):** SpecForge owns engine-independent adapter contracts (`VerificationAdapter`, `MbtAdapter`, `DstAdapter`, `FormalAdapter`) and proves DST via `examples/ledger_dst_spike.rs`; tool-specific wrappers live in downstream crates (`SFV-NFR-003`).
2. **Async background job daemon (`SFV-NFR-005` / `EARS-020`):** Run states (`RunState`) and plan orchestration (`orchestrator/{mod,run}.rs`) are pure/synchronous; background scheduling and live streaming belong in the consumer host.
3. **Trace redaction and retention hooks (`SFV-NFR-010`):** Secret scrubbing, PII redaction, and retention TTLs belong in consumer adapters/storage layers before sealing blobs into `FsLedger`.
4. **Structured type-level PBT value generators (`SFV-FR-033`):** `ProptestAdapter` generates event sequences over `StateMachine` alphabets; domain-type value strategies are supplied by consumer adapters via `VerificationAdapter` or `ProptestAdapter::with_runner`.
5. **Gherkin `Scenario Outline` and multi-writer concurrent ledgers:** `Scenario Outline` is refused as a defect (`SF-002`, [strategy.md](strategy.md) D7); `FsLedger` provides single-writer append-only JSONL persistence, leaving locked or database-backed multi-writer stores behind the `EvidenceLedger` contract.

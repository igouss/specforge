# Requirements Specification — Specification-Driven Verification

**Document ID:** SF-SDV-REQ-001  
**Version:** 0.1  
**Status:** Draft baseline  
**Parent system:** Existing SpecForge project

## 1. Purpose

Specify the requirements for extending SpecForge with an integrated verification capability that coordinates MBT, PBT, DST, formal verification, counterexample analysis, evidence management, and assurance assessment.

## 2. Goals

- G-01: Make verification intent first-class in the Specification Graph.
- G-02: Reuse specialized verification tools through stable adapter contracts.
- G-03: Connect generated scenarios and executions to the exact specifications and models that motivated them.
- G-04: Make failures reproducible and counterexamples actionable.
- G-05: Preserve immutable evidence and its provenance.
- G-06: Assess evidence against explicit claims, assumptions, scope, and method limitations.
- G-07: Detect stale evidence when relevant source artifacts change.
- G-08: Use verification findings to guide additional exploration and engineering decisions.

## 3. Non-goals

- NG-01: Build a new general-purpose programming or specification language.
- NG-02: Replace established PBT, MBT, DST, theorem-proving, or model-checking engines.
- NG-03: Guarantee correctness merely because a test campaign passes.
- NG-04: Collapse all verification outcomes into a single confidence percentage.
- NG-05: Automatically rewrite requirements or behavioral models based on generated findings.
- NG-06: Make production telemetry a substitute for controlled verification.
- NG-07: Require a probabilistic inference engine for the initial release.
- NG-08: Require a particular external tool or database in the canonical domain model.

## 4. Actors and external systems

| Actor/system | Responsibility |
|---|---|
| Specification author | Authors requirements, models, properties, assumptions, and claims |
| Verification engineer | Defines verification plans, coverage goals, and acceptance criteria |
| Developer/implementer | Supplies implementation under test and resolves implementation defects |
| AI engineering agent | Proposes scenarios, analysis, diagnoses, and plans within approved boundaries |
| Human reviewer | Approves consequential specification/model/assumption changes |
| MBT engine | Generates behavioral paths/scenarios |
| PBT engine | Generates input data and action sequences; may shrink failures |
| DST engine | Executes under controlled time, scheduling, and fault conditions |
| Formal engine | Checks mathematical models and returns proof/check status or counterexamples |
| Evidence evaluator | Assesses evidence relevance, validity, freshness, and limitations |

## 5. Functional requirements

### 5.1 Specification and graph integration

- SFV-FR-001: The system MUST represent a `Property` as a first-class typed artifact in the Specification Graph.
- SFV-FR-002: A property MUST have a stable ID, kind, subject/scope, formal or executable predicate reference, assumptions, and verification status.
- SFV-FR-003: The system MUST support at least safety and liveness property classifications.
- SFV-FR-004: A property MUST be linkable to requirements, behavioral models, state machines, invariants, scenarios, and verification claims.
- SFV-FR-005: The system MUST preserve the distinction between a requirement, a model, a property, a scenario, an execution, a finding, evidence, and a claim.
- SFV-FR-006: Verification inputs MUST resolve against a revision-bound, immutable graph snapshot.
- SFV-FR-007: The system MUST represent assumptions explicitly and link them to the properties and verification runs that depend on them.
- SFV-FR-008: The system MUST represent verification scope and bounds, including relevant model bounds, execution horizon, topology, and fault assumptions where applicable.

### 5.2 Verification planning and orchestration

- SFV-FR-010: The system MUST represent a `VerificationPlan` containing one or more verification activities, target properties, scope, assumptions, budgets, and required evidence.
- SFV-FR-011: The system MUST support selecting one or more methods: example-based, PBT, MBT, DST, and formal verification.
- SFV-FR-012: The system MUST permit multiple engines/methods to contribute separate results to the same claim without erasing method-specific semantics.
- SFV-FR-013: The orchestrator MUST select activities based on explicit plan configuration and MAY recommend activities based on coverage gaps, counterexamples, or stale evidence.
- SFV-FR-014: Every execution MUST identify the exact graph snapshot and source revisions used.
- SFV-FR-015: The system MUST record requested, started, completed, failed, cancelled, and inconclusive run states distinctly.

### 5.3 Model-based testing

- SFV-FR-020: The system MUST provide an MBT adapter contract for importing or generating scenarios from behavioral models.
- SFV-FR-021: MBT scenarios MUST retain traceability to the model states, transitions, requirements, and properties they exercise.
- SFV-FR-022: The system MUST record the model traversal strategy and coverage objective used for scenario generation.
- SFV-FR-023: The system MUST support deterministic replay metadata when the MBT engine exposes seeds or equivalent controls.
- SFV-FR-024: The system SHOULD support state, transition, transition-pair, and requirement coverage metrics when the source model supports them.

### 5.4 Property-based testing

- SFV-FR-030: The system MUST provide a PBT adapter contract for generated inputs and/or action sequences.
- SFV-FR-031: PBT results MUST record generator identity/version, seed or replay token, number of cases, constraints, and shrink information where available.
- SFV-FR-032: The system MUST preserve the distinction between generated input variation and behavioral path variation.
- SFV-FR-033: The system SHOULD allow structured generators to be associated with graph types and constraints.
- SFV-FR-034: A minimized failing input MUST retain a link to the original failing case and execution.

### 5.5 Deterministic simulation testing

- SFV-FR-040: The system MUST provide a DST adapter contract without requiring the core platform to implement a DST runtime.
- SFV-FR-041: DST evidence MUST capture available deterministic replay data, including seed, scheduler decisions, virtual-time events, fault injections, and runtime configuration.
- SFV-FR-042: The system MUST record which determinism guarantees are provided by the external engine and which are not.
- SFV-FR-043: A DST execution trace MUST be linkable to the scenario, property evaluation, counterexample, implementation revision, and graph snapshot.
- SFV-FR-044: The system SHOULD support replay of a failing execution from its captured replay descriptor.

### 5.6 Formal verification

- SFV-FR-050: The system MUST provide an adapter contract for model checkers and/or theorem provers.
- SFV-FR-051: Formal results MUST preserve the engine's actual result category, bounds, assumptions, and proof/check artifacts.
- SFV-FR-052: The system MUST distinguish proved, refuted, bounded-success, bounded-failure, unknown, timeout, and unsupported outcomes where applicable.
- SFV-FR-053: The system MUST NOT translate formal proof results into probabilistic confidence scores.
- SFV-FR-054: The system SHOULD support links between a SpecForge property and its external formal representation.

### 5.7 Property evaluation and findings

- SFV-FR-060: The system MUST provide a common property-evaluation result model for trace-checkable properties.
- SFV-FR-061: A property evaluation MUST identify the property revision, evaluator identity/version, input evidence, result, and scope.
- SFV-FR-062: The system MUST distinguish `satisfied`, `violated`, `inconclusive`, `not_evaluated`, and `invalid_evaluation`.
- SFV-FR-063: A violation MUST create or reference a `Finding`.
- SFV-FR-064: A finding MUST be classifiable as implementation, specification, behavioral-model, environment/assumption, verification, or expected-behavior issue; classification MAY remain unresolved.
- SFV-FR-065: The system MUST NOT infer the root cause solely from the fact that a property was violated.

### 5.8 Counterexamples

- SFV-FR-070: A counterexample MUST be a first-class artifact representing a concrete witness of a property violation or formal-model failure.
- SFV-FR-071: A counterexample MUST reference the violated property, originating finding, execution/model-check run, and relevant source revisions.
- SFV-FR-072: Where available, a counterexample MUST preserve initial state, ordered events/transitions, generated inputs, schedule decisions, virtual-time progression, fault injections, and violation location.
- SFV-FR-073: The system MUST preserve the original counterexample when a minimized version is created.
- SFV-FR-074: The system SHOULD support input, scenario, schedule, and environment minimization as separate capabilities.
- SFV-FR-075: Minimization MUST preserve causal and model-validity constraints; invalid minimized traces MUST be rejected or marked invalid.
- SFV-FR-076: A counterexample MUST be replayable when its source engine supports replay; otherwise the limitation MUST be explicit.
- SFV-FR-077: Counterexample analysis MAY recommend additional scenarios or changes but MUST NOT directly mutate authoritative specifications.

### 5.9 Evidence ledger

- SFV-FR-080: The system MUST provide an append-only Evidence Ledger.
- SFV-FR-081: An evidence record MUST have a stable ID, record type, producer, timestamp, content digest, source revisions, graph snapshot, and provenance links.
- SFV-FR-082: Evidence records MUST NOT be modified in place; corrections and supersession MUST be represented by new records and relationships.
- SFV-FR-083: The ledger MUST support evidence relationships including supports, contradicts, derived-from, supersedes, invalidates, and corroborates.
- SFV-FR-084: The ledger MUST retain negative evidence, including counterexamples and failed/inconclusive runs.
- SFV-FR-085: The system MUST be able to export evidence and provenance in a machine-readable format.
- SFV-FR-086: The system SHOULD support content-addressed artifact storage for large traces and logs, while keeping metadata and digests in the ledger.

### 5.10 Claims and assurance assessment

- SFV-FR-090: The system MUST represent a `VerificationClaim` as a proposition about a system or component under an explicit scope.
- SFV-FR-091: A claim MUST link to supporting/contradicting evidence, assumptions, scope, and assessment history.
- SFV-FR-092: The system MUST distinguish evidence facts from assessments and engineering decisions.
- SFV-FR-093: The evidence evaluator MUST assess relevance, validity, provenance completeness, freshness, scope compatibility, and known dependence/correlation.
- SFV-FR-094: The system MUST report unresolved contradictions and evidence gaps.
- SFV-FR-095: The system MUST support an assurance argument structure with claim, argument/rationale, evidence, and assumptions.
- SFV-FR-096: The system MUST use qualified statuses such as supported, refuted, inconclusive, stale, and requires-review; status MUST NOT imply universal proof unless the method actually establishes it.
- SFV-FR-097: The system MAY compute statistical estimates when a defensible probabilistic model and sampling assumptions are declared.
- SFV-FR-098: Every statistical estimate MUST include its method, assumptions, population/sampling scope, sample size, uncertainty interval where applicable, and limitations.
- SFV-FR-099: The system MUST NOT derive a confidence percentage by simply counting passing tests or combining heterogeneous evidence scores.

### 5.11 Freshness and dependency management

- SFV-FR-100: The system MUST maintain dependency links from evidence to the exact requirements, properties, models, assumptions, implementation revisions, tools, and configurations used.
- SFV-FR-101: When a dependency changes, the system MUST identify potentially stale evidence and affected claims.
- SFV-FR-102: The system MUST distinguish stale, invalidated, superseded, and still-applicable evidence.
- SFV-FR-103: The system SHOULD calculate the impact set of a changed graph node using dependency relationships.
- SFV-FR-104: The system MUST preserve historical evidence even when it becomes stale.

### 5.12 Feedback and engineering decisions

- SFV-FR-110: The system MUST support findings that recommend a next verification activity.
- SFV-FR-111: The system SHOULD recommend exploration based on uncovered states/transitions, weak input exploration, untested schedules, unverified assumptions, and stale evidence.
- SFV-FR-112: The system MUST represent a `VerificationDecision` separately from a counterexample and evidence record.
- SFV-FR-113: A decision MUST record the finding/evidence considered, diagnosis, rationale, disposition, approver, and resulting artifact changes.
- SFV-FR-114: Changes to authoritative requirements, models, properties, or assumptions MUST follow the project's normal review/approval workflow.
- SFV-FR-115: The system SHOULD support replaying previously discovered counterexamples as regression checks.

## 6. Non-functional requirements

- SFV-NFR-001: The extension MUST preserve compatibility with existing SpecForge project files and graph semantics, or provide explicit migrations.
- SFV-NFR-002: All persisted IDs MUST follow existing SpecForge identity conventions.
- SFV-NFR-003: Core domain semantics MUST be independent of any single external verification engine.
- SFV-NFR-004: Verification runs MUST be isolated sufficiently to prevent cross-run state contamination.
- SFV-NFR-005: Long-running activities MUST expose progress and terminal state.
- SFV-NFR-006: The system MUST validate imported tool output against versioned adapter schemas.
- SFV-NFR-007: The system MUST record tool versions and relevant configuration for reproducibility.
- SFV-NFR-008: The system SHOULD support incremental recomputation of evidence assessments.
- SFV-NFR-009: The system MUST provide machine-readable diagnostics for invalid properties, missing assumptions, incompatible scopes, and malformed evidence.
- SFV-NFR-010: Sensitive implementation data and large traces MUST follow the existing project's security and retention policies.
- SFV-NFR-011: The system SHOULD support offline/local execution where supported by the integrated engines.
- SFV-NFR-012: The system MUST provide deterministic serialization for canonical graph snapshots and evidence metadata.

## 7. Required outcome semantics

| Outcome | Meaning |
|---|---|
| satisfied | Evaluator found no violation in the evaluated scope |
| violated | A valid witness demonstrates a property violation |
| inconclusive | Method/budget/bounds did not establish satisfaction or violation |
| proved | Formal method established the proposition under recorded assumptions |
| bounded_success | Property holds within declared finite bounds |
| bounded_failure | Counterexample exists within declared finite bounds |
| stale | Evidence dependencies changed; applicability requires reassessment |
| invalid | Evidence or evaluation failed validation |

`passed` may be used as a test-run status for a particular test, but MUST NOT be treated as a universal verification conclusion.

## 8. Priority

- P0: graph integration, properties, claims, verification plan, common results, evidence ledger, provenance, stale detection, first vertical slice.
- P1: MBT/PBT adapters, counterexample registry, replay, regression corpus, coverage feedback.
- P2: DST adapters, formal verification adapters, assurance argument export.
- P3: qualified probabilistic assessment and advanced exploration planning.

## 9. Acceptance summary

The release must demonstrate at least one complete path from specification to property evaluation to immutable evidence and claim assessment. Detailed scenarios are in `specifications/ACCEPTANCE_FEATURES.feature`.

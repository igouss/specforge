# Implementation Roadmap

The phases are a recommended sequence, not a requirement to build every capability before delivering value.

## Phase 0 — Repository reconnaissance and mapping

Deliverables:
- Existing architecture and graph inventory.
- Mapping of current entities to this domain model.
- Gap analysis and compatibility report.
- ADRs for any necessary schema or boundary changes.

Exit criteria:
- No duplicate abstractions proposed without justification.
- Implementation plan is mapped to existing project conventions.

## Phase 1 — Verification foundation (P0)

Implement:
- Property and VerificationClaim graph artifacts.
- Assumption and scope representation.
- VerificationPlan and VerificationRun lifecycle.
- Common result vocabulary.
- Immutable EvidenceRecord and provenance relationships.
- Evidence-to-source dependency links.
- Basic stale evidence detection.

Exit criteria:
- Unit/integration tests cover graph validation, append-only behavior, and dependency invalidation.

## Phase 2 — First executable vertical slice (P0)

Implement:
- One trace-checkable safety property.
- One small executable state machine or existing model representation.
- One scenario.
- One property evaluator.
- One engine integration using the project's most suitable existing test runner.
- One claim assessment and evidence record.

Exit criteria:
- Automated end-to-end test demonstrates specification → scenario → execution → property result → evidence → claim.
- A source change marks dependent evidence stale.

## Phase 3 — MBT and PBT adapters (P1)

Implement:
- MBT scenario generation/import contract.
- Model coverage records.
- PBT generator/input contract.
- Seed/replay and shrink metadata.
- Counterexample registry and original/minimized lineage.

Exit criteria:
- A model-generated scenario and a generated input can both be traced to their source definitions.
- A failing generated case is preserved and minimized when supported.

## Phase 4 — DST integration (P2)

Implement:
- DST adapter contract.
- Schedule, virtual-time, fault, and replay metadata.
- Determinism guarantee declaration.
- Counterexample replay integration.

Exit criteria:
- A DST run contributes traceable evidence.
- A captured failure is replayed where the engine supports it.

## Phase 5 — Formal verification adapters (P2)

Implement:
- Formal property mapping.
- Model checker/prover adapter.
- Formal result normalization preserving bounds and assumptions.
- External proof/counterexample artifact references.

Exit criteria:
- A bounded result is explicitly represented as bounded.
- A proof result retains its actual scope and assumptions.

## Phase 6 — Assurance and feedback planning (P1/P2)

Implement:
- Evidence assessment dimensions.
- Claim–argument–evidence structure.
- Contradiction and gap detection.
- Exploration recommendations.
- VerificationDecision workflow and review integration.

Exit criteria:
- A coverage gap recommends a targeted activity.
- A counterexample can be diagnosed and dispositioned without automatic source mutation.

## Phase 7 — Statistical assessment (P3, optional)

Only proceed when a concrete use case and defensible sampling model exist.

Implement:
- Declared statistical models.
- Qualified estimates and uncertainty.
- Correlation/dependence handling.
- Clear separation from proof and deterministic evidence.

Exit criteria:
- No unqualified single confidence score.
- Estimates are reproducible from retained data and method metadata.

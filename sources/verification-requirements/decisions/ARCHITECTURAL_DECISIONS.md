# Architectural Decision Records

These are baseline decisions for the extension. The implementation agent should reconcile them with existing SpecForge decisions and create formal ADRs in the repository's established format.

## ADR-SDV-001 — Extend SpecForge; do not create a separate platform

**Decision:** Implement verification as a capability of the existing SpecForge project.

**Rationale:** Existing specification language, graph, IDs, source formats, and Git workflow are the foundation. A separate platform would duplicate semantics and fragment traceability.

## ADR-SDV-002 — Specification Graph is the canonical integration model

**Decision:** External engines consume revision-bound graph representations through adapters. Their internal schemas do not become the canonical model.

**Rationale:** Engine replacement and multi-engine verification must not force specification rewrites.

## ADR-SDV-003 — Reuse engines; build integration and evidence semantics

**Decision:** Integrate established MBT, PBT, DST, and formal tools. Build the shared domain model, orchestration, adapter contracts, evidence ledger, assessment, traceability, and feedback.

**Rationale:** Specialized engines are costly to reproduce. SpecForge's differentiating value is connecting their results to one specification.

## ADR-SDV-004 — Evidence is append-only

**Decision:** Evidence records are immutable. Corrections, new runs, and changed interpretations create new records and explicit relationships.

**Rationale:** Historical provenance and auditability must remain trustworthy.

## ADR-SDV-005 — Separate evidence from assessment

**Decision:** The ledger records facts; an evaluator produces assessments. Assessments may be recomputed without mutating evidence.

**Rationale:** New evidence or assessment policy can change the interpretation of historical facts.

## ADR-SDV-006 — Probabilistic confidence is qualified and optional

**Decision:** Do not use a universal confidence percentage. Statistical estimates require explicit models, assumptions, scope, sample data, and uncertainty.

**Rationale:** Test counts do not directly establish probability of correctness; formal proof and deterministic facts have different semantics.

## ADR-SDV-007 — Counterexamples do not authorize automatic specification changes

**Decision:** A counterexample triggers diagnosis and a VerificationDecision. Authoritative changes follow normal review/approval.

**Rationale:** A failure may be caused by implementation, specification, model, environment, assumptions, or verification logic.

## ADR-SDV-008 — Preserve result distinctions

**Decision:** Keep `satisfied`, `violated`, `inconclusive`, formal `proved`, bounded outcomes, and claim assessments distinct.

**Rationale:** A finite successful campaign is not a proof; bounded success is not unbounded success.

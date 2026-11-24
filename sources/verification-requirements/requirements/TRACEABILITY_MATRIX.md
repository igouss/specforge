# Requirements Traceability Matrix

This matrix is a starting point. The implementation agent should add concrete test IDs and code references as implementation proceeds.

| Requirement range | Primary artifact/component | Verification evidence expected |
|---|---|---|
| SFV-FR-001–008 | Specification Graph integration | Graph schema and validation tests |
| SFV-FR-010–015 | Verification planning/orchestration | Plan lifecycle and run-state tests |
| SFV-FR-020–024 | MBT adapter | Adapter contract tests and scenario provenance |
| SFV-FR-030–034 | PBT adapter | Generator/seed/shrinking tests |
| SFV-FR-040–044 | DST adapter | Replay descriptor and trace integration tests |
| SFV-FR-050–054 | Formal adapter | Status/bounds/assumption mapping tests |
| SFV-FR-060–065 | Property evaluator | Predicate evaluation and result semantics tests |
| SFV-FR-070–077 | Counterexample manager | Capture, validation, minimization, replay tests |
| SFV-FR-080–086 | Evidence Ledger | Append-only, digest, provenance, export tests |
| SFV-FR-090–099 | Evidence/assurance assessment | Claim support, contradiction, statistical policy tests |
| SFV-FR-100–104 | Freshness engine | Dependency impact and stale-state tests |
| SFV-FR-110–115 | Feedback/decisions | Recommendation and approval workflow tests |
| SFV-NFR-001–012 | Cross-cutting | Compatibility, isolation, validation, serialization tests |

## Minimum vertical-slice traceability

| Step | Required link |
|---|---|
| Requirement | Stable requirement ID and source revision |
| Behavioral model | Model ID and revision |
| Property | Property ID and revision |
| Scenario | Scenario ID and generated-from relationship |
| Run | Run ID, implementation revision, graph snapshot |
| Evaluation | Evaluator version, property revision, result |
| Evidence | Immutable record ID, digest, provenance |
| Claim | Claim ID, scope, assumptions, assessment |
| Change | Dependency change and stale evidence impact |

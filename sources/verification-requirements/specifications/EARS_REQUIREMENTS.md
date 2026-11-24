# EARS-Style Requirements

**Document ID:** SF-SDV-EARS-001  
These requirements complement the numbered functional requirements. They express externally observable system behavior in EARS form.

## Ubiquitous requirements

- EARS-001: The system shall preserve the existing SpecForge Specification Graph as the canonical typed representation of verification definitions.
- EARS-002: The system shall associate each verification run with an immutable graph snapshot and source revision set.
- EARS-003: The system shall preserve evidence records as append-only artifacts.
- EARS-004: The system shall distinguish evidence records, evidence assessments, verification claims, findings, counterexamples, and verification decisions.

## Event-driven requirements

- EARS-010: When a verification run is created, the system shall resolve and record its target graph snapshot, source revisions, properties, assumptions, and tool configuration.
- EARS-011: When a property evaluation reports a violation, the system shall create or associate a finding and preserve the evaluation context.
- EARS-012: When a valid counterexample is captured, the system shall preserve its original form and associate it with the violated property and originating run.
- EARS-013: When a counterexample is minimized, the system shall preserve the original counterexample and record the minimization method and resulting artifact.
- EARS-014: When a source dependency changes, the system shall identify evidence and claims that may be stale.
- EARS-015: When a verification engine returns an inconclusive result, the system shall preserve that result as inconclusive and shall not report it as satisfied.
- EARS-016: When an external adapter returns data, the system shall validate the data against the adapter contract before accepting it.
- EARS-017: When an evidence assessment detects contradictory evidence, the system shall surface the contradiction and require explicit disposition.
- EARS-018: When a verification decision changes an authoritative artifact, the system shall record the decision and resulting revision relationship.

## State-driven requirements

- EARS-020: While a verification run is active, the system shall expose its current state and available progress information.
- EARS-021: While evidence is stale, the system shall mark affected claims as requiring reassessment or otherwise clearly disclose the stale dependency.
- EARS-022: While a claim has unresolved contradictory evidence, the system shall not mark the claim as supported without qualification.
- EARS-023: While a formal result is bounded, the system shall display the declared bounds alongside the result.

## Unwanted behavior requirements

- EARS-030: If a counterexample diagnosis is unresolved, the system shall not automatically change the specification.
- EARS-031: If statistical assumptions are absent or invalid, the system shall not publish a numerical confidence estimate.
- EARS-032: If replay metadata is insufficient, the system shall not claim that the counterexample is reproducible.
- EARS-033: If imported evidence fails schema or integrity validation, the system shall reject or quarantine it and report diagnostics.
- EARS-034: If a finite simulation campaign completes without a violation, the system shall not infer universal correctness.

## Optional-feature requirements

- EARS-040: Where an external engine supports deterministic replay, the system shall retain the replay descriptor.
- EARS-041: Where an external engine supports shrinking, the system shall retain shrink metadata and link the minimized case to the original.
- EARS-042: Where an external formal tool provides proof artifacts, the system shall retain references or content digests for those artifacts.
- EARS-043: Where a defensible statistical model is configured, the system may compute a qualified estimate with explicit assumptions and uncertainty.

# Orchestration: verification plan execution, run lifecycle and evidence correlation

SpecForge orchestrates multi-activity verification plans across registered method adapters, enforces explicit non-regressing run lifecycles, checks required evidence declarations, and discloses correlated supporting evidence so repeated runs are never mistaken for independent corroboration.

### SF-019 @ v1 — verification plans orchestrate adapters across explicit run lifecycles and disclose correlated evidence

> WHEN a verification plan is compiled with `required_evidence`, SpecForge SHALL include `required_evidence` in the plan's semantic digest and reject any blank entry as a block defect.
>
> WHEN a verification plan is prepared or executed against a snapshot, adapter registry, and evidence ledger, SpecForge SHALL reject unknown plans or plans with no target properties, bind each run to the snapshot digest, execute each activity through the adapter registered for its method while preserving separate method-specific evidence records, validate returned envelope identity (including method, activity_id, claim_ids, and conclusive outcome for completed runs) before sealing into the ledger, fail any activity whose method has no registered adapter without reporting completed, and report any `required_evidence` entry as missing unless at least one matching run exists and every matching run completed with a positive outcome (`satisfied`, `proved`, or `bounded_success`).
>
> WHEN a verification run transitions between lifecycle states, SpecForge SHALL permit only `Requested -> Started -> Completed | Failed | Cancelled | Inconclusive` and `Requested -> Cancelled`, SHALL enforce monotonic progress advancement, and SHALL reject any illegal transition including backwards progress or mutation of a terminal run.
>
> WHEN two or more current supporting evidence records for the same claim share the same `(producer.name, method, seed)`, the same `replay_token`, or the same `(producer.name, method, property_id, snapshot_id)` with identical non-empty `replay.events`, SpecForge SHALL disclose an explicit `CorrelationNote` in `ClaimAssessment::correlations` and `rationale`.

```gherkin
@SF-019 @v1
Feature: SF-019 — verification plans orchestrate adapters across explicit run lifecycles and disclose correlated evidence

  @one @witness
  Scenario: multi-activity verification plan executes across registered adapters and records separate immutable evidence
    Given a compiled snapshot from "sdv-orchestration.md"
    And an empty evidence ledger
    And an adapter registry with safe property-based and deterministic-simulation adapters
    When verification plan "PLAN-ORCH-001" is executed with seed 42 at "2026-06-01T12:00:00Z"
    Then the plan execution report has 2 completed runs with distinct methods "property_based, deterministic_simulation"
    And the plan execution report recorded 2 immutable evidence records in the ledger
    And the plan execution report has no missing required evidence
    And the adapter registry debug summary lists the registered methods
    And assessing claim "CLM-MSG-001" is "supported" with 0 correlation notes

  @near-miss
  Scenario: run lifecycle enforces legal state transitions and rejects mutations from terminal states
    Given a compiled snapshot from "sdv-orchestration.md"
    When verification plan "PLAN-ORCH-001" is prepared
    Then 2 runs are prepared in state "requested" bound to the snapshot
    And a prepared run transitions cleanly through requested, started, progress, and completed
    And a started run rejects backwards progress advancement
    And a started run allows idempotent progress updates, clamps progress to a positive budget, and allows unbounded progress when budget is zero
    And starting a run with a blank timestamp is rejected
    And a completed run rejects further start, progress, fail, cancel, and inconclusive transitions
    And a prepared run can cancel directly from requested and rejects transitions after cancelled
    And a prepared run can transition from started to inconclusive and rejects transitions after inconclusive
    And a prepared run rejects completing, failing, or advancing progress directly from requested

  @near-miss
  Scenario: missing adapter fails the activity cleanly and reports missing required evidence
    Given a compiled snapshot from "sdv-orchestration.md"
    And an empty evidence ledger
    And an adapter registry with only a safe property-based adapter
    When verification plan "PLAN-MISSING-001" is executed with seed 7 at "2026-06-01T12:05:00Z"
    Then activity "ACT-PBT-002" finishes in state "completed" with an evidence record
    And activity "ACT-FORMAL-001" finishes in state "failed" with no evidence record and an explicit error
    And the plan execution report reports missing required evidence "formal"

  @near-miss
  Scenario: unknown plans, empty plan targets, blank required_evidence, and mismatched envelopes are rejected
    Given a compiled snapshot from "sdv-orchestration.md"
    And an empty evidence ledger
    When unknown verification plan "PLAN-NOPE-999" is prepared
    Then orchestration fails with an error mentioning "unknown verification plan"
    When empty-target verification plan "PLAN-EMPTY-001" is prepared
    Then orchestration fails with an error mentioning "no target properties"
    When a plan with a blank required_evidence entry is compiled
    Then blank required_evidence compilation fails with a graph defect mentioning "required_evidence"
    When verification plan "PLAN-ORCH-001" is executed against an adapter returning a mismatched envelope
    Then orchestration fails with an error mentioning "mismatch" and leaves the ledger empty
    When verification plan "PLAN-ORCH-001" is executed against an adapter returning a mismatched method envelope
    Then orchestration fails with an error mentioning "mismatch" and leaves the ledger empty
    When verification plan "PLAN-ORCH-001" is executed against an adapter returning a mismatched activity envelope
    Then orchestration fails with an error mentioning "mismatch" and leaves the ledger empty
    When verification plan "PLAN-ORCH-001" is executed against an adapter returning a mismatched claims envelope
    Then orchestration fails with an error mentioning "mismatch" and leaves the ledger empty
    When verification plan "PLAN-ORCH-001" is executed against an adapter returning a completed inconclusive envelope
    Then orchestration fails with an error mentioning "mismatch" and leaves the ledger empty
    When verification plan "PLAN-ORCH-001" is executed with seed 1 at "   "
    Then orchestration fails with an error mentioning "timestamp" and leaves the ledger empty

  @near-miss
  Scenario: violated runs and multi-target partial failures never satisfy required_evidence
    Given a compiled snapshot from "sdv-orchestration.md"
    And an empty evidence ledger
    And an adapter registry with violating property-based and safe deterministic-simulation adapters
    When verification plan "PLAN-ORCH-001" is executed with seed 42 at "2026-06-01T12:06:00Z"
    Then the plan execution report reports missing required evidence "property_based"
    Given an empty evidence ledger
    And an adapter registry with only a safe property-based adapter
    When verification plan "PLAN-MULTI-001" is executed with seed 7 at "2026-06-01T12:07:00Z"
    Then the plan execution report reports missing required evidence "property_based"

  @near-miss
  Scenario: correlated supporting evidence is disclosed explicitly while independent runs report no correlation
    Given a compiled snapshot from "sdv-orchestration.md"
    And an empty evidence ledger
    And an adapter registry with safe property-based and deterministic-simulation adapters
    When verification plan "PLAN-ORCH-001" is executed with seed 99 at "2026-06-01T12:10:00Z"
    And verification plan "PLAN-ORCH-001" is executed again with seed 99 at "2026-06-01T12:11:00Z"
    And claim "CLM-MSG-001" is assessed after orchestration
    Then the orchestrated claim assessment discloses correlation notes mentioning "property_based" and "seed 99"
    And the orchestrated claim assessment rationale discloses the correlation
    Given an empty evidence ledger
    When two supporting records share identical replay token "shared-token-1" for "PROP-MSG-001" and claim "CLM-MSG-001"
    And claim "CLM-MSG-001" is assessed after orchestration
    Then the orchestrated claim assessment discloses a correlation note mentioning "shared-token-1"
    Given an empty evidence ledger
    When two supporting records share identical replay events "deliver, ack" for "PROP-MSG-001" and claim "CLM-MSG-001"
    And claim "CLM-MSG-001" is assessed after orchestration
    Then the orchestrated claim assessment discloses a correlation note mentioning "identical replay events"

  @one
  Scenario: repeated plan runs allocate sequential evidence ids and match activity, property, and qualified required_evidence entries
    Given a compiled snapshot from "sdv-orchestration.md"
    And an empty evidence ledger
    And an adapter registry with safe property-based and deterministic-simulation adapters
    When verification plan "PLAN-REQ-FORMS-001" is executed with seed 10 at "2026-06-01T12:20:00Z"
    And verification plan "PLAN-REQ-FORMS-001" is executed again with seed 11 at "2026-06-01T12:21:00Z"
    And verification plan "PLAN-REQ-FORMS-001" is executed again with seed 12 at "2026-06-01T12:22:00Z"
    Then the plan execution report has no missing required evidence
    And the plan execution report evidence ids are "EV-PLAN-REQ-FORMS-001-ACT-PBT-004-PROP-MSG-001-3, EV-PLAN-REQ-FORMS-001-ACT-DST-004-PROP-MSG-001-3"
    And the ledger holds 6 evidence records with sequential suffixes for "PLAN-REQ-FORMS-001"
```

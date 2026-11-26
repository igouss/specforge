# Exploration planner

### SF-014 @ v1 — the exploration planner recommends explained verification activities without mutating state

> WHEN the exploration planner inspects a graph snapshot, evidence ledger, coverage records, claim assessments, and counterexample corpus, SpecForge SHALL emit deterministic, explained recommendations for uncovered states, uncovered transitions, uncovered transition pairs, stale coverage or claim evidence, unresolved contradictions, unverified claims, and active regression counterexamples.
>
> WHILE generating exploration recommendations, SpecForge SHALL operate strictly read-only, suppress regression recommendations only when valid current replay evidence resolves a counterexample without conflicting regressions, and SHALL NOT mutate the graph snapshot, evidence ledger, or counterexample registry.

```gherkin
@SF-014 @v1
Feature: SF-014 — the exploration planner recommends explained verification activities without mutating state

  @one @witness
  Scenario: the planner recommends activities for coverage gaps, stale claims, and active counterexamples without mutating state
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And an empty counterexample registry
    When the proptest adapter verifies "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 against the safe message broker
    And the snapshot changes to "sdv-message-property-edit.md"
    And coverage gaps, stale claim assessment, and an active counterexample are planned
    Then the planner recommends "uncovered_states" via "model_based" targeting "SM-MSG-001" naming "redelivered"
    And the planner recommends "uncovered_transitions" via "model_based" targeting "SM-MSG-001" naming "acknowledged--redeliver-->redelivered"
    And the planner recommends "uncovered_transition_pairs" via "model_based" targeting "SM-MSG-001"
    And the planner recommends "stale_evidence" targeting "CLM-MSG-001" naming "EV-PBT-1"
    And the planner recommends "regression_counterexample" via "property_based" targeting "PROP-MSG-001"
    And planning leaves the snapshot digest and ledger unchanged

  @near-miss
  Scenario: the planner recommends review for contradictory claims and verification for unverified claims
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And an empty counterexample registry
    When an unverified claim is planned
    Then the planner recommends "unverified_claim" targeting "CLM-MSG-001" naming "PROP-MSG-001"
    When contradictory current evidence exists for "CLM-MSG-001" and is planned
    Then the planner recommends "unresolved_contradiction" targeting "CLM-MSG-001"
    And planning is deterministic across repeated calls
    When multiple assessments with different statuses for "CLM-MSG-001" are planned
    Then recommendation 1 has id "REC-001" and reason "unverified_claim"
    And recommendation 2 has id "REC-002" and reason "stale_evidence"

  @near-miss
  Scenario: resolved counterexamples suppress recommendations until a regression occurs and stale coverage emits stale_evidence
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And an empty counterexample registry
    When a registered counterexample is resolved by a current passing replay and planned
    Then the planner emits no "regression_counterexample" recommendation
    When a subsequent failing replay occurs for that counterexample and is planned
    Then the planner recommends "regression_counterexample" via "property_based" targeting "PROP-MSG-001"
    When a stale coverage record with a mismatched model digest is planned
    Then the planner recommends "stale_evidence" via "model_based" targeting "SM-MSG-001"
    And the planner emits no "uncovered_states" recommendation

  @near-miss
  Scenario: conflicting, inconclusive, or unrelated replays do not resolve an active counterexample
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And an empty counterexample registry
    When a registered counterexample has a failing replay followed by a passing replay and is planned
    Then the planner recommends "regression_counterexample" via "property_based" targeting "PROP-MSG-001"
    When a registered counterexample has only an inconclusive replay and is planned
    Then the planner recommends "regression_counterexample" via "property_based" targeting "PROP-MSG-001"
    When a registered counterexample has only a passing replay for a different counterexample on "PROP-MSG-001" and is planned
    Then the planner recommends "regression_counterexample" via "property_based" targeting "PROP-MSG-001"
```

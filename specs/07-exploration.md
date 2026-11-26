# Exploration: MBT coverage, counterexample corpus and exploration planner

SpecForge imports validated model-based testing scenarios, computes separate coverage dimensions over state-machine traces, maintains an immutable counterexample registry with validated minimized traces and regression replay, and produces read-only, explained exploration recommendations.

### SF-012 @ v1 — model-based testing imports validated scenarios and computes separate coverage dimensions

> WHEN an MBT adapter imports scenarios for a state machine in a graph snapshot, SpecForge SHALL validate required provenance fields including importer version and non-empty event sequences, verify the source model digest matches the snapshot, verify every event follows a declared transition from the initial state, and convert valid imports into generated scenarios with computed covered states and transitions.
>
> IF an imported scenario declares a mismatched model digest, unknown state machine, missing provenance field, empty event sequence, or invalid event transition, THEN SpecForge SHALL reject the import with a descriptive coverage error.
>
> WHEN coverage is computed for a state machine over execution traces, SpecForge SHALL validate trace continuity and declared transitions and report separate counts and uncovered sets for states, transitions, consecutive transition pairs, and linked requirements without collapsing them into a single score.

```gherkin
@SF-012 @v1
Feature: SF-012 — model-based testing imports validated scenarios and computes separate coverage dimensions

  @one @witness
  Scenario: importing valid MBT scenarios produces traceable generated scenarios with coverage
    Given a compiled snapshot from "sdv-message.md"
    When an MBT adapter imports valid scenarios for state machine "SM-MSG-001"
    Then 2 imported scenarios are validated for "SM-MSG-001"
    And every validated imported scenario records its strategy and source model digest
    And the validated imported scenarios cover states "queued, delivered, acknowledged"
    And the validated imported scenarios cover transitions "deliver, acknowledge"

  @near-miss
  Scenario: importing a scenario with stale digest, missing provenance, empty events, or invalid transition is rejected
    Given a compiled snapshot from "sdv-message.md"
    When an imported scenario with a stale model digest is validated for "SM-MSG-001"
    Then the scenario import fails with a digest mismatch error
    And the import error message mentions "digest mismatch"
    When an imported scenario with an impossible transition sequence is validated for "SM-MSG-001"
    Then the scenario import fails with an invalid scenario error
    When an imported scenario with empty events is validated for "SM-MSG-001"
    Then the scenario import fails with an invalid scenario error
    When an imported scenario with an empty importer version is validated for "SM-MSG-001"
    Then the scenario import fails with an invalid scenario error

  @one @witness
  Scenario: computing coverage over partial and zero-transition traces reports separate dimensions
    Given a compiled snapshot from "sdv-message.md"
    When coverage is computed for "SM-MSG-001" over the safe message trace
    Then state coverage reports 3 of 4 states covered and uncovered states "redelivered"
    And transition coverage reports 2 of 3 transitions covered and uncovered transitions "acknowledged--redeliver-->redelivered"
    And transition pair coverage reports 1 of 2 pairs covered and 1 uncovered pair
    And requirement coverage reports 1 of 1 linked requirements covered
    When coverage is computed for "SM-MSG-001" over a zero-transition trace
    Then state coverage reports 1 of 4 states covered and uncovered states "acknowledged, delivered, redelivered"
    And requirement coverage reports 0 of 1 linked requirements covered

  @near-miss
  Scenario: computing coverage over a discontinuous or undeclared transition trace is rejected
    Given a compiled snapshot from "sdv-message.md"
    When coverage is computed for "SM-MSG-001" over a discontinuous trace
    Then coverage computation fails with an invalid trace error
    And the coverage error message mentions "invalid execution trace"
    When coverage is computed for "SM-MSG-001" over a continuous undeclared transition trace
    Then coverage computation fails with an invalid trace error
    When coverage is computed for "SM-MSG-001" over a trace landing in the wrong target state
    Then coverage computation fails with an invalid trace error
```

### SF-013 @ v1 — the counterexample registry validates minimized traces and replays regressions into the ledger

> WHEN an original counterexample is registered, SpecForge SHALL append it immutably to the registry and reject empty or duplicate identifiers or entries carrying minimization linkage or metadata with a descriptive corpus error.
>
> WHEN a minimized counterexample is registered, SpecForge SHALL verify it references a registered non-minimized original, carries consistent minimization metadata not exceeding the original length, validates replay descriptors (requiring a seed or token for replayable zero-event traces and rejecting mismatched replay events on both replayable and non-replayable traces), matches the original and snapshot property identity and digest, follows only declared continuous transitions from the machine's initial state, and reproduces the exact recorded violation before appending it without mutating the original.
>
> WHEN the counterexample corpus is replayed against a verification adapter, SpecForge SHALL replay the latest minimized child of each original when present, verify envelope identity and non-colliding replay evidence identifiers before appending replay evidence atomically to the ledger without mutating stored counterexamples, and report non-replayable entries as skipped rather than resolved.

```gherkin
@SF-013 @v1
Feature: SF-013 — the counterexample registry validates minimized traces and replays regressions into the ledger

  @one @witness
  Scenario: registering an original and valid minimized counterexample preserves both and replays regressions
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And an empty counterexample registry
    And the proptest adapter verifies "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 against the buggy redelivering broker
    When the discovered original and minimized counterexamples are registered in the corpus
    Then the registry holds 1 original and 1 minimized counterexample
    When the corpus is replayed against the buggy redelivering broker
    Then the corpus replay entry 1 has outcome "reproduced" and emitted replay evidence
    And the corpus replay entry 1 targeted the minimized counterexample
    When the corpus is replayed against the safe message broker
    Then the corpus replay entry 1 has outcome "resolved" and emitted replay evidence
    And the registered original counterexample is unchanged

  @one
  Scenario: registering distinct originals succeeds while duplicate or malformed originals are rejected
    Given a compiled snapshot from "sdv-message.md"
    And an empty counterexample registry
    When two distinct original counterexamples are registered for "PROP-MSG-001"
    Then the registry holds 2 original and 0 minimized counterexample
    When a duplicate original counterexample is registered for "PROP-MSG-001"
    Then corpus registration fails with a duplicate id error
    When an original counterexample with an empty id is registered
    Then corpus registration fails with an empty id error
    When an original counterexample carrying an original_id is registered
    Then minimized registration fails with an invalid minimized error
    When an original counterexample carrying minimization metadata is registered
    Then minimized registration fails with an invalid minimized error

  @near-miss
  Scenario: invalid minimized counterexamples and chained minimizations are rejected
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And an empty counterexample registry
    And the proptest adapter verifies "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 against the buggy redelivering broker
    When a forged minimized counterexample skipping a transition is registered
    Then minimized registration fails with an invalid minimized error
    When a forged minimized counterexample with a wrong destination state is registered
    Then minimized registration fails with an invalid minimized error
    When a forged minimized counterexample that no longer violates is registered
    Then minimized registration fails with an invalid minimized error
    When a forged minimized counterexample with mismatched replay events is registered
    Then minimized registration fails with an invalid minimized error
    When a minimized counterexample chaining onto another minimized counterexample is registered
    Then minimized registration fails with an invalid minimized error
    When the snapshot changes to "sdv-message-property-edit.md"
    And a minimized counterexample matching the updated property digest is registered against the original
    Then corpus registration fails with a model mismatch error

  @near-miss
  Scenario: minimization metadata validation accepts equal-length traces and rejects longer or malformed metadata
    Given a compiled snapshot from "sdv-predicates.md"
    And an empty counterexample registry
    When a 3-event original counterexample for "PROP-PRED-001" is registered
    And an equal-length 3-event minimized counterexample for "PROP-PRED-001" is registered
    Then minimized registration succeeds
    When a 4-event minimized counterexample longer than the original for "PROP-PRED-001" is registered
    Then minimized registration fails with an invalid minimized error
    When a minimized counterexample missing minimization metadata for "PROP-PRED-001" is registered
    Then minimized registration fails with an invalid minimized error
    When a minimized counterexample with an empty minimization method for "PROP-PRED-001" is registered
    Then minimized registration fails with an invalid minimized error
    When a minimized counterexample with wrong original length metadata for "PROP-PRED-001" is registered
    Then minimized registration fails with an invalid minimized error
    When a minimized counterexample with wrong minimized length metadata for "PROP-PRED-001" is registered
    Then minimized registration fails with an invalid minimized error

  @near-miss
  Scenario: minimized replay validation enforces step-zero replay sources and non-replayable event alignment
    Given a compiled snapshot from "sdv-predicates.md"
    And an empty counterexample registry
    When a step-zero original counterexample for "PROP-PRED-004" is registered
    And a replayable step-zero minimized counterexample with seed and no token is registered
    Then minimized registration succeeds
    When a replayable step-zero minimized counterexample with neither seed nor token is registered
    Then minimized registration fails with an invalid minimized error
    Given a compiled snapshot from "sdv-message.md"
    And an empty counterexample registry
    When a 3-event original counterexample for "PROP-MSG-001" is registered
    And a non-replayable minimized counterexample with empty replay events is registered
    Then minimized registration succeeds
    When a non-replayable minimized counterexample with matching replay events is registered
    Then minimized registration succeeds
    When a non-replayable minimized counterexample with mismatched replay events is registered
    Then minimized registration fails with an invalid minimized error

  @near-miss
  Scenario: a non-replayable corpus entry is skipped and replay failures leave the ledger unchanged
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And an empty counterexample registry
    And the proptest adapter verifies "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 against the buggy redelivering broker
    When a non-replayable counterexample is registered and the corpus is replayed
    Then the corpus replay entry 1 has outcome "skipped_not_replayable" and no replay evidence
    When a replayable counterexample is replayed through a mismatched-envelope adapter
    Then corpus replay fails and leaves the ledger unchanged
    When two replayable originals are replayed with a second-entry evidence id collision
    Then corpus replay fails and leaves the ledger unchanged
```


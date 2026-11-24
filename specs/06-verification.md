# Verification: state-machine scenarios, property evaluation, counterexamples and adapters

SpecForge generates traceable scenarios from behavioral models, evaluates trace-checkable safety properties, preserves original and minimized counterexamples with replay metadata, records separate engineering decisions, and integrates external engines (`proptest`, `cucumber`) behind a common adapter contract.

### SF-010 @ v1 — a state machine generates traceable scenarios and evaluates safety properties

> WHEN scenarios are generated from a state machine in a graph snapshot, SpecForge SHALL record the traversal strategy, coverage objective, covered states and transitions, and source model revision for each scenario.
>
> WHEN a safety property is evaluated over an execution trace, SpecForge SHALL return `satisfied` if no state or step violates the predicate, `violated` with the violation step, an unresolved finding, and a concrete counterexample if a violation occurs, or `invalid_evaluation` if the predicate cannot be parsed.
>
> WHEN a verification decision disposes a finding, SpecForge SHALL record the diagnosis, rationale, disposition and approver separately from the counterexample and evidence without mutating any specification artifact.

```gherkin
@SF-010 @v1
Feature: SF-010 — a state machine generates traceable scenarios and evaluates safety properties

  @one @witness
  Scenario: a state machine generates scenarios covering its transitions with provenance
    Given a compiled snapshot from "sdv-message.md"
    When scenarios are generated from state machine "SM-MSG-001"
    Then at least 2 scenarios are generated from "SM-MSG-001"
    And every generated scenario records its strategy and source model digest
    And the generated scenarios cover transitions "deliver, acknowledge"

  @one @witness
  Scenario: evaluating a safety property over a safe trace yields satisfied
    Given a compiled snapshot from "sdv-message.md"
    When property "PROP-MSG-001" is evaluated on the safe message trace
    Then the evaluation result is "satisfied"
    And no finding is created

  @near-miss
  Scenario: evaluating a safety property over a violating trace captures a finding and counterexample
    Given a compiled snapshot from "sdv-message.md"
    When property "PROP-MSG-001" is evaluated on the redelivery violation trace
    Then the evaluation result is "violated"
    And a finding is created with root cause "unresolved"
    And a counterexample is captured at step 3 referencing "PROP-MSG-001"
    When a verification decision disposes that finding as "implementation" by "alice"
    Then the decision records diagnosis "implementation" and approver "alice"
    And the snapshot digest is unchanged

  @near-miss
  Scenario: evaluating a compound or unparseable predicate yields invalid_evaluation
    Given a compiled snapshot from "sdv-compound-predicate.md"
    When property "PROP-CMP-001" is evaluated on the safe message trace
    Then the evaluation result is "invalid_evaluation"
    And no finding is created
```

### SF-011 @ v1 — engine adapters run verification, shrink and replay counterexamples, and emit evidence

> WHEN the `proptest` PBT adapter executes a verification activity against a graph snapshot with a deterministic seed, SpecForge SHALL record engine and adapter versions, seed/replay metadata, and an immutable evidence record in the ledger.
>
> IF a `proptest` run finds a property violation, THEN SpecForge SHALL preserve the original failing counterexample and the shrunk minimized counterexample linked to the original, record a valid replay descriptor, and support deterministic replay that emits a new evidence record without mutating the original counterexample.
>
> IF a run exhausts its budget without establishing satisfaction or violation, THEN SpecForge SHALL record an `inconclusive` result and SHALL NOT report it as `satisfied`.
>
> WHEN the Cucumber adapter records a project story run, SpecForge SHALL append an immutable evidence record tied to the compiled graph snapshot and source revisions.

```gherkin
@SF-011 @v1
Feature: SF-011 — engine adapters run verification, shrink and replay counterexamples, and emit evidence

  @one @witness
  Scenario: proptest vertical slice runs a safe model, records evidence, and detects staleness on change
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    When the proptest adapter verifies "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 against the safe message broker
    Then the adapter run result is "satisfied"
    And the ledger holds 1 evidence record
    And assessing claim "CLM-MSG-001" against the current snapshot yields "supported"
    When the snapshot changes to "sdv-message-property-edit.md"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "stale"

  @near-miss @witness
  Scenario: proptest captures original and shrunk counterexamples and replays deterministically
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    When the proptest adapter verifies "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 against the buggy redelivering broker
    Then the adapter run result is "violated"
    And a minimized counterexample is linked to its original counterexample
    And the minimized counterexample is no longer than the original
    And the counterexample replay descriptor is replayable with seed 42
    When the minimized counterexample is replayed against the buggy redelivering broker
    Then the replay reproduces "violated"
    And the ledger holds 2 evidence records
    And the original counterexample is unchanged

  @near-miss
  Scenario: an exhausted budget records an inconclusive run and never passes the claim
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    When the proptest adapter runs "PROP-MSG-001" for claim "CLM-MSG-001" with zero case budget
    Then the adapter run result is "inconclusive"
    And assessing claim "CLM-MSG-001" against the current snapshot yields "inconclusive"

  @one
  Scenario: the cucumber adapter records a project story run as immutable evidence
    Given the project "ledger"
    And an empty evidence ledger
    When the cucumber adapter verifies the project and records evidence
    Then the cucumber evidence record has evaluation result "satisfied"
    And the ledger holds 1 evidence record
    And ledger integrity verification passes

  @near-miss
  Scenario: replaying a non-replayable counterexample is rejected
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And the proptest adapter verifies "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 against the buggy redelivering broker
    When the counterexample is stripped of replayability and replayed
    Then replay is rejected as not replayable
```

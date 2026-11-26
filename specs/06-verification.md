# Verification: state-machine scenarios, property evaluation, counterexamples and adapters

SpecForge generates traceable scenarios from behavioral models, evaluates trace-checkable safety properties, preserves original and minimized counterexamples with replay metadata, records separate engineering decisions, and integrates external engines (`proptest`, `cucumber`) behind a common adapter contract.

### SF-010 @ v1 — a state machine generates traceable scenarios and evaluates safety properties

> WHEN scenarios are generated from a state machine in a graph snapshot, SpecForge SHALL record the traversal strategy, coverage objective, covered states and transitions, and source model revision for each scenario.
>
> WHEN a safety or state-invariant property is evaluated over an execution trace, SpecForge SHALL return `satisfied` if no state or step violates the predicate, `violated` with the violation step, an unresolved finding, and a concrete counterexample if a violation occurs, `inconclusive` if an input predicate observes no matching inputs, or `invalid_evaluation` if the predicate language, property kind, predicate syntax, or trace continuity is invalid.
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
    And the generated scenarios are numbered "SCN-SM-MSG-001-001, SCN-SM-MSG-001-002, SCN-SM-MSG-001-003"
    And every generated scenario follows valid transitions of "SM-MSG-001"
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
    And the counterexample trace records 1-based steps and virtual times
    When a verification decision disposes that finding as "implementation" by "alice"
    Then the decision records diagnosis "implementation" and approver "alice"
    And the snapshot digest is unchanged

  @one
  Scenario: quoted safety predicates and negated always-state predicates evaluate accurately
    Given a compiled snapshot from "sdv-predicates.md"
    When property "PROP-PRED-001" is evaluated on events "deliver, ack"
    Then the evaluation result is "satisfied"
    And no finding is created
    When property "PROP-PRED-001" is evaluated on events "deliver, ack, redeliver"
    Then the evaluation result is "violated"
    And a counterexample is captured at step 3 referencing "PROP-PRED-001"
    When property "PROP-PRED-002" is evaluated on events "deliver, ack"
    Then the evaluation result is "satisfied"
    When property "PROP-PRED-002" is evaluated on events "deliver, ack, redeliver"
    Then the evaluation result is "violated"
    And a counterexample is captured at step 3 referencing "PROP-PRED-002"

  @one
  Scenario: state-invariant always-state predicates check initial and stepped states
    Given a compiled snapshot from "sdv-predicates.md"
    When property "PROP-PRED-003" is evaluated on events "tick"
    Then the evaluation result is "satisfied"
    And no finding is created
    When property "PROP-PRED-003" is evaluated on events "deliver"
    Then the evaluation result is "violated"
    And a counterexample is captured at step 1 referencing "PROP-PRED-003"
    When property "PROP-PRED-004" is evaluated on events "tick"
    Then the evaluation result is "violated"
    And a counterexample is captured at step 0 referencing "PROP-PRED-004"

  @one
  Scenario: input-constrained safety predicates distinguish satisfied, violated, and inconclusive traces
    Given a compiled snapshot from "sdv-predicates.md"
    When property "PROP-PRED-005" is evaluated on a trace with input "mode" set to "strict"
    Then the evaluation result is "satisfied"
    And no finding is created
    When property "PROP-PRED-005" is evaluated on a trace with input "mode" set to "relaxed"
    Then the evaluation result is "violated"
    And a counterexample is captured at step 1 referencing "PROP-PRED-005"
    When property "PROP-PRED-005" is evaluated on events "deliver"
    Then the evaluation result is "inconclusive"
    And no finding is created

  @near-miss
  Scenario: evaluating a compound or unparseable predicate yields invalid_evaluation
    Given a compiled snapshot from "sdv-compound-predicate.md"
    When property "PROP-CMP-001" is evaluated on the safe message trace
    Then the evaluation result is "invalid_evaluation"
    And no finding is created

  @near-miss
  Scenario: malformed quotes, unsupported languages or kinds, and discontinuous traces yield invalid_evaluation
    Given a compiled snapshot from "sdv-predicates.md"
    When property "PROP-PRED-006" is evaluated on events "deliver"
    Then the evaluation result is "invalid_evaluation"
    And no finding is created
    When property "PROP-PRED-007" is evaluated on events "deliver"
    Then the evaluation result is "invalid_evaluation"
    When property "PROP-PRED-008" is evaluated on events "deliver"
    Then the evaluation result is "invalid_evaluation"
    When property "PROP-PRED-009" is evaluated on events "deliver"
    Then the evaluation result is "invalid_evaluation"
    When property "PROP-PRED-010" is evaluated on events "deliver"
    Then the evaluation result is "invalid_evaluation"
    When property "PROP-PRED-001" is evaluated on a discontinuous trace
    Then the evaluation result is "invalid_evaluation"
    And no finding is created
```

### SF-011 @ v1 — engine adapters run verification, shrink and replay counterexamples, and emit evidence

> WHEN the `proptest` PBT adapter validates or executes a verification activity against a graph snapshot with a deterministic seed, SpecForge SHALL reject requests with empty identifiers, unknown claims or properties, or transitionless machines, drive trace generation from the seed, and record engine and adapter versions, seed/replay metadata, and an immutable evidence record in the ledger.
>
> IF a `proptest` run finds a property violation, THEN SpecForge SHALL preserve the original failing counterexample and the shrunk minimized counterexample linked to the original, reject causally inconsistent or non-reproducible candidate traces during shrinking, record a valid replay descriptor, and support deterministic replay (including step-zero violations with empty events while rejecting non-replayable or positive-step eventless counterexamples) that emits a new evidence record without mutating the original counterexample.
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
    And shrinking completes within the attempt budget
    And the counterexample replay descriptor is replayable with seed 42
    When the minimized counterexample is replayed against the buggy redelivering broker
    Then the replay reproduces "violated"
    And the ledger holds 2 evidence records
    And the original counterexample is unchanged

  @one
  Scenario: proptest seeds deterministically control generated traces and reject invalid requests
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    When the proptest adapter verifies "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 against the buggy redelivering broker
    And the proptest adapter re-runs "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 against the buggy redelivering broker
    Then the two proptest runs produced identical original counterexamples
    When the proptest adapter re-runs "PROP-MSG-001" for claim "CLM-MSG-001" with seed 99 against the buggy redelivering broker
    Then the two proptest runs produced distinct original counterexamples
    When the proptest adapter validates a request with empty run id
    Then the proptest request is rejected as invalid
    When the proptest adapter validates a request with empty activity id
    Then the proptest request is rejected as invalid
    When the proptest adapter validates a request referencing unknown claim "CLM-UNKNOWN-999"
    Then the proptest request is rejected as invalid
    When the proptest adapter validates a request referencing unknown property "PROP-UNKNOWN-999"
    Then the proptest request is rejected as invalid

  @near-miss
  Scenario: proptest shrinking rejects causally inconsistent or flaky candidate traces
    Given a compiled snapshot from "sdv-message.md"
    When the proptest adapter shrinks "PROP-MSG-001" against a runner forging a wrong initial state on shorter traces
    Then shrinking preserves a causally consistent minimized trace of length 3
    When the proptest adapter shrinks "PROP-MSG-001" against a runner forging a wrong trace length on shorter traces
    Then shrinking preserves a causally consistent minimized trace of length 3
    When the proptest adapter shrinks "PROP-MSG-001" against a runner forging a wrong event name on shorter traces
    Then shrinking preserves a causally consistent minimized trace of length 3
    When the proptest adapter shrinks "PROP-MSG-001" against a flaky runner whose shrunk violation stops reproducing
    Then shrinking falls back to the original counterexample

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
  Scenario: replaying non-replayable or eventless positive-step counterexamples is rejected while step-zero replays succeed
    Given a compiled snapshot from "sdv-predicates.md"
    When a step-zero counterexample for "PROP-PRED-004" with empty replay events is replayed
    Then the replay reproduces "violated"
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And the proptest adapter verifies "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 against the buggy redelivering broker
    When the counterexample is stripped of replayability and replayed
    Then replay is rejected as not replayable
    When the counterexample replay events are cleared while keeping replayable true and replayed
    Then replay is rejected as not replayable
```

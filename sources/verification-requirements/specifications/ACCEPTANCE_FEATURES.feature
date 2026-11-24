@specforge @specification_driven_verification
Feature: Specification-driven verification

  Background:
    Given a SpecForge project with a version-controlled Specification Graph
    And the project has a revision-bound graph snapshot

  @P0 @properties
  Scenario: Define a property linked to a requirement and behavioral model
    Given a requirement "REQ-MSG-001"
    And a message lifecycle state machine "SM-MSG-001"
    When I define a safety property "PROP-MSG-001"
    And I link the property to the requirement and state machine
    Then the property is stored as a typed graph artifact
    And its stable identifier and source revision are preserved
    And its assumptions and verification scope are explicit

  @P0 @verification-plan
  Scenario: Create a verification plan
    Given a property "PROP-MSG-001"
    When I create a verification plan targeting that property
    Then the plan identifies its methods, scope, assumptions, budgets, and evidence requirements
    And the plan is bound to an immutable graph snapshot

  @P0 @execution @evidence
  Scenario: Execute a property check and record immutable evidence
    Given a verification plan with one executable scenario
    When the verification activity completes successfully
    Then the property evaluation result is recorded
    And an evidence record is appended to the Evidence Ledger
    And the evidence records the graph snapshot, source revisions, tool identity, and configuration
    And the evidence is linked to the verification claim

  @P0 @negative-evidence
  Scenario: Preserve a property violation as a counterexample
    Given a verification plan targeting a safety property
    When an execution violates the property
    Then the result is "violated"
    And a finding is created
    And a counterexample is captured with the available execution context
    And the counterexample is linked to the property, run, and evidence record
    And the system does not assume the implementation is the root cause

  @P0 @inconclusive
  Scenario: Do not treat incomplete exploration as a pass
    Given a verification run with a finite execution budget
    When the budget expires before the property is established or refuted
    Then the result is "inconclusive"
    And the claim is not marked as verified solely from this run

  @P0 @append-only
  Scenario: Preserve evidence history
    Given an existing evidence record
    When a later run produces a different result
    Then a new evidence record is appended
    And the original evidence record remains unchanged
    And the relationship between the records is recorded where relevant

  @P0 @freshness
  Scenario: Identify stale evidence after a source change
    Given evidence linked to property "PROP-MSG-001"
    When the property or one of its declared dependencies changes
    Then the affected evidence is marked as stale or requiring reassessment
    And the historical evidence remains available
    And affected verification claims are identified

  @P1 @mbt
  Scenario: Generate a scenario from a behavioral model
    Given a state machine with defined states and transitions
    When an MBT adapter generates a scenario
    Then the scenario records its generation strategy and coverage objective
    And the scenario links to the transitions it exercises

  @P1 @pbt
  Scenario: Preserve generated input and shrink metadata
    Given a PBT adapter generates a failing input
    When the property fails
    Then the generated input, generator version, and seed or replay token are recorded
    And shrink metadata is preserved when available
    And the minimized case links to the original failing case

  @P1 @counterexample-replay
  Scenario: Replay a captured counterexample
    Given a counterexample with a valid replay descriptor
    When I request replay against a compatible implementation revision
    Then the system executes the recorded scenario and replay controls
    And the replay result is recorded as new evidence
    And the original counterexample remains unchanged

  @P1 @diagnosis
  Scenario: Record a diagnosis without silently changing specifications
    Given a counterexample with unresolved root cause
    When an agent proposes a specification or model change
    Then the proposal is recorded as a recommendation
    And no authoritative artifact changes without the project's approval workflow

  @P1 @coverage-feedback
  Scenario: Recommend exploration for a coverage gap
    Given a verification assessment with uncovered state transitions
    When the exploration planner evaluates the assessment
    Then it recommends one or more targeted MBT activities
    And each recommendation explains the coverage gap it addresses

  @P2 @formal
  Scenario: Preserve bounded formal verification results
    Given a formal verification adapter configured with finite bounds
    When the model checker reports that a property holds within those bounds
    Then the result is recorded as "bounded_success"
    And the bounds and assumptions are retained
    And the result is not represented as an unbounded proof

  @P2 @contradiction
  Scenario: Surface contradictory evidence
    Given a claim with evidence supporting it
    When a new valid counterexample contradicts that evidence
    Then the assessment reports the contradiction
    And the claim requires review or is refuted according to the configured policy
    And no previous evidence is deleted or overwritten

  @P3 @probabilistic
  Scenario: Compute a qualified statistical estimate
    Given a claim with a declared statistical model and sampling assumptions
    When the evidence evaluator computes an estimate
    Then the estimate records the method, sample scope, sample size, uncertainty, and limitations
    And the estimate is kept separate from formal proof status

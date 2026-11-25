# Standing: which scenarios can run, and what is wrong

An assessment joins a trace with three things the consumer supplies: whether each step is bound, the pending table, and the ids its invariant checker monitors. The fixture `standing.md` declares SF-911 with the scenarios "a transfer moves credits" (Given an account with 100 credits; When 60 credits are transferred; Then the total is 100) and "a transfer during a partition is retried" (Given an account with 100 credits; When the network is partitioned; Then the total is 100), SF-912 with the scenario "a node restarts" (Given an account with 100 credits; When the node restarts; Then the total is 100), and SF-913 with no story.

### SF-003 @ v1 — a scenario stands as pending, executable or unbound

> WHEN a trace is assessed, each scenario SHALL stand as pending if the pending table names it outside CommonMark fenced code blocks (at most three leading spaces, at least three fence characters, no backtick in a backtick fence's info string, closed only by a matching fence of at least the opening length with no trailing text), otherwise as executable if every one of its steps (including folded `Background` steps) is bound exactly once, otherwise as unbound.
>
> A requirement SHALL be `not started` while none of its scenarios is executable, `bound` when at least one is executable and every other is executable or pending, and `partial` otherwise. A requirement that is not started SHALL raise no finding for its unbound steps.

```gherkin
@SF-003 @v1
Feature: SF-003 — a scenario stands as pending, executable or unbound

  @zero
  Scenario: with nothing bound, nothing has started and nothing is wrong
    Given the document "standing.md"
    And no step is bound
    When the trace is assessed
    Then requirement "SF-911" is "not started"
    And requirement "SF-912" is "not started"
    And requirement "SF-913" is "not started"
    And there are no findings

  @one @witness
  Scenario: a requirement whose every step is bound is bound
    Given the document "standing.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the network is partitioned; the total is 100" are bound
    When the trace is assessed
    Then requirement "SF-911" is "bound" with 2 of 2 scenarios executable and 0 pending
    And the scenario "a transfer moves credits" of "SF-911" stands "executable"
    And there are no findings

  @near-miss
  Scenario: steps shared with a bound requirement do not start another
    Given the document "standing.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the network is partitioned; the total is 100" are bound
    When the trace is assessed
    Then requirement "SF-912" is "not started"
    And the scenario "a node restarts" of "SF-912" stands "unbound"
    And there are no findings

  @many
  Scenario: a pending scenario does not keep its requirement from being bound
    Given the document "standing.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the total is 100" are bound
    And the pending table names "SF-911" "a transfer during a partition is retried"
    When the trace is assessed
    Then requirement "SF-911" is "bound" with 1 of 2 scenarios executable and 1 pending
    And the scenario "a transfer during a partition is retried" of "SF-911" stands "pending"
    And there are no findings

  @near-miss
  Scenario: table rows inside fenced code blocks in the pending table are ignored
    Given the document "standing.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the total is 100" are bound
    And the pending table is the file "pending-fenced.md"
    When the trace is assessed
    Then requirement "SF-911" is "bound" with 1 of 2 scenarios executable and 1 pending
    And the scenario "a transfer during a partition is retried" of "SF-911" stands "pending"
    And there are no findings

  @near-miss
  Scenario: CommonMark fence rules govern which pending table rows are ignored
    Given the document "standing.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the total is 100" are bound
    And the pending table is the file "pending-fence-edges.md"
    When the trace is assessed
    Then requirement "SF-911" is "bound" with 1 of 2 scenarios executable and 1 pending
    And the scenario "a transfer during a partition is retried" of "SF-911" stands "pending"
    And the scenario "a node restarts" of "SF-912" stands "pending"
    And there are no findings

  @near-miss
  Scenario: an unbound feature background step leaves every inheriting scenario unbound
    Given the document "background-and-rules.md"
    And the steps "60 credits are transferred; the total is 100; transfers carry no fee; an audit log is open" are bound
    When the trace is assessed
    Then requirement "SF-934" is "not started"
    And the scenario "a top-level transfer" of "SF-934" stands "unbound"
    And the scenario "a fee-free transfer" of "SF-934" stands "unbound"
    And the scenario "an audited transfer" of "SF-934" stands "unbound"
    And there are no findings
```

### SF-004 @ v1 — what does not add up is a finding

> IF a pending entry names no scenario of the trace, or names a scenario whose every step is bound; IF a requirement is partial; IF a step of any scenario is matched by more than one binding; or IF a monitored id names no requirement of the trace, THEN the assessment SHALL carry a finding for each, and an assessment with any finding SHALL be unfit. A partial requirement SHALL raise one finding for each unbound step (including any folded `Background` step) of each of its unbound scenarios.

```gherkin
@SF-004 @v1
Feature: SF-004 — what does not add up is a finding

  @zero @witness
  Scenario: a monitored requirement that exists raises nothing
    Given the document "standing.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the network is partitioned; the total is 100" are bound
    And the requirements "SF-911" are monitored
    When the trace is assessed
    Then there are no findings
    And the assessment is fit
    And requirement "SF-911" is monitored
    And requirement "SF-912" is not monitored

  @near-miss
  Scenario: a pending entry that names no scenario is stale
    Given the document "standing.md"
    And no step is bound
    And the pending table names "SF-911" "a transfer that never was"
    When the trace is assessed
    Then the findings are "pending entry SF-911 \"a transfer that never was\" names no scenario"
    And the assessment is unfit

  @near-miss
  Scenario: a pending entry for a scenario that could run is a finding
    Given the document "standing.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the network is partitioned; the total is 100" are bound
    And the pending table names "SF-911" "a transfer during a partition is retried"
    When the trace is assessed
    Then the findings are "pending entry SF-911 \"a transfer during a partition is retried\" names a scenario whose every step is bound"

  @one @near-miss
  Scenario: a started requirement with an unbound scenario is partial
    Given the document "standing.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the total is 100" are bound
    When the trace is assessed
    Then requirement "SF-911" is "partial"
    And the findings are "SF-911 \"a transfer during a partition is retried\" has the unbound step \"When the network is partitioned\""

  @many @near-miss
  Scenario: an ambiguous step is a finding wherever it occurs
    Given the document "standing.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the network is partitioned" are bound
    And the step "the total is 100" is bound twice
    When the trace is assessed
    Then the findings are "SF-911 \"a transfer moves credits\" has the ambiguous step \"Then the total is 100\"; SF-911 \"a transfer during a partition is retried\" has the ambiguous step \"Then the total is 100\"; SF-912 \"a node restarts\" has the ambiguous step \"Then the total is 100\""
    And requirement "SF-911" is "not started"

  @near-miss
  Scenario: a monitored id the trace does not hold is a finding
    Given the document "standing.md"
    And no step is bound
    And the requirements "SF-911, SF-999" are monitored
    When the trace is assessed
    Then the findings are "monitored requirement SF-999 is not in the trace"

  @near-miss
  Scenario: an unbound rule background step makes a started requirement partial
    Given the document "background-and-rules.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the total is 100; transfers carry no fee" are bound
    When the trace is assessed
    Then requirement "SF-934" is "partial" with 2 of 3 scenarios executable and 0 pending
    And the scenario "an audited transfer" of "SF-934" stands "unbound"
    And the findings are "SF-934 \"an audited transfer\" has the unbound step \"Given an audit log is open\""
    And the assessment is unfit
```

### SF-005 @ v1 — the assessment draws as one table

> WHEN an assessment is drawn, SpecForge SHALL render a Markdown document holding one table row per requirement in trace order (requirement, title, progress, scenarios, executable, pending, monitored) and one totals line, and the same assessment SHALL always draw the same bytes. The golden files in `tests/fixtures/` are the form.

```gherkin
@SF-005 @v1
Feature: SF-005 — the assessment draws as one table

  @zero
  Scenario: an empty trace draws a table with no rows
    Given the document "prose-only.md"
    And no step is bound
    When the trace is assessed
    Then the view is the file "empty.view.md"

  @many @witness
  Scenario: every requirement draws one row
    Given the document "standing.md"
    And the steps "an account with 100 credits; 60 credits are transferred; the total is 100" are bound
    And the pending table names "SF-911" "a transfer during a partition is retried"
    And the requirements "SF-911" are monitored
    When the trace is assessed
    Then the view is the file "standing.view.md"
```

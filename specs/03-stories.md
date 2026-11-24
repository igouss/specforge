# Stories: a project's specifications, run

One call runs a project's stories: the consumer names its specification directory, its pending table, its committed view and a scratch directory, and supplies a cucumber world. Sample projects live in `tests/projects/`; the sample world binds the ledger steps "an account with 100 credits", "60 credits are transferred" and "the total is {int}", and nothing else. The project `ledger` holds the specification `ledger.md` (SF-921 with the scenarios "a transfer moves credits" and "a transfer during a partition is retried", the second pending), its pending table and its committed view.

### SF-006 @ v1 — a project's stories run from its specifications

> WHEN a project's stories are verified, SpecForge SHALL compile every `*.md` file directly in the specification directory, in file-name order, assess the trace against the world's step definitions, the pending table and the monitored ids, and run exactly the executable scenarios, from features written to the scratch directory and never beside the specifications.
>
> IF a file cannot be read, the documents are defective, the assessment is unfit, or the committed view differs from the drawn one, THEN SpecForge SHALL run no scenario and SHALL report which, naming every defect or finding.
>
> IF no scenario ran, a scenario that ran did not pass, or the number of scenarios that ran differs from the number of executable scenarios, THEN the verification SHALL fail.
>
> IF the scratch directory holds anything but feature files, THEN SpecForge SHALL run no scenario and SHALL report the entry, so that nothing but the features it writes can run.
>
> WHEN a project's view is drawn, SpecForge SHALL write the view of a fit assessment to the view path and SHALL run no scenario.
>
> A run SHALL report `stories: <ran> of <scenarios> ran, <passed> passed, <pending> pending`, where scenarios counts those of requirements that have started.

```gherkin
@SF-006 @v1
Feature: SF-006 — a project's stories run from its specifications

  @one @witness
  Scenario: the executable scenarios run and the pending one is counted
    Given the project "ledger"
    When its stories are verified
    Then the outcome reads "stories: 1 of 2 ran, 1 passed, 1 pending"
    And the verification passes
    And no feature file lies in its specification directory

  @zero @near-miss
  Scenario: a run of zero scenarios never passes
    Given the project "ledger"
    And its specification "ledger.md" is replaced by the document "one-requirement.md"
    And its pending table is emptied
    And its view is drawn
    When its stories are verified
    Then the outcome reads "stories: 0 of 0 ran, 0 passed, 0 pending"
    And the verification fails

  @near-miss
  Scenario: a failing scenario fails the verification
    Given the project "ledger"
    And its specification "ledger.md" is replaced by the document "ledger-broken.md"
    And its view is drawn
    When its stories are verified
    Then the outcome reads "stories: 1 of 2 ran, 0 passed, 1 pending"
    And the verification fails

  @near-miss
  Scenario: a stale view runs nothing
    Given the project "ledger"
    And its committed view is replaced by the file "empty.view.md"
    When its stories are verified
    Then the outcome reads "the committed view is stale: draw it again"
    And the verification fails
    And no scenario ran

  @near-miss
  Scenario: a missing view is stale
    Given the project "ledger"
    And its committed view is deleted
    When its stories are verified
    Then the outcome reads "the committed view is stale: draw it again"
    And the verification fails

  @near-miss
  Scenario: an unfit assessment runs nothing and names its findings
    Given the project "ledger"
    And its pending table is emptied
    When its stories are verified
    Then the outcome reads "SF-921 \"a transfer during a partition is retried\" has the unbound step \"When the network is partitioned\""
    And the verification fails
    And no scenario ran

  @near-miss
  Scenario: defective documents run nothing and name their defects
    Given the project "ledger"
    And its specification "ledger.md" is replaced by the document "untagged.md"
    When its stories are verified
    Then the outcome reads "untagged.md:7: the story carries no requirement tag"
    And the verification fails
    And no scenario ran

  @many
  Scenario: a monitored id the specifications do not hold runs nothing
    Given the project "ledger"
    And it monitors the requirements "SF-921, SF-999"
    When its stories are verified
    Then the outcome reads "monitored requirement SF-999 is not in the trace"
    And the verification fails
    And no scenario ran

  @near-miss
  Scenario: a specification that cannot be read runs nothing
    Given the project "ledger"
    And its specification directory holds a link "missing.md" to a document that does not exist
    When its stories are verified
    Then the outcome names "missing.md"
    And the verification fails
    And no scenario ran

  @near-miss
  Scenario: a directory named like a specification is ignored
    Given the project "ledger"
    And its specification directory holds a directory "notes.md"
    When its stories are verified
    Then the outcome reads "stories: 1 of 2 ran, 1 passed, 1 pending"
    And the verification passes

  @near-miss
  Scenario: a scratch directory that holds anything else runs nothing
    Given the project "ledger"
    And its scratch directory holds the file "notes.txt"
    When its stories are verified
    Then the outcome names "notes.txt"
    And the verification fails
    And no scenario ran

  @one
  Scenario: drawing writes the view and runs nothing
    Given the project "ledger"
    And its committed view is deleted
    When its view is drawn
    Then its committed view is the file "ledger.view.md"
    And no scenario ran
    And the verification passes

  @near-miss
  Scenario: an unfit assessment draws nothing
    Given the project "ledger"
    And its pending table is emptied
    And its committed view is deleted
    When its view is drawn
    Then its committed view does not exist
    And the verification fails
```

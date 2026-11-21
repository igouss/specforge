# Trace: from documents to requirements and their stories

A specification document is Markdown. A requirement is a level-three heading with an id and a version; the blockquote below it is its statement; a fenced `gherkin` block below it is its story. Fixture documents named in the stories live in `tests/fixtures/`.

### SF-001 @ v1 — a requirement heading becomes a requirement

> WHEN documents are compiled, SpecForge SHALL yield one requirement for each heading of the form `### <ID> @ v<N> — <title>`, in document order and then heading order, carrying its id, its version, its title, the first blockquote between the heading and the next heading as its statement on one line, and the path and line of the heading.
>
> A level-three heading of any other form SHALL be read as prose.

```gherkin
@SF-001 @v1
Feature: SF-001 — a requirement heading becomes a requirement

  @zero
  Scenario: a document that declares nothing yields an empty trace
    Given the document "prose-only.md"
    When the documents are compiled
    Then the trace holds no requirements
    And there are no defects

  @one @witness
  Scenario: one heading yields one requirement
    Given the document "one-requirement.md"
    When the documents are compiled
    Then the trace holds the requirements "SF-901"
    And requirement "SF-901" has the version "v1"
    And requirement "SF-901" has the title "an admission mints an identity"
    And requirement "SF-901" has the statement "WHEN an item is admitted, the factory SHALL mint an identity."
    And requirement "SF-901" is declared at "one-requirement.md" line 3

  @many
  Scenario: several documents keep document order, then heading order
    Given the document "two-requirements.md"
    And the document "one-requirement.md"
    When the documents are compiled
    Then the trace holds the requirements "SF-902, SF-903, SF-901"

  @near-miss
  Scenario: a heading with no version is prose
    Given the document "heading-without-version.md"
    When the documents are compiled
    Then the trace holds no requirements
    And there are no defects
```

### SF-002 @ v1 — a story attaches to the requirement its tag names

> WHEN a document holds a fenced `gherkin` block, SpecForge SHALL attach the block's feature, as a story, to the requirement named by the feature's `@<ID>` tag, with the name, tags and steps of every scenario, the block's text and the line of its opening fence.
>
> IF a block cannot be parsed, carries no requirement tag or more than one, holds a scenario with no steps or a scenario with an Examples table (a Scenario Outline), names a requirement that no heading declares, or does not sit between the heading of the requirement it names and the next requirement heading; or IF an id is declared a second time, a scenario name repeats within one requirement, or a requirement has no statement, THEN SpecForge SHALL yield no trace and SHALL report every such defect as `<path>:<line>: <sentence>`. The line is the opening fence for a defect of a block and the heading for a defect of a requirement.

```gherkin
@SF-002 @v1
Feature: SF-002 — a story attaches to the requirement its tag names

  @zero
  Scenario: a requirement with no story has no scenarios
    Given the document "two-requirements.md"
    When the documents are compiled
    Then requirement "SF-903" has no scenarios

  @one @witness
  Scenario: one story brings its scenario and steps
    Given the document "one-requirement.md"
    When the documents are compiled
    Then requirement "SF-901" has the scenarios "one admission mints one identity"
    And the scenario "one admission mints one identity" of "SF-901" has the steps "Given an empty factory; When an item is admitted; Then one identity exists"
    And the scenario "one admission mints one identity" of "SF-901" has the tags "one, witness"
    And the story of "SF-901" opens at line 8

  @many
  Scenario: several blocks under one heading all attach
    Given the document "two-requirements.md"
    When the documents are compiled
    Then requirement "SF-902" has the scenarios "a first transfer; a second transfer; a refused transfer"

  @near-miss
  Scenario: a block that is not Gherkin is a defect
    Given the document "unparseable.md"
    When the documents are compiled
    Then there is no trace
    And the only defect starts with "unparseable.md:7: the story cannot be parsed: "

  @near-miss
  Scenario: a story with no requirement tag is a defect
    Given the document "untagged.md"
    When the documents are compiled
    Then there is no trace
    And the defects are "untagged.md:7: the story carries no requirement tag"

  @near-miss
  Scenario: a story naming an undeclared requirement is a defect
    Given the document "undeclared.md"
    When the documents are compiled
    Then the defects are "undeclared.md:3: the story is tagged SF-999, which no heading declares"

  @near-miss
  Scenario: a story under another requirement's heading is a defect
    Given the document "misplaced.md"
    When the documents are compiled
    Then the defects are "misplaced.md:11: the story is tagged SF-908 but does not sit under its heading"

  @near-miss
  Scenario: an id declared twice is a defect
    Given the document "redeclared.md"
    When the documents are compiled
    Then the defects are "redeclared.md:7: SF-910 is declared a second time"

  @near-miss
  Scenario: a repeated scenario name is a defect
    Given the document "repeated.md"
    When the documents are compiled
    Then the defects are "repeated.md:7: SF-914 repeats the scenario \"a transfer\""

  @near-miss
  Scenario: a requirement with no statement is a defect
    Given the document "unstated.md"
    When the documents are compiled
    Then the defects are "unstated.md:3: SF-915 states nothing"

  @near-miss
  Scenario: a story with two requirement tags is a defect
    Given the document "overtagged.md"
    When the documents are compiled
    Then the defects are "overtagged.md:7: the story carries more than one requirement tag"

  @near-miss
  Scenario: a scenario with no steps is a defect
    Given the document "stepless.md"
    When the documents are compiled
    Then the defects are "stepless.md:7: the scenario \"nothing happens\" has no steps; stepless.md:7: the scenario \"nothing else happens\" has no steps"

  @near-miss
  Scenario: a scenario outline is a defect until it is specified
    Given the document "outline.md"
    When the documents are compiled
    Then the defects are "outline.md:7: the scenario outline \"a transfer in <currency>\" is not supported"

  @near-miss
  Scenario: a blockquote under a later heading is not the statement
    Given the document "statement-elsewhere.md"
    When the documents are compiled
    Then the defects are "statement-elsewhere.md:3: SF-918 states nothing"

  @many @near-miss
  Scenario: every defect is reported, not only the first
    Given the document "untagged.md"
    And the document "unstated.md"
    When the documents are compiled
    Then there is no trace
    And the defects are "untagged.md:7: the story carries no requirement tag; unstated.md:3: SF-915 states nothing"
```

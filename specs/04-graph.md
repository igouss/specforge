# Graph: typed verification artifacts and hashed snapshots

A specification document may also hold fenced `specforge` YAML blocks declaring typed verification artifacts (`assumption`, `state_machine`, `property`, `claim`, `plan`). Ordinary `yaml` fenced blocks remain prose. Compiling documents into a `GraphSnapshot` validates cross-references and computes deterministic SHA-256 content digests per document, per artifact and for the snapshot as a whole.

### SF-007 @ v1 — verification artifacts compile into a hashed graph snapshot

> WHEN documents are compiled into a graph snapshot, SpecForge SHALL parse every fenced `specforge` YAML block into its typed artifact (`assumption`, `state_machine`, `property`, `claim`, or `plan`), ignore ordinary `yaml` blocks, validate all declared identifiers and cross-references, and assign a deterministic SHA-256 content digest to each source document, each artifact and the snapshot.
>
> IF a `specforge` block is malformed, declares a duplicate artifact identifier, references an undeclared requirement, assumption, state machine, property or claim, or defines a state-machine transition naming an undeclared state, THEN SpecForge SHALL yield no snapshot and SHALL report every defect as `<path>:<line>: <sentence>`.

```gherkin
@SF-007 @v1
Feature: SF-007 — verification artifacts compile into a hashed graph snapshot

  @one @witness
  Scenario: a specification with verification artifacts compiles to a hashed snapshot
    Given the graph document "sdv-message.md"
    When the graph snapshot is compiled
    Then the snapshot compiles with no defects
    And the snapshot holds assumption "ASM-MSG-001", state machine "SM-MSG-001", property "PROP-MSG-001", claim "CLM-MSG-001", and plan "PLAN-MSG-001"
    And the snapshot digest is deterministic across repeated compilations

  @zero
  Scenario: an ordinary yaml code block is ignored as prose
    Given the graph document "sdv-prose-yaml.md"
    When the graph snapshot is compiled
    Then the snapshot compiles with no defects
    And the snapshot holds 0 verification artifacts

  @near-miss
  Scenario: changing an unrelated requirement preserves a property's dependency digest
    Given the graph document "sdv-message.md"
    When the graph snapshot is compiled as baseline
    And the graph document "sdv-message-unrelated-edit.md" is compiled as revised
    Then the snapshot digest changes
    And the digest of property "PROP-MSG-001" is unchanged
    And the digest of state machine "SM-MSG-001" is unchanged

  @near-miss
  Scenario: dangling references and unknown states are reported as graph defects
    Given the graph document "sdv-defective.md"
    When the graph snapshot is compiled
    Then there is no snapshot
    And the graph defects include "sdv-defective.md:7: state machine SM-BAD-001 references unknown state \"ghost\""
    And the graph defects include "sdv-defective.md:22: property PROP-BAD-001 references unknown assumption ASM-MISSING-999"
    And the graph defects include "sdv-defective.md:22: property PROP-BAD-001 references unknown requirement REQ-MISSING-999"

  @near-miss
  Scenario: duplicate artifact identifiers are reported as graph defects
    Given the graph document "sdv-duplicate.md"
    When the graph snapshot is compiled
    Then there is no snapshot
    And the graph defects include "sdv-duplicate.md:14: artifact ASM-DUP-001 is declared a second time"

  @near-miss
  Scenario: an unknown field typo in a specforge block is reported as a graph defect
    Given the graph document "sdv-unknown-field.md"
    When the graph snapshot is compiled
    Then there is no snapshot
    And a graph defect mentions "unknown field"
```

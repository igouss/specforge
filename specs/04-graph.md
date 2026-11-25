# Graph: typed verification artifacts and hashed snapshots

A specification document may also hold fenced `specforge` YAML blocks declaring typed verification artifacts (`assumption`, `state_machine`, `property`, `claim`, `plan`). Ordinary `yaml` fenced blocks remain prose. Compiling documents into a `GraphSnapshot` validates cross-references and computes deterministic SHA-256 content digests per document, per artifact and for the snapshot as a whole.

### SF-007 @ v1 — verification artifacts compile into a hashed graph snapshot

> WHEN documents are compiled into a graph snapshot, SpecForge SHALL parse every fenced `specforge` YAML block into its typed artifact (`assumption`, `state_machine`, `property`, `claim`, or `plan`), ignore ordinary `yaml` blocks, validate all declared identifiers and cross-references, and assign a deterministic SHA-256 content digest to each source document, each artifact and the snapshot.
>
> IF a `specforge` block is malformed, declares an invalid or duplicate artifact identifier, references an undeclared requirement, assumption, state machine, property or claim, or defines a state-machine transition naming an undeclared state, THEN SpecForge SHALL yield no snapshot and SHALL report every defect as `<path>:<line>: <sentence>`.

```gherkin
@SF-007 @v1
Feature: SF-007 — verification artifacts compile into a hashed graph snapshot

  @one @witness
  Scenario: a specification with verification artifacts compiles to a hashed snapshot
    Given the graph document "sdv-message.md"
    When the graph snapshot is compiled
    Then the snapshot compiles with no defects
    And the snapshot holds 5 verification artifacts
    And the snapshot holds assumption "ASM-MSG-001", state machine "SM-MSG-001", property "PROP-MSG-001", claim "CLM-MSG-001", and plan "PLAN-MSG-001"
    And property "PROP-MSG-001" lists verification methods "model_based, property_based"
    And the snapshot digest formats as a valid 64-character lowercase SHA-256 hex digest
    And the snapshot digest is deterministic across repeated compilations

  @one
  Scenario: a two-segment artifact identifier compiles cleanly
    Given the graph document "sdv-two-part-id.md"
    When the graph snapshot is compiled
    Then the snapshot compiles with no defects
    And the snapshot holds 1 verification artifacts
    And the snapshot holds assumption "ASM-001"

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
  Scenario: a syntactically valid but undeclared requirement reference is reported as a graph defect
    Given the graph document "sdv-unknown-req.md"
    When the graph snapshot is compiled
    Then there is no snapshot
    And the graph defects include "sdv-unknown-req.md:7: state machine SM-REQ-001 references unknown requirement REQ-999"

  @near-miss
  Scenario: malformed artifact identifiers are reported as graph defects
    Given the graph document "sdv-invalid-ids.md"
    When the graph snapshot is compiled
    Then there is no snapshot
    And the graph defects include "sdv-invalid-ids.md:7: \"123\" is not a valid artifact identifier"
    And the graph defects include "sdv-invalid-ids.md:14: \"ASM-\" is not a valid artifact identifier"
    And the graph defects include "sdv-invalid-ids.md:21: \"ASM-abc\" is not a valid artifact identifier"
    And the graph defects include "sdv-invalid-ids.md:28: \"asm-001\" is not a valid artifact identifier"
    And the graph defects include "sdv-invalid-ids.md:35: \"ASM--001\" is not a valid artifact identifier"

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

  @near-miss
  Scenario: malformed SHA-256 content digests are rejected
    When content digests "deadbeef, AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA, gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg" are validated
    Then none of those content digests are accepted
```

# Evidence and Assurance: append-only ledger, freshness and claim assessment

Evidence records are immutable facts with provenance and self-verifying SHA-256 content digests. Assessments are recomputed from the current graph snapshot and the ledger without mutating history.

### SF-008 @ v1 — the evidence ledger appends immutable provenance records and links

> WHEN an evidence record is appended to the ledger, SpecForge SHALL validate its schema and SHA-256 content digest, refuse duplicate record identifiers or tampered payloads, preserve all earlier records including negative and inconclusive evidence, and support typed append-only links (`supports`, `contradicts`, `corroborates`, `derived_from`, `supersedes`, `invalidates`).
>
> WHEN the ledger is persisted to disk (`ledger-fs`), SpecForge SHALL write records and links to an append-only JSONL log and store raw artifacts as content-addressed blobs verified on reload; IF any stored record or blob fails integrity validation on reload, THEN SpecForge SHALL reject it with an integrity error.

```gherkin
@SF-008 @v1
Feature: SF-008 — the evidence ledger appends immutable provenance records and links

  @one @witness
  Scenario: appending a valid evidence record preserves provenance and verifies integrity
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    When a satisfied evidence record "EV-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    Then the ledger holds 1 evidence record
    And ledger integrity verification passes
    And exporting the ledger as JSON is deterministic

  @near-miss
  Scenario: appending a duplicate evidence id or tampered digest is refused
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a duplicate evidence record "EV-001" is appended
    Then the append is refused as duplicate
    When a tampered evidence record "EV-002" is appended
    Then the append is refused as invalid digest
    And the ledger holds 1 evidence record

  @many
  Scenario: a later run appends a new record without modifying history
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a violated evidence record "EV-002" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a link "contradicts" is appended from "EV-002" to "EV-001"
    Then the ledger holds 2 evidence records
    And record "EV-001" still has evaluation result "satisfied"
    And record "EV-002" has evaluation result "violated"

  @one @witness
  Scenario: the filesystem ledger persists records and content-addressed blobs and detects tampering
    Given a temporary filesystem ledger
    And a compiled snapshot from "sdv-message.md"
    When a satisfied evidence record "EV-FS-001" with blob payload "trace-payload-v1" is persisted
    And the filesystem ledger is reloaded
    Then the reloaded ledger holds 1 evidence record
    And the blob for "EV-FS-001" reads back as "trace-payload-v1"
    When the persisted ledger file is tampered with
    And the filesystem ledger is reloaded
    Then reloading the filesystem ledger fails with an integrity error

  @near-miss
  Scenario: tampering with a stored blob is rejected when the filesystem ledger reloads
    Given a temporary filesystem ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-BLOB-001" with blob payload "original-trace-bytes" is persisted
    When the stored blob for "EV-BLOB-001" is tampered with
    And the filesystem ledger is reloaded
    Then reloading the filesystem ledger fails with a blob integrity error
```

### SF-009 @ v1 — claims are assessed with surgical staleness and explicit contradictions

> WHEN a verification claim is assessed against a graph snapshot and an evidence ledger, SpecForge SHALL evaluate evidence validity, provenance, freshness against each pinned dependency digest, contradictions, and inconclusive or bounded outcomes, returning a qualified claim status (`supported`, `refuted`, `inconclusive`, `requires_review`, `stale`, `invalid`) with explicit rationale and never an unqualified confidence percentage.
>
> WHEN a dependency of a claim or property changes in a newer snapshot, SpecForge SHALL compute the impact set, mark only affected evidence as `stale` while preserving it historically, and leave evidence whose dependencies did not change as `current`.

```gherkin
@SF-009 @v1
Feature: SF-009 — claims are assessed with surgical staleness and explicit contradictions

  @one @witness
  Scenario: fresh satisfied evidence supports a claim with explicit rationale
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "supported"
    And evidence "EV-001" has freshness "current"

  @near-miss
  Scenario: inconclusive evidence never marks a claim supported
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And an inconclusive evidence record "EV-INC-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "inconclusive"

  @near-miss
  Scenario: changing a linked property marks dependent evidence stale while preserving history
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When the snapshot changes to "sdv-message-property-edit.md"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "stale"
    And evidence "EV-001" has freshness "stale"
    And the ledger still holds 1 evidence record
    And the impact set names stale evidence "EV-001" and affected claim "CLM-MSG-001"

  @near-miss
  Scenario: changing an unrelated requirement does not stale unrelated evidence
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When the snapshot changes to "sdv-message-unrelated-edit.md"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "supported"
    And evidence "EV-001" has freshness "current"

  @near-miss
  Scenario: contradictory current evidence surfaces a contradiction requiring review
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a violated evidence record "EV-002" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "requires_review"
    And the claim assessment reports 1 contradiction

  @one
  Scenario: a bounded formal outcome preserves its declared bound without claiming unbounded proof
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And a bounded-success formal evidence record "EV-FORMAL-001" with bound "depth <= 8" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "supported"
    And the claim assessment rationale mentions "depth <= 8"
    And the claim assessment is not an unbounded proof

  @near-miss
  Scenario: evidence from a mismatched scope does not support a claim
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-SCOPE-001" in scope "local-mock" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "inconclusive"
    And the claim assessment rationale mentions "scope"
```

# Evidence and Assurance: append-only ledger, freshness and claim assessment

Evidence records are immutable facts with provenance and self-verifying SHA-256 content digests. Assessments are recomputed from the current graph snapshot and the ledger without mutating history.

### SF-008 @ v1 — the evidence ledger appends immutable provenance records and links

> WHEN an evidence record is appended to the ledger, SpecForge SHALL validate its schema and SHA-256 content digest, refuse duplicate record identifiers or tampered payloads, preserve all earlier records including negative and inconclusive evidence, and support typed append-only links (`supports`, `contradicts`, `corroborates`, `derived_from`, `supersedes`, `invalidates`).
>
> WHEN the ledger is exported as canonical JSON or imported from JSON, SpecForge SHALL preserve all records and links and re-verify full ledger integrity on import, rejecting invalid records, tampered digests, or dangling links with a diagnostic error naming the offending record, link, or field.
>
> WHEN the ledger is persisted to disk (`ledger-fs`), SpecForge SHALL write records and links to an append-only JSONL log and store raw artifacts as content-addressed blobs verified on reload and deduplication; IF any stored record, link, JSONL line, or blob fails integrity validation on reload, THEN SpecForge SHALL reject it with an integrity or corrupt-entry error identifying the offending entry, 1-based line number, or blob digest.

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
    And re-importing the exported ledger JSON preserves 1 record and 0 links

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

  @near-miss
  Scenario: malformed evidence records are rejected when sealed or appended with forged digests
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    When an evidence payload with invalid field "empty_id" is sealed
    Then sealing is refused with error mentioning "id must not be empty"
    When an evidence payload with invalid field "empty_producer" is sealed
    Then sealing is refused with error mentioning "producer.name"
    When an evidence payload with invalid field "empty_adapter" is sealed
    Then sealing is refused with error mentioning "adapter.name"
    When an evidence payload with invalid field "empty_timestamp" is sealed
    Then sealing is refused with error mentioning "timestamp"
    When an evidence payload with invalid field "empty_run_id" is sealed
    Then sealing is refused with error mentioning "run_id"
    When an evidence payload with invalid field "empty_scope" is sealed
    Then sealing is refused with error mentioning "scope"
    When an evidence payload with invalid field "positive_failed_run" is sealed
    Then sealing is refused with error mentioning "positive outcome"
    When an evidence payload with invalid field "replay_empty_engine" is sealed
    Then sealing is refused with error mentioning "engine"
    When an evidence payload with invalid field "replay_no_determinism" is sealed
    Then sealing is refused with error mentioning "replay token"
    When a forged valid-digest evidence record "EV-FORGED" with invalid field "positive_failed_run" is appended
    Then the append is refused with error mentioning "EV-FORGED"
    And the ledger holds 0 evidence records

  @near-miss
  Scenario: invalid, tampered, and dangling evidence links are refused
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a link "supports" is sealed from "" to "EV-001"
    Then link sealing is refused with error mentioning "from_id"
    When a link "supports" is sealed from "EV-001" to ""
    Then link sealing is refused with error mentioning "to_id"
    When a link "supports" is sealed from "EV-001" to "EV-001"
    Then link sealing is refused with error mentioning "self-link"
    When a tampered link "supports" from "EV-001" to "CLM-MSG-001" is appended
    Then the append is refused as invalid digest
    And the append error message mentions "EV-001->CLM-MSG-001"
    When an attempted link "supports" is appended from "EV-UNKNOWN" to "CLM-MSG-001"
    Then the append is refused as missing record mentioning "EV-UNKNOWN"
    When an attempted link "supersedes" is appended from "EV-001" to "EV-MISSING"
    Then the append is refused as missing record mentioning "EV-MISSING"
    When an attempted link "derived_from" is appended from "EV-001" to "EV-MISSING"
    Then the append is refused as missing record mentioning "EV-MISSING"
    When an attempted link "corroborates" is appended from "EV-001" to "EV-MISSING"
    Then the append is refused as missing record mentioning "EV-MISSING"
    When an attempted link "invalidates" is appended from "EV-001" to "EV-MISSING"
    Then the append is refused as missing record mentioning "EV-MISSING"
    When a link "supports" is appended from "EV-001" to "CLM-MSG-001"
    Then ledger integrity verification passes

  @near-miss
  Scenario: exporting and re-importing a linked ledger verifies every record and link
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a satisfied evidence record "EV-002" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a link "supports" is appended from "EV-001" to "CLM-MSG-001"
    And a link "supersedes" is appended from "EV-002" to "EV-001"
    Then ledger integrity verification passes
    And exporting the ledger as JSON is deterministic
    And re-importing the exported ledger JSON preserves 2 records and 2 links
    When the exported ledger JSON is mutated with "tampered_record" and imported
    Then ledger JSON import fails with error mentioning "EV-001"
    When the exported ledger JSON is mutated with "tampered_link" and imported
    Then ledger JSON import fails with error mentioning "EV-002->EV-001"
    When the exported ledger JSON is mutated with "missing_link_target" and imported
    Then ledger JSON import fails with error mentioning "EV-GHOST"
    When the exported ledger JSON is mutated with "missing_link_source" and imported
    Then ledger JSON import fails with error mentioning "EV-PHANTOM"
    When the exported ledger JSON is mutated with "forged_invalid_record" and imported
    Then ledger JSON import fails with error mentioning "EV-FORGED"

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

  @many
  Scenario: the filesystem ledger persists links, deduplicates content-addressed blobs, and enforces read-only mode
    Given a temporary filesystem ledger
    And a compiled snapshot from "sdv-message.md"
    When a satisfied evidence record "EV-FS-1" with blob payload "shared-trace-bytes" is persisted
    And a satisfied evidence record "EV-FS-2" with blob payload "shared-trace-bytes" is persisted
    And a link "supersedes" from "EV-FS-2" to "EV-FS-1" is persisted to the filesystem ledger
    And the filesystem ledger is reloaded
    Then the reloaded ledger holds 2 evidence records
    And the reloaded ledger holds 1 evidence link from "EV-FS-2" to "EV-FS-1"
    When the filesystem ledger is opened read-only
    Then the read-only ledger holds 2 evidence records and 1 link
    And appending a link to the read-only filesystem ledger is refused
    When the stored blob for "EV-FS-1" is tampered with
    And writing blob payload "shared-trace-bytes" to the filesystem ledger is attempted
    Then the blob write fails with a blob integrity error mentioning the blob digest

  @near-miss
  Scenario: a corrupt JSONL line in the filesystem ledger is rejected with its 1-based line number
    Given a temporary filesystem ledger
    And a compiled snapshot from "sdv-message.md"
    And a satisfied evidence record "EV-LINE-1" with blob payload "line-one-payload" is persisted
    When a malformed JSONL line is appended to the persisted ledger file
    And the filesystem ledger is reloaded
    Then reloading the filesystem ledger fails with a corrupt entry error at line 2
```

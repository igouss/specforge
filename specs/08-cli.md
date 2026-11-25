# Command-line interface: compiling, verifying, assessing, inspecting ledgers, computing impact, building arguments, and recommending activities

The `specforge` CLI exposes graph compilation, claim assessment, property-based verification with filesystem ledger persistence, ledger verification and export, cross-snapshot impact analysis, assurance argument synthesis, and read-only exploration recommendations with strict non-zero exit codes whenever a check is incomplete, inconclusive, stale, violated, refuted, defective, or has open recommendations.

### SF-015 @ v1 — the CLI compiles specifications, verifies properties, assesses claims, inspects ledgers, computes impact, builds arguments, and recommends activities with strict exit codes

> WHEN `specforge compile <specs-dir>` is invoked, SpecForge SHALL read Markdown files directly in `<specs-dir>` in sorted filename order while skipping directories and non-markdown entries, compile the verification graph, and exit `0` with the snapshot digest and artifact digests, or exit `2` reporting every defect or I/O error.
>
> WHEN `specforge assess <specs-dir> --ledger <dir>` is invoked, SpecForge SHALL open and verify the filesystem ledger read-only without mutating disk, assess every claim in the snapshot or the requested `--claim <id>`, exit `0` only when at least one claim is assessed and every assessed claim is `supported`, exit `1` when zero claims are declared or any claim is `inconclusive`, `stale`, `requires_review`, `refuted`, or `invalid`, and exit `2` on defects, unknown claims, invalid usage, or ledger errors.
>
> WHEN `specforge verify <specs-dir> --property <id>` is invoked, SpecForge SHALL execute `ProptestAdapter::for_machine` using the configured or default seed (`0`) and budget (`32`), persist a sealed evidence record and any counterexample blob when `--ledger <dir>` is supplied, exit `0` only when the outcome is `satisfied`, exit `1` when `violated` or `inconclusive`, and exit `2` on defects, invalid arguments, or I/O errors.
>
> WHEN `specforge ledger <dir> {verify|export|list}` is invoked, SpecForge SHALL open and verify every record, link, and blob in the filesystem ledger read-only without mutating disk and exit `0` with the requested report, or exit `2` on any integrity, I/O, or usage error.
>
> WHEN `specforge impact --from <old-specs-dir> --to <new-specs-dir> --ledger <dir>` is invoked, SpecForge SHALL compute the impact set between both compiled snapshots against the filesystem ledger, exit `0` only when no evidence is stale and no claim is affected, exit `1` when any evidence is stale or any claim is affected, and exit `2` on defects, I/O, or usage errors.
>
> WHEN `specforge argument <specs-dir> --ledger <dir> --claim <id> [--subclaim <id>...] [--json]` is invoked, SpecForge SHALL open the filesystem ledger read-only without mutating disk, synthesize an assurance argument for the root claim and optional subclaims, emit deterministic Markdown by default or canonical JSON with `--json`, exit `0` only when the argument status is `supported`, exit `1` for any non-supported status, and exit `2` on defects, unknown, duplicate, or self-referencing claims, invalid usage, or ledger errors.
>
> WHEN `specforge recommend <specs-dir> --ledger <dir> [--json]` is invoked, SpecForge SHALL open the filesystem ledger read-only without mutating disk, assess all claims in the snapshot, hydrate persisted counterexample blobs into a counterexample corpus, compute exploration recommendations while ensuring every `refuted` claim emits an actionable recommendation, emit a human-readable list or canonical JSON with `--json`, exit `0` when zero recommendations are emitted, exit `1` when one or more recommendations exist, and exit `2` on defects, invalid usage, or ledger errors.

```gherkin
@SF-015 @v1
Feature: SF-015 — the CLI compiles specifications, verifies properties, assesses claims, inspects ledgers, computes impact, builds arguments, and recommends activities with strict exit codes

  @one @witness
  Scenario: compiling a valid specification directory succeeds and a defective directory fails with exit 2
    Given a CLI workspace with a valid specification directory containing non-markdown and nested entries
    When the CLI runs compile on the valid specification directory
    Then the CLI exits with code 0
    And the CLI stdout contains "snapshot_id:"
    And the CLI stdout contains "PROP-MSG-001"
    When the CLI runs compile with JSON output on the valid specification directory
    Then the CLI exits with code 0
    And the CLI JSON field "snapshot_id" is a 64-character hex digest
    And the CLI JSON object "digests" contains key "CLM-MSG-001"
    When the CLI runs compile on a defective specification directory
    Then the CLI exits with code 2
    And the CLI stderr contains "SM-BAD-001"
    And the CLI stderr contains "PROP-BAD-001"
    When the CLI runs compile with JSON output on a multi-file specification directory
    Then the CLI exits with code 0
    And the CLI JSON field "snapshot_id" is a 64-character hex digest
    And the CLI JSON object "digests" contains key "SM-MSG-001"
    And the CLI JSON object "digests" contains key "CLM-MSG-001"
    And the multi-file compile snapshot digest matches sorted source compilation
    When the CLI runs compile on a specification directory containing a dangling markdown symlink
    Then the CLI exits with code 2

  @one @witness
  Scenario: verifying a satisfied property records evidence in a filesystem ledger and claim assessment exits 0
    Given a CLI workspace with a safe specification directory and an empty ledger directory
    When the CLI assesses the safe specification directory against the ledger
    Then the CLI exits with code 1
    And the CLI stdout contains "inconclusive"
    And the empty ledger directory has no blobs subdirectory created
    When the CLI verifies property "PROP-MSG-001" for claim "CLM-MSG-001" with seed 7 and budget 16 into the ledger as JSON
    Then the CLI exits with code 0
    And the CLI JSON field "outcome" equals "satisfied"
    And the CLI JSON field "evidence_id" equals "EV-CLI-1"
    When the CLI assesses the safe specification directory against the ledger
    Then the CLI exits with code 0
    And the CLI stdout contains "CLM-MSG-001"
    And the CLI stdout contains "supported"
    When the CLI assesses claim "CLM-MSG-001" on the safe specification directory against the ledger as JSON
    Then the CLI exits with code 0
    And the CLI JSON field "passed" equals boolean true

  @one @witness
  Scenario: verifying a violated or zero-budget property exits 1 and persists counterexample blobs
    Given a CLI workspace with a violating specification directory and an empty ledger directory
    When the CLI verifies property "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 and budget 128 into the ledger as JSON
    Then the CLI exits with code 1
    And the CLI JSON field "outcome" equals "violated"
    And the CLI ledger contains a persisted counterexample blob
    When the CLI assesses the violating specification directory against the ledger
    Then the CLI exits with code 1
    And the CLI stdout contains "refuted"
    When the CLI verifies property "PROP-MSG-001" for claim "CLM-MSG-001" with seed 0 and budget 0 without a ledger
    Then the CLI exits with code 1
    And the CLI stdout contains "inconclusive"

  @one @witness
  Scenario: ledger verify, export, and list inspect integrity and reject tampered ledgers with exit 2
    Given a CLI workspace with a violating specification directory and an empty ledger directory
    And the CLI verifies property "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 and budget 128 into the ledger as JSON
    When the CLI runs ledger verify on the ledger directory
    Then the CLI exits with code 0
    And the CLI stdout contains "valid"
    When the CLI runs ledger export on the ledger directory
    Then the CLI exits with code 0
    And the CLI stdout matches the canonical ledger export JSON
    When the CLI runs ledger list with JSON output on the ledger directory
    Then the CLI exits with code 0
    And the CLI JSON array "records" has length 1
    When a persisted ledger blob is tampered on disk
    And the CLI runs ledger verify on the ledger directory
    Then the CLI exits with code 2
    When the CLI assesses the violating specification directory against the ledger
    Then the CLI exits with code 2

  @one @witness
  Scenario: impact exits 0 on unrelated edits and exits 1 when evidence becomes stale or claims are affected
    Given a CLI workspace with baseline, unrelated-edit, and property-edit safe specification directories
    And the CLI verifies property "PROP-MSG-001" for claim "CLM-MSG-001" with seed 7 and budget 16 into the ledger as JSON
    When the CLI computes impact from baseline to unrelated-edit against the ledger as JSON
    Then the CLI exits with code 0
    And the CLI JSON array "stale_evidence_ids" has length 0
    And the CLI JSON array "affected_claim_ids" has length 0
    When the CLI computes impact from baseline to property-edit against the ledger as JSON
    Then the CLI exits with code 1
    And the CLI JSON array "stale_evidence_ids" has length 1
    And the CLI JSON array "affected_claim_ids" has length 1
    When the CLI assesses the property-edit specification directory against the ledger
    Then the CLI exits with code 1
    And the CLI stdout contains "stale"

  @near-miss
  Scenario: assessing zero claims, unknown targets, or invalid CLI invocations never reports false green
    Given a CLI workspace with a claimless specification directory and an empty ledger directory
    When the CLI assesses the claimless specification directory against the ledger
    Then the CLI exits with code 1
    When the CLI assesses claim "CLM-UNKNOWN-999" on the claimless specification directory against the ledger as JSON
    Then the CLI exits with code 2
    When the CLI verifies property "PROP-UNKNOWN-999" on the claimless specification directory
    Then the CLI exits with code 2
    When the CLI runs with raw arguments ""
    Then the CLI exits with code 2
    When the CLI runs with raw arguments "unknown-cmd"
    Then the CLI exits with code 2
    When the CLI runs with raw arguments "compile --bogus"
    Then the CLI exits with code 2
    When the CLI runs with raw arguments "verify specs --property PROP-MSG-001 --budget notanumber"
    Then the CLI exits with code 2
    When the CLI runs ledger verify on a nonexistent ledger directory
    Then the CLI exits with code 2
    When the CLI runs ledger verify on the empty ledger directory
    Then the CLI exits with code 0
    And the empty ledger directory has no blobs subdirectory created

  @one @witness
  Scenario: argument synthesizes Markdown and JSON assurance reports with strict exit codes
    Given a CLI workspace with a safe assurance specification directory and an empty ledger directory
    When the CLI builds an argument for claim "CLM-MSG-001" on the safe directory against the ledger
    Then the CLI exits with code 1
    And the CLI stdout contains "# Assurance Argument: CLM-MSG-001"
    And the CLI stdout contains "Status: `inconclusive`"
    And the empty ledger directory has no blobs subdirectory created
    When the CLI verifies property "PROP-MSG-001" for claim "CLM-MSG-001" with seed 7 and budget 16 into the ledger as JSON
    Then the CLI exits with code 0
    When the CLI builds an argument for claim "CLM-MSG-001" on the safe directory against the ledger
    Then the CLI exits with code 0
    And the CLI stdout contains "# Assurance Argument: CLM-MSG-001"
    And the CLI stdout contains "Status: `supported`"
    When the CLI builds an argument for claim "CLM-MSG-001" with subclaims "CLM-MSG-002" on the safe directory against the ledger as JSON
    Then the CLI exits with code 1
    And the CLI JSON field "status" equals "inconclusive"
    And the CLI JSON array "subclaims" has length 1
    When the CLI verifies property "PROP-MSG-002" for claim "CLM-MSG-002" with seed 7 and budget 16 into the ledger as JSON
    Then the CLI exits with code 0
    When the CLI builds an argument for claim "CLM-MSG-001" with subclaims "CLM-MSG-002" on the safe directory against the ledger as JSON
    Then the CLI exits with code 0
    And the CLI JSON field "status" equals "supported"
    And the CLI JSON array "subclaims" has length 1
    When the CLI builds an argument for claim "CLM-MSG-001" with subclaims "CLM-MSG-001" on the safe directory against the ledger as JSON
    Then the CLI exits with code 2
    And the CLI stderr contains "self-referencing"
    When the CLI builds an argument for claim "CLM-MSG-001" with subclaims "CLM-MSG-002,CLM-MSG-002" on the safe directory against the ledger as JSON
    Then the CLI exits with code 2
    And the CLI stderr contains "duplicate subclaim"
    When the CLI builds an argument for claim "CLM-UNKNOWN-999" on the safe directory against the ledger as JSON
    Then the CLI exits with code 2
    And the CLI stderr contains "unknown claim"
    When the CLI runs with raw arguments "argument specs --ledger led"
    Then the CLI exits with code 2
    When the CLI builds an argument for claim "CLM-MSG-001" on a defective specification directory against the ledger
    Then the CLI exits with code 2
    When the CLI builds an argument for claim "CLM-MSG-001" on the safe directory against a nonexistent ledger
    Then the CLI exits with code 2

  @one @witness
  Scenario: recommend emits human-readable and JSON recommendations with strict exit codes
    Given a CLI workspace with baseline, unrelated-edit, and property-edit safe specification directories
    When the CLI runs recommend on the safe directory against the ledger
    Then the CLI exits with code 1
    And the CLI stdout contains "recommendations: 1"
    And the CLI stdout contains "unverified_claim"
    And the empty ledger directory has no blobs subdirectory created
    When the CLI verifies property "PROP-MSG-001" for claim "CLM-MSG-001" with seed 7 and budget 16 into the ledger as JSON
    Then the CLI exits with code 0
    When the CLI runs recommend on the safe directory against the ledger as JSON
    Then the CLI exits with code 0
    And the CLI JSON field "clean" equals boolean true
    And the CLI JSON array "recommendations" has length 0
    When the CLI runs recommend on the property-edit directory against the ledger as JSON
    Then the CLI exits with code 1
    And the CLI JSON field "clean" equals boolean false
    And the CLI JSON array "recommendations" has length 1
    When the CLI runs recommend on a defective specification directory against the ledger
    Then the CLI exits with code 2
    When the CLI runs recommend on the safe directory against a nonexistent ledger
    Then the CLI exits with code 2

  @near-miss
  Scenario: recommend hydrates persisted counterexamples and exits 1 on refuted claims
    Given a CLI workspace with a violating specification directory and an empty ledger directory
    When the CLI verifies property "PROP-MSG-001" for claim "CLM-MSG-001" with seed 42 and budget 128 into the ledger as JSON
    Then the CLI exits with code 1
    And the CLI JSON field "outcome" equals "violated"
    When the CLI runs recommend on the violating directory against the ledger as JSON
    Then the CLI exits with code 1
    And the CLI JSON field "clean" equals boolean false
    And the CLI JSON array "recommendations" has length 1
    And the CLI stdout contains "regression_counterexample"
    Given a CLI workspace with a violating specification directory and a blobless violated ledger
    When the CLI runs recommend on the violating directory against the ledger as JSON
    Then the CLI exits with code 1
    And the CLI JSON field "clean" equals boolean false
    And the CLI JSON array "recommendations" has length 1
    And the CLI stdout contains "regression_counterexample"
```

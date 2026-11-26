# Assurance: claim assessment, freshness overrides, and surgical impact

Claims are assessed deterministically against the current graph snapshot and append-only evidence ledger without mutating history.

### SF-009 @ v1 — claims are assessed with surgical staleness and explicit contradictions

> WHEN a verification claim is assessed against a graph snapshot and an evidence ledger, SpecForge SHALL evaluate evidence validity, provenance, freshness against each pinned dependency digest, contradictions, and inconclusive or bounded outcomes, returning a qualified claim status (`supported`, `refuted`, `inconclusive`, `requires_review`, `stale`, `invalid`) with explicit rationale and never an unqualified confidence percentage.
>
> WHEN a dependency of a claim or property changes in a newer snapshot, SpecForge SHALL compute the impact set, mark only affected evidence as `stale` while preserving it historically, and leave evidence whose dependencies did not change as `current`.
>
> WHEN an `invalidates` or `supersedes` link targets an earlier evidence record, SpecForge SHALL mark that record `invalidated` or `superseded` only when the link can override it: a completed conclusive record may override a violation, an inconclusive record may override a positive record, and neither an inconclusive nor an incomplete run SHALL silence a negative outcome.
>
> WHEN evidence is linked to a claim via `supports` or `contradicts` links or contradicts supporting evidence via a ledger link, SpecForge SHALL scope link relevance to the targeted claim or supporting evidence, enforce required dependency pins and provenance metadata, withhold `supported` when any required property is uncovered, and distinguish absent evidence from current inconclusive or partial evidence in rationale.

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
    And the claim assessment rationale does not mention "no current evidence linked to claim"

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

  @near-miss
  Scenario: absent evidence, incomplete provenance, and uncovered properties never mark a claim supported
    Given an empty evidence ledger
    And a compiled snapshot from "sdv-message.md"
    When claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "inconclusive"
    And the claim assessment rationale mentions "no current evidence linked to claim"
    Given an empty evidence ledger
    And a satisfied evidence record "EV-NO-SRC" without source revisions is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "inconclusive"
    And the claim assessment rationale mentions "incomplete provenance metadata"
    Given an empty evidence ledger
    And a satisfied evidence record "EV-NO-PROP" without a property is appended for claim "CLM-MSG-001"
    When claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "inconclusive"
    And the claim assessment rationale mentions "PROP-MSG-001"
    And the claim assessment rationale does not mention "no current evidence linked to claim"

  @near-miss
  Scenario: invalidation and supersession links obey override rules without silencing negative outcomes
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And a violated evidence record "EV-VIOL-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And an inconclusive evidence record "EV-INC-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a link "invalidates" is appended from "EV-INC-1" to "EV-VIOL-1"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "refuted"
    And evidence "EV-VIOL-1" has freshness "current"
    Given an empty evidence ledger
    And a violated evidence record "EV-VIOL-2" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And an inconclusive evidence record "EV-INC-2" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a link "supersedes" is appended from "EV-INC-2" to "EV-VIOL-2"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "refuted"
    And evidence "EV-VIOL-2" has freshness "current"
    Given an empty evidence ledger
    And a violated evidence record "EV-VIOL-3" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a failed violated evidence record "EV-FAIL-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a link "supersedes" is appended from "EV-FAIL-1" to "EV-VIOL-3"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "refuted"
    And evidence "EV-VIOL-3" has freshness "current"
    Given an empty evidence ledger
    And a violated evidence record "EV-VIOL-4" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a satisfied evidence record "EV-FIX-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a satisfied evidence record "EV-KEEP-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a link "supersedes" is appended from "EV-FIX-1" to "EV-VIOL-4"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "supported"
    And evidence "EV-VIOL-4" has freshness "superseded"
    And evidence "EV-FIX-1" has freshness "current"
    And evidence "EV-KEEP-1" has freshness "current"
    Given an empty evidence ledger
    And a satisfied evidence record "EV-SAT-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And an inconclusive evidence record "EV-INC-3" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And an inconclusive evidence record "EV-INC-4" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a link "invalidates" is appended from "EV-INC-3" to "EV-SAT-1"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "inconclusive"
    And evidence "EV-SAT-1" has freshness "invalidated"
    And evidence "EV-INC-3" has freshness "current"
    And evidence "EV-INC-4" has freshness "current"

  @near-miss
  Scenario: ledger links scope relevance by target and relation, enforce claim pins, and surface evidence contradictions
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    And a standalone satisfied evidence record "EV-LINK-NOPIN" with property pins only is appended for property "PROP-MSG-001"
    When a link "supports" is appended from "EV-LINK-NOPIN" to "CLM-MSG-001"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "stale"
    And evidence "EV-LINK-NOPIN" has freshness "stale"
    Given an empty evidence ledger
    And a standalone satisfied evidence record "EV-LINK-PINNED" with claim pins for "CLM-MSG-001" is appended for property "PROP-MSG-001"
    When a link "supports" is appended from "EV-LINK-PINNED" to "CLM-MSG-001"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "supported"
    And evidence "EV-LINK-PINNED" has freshness "current"
    Given an empty evidence ledger
    And a satisfied evidence record "EV-SUP-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a standalone violated evidence record "EV-OTHER-CLM" with property pins only is appended for property "PROP-MSG-002"
    And a standalone satisfied evidence record "CLM-MSG-001" with property pins only is appended for property "PROP-MSG-002"
    And a standalone violated evidence record "EV-DERIVED-BAD" with property pins only is appended for property "PROP-MSG-002"
    When a link "supports" is appended from "EV-OTHER-CLM" to "CLM-MSG-002"
    And a link "contradicts" is appended from "EV-SUP-1" to "CLM-MSG-002"
    And a link "derived_from" is appended from "EV-DERIVED-BAD" to "CLM-MSG-001"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "supported"
    Given an empty evidence ledger
    And a satisfied evidence record "EV-SUP-2" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a standalone satisfied evidence record "EV-CONTRA-EV" with property pins only is appended for property "PROP-MSG-001"
    When a link "contradicts" is appended from "EV-CONTRA-EV" to "EV-SUP-2"
    And claim "CLM-MSG-001" is assessed against the current snapshot
    Then the claim status is "requires_review"
    And the claim assessment reports 1 contradiction

  @near-miss
  Scenario: impact set reports affected claims without evidence, unpinned changed dependencies, and surgical link propagation
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    When the snapshot changes to "sdv-message-property-edit.md"
    Then the impact set names affected claim "CLM-MSG-001" with 0 stale evidence records
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And a satisfied evidence record "EV-CLM-ONLY-PIN" pinning only "CLM-MSG-001" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When the snapshot changes to "sdv-message-property-edit.md"
    Then the impact set names stale evidence "EV-CLM-ONLY-PIN" and affected claim "CLM-MSG-001"
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And a standalone satisfied evidence record "EV-REQ2-LINKED" pinning "CLM-MSG-001, PROP-MSG-001, REQ-002" is appended for property "PROP-MSG-001"
    When a link "supports" is appended from "EV-REQ2-LINKED" to "CLM-MSG-001"
    And the snapshot changes to "sdv-message-unrelated-edit.md"
    Then the impact set names stale evidence "EV-REQ2-LINKED" and affected claim "CLM-MSG-001"
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And a standalone satisfied evidence record "EV-FRESH-LINKED" with claim pins for "CLM-MSG-001" is appended for property "PROP-MSG-001"
    And a standalone satisfied evidence record "EV-REQ2-ORPHAN" pinning "REQ-002" is appended for property "PROP-MSG-001"
    When a link "supports" is appended from "EV-FRESH-LINKED" to "CLM-MSG-001"
    And the snapshot changes to "sdv-message-unrelated-edit.md"
    Then the impact set names stale evidence "EV-REQ2-ORPHAN" without affecting claim "CLM-MSG-001"
```

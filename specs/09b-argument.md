# Assurance arguments

### SF-018 @ v1 — assurance arguments synthesize claims, subclaims, and defeaters into deterministic JSON and Markdown exports

> WHEN an assurance argument is built for a root claim and optional subclaims against a graph snapshot and evidence ledger, SpecForge SHALL assess the root claim and each subclaim, capture scope, assumptions, evidence items with limitations, defeaters, rationale, status, and unbounded proof standing without collapsing into probabilistic scores, and reject unknown or self-referencing claim identifiers.
>
> IF any subclaim is refuted, requires review, stale, invalid, or inconclusive, THEN SpecForge SHALL disclose that subclaim as a defeater on the root argument, withhold unbounded proof, and prevent the root argument from reporting supported; and every non-supported claim or subclaim SHALL surface at least one explicit defeater (`integrity_failure`, `disqualified_evidence`, `contradiction`, `evidence_gap`, or outcome/freshness defeater).
>
> WHEN an assurance argument is exported, SpecForge SHALL produce deterministic canonical JSON and deterministic Markdown containing the claim, subclaims with indented evidence, defeaters, and rationale, assumptions, and defeaters.

```gherkin
@SF-018 @v1
Feature: SF-018 — assurance arguments synthesize claims, subclaims, and defeaters into deterministic JSON and Markdown exports

  @one @witness
  Scenario: building an assurance argument with proved root and bounded or proved subclaim exports deterministic JSON and Markdown
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    And formal proved evidence exists for "PROP-MSG-001" and claim "CLM-MSG-001"
    And bounded formal evidence "depth <= 12" exists for "PROP-MSG-002" and claim "CLM-MSG-002"
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims "CLM-MSG-002"
    Then the assurance argument status is "supported" and unbounded proof is false
    And the assurance argument has 0 defeaters
    And the assurance argument lists assumption "ASM-MSG-001" and scope "single-broker"
    And the assurance argument includes 1 evidence item and 1 subclaim
    And the assurance argument JSON and Markdown exports are deterministic and disclose "depth <= 12"
    And the exported Markdown includes "method: `formal`" and "outcome: `formal:proved`"
    And the exported Markdown includes "outcome: `formal:bounded_success(depth <= 12)`" and "- Assumptions: ASM-MSG-001"
    And the exported Markdown includes indented subclaim rationale for "CLM-MSG-002"
    Given an empty evidence ledger
    And formal proved evidence exists for "PROP-MSG-001" and claim "CLM-MSG-001" with a supports link
    And formal proved evidence exists for "PROP-MSG-002" and claim "CLM-MSG-002"
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims "CLM-MSG-002"
    Then the assurance argument status is "supported" and unbounded proof is true
    And the assurance argument has 0 defeaters

  @near-miss
  Scenario: non-supported claims always surface explicit defeaters for gaps, disqualifications, contradictions, and integrity failures
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims ""
    Then the assurance argument status is "inconclusive" and unbounded proof is false
    And the assurance argument discloses a defeater of kind "evidence_gap" naming "PROP-MSG-001"
    Given an empty evidence ledger
    And disqualified wrong-scope formal evidence exists for "PROP-MSG-001" and claim "CLM-MSG-001"
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims ""
    Then the assurance argument status is "inconclusive" and unbounded proof is false
    And the assurance argument discloses a defeater of kind "disqualified_evidence" naming "EV-DISQ-1"
    Given an empty evidence ledger
    And claim-contradicting evidence link exists for "PROP-MSG-001" and claim "CLM-MSG-001"
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims ""
    Then the assurance argument status is "refuted" and unbounded proof is false
    And the assurance argument discloses a defeater of kind "contradiction" naming "EV-LINK-CONTRA"
    Given an empty evidence ledger
    And invalid-evaluation evidence exists for "PROP-MSG-001" and claim "CLM-MSG-001"
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims ""
    Then the assurance argument status is "invalid" and unbounded proof is false
    And the assurance argument discloses a defeater of kind "invalid_evidence" naming "EV-INVALID-1"

  @near-miss
  Scenario: contradictory root evidence and refuted or inconclusive subclaims surface as explicit defeaters
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    And formal proved evidence exists for "PROP-MSG-001" and claim "CLM-MSG-001"
    And refuted formal evidence exists for "PROP-MSG-002" and claim "CLM-MSG-002"
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims "CLM-MSG-002"
    Then the assurance argument status is "refuted" and unbounded proof is false
    And the assurance argument discloses a defeater of kind "subclaim_defeater" naming "CLM-MSG-002"
    And the exported Markdown includes "subclaim_defeater" and "CLM-MSG-002"
    When contradictory DST violation evidence is also appended for "PROP-MSG-001" and claim "CLM-MSG-001"
    And an assurance argument is built for claim "CLM-MSG-001" with subclaims ""
    Then the assurance argument status is "requires_review" and unbounded proof is false
    And the assurance argument discloses a defeater of kind "contradiction" naming "EV-DST-CONTRA"
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims "CLM-MSG-002"
    Then the assurance argument status is "requires_review" and unbounded proof is false
    And the assurance argument discloses a defeater of kind "subclaim_defeater" naming "CLM-MSG-002"

  @near-miss
  Scenario: unknown or self-referencing claim identifiers are rejected when building an assurance argument
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims "CLM-MSG-001"
    Then assurance argument construction fails with an error mentioning "self-referencing"
    When an assurance argument is built for claim "CLM-UNKNOWN-999" with subclaims ""
    Then assurance argument construction fails with an error mentioning "unknown claim"
```

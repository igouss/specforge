# External assurance: DST adapter, formal adapter and assurance argument export

SpecForge defines pure, engine-agnostic normalization contracts for deterministic simulation testing (DST) adapters and formal verification adapters, and builds deterministic, defeater-preserving assurance arguments with JSON and Markdown export.

### SF-016 @ v1 — deterministic simulation reports normalize into replay-verified evidence envelopes

> WHEN a deterministic simulation report is normalized against a graph snapshot and request, SpecForge SHALL validate the property, subject machine, claims, request identifiers, engine and adapter versions, trace initial state and event fields, and monotonic virtual clocks when virtual clock determinism is claimed, hash the canonical DST replay metadata, and evaluate the property over the simulation trace while preserving fault and virtual time annotations.
>
> WHEN the DST replay metadata satisfies all determinism guarantees and records a seed or trace events, SpecForge SHALL mark the replay descriptor replayable; OTHERWISE SpecForge SHALL set replayable to false and append an explicit limitation naming every missing determinism guarantee.
>
> IF a DST run reports an inconclusive, failed, or cancelled run state, or if property evaluation is inconclusive or not evaluated, THEN SpecForge SHALL NOT report the verification outcome as satisfied and SHALL append an explicit limitation.

```gherkin
@SF-016 @v1
Feature: SF-016 — deterministic simulation reports normalize into replay-verified evidence envelopes

  @one @witness
  Scenario: a fully deterministic simulation run normalizes with replayable metadata and preserves faults and virtual time
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    When a deterministic DST adapter executes a violating fault-injected simulation for "PROP-MSG-001" and claim "CLM-MSG-001"
    Then the DST envelope has run state "completed" and outcome "violated"
    And the DST replay descriptor is replayable with a deterministic metadata digest
    And the DST counterexample preserves fault injection and virtual time annotations
    When the deterministic DST adapter replays the DST counterexample
    Then the replayed DST envelope reproduces the violation
    When a deterministic DST adapter executes a safe simulation for "PROP-MSG-001" and claim "CLM-MSG-001"
    Then the DST envelope has run state "completed" and outcome "satisfied"
    And the DST envelope seals into the evidence ledger with its replay metadata digest

  @near-miss
  Scenario: a non-deterministic simulation run is marked non-replayable with explicit missing guarantees
    Given a compiled snapshot from "sdv-assurance.md"
    When a non-deterministic DST report missing virtual clock and deterministic faults is normalized for "PROP-MSG-001"
    Then the DST replay descriptor is not replayable
    And the DST limitations name missing guarantees "virtual_clock, deterministic_faults"

  @near-miss
  Scenario: an inconclusive, failed, or cancelled DST run never reports satisfied and invalid versions or traces are rejected
    Given a compiled snapshot from "sdv-assurance.md"
    When a safe-trace DST report with run state "inconclusive" is normalized for "PROP-MSG-001"
    Then the DST envelope has run state "inconclusive" and outcome "inconclusive"
    When a safe-trace DST report with run state "failed" is normalized for "PROP-MSG-001"
    Then the DST envelope has run state "failed" and outcome "inconclusive"
    When a safe-trace DST report with run state "cancelled" is normalized for "PROP-MSG-001"
    Then the DST envelope has run state "cancelled" and outcome "inconclusive"
    When a DST report with an empty engine version is normalized for "PROP-MSG-001"
    Then DST normalization fails with an invalid request error
    When a DST report with an empty initial state is normalized for "PROP-MSG-001"
    Then DST normalization fails with an invalid request error
    When a DST report with an unknown initial state is normalized for "PROP-MSG-001"
    Then DST normalization fails with an invalid request error
    When a DST report with an empty trace event name is normalized for "PROP-MSG-001"
    Then DST normalization fails with an invalid request error
    When a DST report with a decreasing virtual clock is normalized for "PROP-MSG-001"
    Then DST normalization fails with an invalid request error
    When a completed DST report evaluates an input property with no matching input
    Then the DST envelope has run state "inconclusive" and outcome "inconclusive"
    And the DST limitations mention "inconclusive"
```

### SF-017 @ v1 — formal verification reports preserve all formal outcomes, bounds, assumptions, and artifact digests

> WHEN a formal verification report is normalized against a graph snapshot and request, SpecForge SHALL validate the property reference, claims, snapshot assumptions, engine and adapter versions, artifact names, and counterexample trace structure, preserve all seven formal outcomes losslessly, preserve `"formal_property_ref"` on positive outcomes, and collect all formal artifact digests.
>
> IF a formal report claims a bounded outcome with an empty bound or degenerate zero bounds, claims an unbounded proof with bounds, without proof artifacts, or with a counterexample trace, claims bounded success with a counterexample trace, or relies on assumptions inconsistent with the property or claim on a positive outcome, THEN SpecForge SHALL reject normalization with an invalid request error.
>
> WHEN a formal report returns bounded success, SpecForge SHALL retain the declared bounds in limitations without claiming unbounded proof, WHEN a formal report returns a counterexample trace, SpecForge SHALL mark the replay descriptor non-replayable with an explicit limitation, and WHEN a formal report returns unknown, timeout, or unsupported, SpecForge SHALL map run state to inconclusive without emitting probabilistic confidence scores.

```gherkin
@SF-017 @v1
Feature: SF-017 — formal verification reports preserve all formal outcomes, bounds, assumptions, and artifact digests

  @one @witness
  Scenario: proved and bounded formal reports normalize with artifact digests and distinct proof strength
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    When a formal adapter checks "PROP-MSG-001" for claim "CLM-MSG-001" with outcome "proved"
    Then the formal envelope has run state "completed" and formal outcome "proved"
    And the formal artifact digests include "proof-script, proof-log, formal_property_ref"
    And the formal envelope seals into the evidence ledger and supports "CLM-MSG-001" as unbounded proof
    Given an empty evidence ledger
    When a formal adapter checks "PROP-MSG-001" for claim "CLM-MSG-001" with bounded success "depth <= 12"
    Then the formal envelope has run state "completed" and formal outcome "bounded_success:depth <= 12"
    And the formal artifact digests include "proof-script, proof-log, formal_property_ref"
    And the formal limitations retain bound "depth <= 12"
    And the formal envelope seals into the evidence ledger and supports "CLM-MSG-001" without unbounded proof

  @near-miss
  Scenario: refuted, bounded failure, unknown, timeout, and unsupported formal outcomes are preserved without false greens
    Given a compiled snapshot from "sdv-assurance.md"
    When a formal adapter checks "PROP-MSG-001" for claim "CLM-MSG-001" with outcome "refuted"
    Then the formal envelope has run state "completed" and formal outcome "refuted"
    And the formal envelope records a finding and counterexample
    And the formal counterexample is not replayable and discloses a replay limitation
    When a formal adapter checks "PROP-MSG-001" for claim "CLM-MSG-001" with bounded failure "depth <= 8"
    Then the formal envelope has run state "completed" and formal outcome "bounded_failure:depth <= 8"
    And the formal limitations retain bound "depth <= 8"
    And the formal counterexample is not replayable and discloses a replay limitation
    When a formal adapter checks "PROP-MSG-001" for claim "CLM-MSG-001" with outcome "timeout"
    Then the formal envelope has run state "inconclusive" and formal outcome "timeout"
    When a formal adapter checks "PROP-MSG-001" for claim "CLM-MSG-001" with outcome "unknown"
    Then the formal envelope has run state "inconclusive" and formal outcome "unknown"
    When a formal adapter checks "PROP-MSG-001" for claim "CLM-MSG-001" with outcome "unsupported"
    Then the formal envelope has run state "inconclusive" and formal outcome "unsupported"

  @near-miss
  Scenario: empty bounds, inconsistent proof assumptions, invalid proved claims, and invalid traces are rejected
    Given a compiled snapshot from "sdv-assurance.md"
    When a formal report with empty bounded success is normalized for "PROP-MSG-001"
    Then formal normalization fails with an invalid request error
    When a proved formal report with undeclared assumption "ASM-EXTRA-001" is normalized for "PROP-MSG-001" and claim "CLM-MSG-001"
    Then formal normalization fails with an invalid request error
    When a bounded success formal report with undeclared assumption "ASM-EXTRA-001" is normalized for "PROP-MSG-001" and claim "CLM-MSG-001"
    Then formal normalization fails with an invalid request error
    When a bounded success formal report with zero max_states is normalized for "PROP-MSG-001"
    Then formal normalization fails with an invalid request error
    When a proved formal report with a bounds description is normalized for "PROP-MSG-001"
    Then formal normalization fails with an invalid request error
    When a proved formal report without proof artifacts is normalized for "PROP-MSG-001"
    Then formal normalization fails with an invalid request error
    When a proved formal report with a counterexample trace is normalized for "PROP-MSG-001"
    Then formal normalization fails with an invalid request error
    When a bounded success formal report with a counterexample trace is normalized for "PROP-MSG-001"
    Then formal normalization fails with an invalid request error
    When a refuted formal report with a discontinuous counterexample trace is normalized for "PROP-MSG-001"
    Then formal normalization fails with an invalid request error
    When a formal report referencing property "PROP-MSG-002" is normalized for request "PROP-MSG-001"
    Then formal normalization fails with an invalid request error
```

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
    And the assurance argument lists assumption "ASM-MSG-001" and scope "single-broker"
    And the assurance argument includes 1 evidence item and 1 subclaim
    And the assurance argument JSON and Markdown exports are deterministic and disclose "depth <= 12"
    And the exported Markdown includes indented subclaim rationale for "CLM-MSG-002"
    Given an empty evidence ledger
    And formal proved evidence exists for "PROP-MSG-001" and claim "CLM-MSG-001"
    And formal proved evidence exists for "PROP-MSG-002" and claim "CLM-MSG-002"
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims "CLM-MSG-002"
    Then the assurance argument status is "supported" and unbounded proof is true

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

  @near-miss
  Scenario: unknown or self-referencing claim identifiers are rejected when building an assurance argument
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    When an assurance argument is built for claim "CLM-MSG-001" with subclaims "CLM-MSG-001"
    Then assurance argument construction fails with an error mentioning "self-referencing"
    When an assurance argument is built for claim "CLM-UNKNOWN-999" with subclaims ""
    Then assurance argument construction fails with an error mentioning "unknown claim"
```

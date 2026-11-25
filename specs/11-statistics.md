# Qualified statistical assessment

SpecForge computes optional, qualified statistical estimates only when an explicit probabilistic model, sampling assumptions, scope, and homogeneous verified evidence are declared, keeping every estimate strictly separate from formal proof status.

### SF-020 @ v1 — qualified statistical estimates require explicit sampling assumptions, homogeneous current evidence, and separation from formal proofs

> WHERE a defensible statistical model and sampling assumptions are declared for a claim in a graph snapshot, SpecForge SHALL compute a deterministic Wilson score or Beta-Binomial qualified estimate recording the hypothesis, method, sampling population, sampling process, independence assumptions, scope, sample size, observed successes and failures, point estimate, confidence or credible interval bounds satisfying `lower_bound <= point_estimate <= upper_bound`, confidence level percentage, declared limitations plus an explicit non-proof disclaimer (and, for Beta-Binomial estimates, disclosure of the normal approximation to the Beta posterior credible interval), evidence identifiers, and `separate_from_formal_proof` set to `true`.
>
> IF statistical assumptions, hypothesis, sampling population, sampling process, scope, or limitations are absent or blank, if the model scope differs from the claim scope, if Beta-Binomial priors are non-positive or non-finite, if the claim is unknown, if sample counts or evidence identifiers are invalid or duplicated, or if any referenced evidence record carries a `violated` outcome while `observed_failures == 0`, THEN SpecForge SHALL reject the request without publishing a numerical estimate.
>
> IF any referenced evidence record is missing, fails digest verification, is stale, invalidated, or superseded, is not linked to the target claim, has a mismatched scope, relies on assumptions not declared on the claim, lacks `source_revisions` provenance, is not completed, or has an inconclusive or invalid evaluation, THEN SpecForge SHALL reject the request with an unverified evidence error.
>
> IF any referenced evidence record uses formal verification or carries a formal outcome, THEN SpecForge SHALL reject the request with a formal-proof-disallowed error, and IF referenced evidence records combine heterogeneous verification methods, THEN SpecForge SHALL reject the request with a heterogeneous-evidence-disallowed error.

```gherkin
@SF-020 @v1
Feature: SF-020 — qualified statistical estimates require explicit sampling assumptions, homogeneous current evidence, and separation from formal proofs

  @one @witness
  Scenario: a Wilson score qualified estimate records full sampling metadata, bounds, disclaimer, and separation from formal proof
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    And a satisfied evidence record "EV-STAT-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-STAT-1" with 95 successes out of 100
    Then the statistical estimate succeeds for claim "CLM-MSG-001" in scope "single-broker"
    And the statistical estimate records Wilson score at 95 percent confidence with sample size 100, 95 successes, and 5 failures
    And the statistical estimate point estimate is within 0.001 of 0.950 with finite ordered bounds strictly inside 0.880 to 0.990
    And the statistical estimate preserves sampling metadata, disclaimer, and separation from formal proof
    And the statistical estimate round-trips through JSON losslessly
    When a statistical Wilson score estimate at 99 percent confidence is computed for claim "CLM-MSG-001" from "EV-STAT-1" with 100 successes out of 100
    Then the statistical estimate succeeds for claim "CLM-MSG-001" in scope "single-broker"
    And the statistical estimate records Wilson score at 99 percent confidence with sample size 100, 100 successes, and 0 failures
    And the statistical estimate has lower bound below 1.0 and upper bound equal to 1.0
    When a statistical Wilson score estimate at 90 percent confidence is computed for claim "CLM-MSG-001" from "EV-STAT-1" with 0 successes out of 10
    Then the statistical estimate succeeds for claim "CLM-MSG-001" in scope "single-broker"
    And the statistical estimate records Wilson score at 90 percent confidence with sample size 10, 0 successes, and 10 failures
    And the statistical estimate has lower bound equal to 0.0 and upper bound above 0.0

  @one @witness
  Scenario: a Beta-Binomial qualified estimate incorporates explicit priors and emits credible bounds
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    And a completed DST evidence record "EV-DST-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a statistical Beta-Binomial estimate with prior alpha 2.0 and beta 2.0 at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-DST-1" with 95 successes out of 100
    Then the statistical estimate succeeds for claim "CLM-MSG-001" in scope "single-broker"
    And the statistical estimate records Beta-Binomial prior 2.0 and 2.0 at 95 percent confidence
    And the statistical estimate point estimate is within 0.001 of 0.9327 with finite ordered bounds strictly inside 0.880 to 0.990
    And the statistical estimate preserves sampling metadata, disclaimer, and separation from formal proof
    And the statistical estimate limitations include "normal approximation to Beta posterior credible interval"
    And the statistical estimate round-trips through JSON losslessly

  @near-miss
  Scenario: missing assumptions, invalid priors, invalid samples, scope mismatch, and unknown claims publish no numerical estimate
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    And a satisfied evidence record "EV-STAT-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a statistical estimate is attempted for claim "CLM-MSG-001" from "EV-STAT-1" with missing "hypothesis"
    Then the statistical estimate fails with error kind "missing_assumptions"
    When a statistical estimate is attempted for claim "CLM-MSG-001" from "EV-STAT-1" with missing "sampling_population"
    Then the statistical estimate fails with error kind "missing_assumptions"
    When a statistical estimate is attempted for claim "CLM-MSG-001" from "EV-STAT-1" with missing "sampling_process"
    Then the statistical estimate fails with error kind "missing_assumptions"
    When a statistical estimate is attempted for claim "CLM-MSG-001" from "EV-STAT-1" with missing "scope"
    Then the statistical estimate fails with error kind "missing_assumptions"
    When a statistical estimate is attempted for claim "CLM-MSG-001" from "EV-STAT-1" with missing "independence_assumptions"
    Then the statistical estimate fails with error kind "missing_assumptions"
    When a statistical estimate is attempted for claim "CLM-MSG-001" from "EV-STAT-1" with missing "blank_independence_assumption"
    Then the statistical estimate fails with error kind "missing_assumptions"
    When a statistical estimate is attempted for claim "CLM-MSG-001" from "EV-STAT-1" with missing "limitations"
    Then the statistical estimate fails with error kind "missing_assumptions"
    When a statistical estimate is attempted for claim "CLM-MSG-001" from "EV-STAT-1" with missing "blank_limitation"
    Then the statistical estimate fails with error kind "missing_assumptions"
    When a statistical estimate is attempted for claim "CLM-MSG-001" from "EV-STAT-1" with model scope "multi-broker"
    Then the statistical estimate fails with error kind "scope_mismatch"
    When a statistical estimate is attempted for unknown claim "CLM-UNKNOWN-999" from "EV-STAT-1"
    Then the statistical estimate fails with error kind "unknown_claim"
    When a statistical Beta-Binomial estimate with invalid prior "zero_alpha" is attempted for claim "CLM-MSG-001" from "EV-STAT-1"
    Then the statistical estimate fails with error kind "invalid_model"
    When a statistical Beta-Binomial estimate with invalid prior "negative_beta" is attempted for claim "CLM-MSG-001" from "EV-STAT-1"
    Then the statistical estimate fails with error kind "invalid_model"
    When a statistical Beta-Binomial estimate with invalid prior "nan_alpha" is attempted for claim "CLM-MSG-001" from "EV-STAT-1"
    Then the statistical estimate fails with error kind "invalid_model"
    When a statistical Beta-Binomial estimate with invalid prior "infinite_beta" is attempted for claim "CLM-MSG-001" from "EV-STAT-1"
    Then the statistical estimate fails with error kind "invalid_model"
    When a statistical estimate is attempted for claim "CLM-MSG-001" with invalid sample "zero_size"
    Then the statistical estimate fails with error kind "invalid_sample"
    When a statistical estimate is attempted for claim "CLM-MSG-001" with invalid sample "mismatched_counts"
    Then the statistical estimate fails with error kind "invalid_sample"
    When a statistical estimate is attempted for claim "CLM-MSG-001" with invalid sample "empty_evidence"
    Then the statistical estimate fails with error kind "invalid_sample"
    When a statistical estimate is attempted for claim "CLM-MSG-001" with invalid sample "duplicate_evidence"
    Then the statistical estimate fails with error kind "invalid_sample"

  @near-miss
  Scenario: formal proof evidence and heterogeneous verification methods are disallowed while homogeneous evidence succeeds
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    And a satisfied evidence record "EV-PBT-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a satisfied evidence record "EV-PBT-2" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a completed DST evidence record "EV-DST-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a proved formal evidence record "EV-FORMAL-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a bounded-success formal evidence record "EV-FORMAL-2" with bound "depth <= 12" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-FORMAL-1" with 10 successes out of 10
    Then the statistical estimate fails with error kind "formal_proof_disallowed"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-FORMAL-2" with 10 successes out of 10
    Then the statistical estimate fails with error kind "formal_proof_disallowed"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-PBT-1, EV-DST-1" with 20 successes out of 20
    Then the statistical estimate fails with error kind "heterogeneous_evidence_disallowed"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-PBT-1, EV-PBT-2" with 19 successes out of 20
    Then the statistical estimate succeeds for claim "CLM-MSG-001" in scope "single-broker"

  @near-miss
  Scenario: missing, stale, invalidated, superseded, unlinked, incomplete, inconclusive, or scope-mismatched evidence is rejected
    Given a compiled snapshot from "sdv-assurance.md"
    And an empty evidence ledger
    And a satisfied evidence record "EV-STAT-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a satisfied evidence record "EV-STAT-2" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a satisfied evidence record "EV-STAT-3" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a satisfied evidence record "EV-OTHER-CLM" is appended for property "PROP-MSG-002" and claim "CLM-MSG-002"
    And a satisfied evidence record "EV-WRONG-SCOPE" in scope "multi-broker" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And an inconclusive evidence record "EV-INC-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And an incomplete run evidence record "EV-INCOMPLETE-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And a violated evidence record "EV-VIOL-STAT-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And an evidence record "EV-EXTRA-ASM-1" with undeclared assumption "ASM-EXTRA-999" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    And an evidence record "EV-NO-REV-1" with empty source revisions is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-MISSING-999" with 10 successes out of 10
    Then the statistical estimate fails with error kind "unverified_evidence"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-OTHER-CLM" with 10 successes out of 10
    Then the statistical estimate fails with error kind "unverified_evidence"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-WRONG-SCOPE" with 10 successes out of 10
    Then the statistical estimate fails with error kind "unverified_evidence"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-INC-1" with 10 successes out of 10
    Then the statistical estimate fails with error kind "unverified_evidence"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-INCOMPLETE-1" with 10 successes out of 10
    Then the statistical estimate fails with error kind "unverified_evidence"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-VIOL-STAT-1" with 100 successes out of 100
    Then the statistical estimate fails with error kind "invalid_sample"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-VIOL-STAT-1" with 95 successes out of 100
    Then the statistical estimate succeeds for claim "CLM-MSG-001" in scope "single-broker"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-EXTRA-ASM-1" with 10 successes out of 10
    Then the statistical estimate fails with error kind "unverified_evidence"
    When a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-NO-REV-1" with 10 successes out of 10
    Then the statistical estimate fails with error kind "unverified_evidence"
    When a link "invalidates" is appended from "EV-STAT-2" to "EV-STAT-1"
    And a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-STAT-1" with 10 successes out of 10
    Then the statistical estimate fails with error kind "unverified_evidence"
    When a link "supersedes" is appended from "EV-STAT-3" to "EV-STAT-2"
    And a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-STAT-2" with 10 successes out of 10
    Then the statistical estimate fails with error kind "unverified_evidence"
    Given a compiled snapshot from "sdv-message.md"
    And an empty evidence ledger
    And a satisfied evidence record "EV-STALE-1" is appended for property "PROP-MSG-001" and claim "CLM-MSG-001"
    When the snapshot changes to "sdv-message-property-edit.md"
    And a statistical Wilson score estimate at 95 percent confidence is computed for claim "CLM-MSG-001" from "EV-STALE-1" with 10 successes out of 10
    Then the statistical estimate fails with error kind "unverified_evidence"
```

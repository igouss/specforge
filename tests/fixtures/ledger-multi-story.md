# Ledger with multiple stories under one requirement

### SF-921 @ v1 — a transfer conserves credits

> WHEN credits are transferred, the ledger SHALL keep the total.

```gherkin
@SF-921 @v1
Feature: SF-921 — transfers move credits

  Scenario: a transfer moves credits
    Given an account with 100 credits
    When 60 credits are transferred
    Then the recipient holds 60
    And the total is 100
```

```gherkin
@SF-921 @v1
Feature: SF-921 — idle accounts conserve credits

  Scenario: an idle account keeps its credits
    Given an account with 100 credits
    Then the recipient holds 0
    And the total is 100
```

# Ledger with background and rules

### SF-921 @ v1 — a transfer conserves credits

> WHEN credits are transferred, the ledger SHALL keep the total.

```gherkin
@SF-921 @v1
Feature: SF-921 — a transfer conserves credits

  Background:
    Given an account with 100 credits

  Scenario: a top-level check keeps the total
    Then the recipient holds 0
    And the total is 100

  Rule: normal transfers

    Background:
      When 60 credits are transferred

    Scenario: a transfer moves credits
      Then the recipient holds 60
      And the total is 100

  Rule: idle accounts

    Scenario: an idle account keeps its credits
      Then the recipient holds 0
      And the total is 100

  Rule: partitioned transfers

    Scenario: a transfer during a partition is retried
      When the network is partitioned
      Then the total is 100
```

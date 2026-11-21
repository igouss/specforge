# Ledger

### SF-921 @ v1 — a transfer conserves credits

> WHEN credits are transferred, the ledger SHALL keep the total.

```gherkin
@SF-921 @v1
Feature: SF-921 — a transfer conserves credits

  Scenario: a transfer moves credits
    Given an account with 100 credits
    When 60 credits are transferred
    Then the total is 100

  Scenario: a transfer during a partition is retried
    Given an account with 100 credits
    When the network is partitioned
    Then the total is 100

  Scenario Outline: a transfer in <currency>
    Given an account with 100 credits
    When 60 credits are transferred
    Then the total is 100

    Examples:
      | currency |
      | credits  |
```

# Outline

### SF-924 @ v1 — a transfer in any currency

> WHEN a transfer is made in a currency, the ledger SHALL keep the total.

```gherkin
@SF-924 @v1
Feature: SF-924 — a transfer in any currency

  Scenario Outline: a transfer in <currency>
    Given an account with 100 credits
    When 60 credits are transferred
    Then the total is 100

    Examples:
      | currency |
      | credits  |
```

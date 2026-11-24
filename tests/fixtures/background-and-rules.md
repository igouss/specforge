# Background and rules

### SF-934 @ v1 — transfers obey ledger rules

> WHEN credits are transferred under ledger rules, the ledger SHALL keep the total.

```gherkin
@SF-934 @v1
Feature: SF-934 — transfers obey ledger rules

  Background:
    Given an account with 100 credits

  @top
  Scenario: a top-level transfer
    When 60 credits are transferred
    Then the total is 100

  Rule: fee-free transfers

    Background:
      Given transfers carry no fee

    @rule-one
    Scenario: a fee-free transfer
      When 60 credits are transferred
      Then the total is 100

  Rule: audited transfers

    Background:
      Given an audit log is open

    Scenario: an audited transfer
      Then the total is 100
```

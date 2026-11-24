# Repeated scenario across rules

### SF-935 @ v1 — scenario names remain unique across rules

> WHEN rules group scenarios, each scenario name SHALL be unique within its requirement.

```gherkin
@SF-935 @v1
Feature: SF-935 — scenario names remain unique across rules

  Rule: first rule

    Scenario: a transfer
      Given an account with 100 credits

  Rule: second rule

    Scenario: a transfer
      Then the total is 100
```

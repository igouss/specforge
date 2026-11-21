# Repeated

A requirement whose story names one scenario twice.

The defect names the requirement's heading, below.

### SF-914 @ v1 — scenario names are unique

> WHEN a story is written, each scenario name SHALL be unique within its requirement.

```gherkin
@SF-914 @v1
Feature: SF-914 — scenario names are unique

  Scenario: a transfer
    Given an account with 100 credits
    When 60 credits are transferred
```

```gherkin
@SF-914 @v1
Feature: SF-914 — the same name again

  Scenario: a transfer
    Given an account with 100 credits
    Then the total is 100
```

# Outline inside a rule

### SF-936 @ v1 — outlines inside rules are refused

> WHEN a rule holds a scenario outline, SpecForge SHALL report an outline defect.

```gherkin
@SF-936 @v1
Feature: SF-936 — outlines inside rules are refused

  Rule: multi-currency transfers

    Scenario Outline: a transfer in <currency>
      Given an account with 100 credits
      When 60 credits are transferred
      Then the total is 100

      Examples:
        | currency |
        | credits  |
```

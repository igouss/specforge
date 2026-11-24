# Stepless scenario inside a rule

### SF-937 @ v1 — scenarios inside rules carry steps

> WHEN a rule holds a scenario with no steps and no background steps, SpecForge SHALL report a stepless defect.

```gherkin
@SF-937 @v1
Feature: SF-937 — scenarios inside rules carry steps

  Rule: empty rule scenarios

    Scenario: nothing happens in this rule
```

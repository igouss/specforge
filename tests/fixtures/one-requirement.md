# One requirement

### SF-901 @ v1 — an admission mints an identity

> WHEN an item is admitted,
> the factory SHALL mint an identity.

```gherkin
@SF-901 @v1
Feature: SF-901 — an admission mints an identity

  @one @witness
  Scenario: one admission mints one identity
    Given an empty factory
    When an item is admitted
    Then one identity exists
```

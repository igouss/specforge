# Misplaced

### SF-908 @ v1 — a story sits under its heading

> WHEN a story is written, it SHALL sit under its requirement's heading.

### SF-907 @ v1 — another requirement

> WHEN another requirement is declared, it SHALL hold only its own stories.

```gherkin
@SF-908 @v1
Feature: SF-908 — a story sits under its heading

  Scenario: a misplaced scenario
    Given an account with 100 credits
```

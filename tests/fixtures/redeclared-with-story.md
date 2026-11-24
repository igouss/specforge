# Redeclared with story

### SF-933 @ v1 — an id is declared once

> WHEN an id is declared, it SHALL be declared once.

### SF-933 @ v1 — an id declared again

> WHEN an id is declared again, a story under it SHALL not also be misplaced.

```gherkin
@SF-933 @v1
Feature: SF-933 — an id declared again

  Scenario: the story sits under the second heading
    Given a duplicate requirement
    When it is compiled
    Then only the redeclaration is reported
```

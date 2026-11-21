# Two requirements

### SF-902 @ v1 — a transfer moves credits

> WHEN a transfer is requested, the ledger SHALL move the credits or refuse the transfer.

```gherkin
@SF-902 @v1
Feature: SF-902 — a transfer moves credits

  Scenario: a first transfer
    Given an account with 100 credits
    When 60 credits are transferred
    Then the total is 100

  Scenario: a second transfer
    Given an account with 100 credits
    When 60 credits are transferred
    Then the total is 100
```

A second story for the same requirement.

```gherkin
@SF-902 @v1
Feature: SF-902 — a transfer is refused

  Scenario: a refused transfer
    Given an account with 100 credits
    When 160 credits are transferred
    Then the total is 100
```

### SF-903 @ v1 — a balance is never negative

> The ledger SHALL never hold a negative balance.

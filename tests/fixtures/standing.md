# Standing

### SF-911 @ v1 — transfers conserve credits

> WHEN credits are transferred, the ledger SHALL keep the total.

```gherkin
@SF-911 @v1
Feature: SF-911 — transfers conserve credits

  Scenario: a transfer moves credits
    Given an account with 100 credits
    When 60 credits are transferred
    Then the total is 100

  Scenario: a transfer during a partition is retried
    Given an account with 100 credits
    When the network is partitioned
    Then the total is 100
```

### SF-912 @ v1 — a restart loses nothing

> WHEN a node restarts, the ledger SHALL keep the total.

```gherkin
@SF-912 @v1
Feature: SF-912 — a restart loses nothing

  Scenario: a node restarts
    Given an account with 100 credits
    When the node restarts
    Then the total is 100
```

### SF-913 @ v1 — an audit replays the ledger

> WHEN an audit is requested, the ledger SHALL replay every transfer.

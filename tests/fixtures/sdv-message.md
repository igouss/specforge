# Message broker verification specification

### REQ-001 @ v1 — acknowledged messages are not redelivered

> WHEN a message has been acknowledged, the broker SHALL NOT redeliver that message.

```specforge
kind: assumption
id: ASM-MSG-001
statement: Broker durable storage preserves acknowledgement records across restarts.
scope: single-broker
```

```specforge
kind: state_machine
id: SM-MSG-001
requirements:
  - REQ-001
initial: queued
states:
  - queued
  - delivered
  - acknowledged
  - redelivered
transitions:
  - name: deliver
    from: queued
    to: delivered
    event: deliver
  - name: acknowledge
    from: delivered
    to: acknowledged
    event: ack
  - name: redeliver
    from: acknowledged
    to: redelivered
    event: redeliver
```

```specforge
kind: property
id: PROP-MSG-001
name: acknowledged-message-not-redelivered
property_kind: safety
subject: SM-MSG-001
predicate:
  language: trace
  expression: "never(state == redelivered)"
scope: single-broker
assumptions:
  - ASM-MSG-001
requirements:
  - REQ-001
methods:
  - model_based
  - property_based
```

```specforge
kind: claim
id: CLM-MSG-001
proposition: No acknowledged message is redelivered within single-broker scope.
scope: single-broker
assumptions:
  - ASM-MSG-001
properties:
  - PROP-MSG-001
requirements:
  - REQ-001
```

```specforge
kind: plan
id: PLAN-MSG-001
targets:
  - PROP-MSG-001
  - CLM-MSG-001
scope: single-broker
assumptions:
  - ASM-MSG-001
activities:
  - id: ACT-PBT-001
    method: property_based
    budget: 32
```

### REQ-002 @ v1 — unrelated status endpoint reports health

> WHEN a client queries health, the broker SHALL return ok.

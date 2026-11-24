# Message broker assurance specification

### REQ-001 @ v1 — acknowledged messages are not redelivered

> WHEN a message has been acknowledged, the broker SHALL NOT redeliver that message.

```specforge
kind: assumption
id: ASM-MSG-001
statement: Broker durable storage preserves acknowledgement records across restarts.
scope: single-broker
```

```specforge
kind: assumption
id: ASM-EXTRA-001
statement: Network links never reorder packets between broker replicas.
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
  - deterministic_simulation
  - formal
```

```specforge
kind: property
id: PROP-MSG-002
name: redeliver-event-never-emitted
property_kind: safety
subject: SM-MSG-001
predicate:
  language: trace
  expression: "never(event == redeliver)"
scope: single-broker
assumptions:
  - ASM-MSG-001
requirements:
  - REQ-001
methods:
  - deterministic_simulation
  - formal
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
kind: claim
id: CLM-MSG-002
proposition: No redeliver event is emitted within single-broker scope.
scope: single-broker
assumptions:
  - ASM-MSG-001
properties:
  - PROP-MSG-002
requirements:
  - REQ-001
```

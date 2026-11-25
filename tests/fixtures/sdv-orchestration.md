# Orchestration verification specification

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
  - property_based
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
kind: plan
id: PLAN-ORCH-001
targets:
  - PROP-MSG-001
  - CLM-MSG-001
scope: single-broker
assumptions:
  - ASM-MSG-001
activities:
  - id: ACT-PBT-001
    method: property_based
    budget: 16
  - id: ACT-DST-001
    method: deterministic_simulation
    budget: 8
required_evidence:
  - property_based
  - deterministic_simulation
```

```specforge
kind: plan
id: PLAN-MISSING-001
targets:
  - PROP-MSG-001
  - CLM-MSG-001
scope: single-broker
assumptions:
  - ASM-MSG-001
activities:
  - id: ACT-PBT-002
    method: property_based
    budget: 8
  - id: ACT-FORMAL-001
    method: formal
    budget: 1
required_evidence:
  - property_based
  - formal
```

```specforge
kind: property
id: PROP-MSG-002
name: never-delivered-sentinel
property_kind: safety
subject: SM-MSG-001
predicate:
  language: trace
  expression: "never(state == delivered)"
scope: single-broker
assumptions:
  - ASM-MSG-001
requirements:
  - REQ-001
methods:
  - property_based
```

```specforge
kind: plan
id: PLAN-MULTI-001
targets:
  - PROP-MSG-001
  - PROP-MSG-002
scope: single-broker
assumptions:
  - ASM-MSG-001
activities:
  - id: ACT-PBT-003
    method: property_based
    budget: 8
required_evidence:
  - property_based
```

```specforge
kind: plan
id: PLAN-EMPTY-001
targets: []
scope: single-broker
assumptions:
  - ASM-MSG-001
activities:
  - id: ACT-EMPTY-001
    method: property_based
    budget: 4
```

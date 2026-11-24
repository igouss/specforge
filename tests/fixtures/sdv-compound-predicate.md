# Compound predicate verification specification

### REQ-001 @ v1 — compound trace predicates are not silently accepted

> WHEN a trace predicate uses an unsupported compound expression, evaluation SHALL return invalid_evaluation.

```specforge
kind: state_machine
id: SM-CMP-001
requirements:
  - REQ-001
initial: queued
states:
  - queued
  - delivered
transitions:
  - name: deliver
    from: queued
    to: delivered
    event: deliver
```

```specforge
kind: property
id: PROP-CMP-001
name: compound-safety-check
property_kind: safety
subject: SM-CMP-001
predicate:
  language: trace
  expression: "never(state == delivered && event == deliver)"
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

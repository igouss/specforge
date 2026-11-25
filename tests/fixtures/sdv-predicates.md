# Predicate evaluation verification specification

### REQ-001 @ v1 — trace predicates cover states, quotes, inputs, and invalid forms

> WHEN safety and state-invariant properties are evaluated over execution traces, the evaluator SHALL check quoted values, state invariants, input constraints, and reject malformed or unsupported forms.

```specforge
kind: state_machine
id: SM-PRED-001
requirements:
  - REQ-001
initial: queued
states:
  - queued
  - delivered
  - acknowledged
  - redelivered
transitions:
  - name: stay
    from: queued
    to: queued
    event: tick
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
id: PROP-PRED-001
name: quoted-negated-state
property_kind: safety
subject: SM-PRED-001
predicate:
  language: trace
  expression: 'always(state != "redelivered")'
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

```specforge
kind: property
id: PROP-PRED-002
name: single-quoted-never-state
property_kind: safety
subject: SM-PRED-001
predicate:
  language: trace
  expression: "never(state == 'redelivered')"
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

```specforge
kind: property
id: PROP-PRED-003
name: always-queued-invariant
property_kind: state_invariant
subject: SM-PRED-001
predicate:
  language: trace
  expression: "always(state == queued)"
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

```specforge
kind: property
id: PROP-PRED-004
name: always-delivered-invariant
property_kind: state_invariant
subject: SM-PRED-001
predicate:
  language: trace
  expression: "always(state == delivered)"
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

```specforge
kind: property
id: PROP-PRED-005
name: strict-input-mode
property_kind: safety
subject: SM-PRED-001
predicate:
  language: trace
  expression: "always(input.mode == strict)"
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

```specforge
kind: property
id: PROP-PRED-006
name: unterminated-quote
property_kind: safety
subject: SM-PRED-001
predicate:
  language: trace
  expression: 'never(state == "redelivered)'
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

```specforge
kind: property
id: PROP-PRED-007
name: trailing-tokens-after-quote
property_kind: safety
subject: SM-PRED-001
predicate:
  language: trace
  expression: 'never(state == "redelivered" extra)'
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

```specforge
kind: property
id: PROP-PRED-008
name: invalid-always-not-equal-lhs
property_kind: safety
subject: SM-PRED-001
predicate:
  language: trace
  expression: "always(event != redeliver)"
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

```specforge
kind: property
id: PROP-PRED-009
name: unsupported-predicate-language
property_kind: safety
subject: SM-PRED-001
predicate:
  language: prose
  expression: "never(state == redelivered)"
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

```specforge
kind: property
id: PROP-PRED-010
name: unsupported-property-kind
property_kind: liveness
subject: SM-PRED-001
predicate:
  language: trace
  expression: "never(state == redelivered)"
scope: single-broker
requirements:
  - REQ-001
methods:
  - model_based
```

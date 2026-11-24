# Defective verification artifacts

### REQ-001 @ v1 — broken links

> WHEN references dangle, compilation SHALL refuse the snapshot.

```specforge
kind: state_machine
id: SM-BAD-001
requirements:
  - REQ-001
initial: start
states:
  - start
transitions:
  - name: vanish
    from: start
    to: ghost
    event: step
```

```specforge
kind: property
id: PROP-BAD-001
name: bad-property
property_kind: safety
subject: SM-BAD-001
predicate:
  language: trace
  expression: "never(state == broken)"
scope: unit
assumptions:
  - ASM-MISSING-999
requirements:
  - REQ-MISSING-999
methods:
  - property_based
```

# Undeclared requirement reference

### REQ-001 @ v1 — undeclared requirements are rejected

> WHEN an artifact references a syntactically valid requirement that is not declared, compilation SHALL report an unknown requirement defect.

```specforge
kind: state_machine
id: SM-REQ-001
requirements:
  - REQ-999
initial: idle
states:
  - idle
```

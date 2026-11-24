# Unknown field typo in specforge block

### REQ-001 @ v1 — unknown fields are rejected

> WHEN a specforge block carries an unknown field, compilation SHALL reject it.

```specforge
kind: assumption
id: ASM-TYPO-001
statement: Durable storage preserves acknowledgements.
scope: single-broker
assumpitons:
  - ASM-MISSING-001
```

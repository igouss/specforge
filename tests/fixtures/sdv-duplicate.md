# Duplicate artifact identifiers

### REQ-001 @ v1 — duplicate ids are rejected

> WHEN an artifact id is declared twice, compilation SHALL refuse the snapshot.

```specforge
kind: assumption
id: ASM-DUP-001
statement: First declaration.
scope: local
```

```specforge
kind: assumption
id: ASM-DUP-001
statement: Second declaration.
scope: local
```

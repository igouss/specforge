# Malformed artifact identifiers

### REQ-001 @ v1 — malformed artifact ids are rejected

> WHEN an artifact declares a malformed identifier, compilation SHALL report an invalid identifier defect.

```specforge
kind: assumption
id: "123"
statement: Missing prefix segment.
scope: local
```

```specforge
kind: assumption
id: ASM-
statement: Missing numeric suffix.
scope: local
```

```specforge
kind: assumption
id: ASM-abc
statement: Non-numeric suffix.
scope: local
```

```specforge
kind: assumption
id: asm-001
statement: Lowercase prefix segment.
scope: local
```

```specforge
kind: assumption
id: ASM--001
statement: Empty prefix segment.
scope: local
```

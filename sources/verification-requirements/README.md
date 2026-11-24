# SpecForge — Specification-Driven Verification Extension

**Status:** Draft for implementation planning  
**Package type:** Requirements and design handoff  
**Target:** Extend the existing SpecForge project; do not create a separate product or replace its existing specification engine.

## Purpose

This package defines an implementation-ready requirements baseline for adding an integrated verification capability to SpecForge. The capability connects:

1. Specification Engineering
2. Model-Based Testing (MBT)
3. Property-Based Testing (PBT)
4. Deterministic Simulation Testing (DST)
5. Formal Verification
6. Evidence Engineering and Assurance Assessment

The central idea is that the **Specification Graph remains the canonical integration model**, while specialized tools perform generation, simulation, checking, and proof. SpecForge records the resulting evidence, preserves provenance, evaluates its relevance to verification claims, and uses findings to guide further engineering work.

## Start here

Recommended reading order:

1. `planning/IMPLEMENTATION_HANDOFF.md`
2. `requirements/REQUIREMENTS.md`
3. `specifications/EARS_REQUIREMENTS.md`
4. `specifications/ACCEPTANCE_FEATURES.feature`
5. `architecture/ARCHITECTURE.md`
6. `domain/DOMAIN_MODEL.md`
7. `verification/VERIFICATION_AND_EVIDENCE.md`
8. `planning/ROADMAP.md`
9. `decisions/ARCHITECTURAL_DECISIONS.md`

## Normative language

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are to be interpreted as requirement keywords. Requirements have stable IDs. An implementation agent should preserve these IDs when creating issues, tests, and implementation artifacts.

## Key architectural constraints

- Git remains the source of truth for authored project definitions.
- The existing SpecForge Specification Graph remains the canonical typed integration model.
- Verification inputs MUST be bound to immutable specification/model revisions.
- External verification engines are integrated through adapters; SpecForge does not initially reimplement them.
- Evidence records are append-only and provenance-preserving.
- Evidence assessment is distinct from evidence storage.
- Probabilistic estimates are optional, explicitly qualified, and never substituted for proof or deterministic facts.
- A counterexample is a first-class artifact and a trigger for diagnosis, not an instruction to automatically change requirements.
- A result of `inconclusive` MUST NOT be represented as `verified` or `passed`.

## Scope note

This package defines the extension's requirements and proposed architecture. It does not assume a particular database, programming language, MBT framework, DST framework, model checker, or statistical inference library. Those choices should be reconciled with the existing SpecForge codebase before implementation.

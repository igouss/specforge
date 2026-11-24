# Implementation Agent Handoff

## Mission

Implement a **Specification-Driven Verification** capability as an extension to the existing SpecForge project.

Do not start by building a new test runner, simulation engine, formal language, or evidence platform from scratch. First inspect the current repository, its existing Specification Language and Meta-Model, Specification Graph, IDs, persistence strategy, CLI/API conventions, and test infrastructure. Map this requirements package onto existing concepts and reuse them wherever possible.

## Expected outcome

A user should be able to:

1. Define a requirement and a behavioral model in the existing SpecForge authoring workflow.
2. Define a property/invariant and link it to the relevant requirement and model elements.
3. define a verification claim and a verification plan.
4. Generate at least one scenario from a model, or import one from an adapter.
5. Execute a property check using an integrated test engine.
6. capture the execution result and, on failure, a reproducible counterexample.
7. Persist an immutable evidence record with provenance.
8. Assess whether the evidence supports, contradicts, or is insufficient for the claim.
9. Identify evidence made stale by changes to linked source artifacts.
10. Replay a previously captured counterexample.

## First task: repository reconnaissance

Before coding, report:

- Existing project structure and architectural boundaries.
- Existing Specification Graph types and relationship semantics.
- Existing source formats and parser/compiler pipeline.
- Existing stable-ID and revision/version conventions.
- Existing plugin/extension or adapter mechanisms.
- Existing persistence and Git integration.
- Existing CLI/API and test conventions.
- Which requirements are already satisfied by existing functionality.
- Conflicts, missing abstractions, and proposed mappings.

Do not duplicate an existing capability under a new name.

## Implementation rules

- Preserve current project conventions unless there is a documented reason to change them.
- Prefer vertical slices over broad scaffolding.
- Keep external tool-specific models behind adapters.
- Do not make an external tool's schema the canonical SpecForge schema.
- Make all runs reproducible to the extent supported by the engine; record limitations explicitly.
- Never claim that finite testing proves universal correctness.
- Never convert a formal proof status into a probability score.
- Do not silently change specifications, properties, assumptions, or acceptance criteria based on an AI diagnosis.
- Require an explicit engineering decision for requirement/model changes proposed from counterexamples.
- Add tests for every implemented requirement and maintain traceability from requirement to test/evidence.
- Treat imported tool output as untrusted data; validate it before accepting it into the graph or ledger.

## Definition of done for the first vertical slice

The first milestone is complete when an automated end-to-end test demonstrates:

- One requirement in the Specification Graph.
- One state machine or equivalent behavioral model.
- One safety property linked to the model.
- One generated or imported scenario.
- One execution through a PBT/MBT-compatible runner (DST is not required for this first slice).
- One property evaluation.
- One immutable evidence record with source revision, engine identity/version, input/scenario reference, and result.
- A verification claim linked to the evidence.
- A demonstrated stale-evidence result after a relevant source change.
- A failure case that creates a counterexample artifact and can be replayed if the engine supports replay.

## Deliverables expected from the implementation agent

1. Repository reconnaissance report.
2. Proposed mapping from this requirements package to existing SpecForge abstractions.
3. Architecture delta and any necessary ADRs.
4. Incremental implementation plan with dependencies.
5. Implemented vertical slice.
6. Automated tests and traceability matrix.
7. Documentation for adding another engine adapter.
8. Explicit list of deferred requirements and limitations.

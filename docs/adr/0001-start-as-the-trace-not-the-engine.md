# 0001. SpecForge starts as the requirement trace, not the simulation engine

Date: 2026-11-21

## Operator decision

Recorded verbatim in [docs/strategy.md](../strategy.md).

## Weakest part at the time

Nothing existed here but a design for sixteen graph entities across six contexts. The first consumer, Symphony, already ran Gherkin stories under a seeded simulation, declared the scope of that run by hand beside the step definitions that decide it, never compared the ids of its invariant enum with the specifications, and had no view of the requirements not yet started.

## Forces in conflict

- The design wants specifications to be the primary interface, which argues for the whole graph; Symphony's own history is four factories that each built a runtime before a consumer needed one.
- The design's simulation context models nodes, a network and storage; the first consumer has one store and a mailbox, and would use none of it.
- A consumer should write only glue, yet step definitions are cucumber's, so the bridge to cucumber has to live with the trace or be rewritten by every consumer.

## Move taken

One crate with a pure core (`compile`, `assess`, `drawn`) and a `cucumber` feature (`stories`) that reads a project from disk and runs it. Five kinds of thing in the trace: requirement, story, scenario, pending entry, monitored id. The view is committed by the consumer and compared on every run, so a requirement that loses its bindings shows as a diff.

## What it preserved

The design's separations that the code can already hold: a specification's validity (defects), its executability (standing and findings) and the outcome of a run are three results, never one. The vocabulary of the export where it applies.

## What would reverse it

- A consumer whose stories need a simulated network, faults or several nodes that it cannot express on its own harness brings the simulation context in.
- A second document format turns the one parser into a seam.
- Symphony's gates slice calling for a verdict pinned to a specification version brings the content hash and the evidence record.

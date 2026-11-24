# Strategy

How SpecForge gets built. The design conversation it started from is archived in [sources/chatgpt-export](../sources/chatgpt-export/conversation.md).

## Operator decision

2026-11-21, verbatim, in reply to the recommendation to start SpecForge as the requirement trace that Symphony's harness lacks:

> Yes, addressing those diagnostic issues and building tools and making changes and fixes is a priority. Don't over-do testing, just pin the expected behaviour to act as executable specification, it's should be done on interface/trait/gherkin story levels. Project is my current priority project. Build this without overcomplicating the design and the implementation. you would do strategic thinking and delegate the tactical implementation to subagents.The objective is to optimize my flow.
>
> plan on strategic level and build this (delegate tactical work to sonnet, opus and haiku team agents based on which one is better suited for the task,  the harness blocks report files from subagents - subagents should return findings as text), you do the orchestration, set-up the git, workspace, prepare to run this project to completion (set-up instructions for future sessions)), set-up an environemt that you can run and test in an environment isolated from this, optimize the process for performace, we want quick feedback, I want to reuse existing opensource components, frameworks and libraries, and I only write the glue code to make this project happen

This approves: building SpecForge as its own repository with Symphony as first consumer; changing Symphony's story runner, seed summary, recipes and views to consume it, including the one new dependency (SpecForge) there; dependencies on existing open-source components, listed below; tests only at interface and Gherkin-story level. It approves nothing else. A weakened gate, a removed test, or a policy the defaults below do not cover goes back to the operator.

2026-11-21, later the same day, verbatim:

> Alright, let's publish this as a public GitHub repository. Make sure that this code is reusable for other projects. Readme that AI agents are welcome to make changes when working with this project. It's still work in progress, so they're welcome to adapt it to their needs.

This approves: publishing this repository publicly at github.com/igouss/specforge; Symphony taking it from there; an open invitation to AI agents to change and adapt it. It replaces assumed default D5.

2026-11-23: approved the Phase 0 mapping of the [Specification-Driven Verification handoff](../sources/verification-requirements/README.md) ([ADR 0002](adr/0002-specification-driven-verification.md)): build Phase 1 + Phase 2 end-to-end; extend `specs/*.md` with typed `specforge` fenced YAML blocks; power the first vertical slice with a pure Rust state-machine + trace property evaluator, a `proptest` PBT adapter, and a Cucumber adapter wrapper; persist the append-only Evidence Ledger as canonical JSONL + content-addressed blobs behind a feature flag while keeping the core `wasm32`-clean.

## Diagnosis

The weakest part is in the first consumer. Symphony runs Gherkin stories under a seeded simulation, and its runner declared its own scope by hand: two constants named the documents and the six requirements to run, beside the step definitions that already decide it. That runner checked the constants against the documents, the pending table and skipped steps; it did not check the ids its invariant enum displays against the specifications, it repeated those ids in a summary line, a replay line and a recipe's doc string, and it showed nothing about the forty-five requirements whose stories wait. The unresolved force is Symphony's gates slice, which must extract and run a managed project's stories and pin a verdict to a specification version: that needs the trace as data, for more than one project. The smallest move is a library that compiles the specifications into that trace, assesses it against the step bindings, and runs the stories from it.

## Assumed defaults

Each is the builder's choice, not the operator's statement, and names what would reverse it.

| Id | Default | Reverses if |
|---|---|---|
| D1 | The first consumer is Symphony's own story and seed harness, through a library. | The operator means the factory gating managed projects first: then a command-line tool and an evidence format lead. |
| D2 | One crate. The cucumber bridge is the feature `cucumber`, not a second crate. | A consumer needs the trace where cucumber cannot build. |
| D3 | Markdown (`specs/*.md`) is the single authoring format: `### <ID> @ v<N> — <title>` headings, blockquote statements, fenced `gherkin` blocks tagged `@<ID>`, and fenced `specforge` YAML blocks for typed verification artifacts (`assumption`, `state_machine`, `property`, `claim`, `plan`). | A consumer's artifacts outgrow inline Markdown blocks and require companion YAML files. |
| D4 | **Settled (2026-11-24, `sf-6y7`)**: a requirement with a story but no monitored invariant is shown in the view as `no` and is not a finding. | Only if a future consumer requires mandatory sweep invariants on every bound requirement. |
| D7 | A Scenario Outline is refused as a defect: cucumber expands and renames its rows before the run is filtered, so the trace cannot say which of them ran. | A consumer writes one: then SF-002 and SF-006 specify how its rows stand and run. |
| D6 | The design export's simulation engine, state machines, fault models, environments, execution planner and YAML ontology are not built. | A consumer exists whose stories need nodes, a network or injected faults that it cannot write on its own harness. |

## What is reused

`gherkin` (the parser cucumber itself uses), `pulldown-cmark` (Markdown), `cucumber` (step definitions and the run), `serde` + `serde_json` + `serde_norway` (typed `specforge` blocks and canonical JSON serialization), `sha2` + `hex` (content digests), `proptest` (optional PBT adapter with seeded execution and shrinking). Gate shape, toolchain and lints come from Symphony.

## Slices

| Slice | Outcome | Requirements |
|---|---|---|
| 1 Trace | Documents compile to a trace; an assessment joins it with bindings, pending entries and monitored ids; the view is drawn; a project's stories run in one call. SpecForge runs its own stories this way. | SF-001 to SF-006 |
| 2 Symphony | Symphony's story runner is glue over SpecForge; its hand lists are gone; its trace view is committed and checked. | Symphony ADR 0010 |
| 3 Verification (Phase 1 + 2) | Revision-bound `GraphSnapshot` with SHA-256 digests, typed verification artifacts in `specs/*.md`, append-only Evidence Ledger (pure core + JSONL/blob persistence), freshness/staleness impact and claim assessment, pure Rust state-machine scenario generator + trace safety evaluator, `proptest` PBT adapter with replayable shrunk counterexamples, and Cucumber evidence adapter. | SF-007 to SF-011 |
| later | CLI tool, DST and formal verification adapters, assurance argument export, qualified statistical models. | none yet |

## Risks

- The trace says a scenario is bound and ran; it does not say the step definitions assert what the sentence says.
- The repository is public and invites change, so nothing in it may carry a consumer's private detail: specifications, fixtures and records name only what any project could read.

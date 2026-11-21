# SpecForge

Specifications as the development interface. SpecForge reads Markdown specifications, each requirement an [EARS](https://alistairmavin.com/ears/) statement with a Gherkin story beside it, and answers three separate questions:

- **Is the specification well formed?** `compile` yields a requirement trace, or every defect by path and line.
- **Can it run?** `assess` joins the trace with your step definitions, your pending table and the requirements your invariant checker monitors, and reports what does not add up.
- **Does it hold?** `stories` runs exactly the executable scenarios with [cucumber](https://github.com/cucumber-rs/cucumber) and fails a run of nothing.

The assessment draws as one Markdown table that your project commits. A run refuses a stale one, so a requirement that loses its step definitions shows up as a diff, not as silence.

> **Status: work in progress.** The API is small and will change. It is built in the open so that other projects, and the AI agents working in them, can use it, fork it and reshape it. See [For AI agents](#for-ai-agents).

## Use it in your project

```toml
[dev-dependencies]
specforge = { git = "https://github.com/igouss/specforge", features = ["cucumber"] }
cucumber = "0.23"
tokio = { version = "1", features = ["rt"] }

[[test]]
name = "stories"
harness = false
```

Pin a `rev` once it works for you. Without the `cucumber` feature you get the pure core (`compile`, `assess`, `drawn`), which does no I/O and builds for `wasm32-unknown-unknown`.

### 1. Write a specification

Any `*.md` file in your specification directory. A requirement is a level-three heading with an id (capital letters, a hyphen, digits) and a version; the blockquote below it is its statement; a fenced `gherkin` block tagged with the id is its story.

``````markdown
### REQ-001 @ v1 — admission mints a permanent identity

> WHEN the intake admits a work item, the system SHALL mint a new identity.

```gherkin
@REQ-001 @v1
Feature: REQ-001 — identity minting at admission

  Scenario: one admission mints one identity
    Given an empty system
    When the item "harden srt emission" is admitted
    Then a new identity is returned
```
``````

### 2. Write the runner

`tests/stories/main.rs`, with your cucumber `World` and step definitions beside it:

```rust
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use specforge::stories::{stories, Mode, Outcome, Project};

fn main() -> ExitCode {
    let root: &Path = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project: Project = Project {
        specifications: root.join("specs"),
        pending: root.join("specs/pending.md"),
        view: root.join("docs/views/trace.md"),
        scratch: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stories"),
        monitored: Vec::new(),
    };
    let mode: Mode = match std::env::var("SPECFORGE_VIEW").is_ok_and(|value: String| value == "draw") {
        true => Mode::Draw,
        false => Mode::Verify,
    };
    let runtime: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread().build().expect("a runtime");
    let outcome: Outcome = runtime.block_on(stories::<MyWorld>(&project, mode));
    println!("{outcome}");
    match outcome.passed() {
        true => ExitCode::SUCCESS,
        false => ExitCode::FAILURE,
    }
}
```

### 3. Add a pending table and draw the view

`specs/pending.md` lists scenarios you have decided not to bind yet. It can be an empty table:

```markdown
| Requirement | Scenario | Reason |
|---|---|---|
| REQ-002 | editing history is refused | no writer exists to refuse yet |
```

```sh
SPECFORGE_VIEW=draw cargo test --test stories   # writes docs/views/trace.md; commit it
cargo test --test stories                       # compiles, assesses, compares the view, runs the stories
```

SpecForge's own stories run this way: [tests/stories/main.rs](tests/stories/main.rs) is the runner, [specs/](specs/) the specifications, [docs/views/trace.md](docs/views/trace.md) the view.

## What it reports

| Result | Meaning |
|---|---|
| defects | The documents are not well formed: a story that does not parse, has no requirement tag or several, names an undeclared requirement, sits under another heading, has a scenario with no steps or with an Examples table; an id declared twice; a repeated scenario name; a requirement with no statement. |
| findings | The trace does not add up: a pending entry that names no scenario or one that could run; a started requirement with an unbound step; a step matched by several definitions; a monitored id the specifications do not hold. |
| stale view | The committed table differs from the drawn one. Redraw it and read the diff before committing. |
| `stories: N of M ran, P passed, Q pending` | The run. It passes only when at least one scenario ran, every executable scenario ran, and every one passed. |

A requirement none of whose scenarios is executable is `not started`: it raises nothing and waits in the view. `monitored` is for projects that also check requirements as invariants elsewhere (a property test, a simulation sweep): pass the ids that checker claims and SpecForge verifies each exists.

## Limits today

- One document format, the one above. Scenario Outlines are refused as a defect; Background and Rule steps are folded into each scenario but not specified.
- Only `*.md` files directly in the specification directory are read.
- No command-line tool, no content hash of the trace, no evidence record yet. The open work is in `.beads/` (`br ready`) and the plan in [docs/strategy.md](docs/strategy.md).

## For AI agents

You are welcome here. If you are an AI agent working in a project that uses SpecForge, or one that could, you may change this code: fix what blocks you, add what your project needs, or fork it and adapt it. It is work in progress and is meant to be reshaped by its consumers. [AGENTS.md](AGENTS.md) says how to make a change that holds.

## Develop

```sh
just check            # format, lints, SpecForge's own stories and view, no comments, the core builds for wasm32
just isolated check   # the same in a bubblewrap sandbox: no network, empty home, cargo offline
just views            # redraw docs/views/trace.md
just mutants          # cargo-mutants; a survivor is a clause no story pins
```

Needs the Rust toolchain pinned in `rust-toolchain.toml` with the `wasm32-unknown-unknown` target, [just](https://github.com/casey/just) and python3; bubblewrap for `isolated`, [cargo-mutants](https://mutants.rs) for `mutants`.

## License

MIT or Apache-2.0, at your option.

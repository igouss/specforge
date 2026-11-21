# SpecForge

Specifications as the development interface: SpecForge reads Markdown specifications, each requirement an EARS statement with a Gherkin story beside it, and answers three separate questions.

- **Is the specification well formed?** `compile` yields a trace, or every defect by path and line.
- **Can it run?** `assess` joins the trace with your step bindings, your pending table and the requirements your invariant checker monitors, and reports what does not add up.
- **Does it hold?** `stories` runs exactly the executable scenarios with [cucumber](https://github.com/cucumber-rs/cucumber) and fails a run of nothing.

The assessment draws as one table, which a project commits; the run refuses a stale one, so a requirement that loses its bindings is a diff, not a silence.

## A specification

```markdown
### REQ-001 @ v1 — admission mints a permanent identity

> WHEN the Intake station admits a work item, the factory SHALL mint a new identity.

```gherkin
@REQ-001 @v1
Feature: REQ-001 — identity minting at admission

  Scenario: one admission mints one identity
    Given an empty factory
    When the item "harden srt emission" is admitted
    Then a new identity is returned
```
```

## A story runner

```rust
let project: Project = Project {
    specifications: root.join("specs"),
    pending: root.join("specs/pending.md"),
    view: root.join("docs/views/trace.md"),
    scratch: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stories"),
    monitored: Vec::new(),
};
let outcome: Outcome = runtime.block_on(stories::<MyWorld>(&project, Mode::Verify));
println!("{outcome}");
```

## Run it

```sh
just check            # every gate
just isolated check   # the same in a sandbox: no network, empty home, cargo offline
just views            # redraw docs/views/trace.md
```

Needs the Rust toolchain pinned in `rust-toolchain.toml`, [just](https://github.com/casey/just), python3, and bubblewrap for `isolated`. Agents read [CLAUDE.md](CLAUDE.md) first; the plan is [docs/strategy.md](docs/strategy.md).

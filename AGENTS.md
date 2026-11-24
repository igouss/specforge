# For agents working with SpecForge

You are welcome to change this project. It is work in progress, published so that other projects can reuse it, and it is meant to be adapted: if the project you are working in needs SpecForge to behave differently, change it here, in a fork, or in a vendored copy, whichever serves that project. You do not need permission to adapt it to your needs.

## To use it

[README.md](README.md) has the dependency line, the document format and a complete runner. [tests/stories/](tests/stories/) is a working consumer: SpecForge runs its own specifications through itself.

## To change it so the change holds

1. **The specifications are the interface.** Behaviour lives in [specs/](specs/) as EARS statements with Gherkin stories (SF-001 to SF-011). Change or add the story first, see it fail, then change the code in `src/`.
2. **Run `just check` before and after.** It takes seconds. A new story is unbound until its step definitions exist in `tests/stories/`; when the trace changes, `just views` redraws `docs/views/trace.md`, and you commit the redrawn view after reading its diff.
3. **Keep the three results separate.** A defect (the documents are malformed), a finding (the trace does not add up) and a run outcome are different answers. Do not fold one into another, and never let an incomplete run report as passed: the worst bug this project can have is a false green.
4. **Tests are the stories.** No unit tests of private functions. `just mutants` shows clauses no story pins.

## House conventions

These bind the upstream repository. In your own fork or copy, keep or drop them as your project requires.

- No code comments; a doc comment carries an EARS contract on the item it binds. `just comments` enforces it.
- Explicit types on signatures, fields, constants and locals whose type the right-hand side does not spell.
- No `unwrap` or `expect` in `src/`; no wildcard arm on this crate's enums.
- The core (`trace`, `assess`, `view`) does no I/O and builds for wasm32; file access and cucumber live in `stories`, behind the `cucumber` feature.
- Commits are `<scope>: <description>`, one move per commit, green on every gate.
- Decisions and their reasons: [docs/strategy.md](docs/strategy.md) and [docs/adr/](docs/adr/). Open work: `br ready` (the tracker's data is in `.beads/`).

## Sending a change back

Pull requests are welcome, from agents and people alike. Say what your project needed and which story pins the new behaviour. If your change is specific to your project, keeping it in your fork is a fine outcome.

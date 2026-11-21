# SpecForge: working rules for this repository

SpecForge compiles Markdown specifications (EARS statements with fenced Gherkin stories) into a requirement trace, assesses the trace against step bindings, and runs the stories. Its first consumer is Symphony (`../symphony`), whose story runner is glue over it.

## Read first

1. [docs/strategy.md](docs/strategy.md): the operator decision (authority for everything here), the assumed defaults, the slices.
2. [specs/](specs/): SF-001 to SF-006. The specifications are the interface; the code follows them.
3. `br ready`: what to do next. `br list -l operator` lists the questions only the operator can answer; do not build past one.

## Commands

| Command | What it proves | When |
|---|---|---|
| `just check` | every gate: format, lints, SpecForge's own stories and view, no comments, the core builds for wasm32 | before every commit |
| `just isolated check` | the same with no network and no user state | before closing a bead |
| `just views` | redraws `docs/views/trace.md` | when a story or a binding changes |
| `just mutants` | native mutation of the crate | before closing a bead that added stories |

## How work is divided

- The orchestrating session drafts the skeleton (types, signatures, EARS contracts in doc comments) and the stories in `specs/`. Implementing agents write bodies, step definitions and fixtures; `specs/`, the golden views in `tests/fixtures/*.view.md` and the public signatures are read-only to them. An agent that thinks the skeleton or a story is wrong stops and says so.
- Reports are text in the agent's reply. The agent that built a thing never certifies it; the orchestrator re-runs the oracle.
- Stop and ask the operator before: adding a dependency not listed in the strategy, changing a public type after its bodies exist, weakening a gate.

## Specification and tests

- One requirement per `### SF-NNN @ v1 — title` heading, an EARS blockquote, one fenced `gherkin` block. The stories run through SpecForge itself (`tests/stories`), so a new story is unbound until its steps exist and the committed view says so.
- Nothing below story level. No tests of private functions.
- A change that Symphony consumes lands here first, then Symphony's `simulation/Cargo.toml` takes the new revision.

## House rules

- No code comments; doc comments carry EARS contracts only.
- Explicit types on signatures, fields, constants and locals whose type the right-hand side does not spell.
- A commit hook rejects any authored file over 20000 bytes: split along a seam.
- Documents state intent, decisions and rules; progress lives in `br`.
- Commits: `<scope>: <description>`, one move per commit, green on every gate.
- `sources/` is an archive: never edit it.

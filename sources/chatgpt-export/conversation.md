# SpecForge — Conversation Export

## Project direction

The project is a new Rust application combining Deterministic Simulation Testing (DST) with Gherkin executable specifications. Its central goal is to make **specifications the primary development interface**.

The intended workflow connects human-readable business behavior to low-level distributed-system simulation:
- Gherkin describes behavior through Given/When/Then.
- Step definitions bridge specifications to simulated actors, fault schedules, and assertions.
- A deterministic simulator controls virtual time, scheduling, network behavior, storage behavior, and seeded pseudo-random choices.
- Invariants are evaluated during or after execution.
- Failures should be reproducible through recorded execution inputs and evidence.

The initial example involved a three-node distributed ledger, Alice transferring 60 credits to Bob, a network partition, a node restart, and verification that the total balance remains 100.

## User's project requests in this conversation

1. Asked for feedback on the idea of combining DST and Gherkin in a new Rust application.
2. Clarified that the core problem to solve is **specifications as the primary development interface**.
3. Requested a project requirements specification using EARS and Gherkin scenarios.
4. Requested a Specification Language and Meta-Model.
5. Accepted all proposed decisions in that design.
6. Requested Specification Engine architecture.
7. Requested the Executable Specification Graph schema and relationship semantics.
8. Requested an ontology for the project to establish a ubiquitous language.
9. Requested an export of the conversation as a ZIP archive.

Some intermediate assistant turns were omitted from the session context, so this file does not attempt to reconstruct their exact wording.

---

# Available design deliverable: Executable Specification Graph

The graph is intended to be more than a document dependency graph. It is an executable semantic model of system intent, describing:
- what the system is required to do;
- what observable behavior demonstrates it;
- which properties must always hold;
- which state transitions are permitted;
- under what environmental conditions the system operates;
- which faults can occur;
- how correctness is evaluated;
- which Rust implementations realize the specifications.

## Conceptual layers

| Layer | Responsibility | Main entities |
|---|---|---|
| Intent | Express desired behavior | Requirement, Constraint |
| Behavior | Describe observable examples | Specification, Scenario, Step |
| Correctness | Define valid system properties | Invariant, StateMachine |
| Simulation | Define execution conditions | Environment, FaultModel |
| Verification | Define how correctness is checked | Verification |
| Implementation | Connect intent to executable code | Binding |
| Evidence | Record what was established | VerificationRun, Evidence |

## Proposed entity catalog

| Entity | Purpose |
|---|---|
| Project | Root project configuration |
| Requirement | Normative system obligation |
| Specification | Behavioral or technical specification |
| Scenario | Concrete executable example |
| Step | Atomic scenario instruction or assertion |
| Rule | Group of related behavioral rules |
| Invariant | Property that must hold |
| StateMachine | Valid states and transitions |
| Constraint | Restriction or assumption |
| Environment | Simulated execution environment |
| FaultModel | Failure behavior and injection model |
| Verification | Verification strategy and selection |
| Binding | Executable Rust implementation reference |
| ExecutionPlan | Generated resolved execution instructions |
| VerificationRun | One execution attempt |
| Evidence | Result, trace, or counterexample |

## Common entity envelope

```rust
pub struct EntityEnvelope {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: String,
    pub source: SourceReference,
    pub revision: EntityRevision,
    pub tags: Vec<String>,
}

pub struct SourceReference {
    pub document_id: DocumentId,
    pub path: String,
    pub content_hash: ContentHash,
    pub span: Option<SourceSpan>,
}
```

IDs should be stable across file moves and renames. Paths are provenance, not identity. Generated entities should identify their generator and source inputs.

## Core entity semantics

- **Requirement:** normative obligation; independent of implementation.
- **Specification:** groups related behavioral or technical intent.
- **Scenario:** executable behavioral example; Gherkin is the authoring interface.
- **Step:** ordered instruction/assertion; executable steps resolve to bindings.
- **Invariant:** property over a defined execution scope.
- **StateMachine:** states, transitions, guards, effects, and optional state invariants.
- **Constraint:** restriction on valid behavior or execution.
- **Assumption:** condition accepted about the environment or external participants.
- **Guarantee:** property the system must provide under stated assumptions.
- **Environment:** declarative simulation conditions.
- **FaultModel:** declaration of possible disturbances, distinct from an actual Fault occurrence.
- **Verification:** strategy and scope for evaluating specifications.
- **Binding:** maps a semantic contract to a Rust implementation.

## Relationship vocabulary

| Relationship | Meaning |
|---|---|
| `contains` | Structural ownership |
| `refines` | Makes a requirement more specific |
| `verifies` | Provides verification coverage |
| `constrains` | Restricts valid behavior |
| `assumes` | Declares an assumption |
| `depends_on` | Requires another entity |
| `monitors` | Requests invariant evaluation |
| `implements` | Provides executable realization |
| `executes_in` | Selects an environment |
| `injects` | Applies a fault model |
| `produces` | Associates generated evidence |
| `selects` | Selects scenarios for verification |
| `evaluated_by` | Selects evaluator implementation |
| `transitions_via` | Supplies transition implementation |
| `uses` | Selects a resource or capability |

Relationships are directional, typed, and schema-validated. Structural and semantic relationships are distinct. The semantic graph may contain cycles where allowed; the execution dependency graph must be acyclic.

A direction convention was clarified in the ontology discussion:
- `Specification --refines--> Requirement`
- `Binding --implements--> Step`
- `Binding --implements--> InvariantEvaluatorContract`
- An Invariant references its evaluator through `evaluated_by`.

## Execution closure

An Execution Closure is the complete set of entities and implementations required to execute a selected specification. It is relationship-aware: traceability relationships such as `verifies` do not automatically become execution dependencies.

An executable scenario generally requires:
- all steps parsed;
- exactly one resolved binding per executable step;
- exactly one resolved environment;
- required environment capabilities;
- evaluators for mandatory invariants;
- compatible fault models;
- resolved state-machine guards/effects;
- available runtime bindings;
- resolved execution limits.

Specification validity, executability, and verification outcome are independent dimensions.

## Rust graph model

```rust
pub struct SpecificationGraph {
    pub revision: GraphRevision,
    pub nodes: BTreeMap<EntityId, GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub struct GraphNode {
    pub envelope: EntityEnvelope,
    pub entity: SpecificationEntity,
}

pub enum SpecificationEntity {
    Project(Project),
    Requirement(Requirement),
    Specification(Specification),
    Rule(Rule),
    Scenario(Scenario),
    Step(Step),
    Invariant(Invariant),
    StateMachine(StateMachine),
    Constraint(Constraint),
    Environment(Environment),
    FaultModel(FaultModel),
    Verification(Verification),
    Binding(Binding),
}

pub struct GraphEdge {
    pub from: EntityId,
    pub to: EntityId,
    pub relationship: Relationship,
    pub attributes: EdgeAttributes,
}
```

The graph should use a central relationship registry defining valid endpoint kinds, direction, dependency behavior, and execution-planning effects. Derived indexes should support incoming/outgoing edge lookup and entity-kind lookup.

## Validation categories

1. Identity rules: unique IDs, valid endpoints, provenance, matching entity kinds.
2. Structural rules: no containment cycles; steps belong to exactly one scenario; authored hierarchy has predictable ownership.
3. Relationship rules: valid endpoint types/direction/attributes; acyclic refinement; valid bindings.
4. Semantic rules: valid state-machine endpoints, evaluators, fault/environment compatibility, assumptions and guarantees.
5. Execution rules: complete bindings, environment, evaluators, fault compatibility, limits, immutable graph revision.

A graph can be structurally and semantically valid but not executable. That should be reported separately rather than invalidating the specification.

## Versioning and evidence

The proposed provenance chain is:

`Git Commit → Source Documents → Specification Compiler → Immutable Graph Snapshot → Execution Plan → Verification Run → Evidence Bundle`

Record:
- Git commit SHA;
- source document content hashes;
- stable entity IDs and semantic revisions;
- graph revision hash;
- execution plan hash;
- binding contract and implementation versions;
- verification run ID;
- content-addressed evidence IDs.

Graph hashing should use canonical semantic content and must not depend on filesystem iteration order, YAML key order, unstable memory addresses, or absolute paths.

---

# Available design deliverable: SpecForge domain ontology and ubiquitous language

## Foundational domain statement

**A specification describes what must be true. An execution explores what happens. Verification establishes whether the observed behavior satisfies the specification.**

These are distinct concepts.

## Six bounded contexts

| Context | Responsibility | Primary question |
|---|---|---|
| Specification | Express system intent | What must the system do? |
| Specification Graph | Connect and interpret specifications | How is intent related? |
| Execution Planning | Resolve executable intent | What exactly should be executed? |
| Simulation | Model system behavior | What happens under these conditions? |
| Verification | Evaluate correctness | Did behavior satisfy the specification? |
| Evidence | Preserve results and provenance | What did we establish? |

Bindings provide the bridge between declarative intent and executable Rust implementations; they can initially live within Execution Planning.

## Canonical vocabulary

### Project and specification

| Term | Definition |
|---|---|
| Project | Version-controlled collection of specifications, bindings, configuration, and source artifacts describing a system |
| Requirement | Normative statement of something the system must satisfy |
| Specification | Formal description of expected system behavior or properties |
| Rule | Named grouping of related behavioral expectations |
| Scenario | Concrete example expressed through preconditions, actions, and expected outcomes |
| Step | Individual instruction or assertion in a scenario |
| Invariant | Property that must remain true throughout a defined scope |
| State Machine | Model defining valid states and transitions |
| Constraint | Restriction on valid behavior or execution conditions |
| Assumption | Condition accepted about the environment or external participants |
| Guarantee | Property the system is obligated to provide under stated assumptions |
| Environment | Declarative description of execution conditions |
| Fault Model | Declaration of failures or disturbances that may affect a simulation |
| Verification | Declaration of strategy and scope used to evaluate specifications |

Important distinctions:
- A requirement expresses an obligation; a specification makes behavior more precise.
- A scenario is one concrete behavior; verification defines how behavior/properties are evaluated.
- An invariant is a persistent/scope-wide property; an assertion is a particular evaluation condition.
- A Fault Model declares possible disturbances; a Fault is an actual occurrence.

### Graph

| Term | Definition |
|---|---|
| Specification Entity | Typed domain object represented in the graph |
| Graph Node | Graph representation of a specification entity |
| Relationship | Typed, directed semantic connection |
| Specification Graph | Derived semantic model containing entities and relationships |
| Graph Revision | Immutable identity for one compiled graph state |
| Refinement | Relationship making a broader requirement more specific |
| Traceability | Navigation between requirements, specifications, implementation, and evidence |
| Coverage | Degree to which obligations are associated with verification activities |
| Dependency | Relationship indicating one entity requires another |
| Execution Closure | Complete set of entities and implementations needed for execution |
| Provenance | Information identifying origin and production history |

### Execution planning

| Term | Definition |
|---|---|
| Execution Selection | Set of specifications selected for execution |
| Execution Closure | Transitive execution dependencies of the selection |
| Execution Plan | Immutable resolved description of what the Simulation Engine executes |
| Resolved Binding | Binding reference resolved to an available implementation |
| Execution Configuration | Effective configuration for an execution |
| Execution Limits | Bounds on resources, events, virtual time, or exploration |
| Plan Revision | Identity of a compiled execution plan |

### Simulation

| Term | Definition |
|---|---|
| Simulation | Deterministic execution of a modeled system under a specified environment and plan |
| Simulated System | System being modeled and exercised |
| Actor | Independently scheduled participant |
| Node | Simulated computing participant hosting application behavior |
| Message | Communication object between participants |
| Event | Occurrence that changes or observes simulation state |
| Simulation Time | Virtual time maintained by the simulation |
| Virtual Clock | Clock abstraction used by simulated components |
| Scheduler | Selects the next eligible simulation activity |
| Interleaving | Particular ordering of concurrently enabled activities |
| Network | Simulated communication medium |
| Partition | Network condition preventing communication between selected participants |
| Storage | Simulated persistence subsystem |
| Fault | Actual failure or disturbance during simulation |
| Fault Injection | Introducing a fault into the simulated system |
| Seed | Value controlling seeded pseudo-random choices |
| Checkpoint | Captured simulation state at a defined point |
| Replay | Re-execution using sufficient recorded information to reproduce an execution |

Important distinctions:
- A Node is infrastructure; an Actor is an independently scheduled behavioral participant. A node can host multiple actors.
- An Event is an occurrence; a Message is a communication object. Message delivery can cause an event.
- Simulation Time is controlled by the simulator; Wall-clock Time is host time.
- A Fault Model is not a Fault occurrence.

### Verification

| Term | Definition |
|---|---|
| Observation | Read-only representation of relevant simulated state |
| Evaluator | Executable implementation evaluating a property |
| Evaluation | Applying an evaluator to an observation or history |
| Assertion | Condition that must hold at a particular point |
| Property | Formally or operationally defined statement about behavior |
| Safety Property | Something undesirable never happens |
| Liveness Property | Something desirable eventually happens under defined assumptions |
| Verification Result | Outcome of evaluating a specification |
| Counterexample | Concrete execution or observation demonstrating a violated property |

Evaluators should be observational and must not mutate simulated state. Liveness properties may require event-history access rather than only a current-state snapshot.

### Evidence

| Term | Definition |
|---|---|
| Verification Run | One attempt to execute a verification plan |
| Execution Trace | Ordered record of relevant simulation events and activities |
| Evidence | Durable artifact supporting a verification result |
| Reproduction Data | Information required to reproduce an execution |
| Counterexample | Failure-producing execution retained as evidence |
| Evidence Bundle | Collection of results, traces, provenance, and reproduction information |

A run is an occurrence; evidence is durable information produced by it.

## Domain invariants

- Every Requirement has stable identity and a normative statement.
- Every Scenario belongs to a Specification or Rule.
- Every Step belongs to exactly one Scenario.
- Every executable Step resolves to exactly one binding.
- Every executable Scenario resolves to exactly one Environment.
- Every executable Invariant resolves to an Evaluator.
- Assumptions and Guarantees are distinct.
- A Requirement is not verified merely because a Scenario references it.
- Containment, refinement, and execution dependencies are acyclic.
- Graph revisions are immutable.
- Simulation Time is independent of Wall-clock Time.
- Scheduler choices are deterministic under identical execution inputs.
- Fault occurrences are distinguishable from Fault Models.
- Evaluators are observational.
- Safety and liveness properties have distinct evaluation semantics.
- Incomplete execution cannot be reported as Passed.
- A failed Verification Run does not invalidate the Specification.
- Counterexamples retain sufficient reproduction information, subject to declared limits.

## Aggregate and ownership boundaries

| Aggregate | Root | Owns |
|---|---|---|
| Project | Project | Project configuration and source organization |
| Specification | Specification | Rules and scenarios |
| Scenario | Scenario | Ordered steps |
| State Machine | StateMachine | States and transitions |
| Environment | Environment | Nodes, topology, simulation configuration |
| Fault Model | FaultModel | Fault definitions and activation parameters |
| Verification | Verification | Selection and verification configuration |
| Binding | Binding | Contract and implementation reference |
| Graph Snapshot | SpecificationGraph | Compiled entities and relationships |
| Verification Run | VerificationRun | Execution result and run metadata |
| Evidence Bundle | EvidenceBundle | Evidence references and provenance |

Ownership and reference must remain distinct. A Scenario structurally owns its Steps but references Requirements, Invariants, Environments, and Fault Models.

## Separate result dimensions

Simulation outcome:

```rust
pub enum SimulationOutcome {
    Completed,
    Terminated,
    TimedOut,
    ResourceExhausted,
    Crashed,
    Cancelled,
}
```

Verification outcome:

```rust
pub enum VerificationOutcome {
    Passed,
    Failed,
    Inconclusive,
    NotEvaluated,
}
```

Verification lifecycle status:

```rust
pub enum VerificationStatus {
    Pending,
    Planning,
    Running,
    Evaluating,
    Completed,
}
```

These dimensions must not be collapsed. A simulation may complete normally while an invariant fails. A timeout is generally inconclusive, not a pass.

## Canonical relationship direction

Use active-voice direction consistently:
- `Specification --refines--> Requirement`
- `Scenario --verifies--> Requirement`
- `Scenario --monitors--> Invariant`
- `Scenario --executes_in--> Environment`
- `Verification --selects--> Scenario`
- `Verification --injects--> FaultModel`
- `Binding --implements--> Step`
- `Binding --implements--> InvariantEvaluatorContract`
- `Invariant --evaluated_by--> Binding`
- `VerificationRun --produces--> Evidence`

## Terms to avoid

| Avoid | Prefer |
|---|---|
| Test | Scenario or Verification |
| Test execution | Verification Run |
| Test result | Verification Result |
| Mock clock | Virtual Clock |
| Failure | Fault or Violation |
| Spec | Specification |
| Graph item | Specification Entity |
| Graph edge | Relationship |
| Random execution | Seeded Exploration |
| Thread | Actor or Activity |
| State snapshot | Observation or Checkpoint |
| Runner | Simulation Engine or Verification Runner |
| Orchestrator | Planner, Coordinator, or Scheduler |

## Suggested project artifact layout

```text
ontology/
├── README.md
├── ontology.yaml
├── glossary.yaml
├── relationships.yaml
├── bounded-contexts.yaml
├── domain-invariants.yaml
├── concepts/
│   ├── project.yaml
│   ├── specification.yaml
│   ├── graph.yaml
│   ├── planning.yaml
│   ├── simulation.yaml
│   ├── verification.yaml
│   └── evidence.yaml
└── adr/
    ├── ADR-001-domain-boundaries.md
    ├── ADR-002-terminology.md
    └── ADR-003-relationship-semantics.md
```

The ontology should become a first-class input to the AI development workflow. Agents should receive canonical definitions and relationship semantics before creating or modifying specifications. The Specification Engine should eventually validate authored vocabulary and relationships against the ontology.

## Foundational domain decisions

| ID | Decision |
|---|---|
| ONT-001 | Specification is the central domain concept |
| ONT-002 | Requirement expresses a normative obligation |
| ONT-003 | Scenario describes an executable behavioral example |
| ONT-004 | Invariant describes a property over an execution scope |
| ONT-005 | Fault Model describes possible faults; Fault represents an occurrence |
| ONT-006 | Specification Graph is a derived semantic model |
| ONT-007 | Execution Plan is distinct from Simulation |
| ONT-008 | Simulation Outcome is distinct from Verification Outcome |
| ONT-009 | Verification Run is distinct from Verification |
| ONT-010 | Evidence is immutable and revision-bound |
| ONT-011 | Actor and Node are separate concepts |
| ONT-012 | Event and Message are separate concepts |
| ONT-013 | Simulation Time is independent of Wall-clock Time |
| ONT-014 | Assumptions and Guarantees are distinct |
| ONT-015 | Structural ownership and semantic relationships are distinct |
| ONT-016 | The ontology is versioned and maintained in Git |

## Overall domain lifecycle

`Intent → Specification → Specification Graph → Execution Planning → Simulation → Verification → Evidence`

Findings feed back into the next specification and implementation revision.

The key conceptual separation is:
- Humans and AI agents express intent.
- Specifications formalize intent.
- The Specification Engine compiles a semantic graph.
- The Execution Planner resolves executable dependencies.
- The Simulation Engine explores behavior deterministically.
- The Verification Engine evaluates correctness.
- The Evidence system preserves what was established.

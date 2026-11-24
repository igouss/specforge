# Domain Model and Ubiquitous Language

## 1. Core concepts

| Entity | Definition | Key relationships |
|---|---|---|
| Requirement | An authoritative statement of required behavior or constraint | refined-by Model; verified-by Claim |
| BehavioralModel | An abstraction of system behavior | contains StateMachine; defines Scenario space |
| StateMachine | States, events, guards, actions, and transitions | realizes behavioral aspects of Requirement |
| Property | A proposition that can be evaluated or formally checked | constrains Subject; verified-by Activities; evaluated-by Results |
| Invariant | A property that must hold in every relevant state | applies-to Model/Scope |
| Assumption | A condition accepted as true for a verification argument | qualifies Property, Plan, Claim, Evidence |
| EnvironmentModel | Models external participants and conditions | used-by Scenario and DST |
| FaultModel | Defines modeled failure modes | used-by DST/MBT |
| Scenario | A behavioral experiment or ordered sequence of actions/events | generated-from Model; executed-as Run |
| VerificationPlan | Declares what to verify, how, under what scope and budget | targets Properties/Claims |
| VerificationActivity | One method-specific unit of work in a plan | dispatched-to Adapter |
| VerificationRun | One actual attempt to execute an activity | produces Result and Evidence |
| ExecutionTrace | Ordered observations/events from an execution | evaluated-against Property |
| PropertyEvaluation | Result of evaluating a property over a model/trace | creates Finding on violation |
| Finding | An observed issue or notable result requiring interpretation | may be witnessed-by Counterexample |
| Counterexample | Concrete witness demonstrating a violation in a specified model/run | violates Property; minimized-from original |
| EvidenceRecord | Immutable provenance-bearing record of an observation/result/artifact | supports/contradicts Claim |
| EvidenceAssessment | Reproducible interpretation of evidence relative to a claim | assesses EvidenceSet |
| VerificationClaim | Proposition about a system under declared scope/assumptions | supported/refuted by assessment |
| AssuranceArgument | Structured rationale connecting claim, subclaims, evidence, assumptions | organizes Claim and Evidence |
| VerificationDecision | Reviewed engineering judgment based on findings and evidence | disposes Finding; authorizes change |
| CoverageRecord | Measurement of explored model/input/execution space | informs Planner |
| ReplayDescriptor | Data required to reproduce an execution where supported | belongs-to Counterexample/Run |

## 2. Important distinctions

### Requirement vs Model vs Property
- A requirement expresses what is required.
- A model abstracts how the system behaves.
- A property expresses a condition that must hold over a defined subject, state, trace, or execution.

### Scenario vs Execution
- A scenario describes an intended experiment.
- An execution is an actual run of that experiment under a particular implementation, environment, and configuration.

### Finding vs Counterexample
- A finding is an interpreted observation, potentially unresolved.
- A counterexample is a concrete witness of a property violation. Not every finding has a counterexample.

### EvidenceRecord vs EvidenceAssessment
- An evidence record stores what happened and its provenance.
- An assessment interprets the evidence for a claim. Assessments may be recomputed; evidence records remain immutable.

### VerificationClaim vs VerificationDecision
- A claim is a proposition to be supported or refuted.
- A decision is an accountable engineering judgment about disposition and next action.

## 3. Property categories

At minimum:

- Safety: something bad never happens.
- Liveness: something good eventually happens, under stated assumptions.
- Bounded response: a condition occurs within a declared bound.
- State invariant: a predicate holds in every state in scope.
- Relational/consistency: a relationship between observations or components holds.

A property definition must state its semantics and scope. Do not assume every property can be evaluated by the same trace checker.

## 4. Result and assessment vocabularies

### Evaluation results
`satisfied`, `violated`, `inconclusive`, `not_evaluated`, `invalid_evaluation`.

### Formal outcomes
`proved`, `refuted`, `bounded_success`, `bounded_failure`, `unknown`, `timeout`, `unsupported`.

### Evidence/claim assessment
`supported`, `refuted`, `inconclusive`, `requires_review`, `stale`, `invalid`.

These vocabularies must remain distinct. A test runner's `passed` is not synonymous with a claim being `supported` or formally `proved`.

## 5. Relationships

Recommended typed relationships:

- `refines`: Requirement → Requirement
- `realized_by`: Requirement → BehavioralModel
- `constrained_by`: Subject/Model → Property
- `assumes`: Property/Plan/Claim → Assumption
- `generated_from`: Scenario → Model
- `covers`: Scenario/Execution → State/Transition/Requirement
- `executes`: Run → Scenario
- `evaluates`: PropertyEvaluation → Property
- `observed_in`: Finding → Run/Trace
- `witnessed_by`: Finding → Counterexample
- `derived_from`: EvidenceRecord → Run/ToolArtifact
- `supports`: EvidenceRecord → Claim
- `contradicts`: EvidenceRecord → Claim
- `supersedes`: EvidenceRecord → EvidenceRecord
- `invalidates`: Change/Decision → EvidenceRecord
- `depends_on`: EvidenceRecord → SourceArtifact/Assumption/ToolConfig
- `disposes`: VerificationDecision → Finding
- `authorizes`: VerificationDecision → ChangeSet

## 6. Illustrative property record

```yaml
property:
  id: PROP-MSG-001
  name: acknowledged-message-not-redelivered
  kind: safety
  subject: MessageLifecycle
  predicate:
    language: temporal
    expression: "always(acknowledged(message) implies not_redelivered(message))"
  scope:
    component: broker
    topology: three-node
  assumptions:
    - ASM-BROKER-DURABILITY-001
  linked_requirements:
    - REQ-MSG-001
  verification_methods:
    - property_based
    - model_based
    - deterministic_simulation
    - model_checking
```

This is illustrative. Reuse the existing SpecForge schema and syntax where possible.

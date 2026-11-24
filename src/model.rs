use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::graph::{ArtifactId, ContentHash, GraphSnapshot, Property, PropertyKind, StateMachine, Transition};
use crate::verify::{
    Counterexample, EvaluationResult, ExecutionTrace, FindingRecord, PropertyEvaluation, ReplayDescriptor, RootCause, TraceEvent, ViolationDetail,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedScenario {
    pub id: String,
    pub model_id: ArtifactId,
    pub model_digest: ContentHash,
    pub strategy: String,
    pub coverage_objective: String,
    pub covered_states: Vec<String>,
    pub covered_transitions: Vec<String>,
    pub events: Vec<String>,
}

struct BfsNode {
    state: String,
    states: Vec<String>,
    transitions: Vec<String>,
    events: Vec<String>,
}

/// When scenarios are generated from a state machine in a snapshot, `generate_scenarios` shall produce deterministic transition-covering paths with full provenance.
pub fn generate_scenarios(snapshot: &GraphSnapshot, machine_id: &str) -> Result<Vec<GeneratedScenario>, String> {
    let machine: &StateMachine = snapshot
        .state_machine(machine_id)
        .ok_or_else(|| format!("unknown state machine {machine_id}"))?;
    let model_digest: ContentHash = snapshot
        .artifact_digest(machine_id)
        .cloned()
        .ok_or_else(|| format!("missing digest for state machine {machine_id}"))?;

    let mut queue: VecDeque<BfsNode> = VecDeque::new();
    let mut visited_states: BTreeSet<String> = BTreeSet::new();
    let mut covered_transitions: BTreeSet<(String, String, String)> = BTreeSet::new();
    let mut scenarios: Vec<GeneratedScenario> = Vec::new();

    visited_states.insert(machine.initial.clone());
    queue.push_back(BfsNode {
        state: machine.initial.clone(),
        states: vec![machine.initial.clone()],
        transitions: Vec::new(),
        events: Vec::new(),
    });

    while let Some(node) = queue.pop_front() {
        for transition in &machine.transitions {
            if transition.from != node.state {
                continue;
            }
            let mut next_states: Vec<String> = node.states.clone();
            next_states.push(transition.to.clone());
            let mut next_transitions: Vec<String> = node.transitions.clone();
            next_transitions.push(transition.name.clone());
            let mut next_events: Vec<String> = node.events.clone();
            next_events.push(transition.event.clone());

            let transition_key: (String, String, String) = (transition.from.clone(), transition.name.clone(), transition.to.clone());
            if covered_transitions.insert(transition_key) {
                let index: usize = scenarios.len() + 1;
                scenarios.push(GeneratedScenario {
                    id: format!("SCN-{}-{index:03}", machine.id),
                    model_id: machine.id.clone(),
                    model_digest: model_digest.clone(),
                    strategy: "bfs_transition_coverage".to_string(),
                    coverage_objective: "all_reachable_transitions".to_string(),
                    covered_states: next_states.clone(),
                    covered_transitions: next_transitions.clone(),
                    events: next_events.clone(),
                });
            }
            if visited_states.insert(transition.to.clone()) {
                queue.push_back(BfsNode {
                    state: transition.to.clone(),
                    states: next_states,
                    transitions: next_transitions,
                    events: next_events,
                });
            }
        }
    }

    Ok(scenarios)
}

/// When events are executed on a state machine, `execute_machine` shall deterministically step matching transitions and record each step in an execution trace.
pub fn execute_machine(machine: &StateMachine, events: &[String]) -> ExecutionTrace {
    let mut current: String = machine.initial.clone();
    let mut trace_events: Vec<TraceEvent> = Vec::new();
    for (index, event_name) in events.iter().enumerate() {
        let next: String = machine
            .transitions
            .iter()
            .find(|transition: &&Transition| transition.from == current && transition.event == *event_name)
            .map_or_else(|| current.clone(), |transition: &Transition| transition.to.clone());
        trace_events.push(TraceEvent {
            step: index + 1,
            event: event_name.clone(),
            from_state: current,
            to_state: next.clone(),
            inputs: BTreeMap::new(),
            fault: None,
            virtual_time: Some((index + 1) as u64),
        });
        current = next;
    }
    ExecutionTrace {
        initial_state: machine.initial.clone(),
        events: trace_events,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TracePredicate {
    NeverState(String),
    AlwaysState(String),
    NeverEvent(String),
    AlwaysInput { key: String, value: String },
}

enum PredicateCheck {
    Satisfied,
    Inconclusive,
    Violated(ViolationDetail),
}

fn is_bare_token(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn parse_value(raw: &str) -> Option<String> {
    let trimmed: &str = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let first: char = trimmed.chars().next()?;
    if first == '"' || first == '\'' {
        let mut chars: std::str::Chars<'_> = trimmed.chars();
        let quote: char = chars.next()?;
        let mut out: String = String::new();
        let mut closed: bool = false;
        while let Some(c) = chars.next() {
            if c == '\\' {
                let escaped: char = chars.next()?;
                out.push(escaped);
            } else if c == quote {
                closed = true;
                break;
            } else {
                out.push(c);
            }
        }
        if !closed || chars.next().is_some() {
            return None;
        }
        return Some(out);
    }
    is_bare_token(trimmed).then(|| trimmed.to_string())
}

fn parse_predicate(expression: &str) -> Option<TracePredicate> {
    let trimmed: &str = expression.trim();
    if let Some(inner) = trimmed.strip_prefix("never(").and_then(|rest: &str| rest.strip_suffix(')')) {
        let (lhs, rhs): (&str, &str) = inner.split_once("==")?;
        let lhs: &str = lhs.trim();
        let rhs: String = parse_value(rhs)?;
        return match lhs {
            "state" => Some(TracePredicate::NeverState(rhs)),
            "event" => Some(TracePredicate::NeverEvent(rhs)),
            _ => None,
        };
    }
    if let Some(inner) = trimmed.strip_prefix("always(").and_then(|rest: &str| rest.strip_suffix(')')) {
        if let Some((lhs, rhs)) = inner.split_once("!=") {
            if lhs.trim() == "state" {
                let rhs: String = parse_value(rhs)?;
                return Some(TracePredicate::NeverState(rhs));
            }
            return None;
        }
        if let Some((lhs, rhs)) = inner.split_once("==") {
            let lhs: &str = lhs.trim();
            let rhs: String = parse_value(rhs)?;
            if lhs == "state" {
                return Some(TracePredicate::AlwaysState(rhs));
            }
            if let Some(key) = lhs.strip_prefix("input.") {
                if is_bare_token(key) {
                    return Some(TracePredicate::AlwaysInput {
                        key: key.to_string(),
                        value: rhs,
                    });
                }
            }
        }
    }
    None
}

fn trace_is_continuous(trace: &ExecutionTrace) -> bool {
    let mut previous_state: &str = trace.initial_state.as_str();
    for event in &trace.events {
        if event.from_state != previous_state {
            return false;
        }
        previous_state = event.to_state.as_str();
    }
    true
}

fn check_predicate(predicate: &TracePredicate, trace: &ExecutionTrace) -> PredicateCheck {
    match predicate {
        TracePredicate::NeverState(forbidden) => {
            if trace.initial_state == *forbidden {
                return PredicateCheck::Violated(ViolationDetail {
                    step: 0,
                    observed: format!("state == {forbidden}"),
                    expected: format!("state != {forbidden}"),
                });
            }
            for event in &trace.events {
                if event.from_state == *forbidden || event.to_state == *forbidden {
                    return PredicateCheck::Violated(ViolationDetail {
                        step: event.step,
                        observed: format!("state == {forbidden}"),
                        expected: format!("state != {forbidden}"),
                    });
                }
            }
            PredicateCheck::Satisfied
        }
        TracePredicate::AlwaysState(required) => {
            if trace.initial_state != *required {
                return PredicateCheck::Violated(ViolationDetail {
                    step: 0,
                    observed: format!("state == {}", trace.initial_state),
                    expected: format!("state == {required}"),
                });
            }
            for event in &trace.events {
                if event.from_state != *required {
                    return PredicateCheck::Violated(ViolationDetail {
                        step: event.step,
                        observed: format!("state == {}", event.from_state),
                        expected: format!("state == {required}"),
                    });
                }
                if event.to_state != *required {
                    return PredicateCheck::Violated(ViolationDetail {
                        step: event.step,
                        observed: format!("state == {}", event.to_state),
                        expected: format!("state == {required}"),
                    });
                }
            }
            PredicateCheck::Satisfied
        }
        TracePredicate::NeverEvent(forbidden) => {
            for event in &trace.events {
                if event.event == *forbidden {
                    return PredicateCheck::Violated(ViolationDetail {
                        step: event.step,
                        observed: format!("event == {forbidden}"),
                        expected: format!("event != {forbidden}"),
                    });
                }
            }
            PredicateCheck::Satisfied
        }
        TracePredicate::AlwaysInput { key, value } => {
            let mut observed_any: bool = false;
            for event in &trace.events {
                if let Some(observed) = event.inputs.get(key) {
                    observed_any = true;
                    if observed != value {
                        return PredicateCheck::Violated(ViolationDetail {
                            step: event.step,
                            observed: format!("input.{key} == {observed}"),
                            expected: format!("input.{key} == {value}"),
                        });
                    }
                }
            }
            if observed_any {
                PredicateCheck::Satisfied
            } else {
                PredicateCheck::Inconclusive
            }
        }
    }
}

fn empty_evaluation(
    property: &Property,
    property_digest: &ContentHash,
    evaluator_id: String,
    evaluator_version: String,
    result: EvaluationResult,
) -> (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) {
    (
        PropertyEvaluation {
            property_id: property.id.clone(),
            property_digest: property_digest.clone(),
            evaluator_id,
            evaluator_version,
            result,
            violation: None,
        },
        None,
        None,
    )
}

/// When a property is evaluated over an execution trace, `evaluate_property` shall return its evaluation result, and on violation an unresolved finding and concrete counterexample.
pub fn evaluate_property(
    property: &Property,
    property_digest: &ContentHash,
    trace: &ExecutionTrace,
    run_id: &str,
    snapshot_id: &ContentHash,
    replay: ReplayDescriptor,
) -> (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) {
    let evaluator_id: String = "specforge-trace-evaluator".to_string();
    let evaluator_version: String = env!("CARGO_PKG_VERSION").to_string();
    let supported_kind: bool = match property.property_kind {
        PropertyKind::Safety | PropertyKind::StateInvariant => true,
        PropertyKind::Liveness | PropertyKind::BoundedResponse | PropertyKind::Relational => false,
    };
    if property.predicate.language != "trace" || !supported_kind || !trace_is_continuous(trace) {
        return empty_evaluation(property, property_digest, evaluator_id, evaluator_version, EvaluationResult::InvalidEvaluation);
    }
    let Some(parsed): Option<TracePredicate> = parse_predicate(&property.predicate.expression) else {
        return empty_evaluation(property, property_digest, evaluator_id, evaluator_version, EvaluationResult::InvalidEvaluation);
    };

    match check_predicate(&parsed, trace) {
        PredicateCheck::Satisfied => empty_evaluation(property, property_digest, evaluator_id, evaluator_version, EvaluationResult::Satisfied),
        PredicateCheck::Inconclusive => empty_evaluation(property, property_digest, evaluator_id, evaluator_version, EvaluationResult::Inconclusive),
        PredicateCheck::Violated(violation) => {
            let finding_id: String = format!("FND-{run_id}-{}", property.id);
            let counterexample_id: String = format!("CE-{run_id}-{}", property.id);
            let evaluation: PropertyEvaluation = PropertyEvaluation {
                property_id: property.id.clone(),
                property_digest: property_digest.clone(),
                evaluator_id,
                evaluator_version,
                result: EvaluationResult::Violated,
                violation: Some(violation.clone()),
            };
            let finding: FindingRecord = FindingRecord {
                id: finding_id.clone(),
                property_id: property.id.clone(),
                run_id: run_id.to_string(),
                root_cause: RootCause::Unresolved,
                counterexample_id: Some(counterexample_id.clone()),
                summary: format!(
                    "property {} violated at step {}: observed {}, expected {}",
                    property.id, violation.step, violation.observed, violation.expected
                ),
            };
            let counterexample: Counterexample = Counterexample {
                id: counterexample_id,
                property_id: property.id.clone(),
                property_digest: property_digest.clone(),
                finding_id,
                run_id: run_id.to_string(),
                snapshot_id: snapshot_id.clone(),
                initial_state: trace.initial_state.clone(),
                events: trace.events.clone(),
                violation,
                replay,
                original_id: None,
                minimization: None,
            };
            (evaluation, Some(finding), Some(counterexample))
        }
    }
}

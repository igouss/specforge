use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::graph::{ArtifactId, ContentHash, GraphSnapshot, StateMachine, Transition};
use crate::model::GeneratedScenario;
use crate::trace::RequirementId;
use crate::verify::ExecutionTrace;

/// When an external model-based testing tool produces a scenario, `ImportedScenario` shall record its source model provenance, generation parameters, event sequence, and importer metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedScenario {
    pub id: String,
    pub model_id: ArtifactId,
    pub model_digest: ContentHash,
    pub strategy: String,
    pub coverage_objective: String,
    pub seed: Option<u64>,
    pub events: Vec<String>,
    pub importer: String,
    pub importer_version: String,
}

/// When consecutive state-machine transitions execute in sequence, `TransitionPair` shall identify the ordered pair of canonical transition keys.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TransitionPair {
    pub first: String,
    pub second: String,
}

/// When coverage is reported for a state machine, `CoverageRecord` shall separate state, transition, consecutive transition-pair, and linked requirement dimensions without collapsing them into a single score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageRecord {
    pub model_id: ArtifactId,
    pub model_digest: ContentHash,
    pub states_total: usize,
    pub states_covered: Vec<String>,
    pub uncovered_states: Vec<String>,
    pub transitions_total: usize,
    pub transitions_covered: Vec<String>,
    pub uncovered_transitions: Vec<String>,
    pub transition_pairs_total: usize,
    pub transition_pairs_covered: Vec<TransitionPair>,
    pub uncovered_transition_pairs: Vec<TransitionPair>,
    pub requirements_total: usize,
    pub requirements_covered: Vec<RequirementId>,
    pub uncovered_requirements: Vec<RequirementId>,
}

/// When scenario import or coverage computation fails, `CoverageError` shall classify the rejection without fabricating partial coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageError {
    UnknownMachine(String),
    MissingDigest(String),
    DigestMismatch {
        machine_id: String,
        expected: ContentHash,
        found: ContentHash,
    },
    InvalidScenario(String),
    InvalidTrace(String),
    Adapter(String),
}

impl fmt::Display for CoverageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoverageError::UnknownMachine(id) => write!(formatter, "unknown state machine {id}"),
            CoverageError::MissingDigest(id) => write!(formatter, "missing digest for state machine {id}"),
            CoverageError::DigestMismatch { machine_id, expected, found } => {
                write!(formatter, "state machine {machine_id} digest mismatch: expected {expected}, found {found}")
            }
            CoverageError::InvalidScenario(why) => write!(formatter, "invalid imported scenario: {why}"),
            CoverageError::InvalidTrace(why) => write!(formatter, "invalid execution trace: {why}"),
            CoverageError::Adapter(why) => write!(formatter, "MBT adapter error: {why}"),
        }
    }
}

/// When an external model-based testing engine supplies scenarios, `MbtAdapter` shall import traceable scenarios for a state machine in a snapshot.
pub trait MbtAdapter {
    fn import_scenarios(&self, snapshot: &GraphSnapshot, machine_id: &str) -> Result<Vec<ImportedScenario>, CoverageError>;
}

/// When a declared state-machine transition is keyed for coverage reporting, `transition_key` shall format it uniquely from its source state, transition name, and destination state.
pub fn transition_key(transition: &Transition) -> String {
    format!("{}--{}-->{}", transition.from, transition.name, transition.to)
}

/// When an imported MBT scenario is validated against a graph snapshot, `validate_imported_scenario` shall verify required provenance fields, machine existence, matching model digest, and declared transition steps before returning a `GeneratedScenario`.
pub fn validate_imported_scenario(snapshot: &GraphSnapshot, scenario: &ImportedScenario) -> Result<GeneratedScenario, CoverageError> {
    if scenario.id.trim().is_empty() {
        return Err(CoverageError::InvalidScenario("scenario id must not be empty".to_string()));
    }
    if scenario.strategy.trim().is_empty() {
        return Err(CoverageError::InvalidScenario(format!("scenario {} strategy must not be empty", scenario.id)));
    }
    if scenario.coverage_objective.trim().is_empty() {
        return Err(CoverageError::InvalidScenario(format!(
            "scenario {} coverage_objective must not be empty",
            scenario.id
        )));
    }
    if scenario.importer.trim().is_empty() {
        return Err(CoverageError::InvalidScenario(format!("scenario {} importer must not be empty", scenario.id)));
    }
    if scenario.importer_version.trim().is_empty() {
        return Err(CoverageError::InvalidScenario(format!(
            "scenario {} importer_version must not be empty",
            scenario.id
        )));
    }
    if scenario.events.is_empty() {
        return Err(CoverageError::InvalidScenario(format!(
            "scenario {} must include at least one event",
            scenario.id
        )));
    }
    let machine_id: &str = scenario.model_id.as_str();
    let machine: &StateMachine = snapshot
        .state_machine(machine_id)
        .ok_or_else(|| CoverageError::UnknownMachine(machine_id.to_string()))?;
    let expected_digest: &ContentHash = snapshot
        .artifact_digest(machine_id)
        .ok_or_else(|| CoverageError::MissingDigest(machine_id.to_string()))?;
    if expected_digest != &scenario.model_digest {
        return Err(CoverageError::DigestMismatch {
            machine_id: machine_id.to_string(),
            expected: expected_digest.clone(),
            found: scenario.model_digest.clone(),
        });
    }

    let mut current: String = machine.initial.clone();
    let mut covered_states: Vec<String> = vec![current.clone()];
    let mut covered_transitions: Vec<String> = Vec::new();
    for (index, event_name) in scenario.events.iter().enumerate() {
        let transition: &Transition = machine
            .transitions
            .iter()
            .find(|candidate: &&Transition| candidate.from == current && candidate.event == *event_name)
            .ok_or_else(|| {
                CoverageError::InvalidScenario(format!(
                    "scenario {} step {} event \"{}\" has no declared transition from state \"{}\" in {}",
                    scenario.id,
                    index + 1,
                    event_name,
                    current,
                    machine.id
                ))
            })?;
        covered_states.push(transition.to.clone());
        covered_transitions.push(transition.name.clone());
        current = transition.to.clone();
    }

    Ok(GeneratedScenario {
        id: scenario.id.clone(),
        model_id: scenario.model_id.clone(),
        model_digest: scenario.model_digest.clone(),
        strategy: scenario.strategy.clone(),
        coverage_objective: scenario.coverage_objective.clone(),
        covered_states,
        covered_transitions,
        events: scenario.events.clone(),
    })
}

/// When execution traces are measured against a state machine in a snapshot, `compute_coverage` shall validate trace continuity and declared transitions and return separate state, transition, consecutive transition-pair, and linked requirement coverage.
pub fn compute_coverage(snapshot: &GraphSnapshot, machine_id: &str, traces: &[ExecutionTrace]) -> Result<CoverageRecord, CoverageError> {
    let machine: &StateMachine = snapshot
        .state_machine(machine_id)
        .ok_or_else(|| CoverageError::UnknownMachine(machine_id.to_string()))?;
    let model_digest: ContentHash = snapshot
        .artifact_digest(machine_id)
        .cloned()
        .ok_or_else(|| CoverageError::MissingDigest(machine_id.to_string()))?;

    let declared_states: BTreeSet<String> = machine.states.iter().cloned().collect();
    let declared_transitions: BTreeSet<String> = machine.transitions.iter().map(transition_key).collect();
    let mut declared_pairs: BTreeSet<TransitionPair> = BTreeSet::new();
    for first in &machine.transitions {
        for second in &machine.transitions {
            if first.to == second.from {
                declared_pairs.insert(TransitionPair {
                    first: transition_key(first),
                    second: transition_key(second),
                });
            }
        }
    }
    let declared_requirements: BTreeSet<RequirementId> = machine.requirements.iter().cloned().collect();

    let mut covered_states: BTreeSet<String> = BTreeSet::new();
    let mut covered_transitions: BTreeSet<String> = BTreeSet::new();
    let mut covered_pairs: BTreeSet<TransitionPair> = BTreeSet::new();

    for (trace_index, trace) in traces.iter().enumerate() {
        if trace.initial_state != machine.initial {
            return Err(CoverageError::InvalidTrace(format!(
                "trace {} initial state \"{}\" does not match machine {} initial state \"{}\"",
                trace_index + 1,
                trace.initial_state,
                machine.id,
                machine.initial
            )));
        }
        covered_states.insert(trace.initial_state.clone());
        let mut previous_state: &str = trace.initial_state.as_str();
        let mut previous_transition: Option<String> = None;

        for event in &trace.events {
            if event.from_state != previous_state {
                return Err(CoverageError::InvalidTrace(format!(
                    "trace {} step {} is discontinuous: expected from_state \"{}\", got \"{}\"",
                    trace_index + 1,
                    event.step,
                    previous_state,
                    event.from_state
                )));
            }
            let matched: &Transition = machine
                .transitions
                .iter()
                .find(|candidate: &&Transition| candidate.from == event.from_state && candidate.event == event.event && candidate.to == event.to_state)
                .ok_or_else(|| {
                    CoverageError::InvalidTrace(format!(
                        "trace {} step {} ({} --{}--> {}) does not match any declared transition of {}",
                        trace_index + 1,
                        event.step,
                        event.from_state,
                        event.event,
                        event.to_state,
                        machine.id
                    ))
                })?;
            let key: String = transition_key(matched);
            covered_states.insert(matched.to.clone());
            covered_transitions.insert(key.clone());
            if let Some(prev_key) = previous_transition {
                covered_pairs.insert(TransitionPair {
                    first: prev_key,
                    second: key.clone(),
                });
            }
            previous_transition = Some(key);
            previous_state = event.to_state.as_str();
        }
    }

    let uncovered_states: Vec<String> = declared_states.difference(&covered_states).cloned().collect();
    let uncovered_transitions: Vec<String> = declared_transitions.difference(&covered_transitions).cloned().collect();
    let uncovered_transition_pairs: Vec<TransitionPair> = declared_pairs.difference(&covered_pairs).cloned().collect();

    let (requirements_covered, uncovered_requirements): (Vec<RequirementId>, Vec<RequirementId>) = if covered_transitions.is_empty() {
        (Vec::new(), declared_requirements.iter().cloned().collect())
    } else {
        (declared_requirements.iter().cloned().collect(), Vec::new())
    };

    Ok(CoverageRecord {
        model_id: machine.id.clone(),
        model_digest,
        states_total: declared_states.len(),
        states_covered: covered_states.into_iter().collect(),
        uncovered_states,
        transitions_total: declared_transitions.len(),
        transitions_covered: covered_transitions.into_iter().collect(),
        uncovered_transitions,
        transition_pairs_total: declared_pairs.len(),
        transition_pairs_covered: covered_pairs.into_iter().collect(),
        uncovered_transition_pairs,
        requirements_total: declared_requirements.len(),
        requirements_covered,
        uncovered_requirements,
    })
}

mod replay;

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::graph::{ContentHash, GraphSnapshot, Property, StateMachine, Transition};
use crate::model::evaluate_property;
use crate::verify::{Counterexample, EvaluationResult, ExecutionTrace, FindingRecord, MinimizationMeta, PropertyEvaluation, TraceEvent};

pub use replay::{replay_corpus, CorpusReplayEntry, CorpusReplayOutcome, CorpusReplaySummary};

/// When counterexample registration, minimization validation, or corpus replay fails, `CorpusError` shall report the specific rejection reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorpusError {
    EmptyId,
    DuplicateId(String),
    MissingOriginal(String),
    InvalidMinimized(String),
    ModelMismatch(String),
    ReplayFailed(String),
}

impl fmt::Display for CorpusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CorpusError::EmptyId => formatter.write_str("counterexample id must not be empty"),
            CorpusError::DuplicateId(id) => write!(formatter, "counterexample {id} is already registered"),
            CorpusError::MissingOriginal(id) => write!(formatter, "original counterexample {id} is not registered"),
            CorpusError::InvalidMinimized(why) => write!(formatter, "invalid minimized counterexample: {why}"),
            CorpusError::ModelMismatch(why) => write!(formatter, "counterexample model mismatch: {why}"),
            CorpusError::ReplayFailed(why) => write!(formatter, "corpus replay failed: {why}"),
        }
    }
}

fn validate_minimization_meta(original: &Counterexample, minimized: &Counterexample) -> Result<(), CorpusError> {
    let meta: &MinimizationMeta = minimized
        .minimization
        .as_ref()
        .ok_or_else(|| CorpusError::InvalidMinimized(format!("minimized counterexample {} is missing minimization metadata", minimized.id)))?;
    if meta.method.trim().is_empty() {
        return Err(CorpusError::InvalidMinimized(format!(
            "minimized counterexample {} minimization method must not be empty",
            minimized.id
        )));
    }
    if meta.original_len != original.events.len() {
        return Err(CorpusError::InvalidMinimized(format!(
            "minimized counterexample {} records original_len {}, expected {}",
            minimized.id,
            meta.original_len,
            original.events.len()
        )));
    }
    if meta.minimized_len != minimized.events.len() {
        return Err(CorpusError::InvalidMinimized(format!(
            "minimized counterexample {} records minimized_len {}, expected {}",
            minimized.id,
            meta.minimized_len,
            minimized.events.len()
        )));
    }
    if minimized.events.len() > original.events.len() {
        return Err(CorpusError::InvalidMinimized(format!(
            "minimized counterexample {} has {} events, exceeding original length {}",
            minimized.id,
            minimized.events.len(),
            original.events.len()
        )));
    }
    Ok(())
}

fn validate_minimized_replay(minimized: &Counterexample) -> Result<(), CorpusError> {
    let trace_event_names: Vec<String> = minimized.events.iter().map(|step: &TraceEvent| step.event.clone()).collect();
    if minimized.replay.replayable {
        if minimized.replay.engine.trim().is_empty() {
            return Err(CorpusError::InvalidMinimized(format!(
                "minimized counterexample {} declares replayable descriptor with empty engine",
                minimized.id
            )));
        }
        let missing_replay_source: bool = minimized.replay.events.is_empty()
            && (!minimized.events.is_empty() || (minimized.replay.seed.is_none() && minimized.replay.replay_token.is_none()));
        if missing_replay_source {
            return Err(CorpusError::InvalidMinimized(format!(
                "minimized counterexample {} declares replayable descriptor without replay events or seed/token",
                minimized.id
            )));
        }
        if minimized.replay.events != trace_event_names {
            return Err(CorpusError::InvalidMinimized(format!(
                "minimized counterexample {} replay events {:?} do not match trace events {:?}",
                minimized.id, minimized.replay.events, trace_event_names
            )));
        }
    } else if !minimized.replay.events.is_empty() && minimized.replay.events != trace_event_names {
        return Err(CorpusError::InvalidMinimized(format!(
            "minimized counterexample {} replay events {:?} do not match trace events {:?}",
            minimized.id, minimized.replay.events, trace_event_names
        )));
    }
    Ok(())
}

/// When a minimized counterexample is validated against its original and snapshot, `validate_minimized` shall verify linkage, non-chained original identity, minimization metadata, replay alignment, property and digest identity, causal transition validity on the subject state machine, and exact preserved violation.
pub fn validate_minimized(snapshot: &GraphSnapshot, original: &Counterexample, minimized: &Counterexample) -> Result<(), CorpusError> {
    if original.original_id.is_some() {
        return Err(CorpusError::InvalidMinimized(format!(
            "minimized counterexample {} cannot chain onto minimized counterexample {}",
            minimized.id, original.id
        )));
    }
    if minimized.original_id.as_deref() != Some(original.id.as_str()) {
        return Err(CorpusError::InvalidMinimized(format!(
            "minimized counterexample {} must reference original {}",
            minimized.id, original.id
        )));
    }
    validate_minimization_meta(original, minimized)?;
    validate_minimized_replay(minimized)?;

    if minimized.property_id != original.property_id || minimized.property_digest != original.property_digest {
        return Err(CorpusError::ModelMismatch(format!(
            "minimized counterexample {} property or digest differs from original {}",
            minimized.id, original.id
        )));
    }
    let property: &Property = snapshot
        .property(minimized.property_id.as_str())
        .ok_or_else(|| CorpusError::ModelMismatch(format!("unknown property {} in snapshot", minimized.property_id)))?;
    let snapshot_digest: &ContentHash = snapshot
        .artifact_digest(property.id.as_str())
        .ok_or_else(|| CorpusError::ModelMismatch(format!("missing digest for property {}", property.id)))?;
    if snapshot_digest != &minimized.property_digest {
        return Err(CorpusError::ModelMismatch(format!(
            "property {} digest in snapshot does not match minimized counterexample {}",
            property.id, minimized.id
        )));
    }
    let machine: &StateMachine = snapshot
        .state_machine(property.subject.as_str())
        .ok_or_else(|| CorpusError::ModelMismatch(format!("unknown subject state machine {}", property.subject)))?;

    if minimized.initial_state != machine.initial {
        return Err(CorpusError::InvalidMinimized(format!(
            "minimized counterexample {} initial state \"{}\" does not match machine {} initial state \"{}\"",
            minimized.id, minimized.initial_state, machine.id, machine.initial
        )));
    }
    let mut previous_state: &str = minimized.initial_state.as_str();
    for event in &minimized.events {
        if event.from_state != previous_state {
            return Err(CorpusError::InvalidMinimized(format!(
                "minimized counterexample {} step {} is discontinuous: expected from_state \"{}\", got \"{}\"",
                minimized.id, event.step, previous_state, event.from_state
            )));
        }
        let valid_transition: bool = machine
            .transitions
            .iter()
            .any(|transition: &Transition| transition.from == event.from_state && transition.event == event.event && transition.to == event.to_state);
        if !valid_transition {
            return Err(CorpusError::InvalidMinimized(format!(
                "minimized counterexample {} step {} ({} --{}--> {}) is not a declared transition of {}",
                minimized.id, event.step, event.from_state, event.event, event.to_state, machine.id
            )));
        }
        previous_state = event.to_state.as_str();
    }

    let trace: ExecutionTrace = ExecutionTrace {
        initial_state: minimized.initial_state.clone(),
        events: minimized.events.clone(),
    };
    let (evaluation, _, _): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) = evaluate_property(
        property,
        snapshot_digest,
        &trace,
        &minimized.run_id,
        &snapshot.snapshot_id,
        minimized.replay.clone(),
    );
    if evaluation.result != EvaluationResult::Violated {
        return Err(CorpusError::InvalidMinimized(format!(
            "minimized counterexample {} no longer violates property {} (result: {})",
            minimized.id, property.id, evaluation.result
        )));
    }
    if evaluation.violation.as_ref() != Some(&minimized.violation) {
        return Err(CorpusError::InvalidMinimized(format!(
            "minimized counterexample {} recorded violation {:?} does not match evaluated violation {:?}",
            minimized.id, minimized.violation, evaluation.violation
        )));
    }
    Ok(())
}

/// When counterexamples are collected for regression tracking, `CounterexampleRegistry` shall remain append-only and never mutate or delete registered entries.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CounterexampleRegistry {
    entries: Vec<Counterexample>,
}

impl CounterexampleRegistry {
    /// When a new counterexample registry is created, `new` shall return an empty append-only registry.
    pub fn new() -> CounterexampleRegistry {
        CounterexampleRegistry::default()
    }

    /// When an original counterexample is registered, `register_original` shall reject empty or duplicate identifiers, any entry carrying `original_id`, and any entry carrying `minimization` metadata, and otherwise append it immutably.
    pub fn register_original(&mut self, counterexample: Counterexample) -> Result<(), CorpusError> {
        if counterexample.id.trim().is_empty() {
            return Err(CorpusError::EmptyId);
        }
        if counterexample.original_id.is_some() {
            return Err(CorpusError::InvalidMinimized(format!(
                "original counterexample {} must not declare original_id",
                counterexample.id
            )));
        }
        if counterexample.minimization.is_some() {
            return Err(CorpusError::InvalidMinimized(format!(
                "original counterexample {} must not declare minimization metadata",
                counterexample.id
            )));
        }
        if self.entries.iter().any(|existing: &Counterexample| existing.id == counterexample.id) {
            return Err(CorpusError::DuplicateId(counterexample.id));
        }
        self.entries.push(counterexample);
        Ok(())
    }

    /// When a minimized counterexample is registered, `register_minimized` shall resolve its original entry, reject chaining onto a minimized entry, reject empty or duplicate identifiers, run `validate_minimized`, and append it without modifying the original.
    pub fn register_minimized(&mut self, snapshot: &GraphSnapshot, counterexample: Counterexample) -> Result<(), CorpusError> {
        if counterexample.id.trim().is_empty() {
            return Err(CorpusError::EmptyId);
        }
        if self.entries.iter().any(|existing: &Counterexample| existing.id == counterexample.id) {
            return Err(CorpusError::DuplicateId(counterexample.id));
        }
        let original_id: &str = counterexample
            .original_id
            .as_deref()
            .ok_or_else(|| CorpusError::InvalidMinimized(format!("minimized counterexample {} must reference an original_id", counterexample.id)))?;
        let original: &Counterexample = self.get(original_id).ok_or_else(|| CorpusError::MissingOriginal(original_id.to_string()))?;
        if original.original_id.is_some() {
            return Err(CorpusError::InvalidMinimized(format!(
                "minimized counterexample {} cannot reference minimized counterexample {} as its original",
                counterexample.id, original.id
            )));
        }
        validate_minimized(snapshot, original, &counterexample)?;
        self.entries.push(counterexample);
        Ok(())
    }

    /// When all counterexamples in the registry are inspected, `entries` shall return every original and minimized entry in registration order.
    pub fn entries(&self) -> &[Counterexample] {
        &self.entries
    }

    /// When a counterexample identifier is queried, `get` shall return the matching registered entry if present.
    pub fn get(&self, id: &str) -> Option<&Counterexample> {
        self.entries.iter().find(|entry: &&Counterexample| entry.id == id)
    }

    /// When minimized variants of an original counterexample are queried, `minimized_for` shall return all registered children referencing that original in registration order.
    pub fn minimized_for(&self, original_id: &str) -> Vec<&Counterexample> {
        self.entries
            .iter()
            .filter(|entry: &&Counterexample| entry.original_id.as_deref() == Some(original_id))
            .collect()
    }

    /// When original counterexamples are queried, `originals` shall return every registered entry without an `original_id` in registration order.
    pub fn originals(&self) -> Vec<&Counterexample> {
        self.entries.iter().filter(|entry: &&Counterexample| entry.original_id.is_none()).collect()
    }
}

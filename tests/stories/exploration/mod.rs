mod corpus;
mod coverage;
mod planner;
mod planner_replays;

use std::collections::BTreeMap;

use specforge::corpus::{CorpusError, CorpusReplaySummary, CounterexampleRegistry};
use specforge::coverage::{CoverageError, CoverageRecord};
use specforge::graph::{ContentHash, StateMachine};
use specforge::model::GeneratedScenario;
use specforge::planner::ExplorationRecommendation;
use specforge::verify::{Counterexample, ExecutionTrace, TraceEvent};

#[derive(Debug, Default)]
pub struct ExplorationState {
    pub validated_imports: Vec<GeneratedScenario>,
    pub import_result: Option<Result<GeneratedScenario, CoverageError>>,
    pub coverage_result: Option<Result<CoverageRecord, CoverageError>>,
    pub registry: CounterexampleRegistry,
    pub original_snapshot: Option<Counterexample>,
    pub minimized_id: Option<String>,
    pub minimized_registration: Option<Result<(), CorpusError>>,
    pub replay_summary: Option<CorpusReplaySummary>,
    pub replay_error: Option<Result<CorpusReplaySummary, CorpusError>>,
    pub replay_counter: usize,
    pub ledger_count_before_replay: Option<usize>,
    pub recommendations: Vec<ExplorationRecommendation>,
    pub second_recommendations: Vec<ExplorationRecommendation>,
    pub snapshot_digest_before_plan: Option<ContentHash>,
    pub ledger_count_before_plan: Option<usize>,
    pub ledger_json_before_plan: Option<String>,
}

pub fn safe_runner(machine: &StateMachine, events: &[String]) -> ExecutionTrace {
    let mut current: String = machine.initial.clone();
    let mut trace_events: Vec<TraceEvent> = Vec::new();
    for (index, event_name) in events.iter().enumerate() {
        let next: String = match (current.as_str(), event_name.as_str()) {
            ("queued", "deliver") => "delivered".to_string(),
            ("delivered", "ack") => "acknowledged".to_string(),
            _ => current.clone(),
        };
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

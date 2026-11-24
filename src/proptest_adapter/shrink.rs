use proptest::strategy::ValueTree;

use super::TraceRunner;
use crate::graph::{ContentHash, Property, StateMachine};
use crate::model::evaluate_property;
use crate::verify::{Counterexample, EvaluationResult, ExecutionTrace, FindingRecord, MinimizationMeta, PropertyEvaluation, ReplayDescriptor};

pub(super) fn replay_for(seed: Option<u64>, events: Vec<String>) -> ReplayDescriptor {
    ReplayDescriptor {
        engine: "proptest".to_string(),
        engine_version: "1".to_string(),
        seed,
        replay_token: seed.map(|value: u64| format!("chacha-seed:{value}")),
        events,
        determinism: "deterministic ChaCha PRNG seed and explicit event sequence".to_string(),
        replayable: true,
    }
}

pub(super) fn trace_is_causally_consistent(machine: &StateMachine, events: &[String], trace: &ExecutionTrace) -> bool {
    if trace.initial_state != machine.initial || trace.events.len() != events.len() {
        return false;
    }
    let mut current: &str = trace.initial_state.as_str();
    for (event, expected_name) in trace.events.iter().zip(events.iter()) {
        if event.from_state != current || event.event != *expected_name {
            return false;
        }
        current = event.to_state.as_str();
    }
    true
}

pub(super) struct ShrinkContext<'a> {
    pub(super) machine: &'a StateMachine,
    pub(super) runner: &'a TraceRunner,
    pub(super) property: &'a Property,
    pub(super) property_digest: &'a ContentHash,
    pub(super) run_id: &'a str,
    pub(super) snapshot_id: &'a ContentHash,
    pub(super) seed: u64,
}

pub(super) fn shrink_tree<V>(tree: &mut V, context: &ShrinkContext<'_>, original: &Counterexample) -> Counterexample
where
    V: ValueTree<Value = Vec<String>>,
{
    let mut best_events: Vec<String> = original.replay.events.clone();
    let mut best_ce: Counterexample = original.clone();
    let mut attempts: usize = 0;

    let mut shrinking: bool = tree.simplify();
    while shrinking && attempts < 256 {
        attempts += 1;
        let candidate: Vec<String> = tree.current();
        let trace: ExecutionTrace = (context.runner)(context.machine, &candidate);
        let consistent: bool = trace_is_causally_consistent(context.machine, &candidate, &trace);
        let replay: ReplayDescriptor = replay_for(Some(context.seed), candidate.clone());
        let (evaluation, _, maybe_ce): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) =
            evaluate_property(context.property, context.property_digest, &trace, context.run_id, context.snapshot_id, replay);
        if consistent && evaluation.result == EvaluationResult::Violated {
            if let Some(ce) = maybe_ce {
                if candidate.len() <= best_events.len() {
                    best_events = candidate;
                    best_ce = ce;
                }
            }
            shrinking = tree.simplify();
        } else {
            shrinking = tree.complicate();
        }
    }

    let verify_trace: ExecutionTrace = (context.runner)(context.machine, &best_events);
    let verify_replay: ReplayDescriptor = replay_for(Some(context.seed), best_events.clone());
    let (verify_eval, _, _): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) = evaluate_property(
        context.property,
        context.property_digest,
        &verify_trace,
        context.run_id,
        context.snapshot_id,
        verify_replay,
    );
    if !trace_is_causally_consistent(context.machine, &best_events, &verify_trace) || verify_eval.result != EvaluationResult::Violated {
        best_events = original.replay.events.clone();
        best_ce = original.clone();
    }

    best_ce.id = format!("{}-min", original.id);
    best_ce.original_id = Some(original.id.clone());
    best_ce.minimization = Some(MinimizationMeta {
        method: "proptest-value-tree-shrink".to_string(),
        version: "1".to_string(),
        attempts,
        original_len: original.replay.events.len(),
        minimized_len: best_events.len(),
    });
    best_ce
}

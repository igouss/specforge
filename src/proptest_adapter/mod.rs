mod shrink;

use proptest::collection::{vec, VecStrategy, VecValueTree};
use proptest::sample::{select, Select, SelectValueTree};
use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};

use crate::graph::{ContentHash, GraphSnapshot, Property, StateMachine, Transition, VerificationMethod};
use crate::model::{evaluate_property, execute_machine};
use crate::verify::{
    AdapterCapabilities, AdapterError, Counterexample, EvaluationResult, ExecutionTrace, FindingRecord, PropertyEvaluation, ReplayDescriptor, RunEnvelope,
    RunState, VerificationAdapter, VerificationOutcome, VerificationRequest,
};

use shrink::{replay_for, shrink_tree, ShrinkContext};

pub type TraceRunner = Box<dyn Fn(&StateMachine, &[String]) -> ExecutionTrace>;

pub struct ProptestAdapter {
    runner: TraceRunner,
    max_sequence_len: usize,
}

impl ProptestAdapter {
    /// When created for pure model checking, `for_machine` shall execute generated event sequences directly on the specification's state machine.
    pub fn for_machine(max_sequence_len: usize) -> ProptestAdapter {
        ProptestAdapter {
            runner: Box::new(execute_machine),
            max_sequence_len: max_sequence_len.max(1),
        }
    }

    /// When created with a custom system runner, `with_runner` shall execute generated event sequences through that runner.
    pub fn with_runner<F>(runner: F, max_sequence_len: usize) -> ProptestAdapter
    where
        F: Fn(&StateMachine, &[String]) -> ExecutionTrace + 'static,
    {
        ProptestAdapter {
            runner: Box::new(runner),
            max_sequence_len: max_sequence_len.max(1),
        }
    }
}

fn seed_bytes(seed: u64) -> [u8; 32] {
    let mut bytes: [u8; 32] = [0u8; 32];
    let encoded: [u8; 8] = seed.to_le_bytes();
    for chunk in bytes.chunks_exact_mut(8) {
        chunk.copy_from_slice(&encoded);
    }
    bytes
}

fn seed_metadata(seed: u64) -> ReplayDescriptor {
    ReplayDescriptor {
        engine: "proptest".to_string(),
        engine_version: "1".to_string(),
        seed: Some(seed),
        replay_token: Some(format!("chacha-seed:{seed}")),
        events: Vec::new(),
        determinism: "deterministic ChaCha PRNG seed without failing event trace".to_string(),
        replayable: false,
    }
}

fn event_alphabet(machine: &StateMachine) -> Vec<String> {
    let mut alphabet: Vec<String> = machine.transitions.iter().map(|transition: &Transition| transition.event.clone()).collect();
    alphabet.sort();
    alphabet.dedup();
    alphabet
}

impl VerificationAdapter for ProptestAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            adapter_id: "specforge-proptest-adapter".to_string(),
            adapter_version: env!("CARGO_PKG_VERSION").to_string(),
            engine_name: "proptest".to_string(),
            engine_version: "1".to_string(),
            methods: vec![VerificationMethod::PropertyBased, VerificationMethod::ModelBased],
            supports_replay: true,
            supports_shrinking: true,
            supports_coverage: false,
        }
    }

    fn validate(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<(), AdapterError> {
        if request.run_id.trim().is_empty() {
            return Err(AdapterError::InvalidRequest("run_id must not be empty".to_string()));
        }
        if request.activity_id.trim().is_empty() {
            return Err(AdapterError::InvalidRequest("activity_id must not be empty".to_string()));
        }
        for claim_id in &request.claim_ids {
            if snapshot.claim(claim_id.as_str()).is_none() {
                return Err(AdapterError::InvalidRequest(format!("unknown claim {claim_id}")));
            }
        }
        let property: &Property = snapshot
            .property(request.property_id.as_str())
            .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown property {}", request.property_id)))?;
        let machine: &StateMachine = snapshot
            .state_machine(property.subject.as_str())
            .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown subject state machine {}", property.subject)))?;
        if machine.transitions.is_empty() {
            return Err(AdapterError::InvalidRequest(format!("state machine {} declares no transitions", machine.id)));
        }
        Ok(())
    }

    fn execute(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<RunEnvelope, AdapterError> {
        self.validate(snapshot, request)?;
        let property: &Property = snapshot
            .property(request.property_id.as_str())
            .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown property {}", request.property_id)))?;
        let property_digest: ContentHash = snapshot
            .artifact_digest(property.id.as_str())
            .cloned()
            .ok_or_else(|| AdapterError::InvalidRequest(format!("missing digest for property {}", property.id)))?;
        let machine: &StateMachine = snapshot
            .state_machine(property.subject.as_str())
            .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown subject state machine {}", property.subject)))?;
        let capabilities: AdapterCapabilities = self.capabilities();
        let seed: u64 = request.seed.unwrap_or(0);

        if request.budget == 0 {
            return Ok(RunEnvelope {
                run_id: request.run_id.clone(),
                activity_id: request.activity_id.clone(),
                adapter_id: capabilities.adapter_id,
                adapter_version: capabilities.adapter_version,
                engine_name: capabilities.engine_name,
                engine_version: capabilities.engine_version,
                snapshot_id: snapshot.snapshot_id.clone(),
                source_revisions: snapshot.sources.clone(),
                property_id: property.id.clone(),
                claim_ids: request.claim_ids.clone(),
                method: VerificationMethod::PropertyBased,
                scope: property.scope.clone(),
                assumptions: property.assumptions.clone(),
                run_state: RunState::Inconclusive,
                outcome: VerificationOutcome::Evaluation(EvaluationResult::Inconclusive),
                evaluation: None,
                finding: None,
                original_counterexample: None,
                minimized_counterexample: None,
                replay: None,
                limitations: vec!["execution budget exhausted before exploring any cases".to_string()],
                raw_status: "budget_exhausted:0".to_string(),
            });
        }

        let alphabet: Vec<String> = event_alphabet(machine);
        let strategy: VecStrategy<Select<String>> = vec(select(alphabet), 1..=self.max_sequence_len);
        let rng: TestRng = TestRng::from_seed(RngAlgorithm::ChaCha, &seed_bytes(seed));
        let mut runner: TestRunner = TestRunner::new_with_rng(Config::default(), rng);
        let mut last_evaluation: Option<PropertyEvaluation> = None;

        for case_index in 0..request.budget {
            let mut tree: VecValueTree<SelectValueTree<String>> = strategy
                .new_tree(&mut runner)
                .map_err(|reason: proptest::test_runner::Reason| AdapterError::Execution(reason.to_string()))?;
            let events: Vec<String> = tree.current();
            let trace: ExecutionTrace = (self.runner)(machine, &events);
            let replay: ReplayDescriptor = replay_for(Some(seed), events);
            let (evaluation, finding, counterexample): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) =
                evaluate_property(property, &property_digest, &trace, &request.run_id, &snapshot.snapshot_id, replay);
            match evaluation.result {
                EvaluationResult::Satisfied => {
                    last_evaluation = Some(evaluation);
                }
                EvaluationResult::Violated => {
                    let context: ShrinkContext<'_> = ShrinkContext {
                        machine,
                        runner: &self.runner,
                        property,
                        property_digest: &property_digest,
                        run_id: &request.run_id,
                        snapshot_id: &snapshot.snapshot_id,
                        seed,
                    };
                    let minimized: Option<Counterexample> = counterexample
                        .as_ref()
                        .map(|original: &Counterexample| shrink_tree(&mut tree, &context, original));
                    let replay_descriptor: Option<ReplayDescriptor> =
                        minimized.as_ref().or(counterexample.as_ref()).map(|ce: &Counterexample| ce.replay.clone());
                    return Ok(RunEnvelope {
                        run_id: request.run_id.clone(),
                        activity_id: request.activity_id.clone(),
                        adapter_id: capabilities.adapter_id,
                        adapter_version: capabilities.adapter_version,
                        engine_name: capabilities.engine_name,
                        engine_version: capabilities.engine_version,
                        snapshot_id: snapshot.snapshot_id.clone(),
                        source_revisions: snapshot.sources.clone(),
                        property_id: property.id.clone(),
                        claim_ids: request.claim_ids.clone(),
                        method: VerificationMethod::PropertyBased,
                        scope: property.scope.clone(),
                        assumptions: property.assumptions.clone(),
                        run_state: RunState::Completed,
                        outcome: VerificationOutcome::Evaluation(EvaluationResult::Violated),
                        evaluation: Some(evaluation),
                        finding,
                        original_counterexample: counterexample,
                        minimized_counterexample: minimized,
                        replay: replay_descriptor,
                        limitations: vec![format!("violation discovered on generated case {}", case_index + 1)],
                        raw_status: format!("violated_at_case:{}", case_index + 1),
                    });
                }
                EvaluationResult::InvalidEvaluation => {
                    return Ok(RunEnvelope {
                        run_id: request.run_id.clone(),
                        activity_id: request.activity_id.clone(),
                        adapter_id: capabilities.adapter_id,
                        adapter_version: capabilities.adapter_version,
                        engine_name: capabilities.engine_name,
                        engine_version: capabilities.engine_version,
                        snapshot_id: snapshot.snapshot_id.clone(),
                        source_revisions: snapshot.sources.clone(),
                        property_id: property.id.clone(),
                        claim_ids: request.claim_ids.clone(),
                        method: VerificationMethod::PropertyBased,
                        scope: property.scope.clone(),
                        assumptions: property.assumptions.clone(),
                        run_state: RunState::Failed,
                        outcome: VerificationOutcome::Evaluation(EvaluationResult::InvalidEvaluation),
                        evaluation: Some(evaluation),
                        finding: None,
                        original_counterexample: None,
                        minimized_counterexample: None,
                        replay: None,
                        limitations: vec!["property evaluator rejected the predicate or trace as invalid".to_string()],
                        raw_status: "evaluation:invalid_evaluation".to_string(),
                    });
                }
                EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated => {
                    let result: EvaluationResult = evaluation.result;
                    return Ok(RunEnvelope {
                        run_id: request.run_id.clone(),
                        activity_id: request.activity_id.clone(),
                        adapter_id: capabilities.adapter_id,
                        adapter_version: capabilities.adapter_version,
                        engine_name: capabilities.engine_name,
                        engine_version: capabilities.engine_version,
                        snapshot_id: snapshot.snapshot_id.clone(),
                        source_revisions: snapshot.sources.clone(),
                        property_id: property.id.clone(),
                        claim_ids: request.claim_ids.clone(),
                        method: VerificationMethod::PropertyBased,
                        scope: property.scope.clone(),
                        assumptions: property.assumptions.clone(),
                        run_state: RunState::Inconclusive,
                        outcome: VerificationOutcome::Evaluation(result),
                        evaluation: Some(evaluation),
                        finding: None,
                        original_counterexample: None,
                        minimized_counterexample: None,
                        replay: None,
                        limitations: vec!["property evaluator did not establish satisfaction or violation".to_string()],
                        raw_status: format!("evaluation:{result}"),
                    });
                }
            }
        }

        Ok(RunEnvelope {
            run_id: request.run_id.clone(),
            activity_id: request.activity_id.clone(),
            adapter_id: capabilities.adapter_id,
            adapter_version: capabilities.adapter_version,
            engine_name: capabilities.engine_name,
            engine_version: capabilities.engine_version,
            snapshot_id: snapshot.snapshot_id.clone(),
            source_revisions: snapshot.sources.clone(),
            property_id: property.id.clone(),
            claim_ids: request.claim_ids.clone(),
            method: VerificationMethod::PropertyBased,
            scope: property.scope.clone(),
            assumptions: property.assumptions.clone(),
            run_state: RunState::Completed,
            outcome: VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
            evaluation: last_evaluation,
            finding: None,
            original_counterexample: None,
            minimized_counterexample: None,
            replay: Some(seed_metadata(seed)),
            limitations: vec![format!(
                "finite property-based test run of {} generated cases (max length {}); does not prove universal correctness",
                request.budget, self.max_sequence_len
            )],
            raw_status: format!("passed_cases:{}", request.budget),
        })
    }

    fn replay(&self, snapshot: &GraphSnapshot, counterexample: &Counterexample, run_id: &str) -> Result<RunEnvelope, AdapterError> {
        if run_id.trim().is_empty() {
            return Err(AdapterError::InvalidRequest("run_id must not be empty".to_string()));
        }
        if !counterexample.replay.replayable {
            return Err(AdapterError::NotReplayable(format!(
                "counterexample {} does not declare a replayable descriptor",
                counterexample.id
            )));
        }
        if counterexample.replay.engine != "proptest" {
            return Err(AdapterError::NotReplayable(format!(
                "counterexample {} replay engine \"{}\" is incompatible with proptest",
                counterexample.id, counterexample.replay.engine
            )));
        }
        if counterexample.violation.step > 0 && counterexample.replay.events.is_empty() {
            return Err(AdapterError::NotReplayable(format!(
                "counterexample {} has no replay events for violation at step {}",
                counterexample.id, counterexample.violation.step
            )));
        }
        let property: &Property = snapshot
            .property(counterexample.property_id.as_str())
            .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown property {}", counterexample.property_id)))?;
        let property_digest: ContentHash = snapshot
            .artifact_digest(property.id.as_str())
            .cloned()
            .ok_or_else(|| AdapterError::InvalidRequest(format!("missing digest for property {}", property.id)))?;
        let machine: &StateMachine = snapshot
            .state_machine(property.subject.as_str())
            .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown subject state machine {}", property.subject)))?;
        let capabilities: AdapterCapabilities = self.capabilities();
        let trace: ExecutionTrace = (self.runner)(machine, &counterexample.replay.events);
        let (evaluation, finding, replay_ce): (PropertyEvaluation, Option<FindingRecord>, Option<Counterexample>) =
            evaluate_property(property, &property_digest, &trace, run_id, &snapshot.snapshot_id, counterexample.replay.clone());
        let result: EvaluationResult = evaluation.result;
        let run_state: RunState = match result {
            EvaluationResult::Satisfied | EvaluationResult::Violated => RunState::Completed,
            EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated => RunState::Inconclusive,
            EvaluationResult::InvalidEvaluation => RunState::Failed,
        };
        Ok(RunEnvelope {
            run_id: run_id.to_string(),
            activity_id: format!("replay-{}", counterexample.id),
            adapter_id: capabilities.adapter_id,
            adapter_version: capabilities.adapter_version,
            engine_name: capabilities.engine_name,
            engine_version: capabilities.engine_version,
            snapshot_id: snapshot.snapshot_id.clone(),
            source_revisions: snapshot.sources.clone(),
            property_id: property.id.clone(),
            claim_ids: Vec::new(),
            method: VerificationMethod::PropertyBased,
            scope: property.scope.clone(),
            assumptions: property.assumptions.clone(),
            run_state,
            outcome: VerificationOutcome::Evaluation(result),
            evaluation: Some(evaluation),
            finding,
            original_counterexample: replay_ce,
            minimized_counterexample: None,
            replay: Some(counterexample.replay.clone()),
            limitations: vec![format!("deterministic replay of counterexample {}", counterexample.id)],
            raw_status: format!("replay:{result}"),
        })
    }
}

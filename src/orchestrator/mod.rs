mod run;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::graph::{ArtifactId, ContentHash, GraphSnapshot, VerificationClaim, VerificationMethod, VerificationPlan};
use crate::ledger::{EvidenceKind, EvidenceLedger, EvidenceRecord};
use crate::verify::{EvaluationResult, FormalOutcome, RunEnvelope, RunState, VerificationAdapter, VerificationOutcome, VerificationRequest};

pub use run::{OrchestratorError, VerificationRun};

use run::require_timestamp;

#[derive(Default)]
pub struct AdapterRegistry {
    adapters: BTreeMap<VerificationMethod, Box<dyn VerificationAdapter>>,
}

impl fmt::Debug for AdapterRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let methods: Vec<VerificationMethod> = self.adapters.keys().copied().collect();
        f.debug_struct("AdapterRegistry").field("methods", &methods).finish()
    }
}

impl AdapterRegistry {
    /// When a registry is created, `new` shall return an empty `AdapterRegistry`.
    pub fn new() -> AdapterRegistry {
        AdapterRegistry::default()
    }

    /// When an adapter is registered for a method, `register` shall bind that method to the adapter.
    pub fn register(&mut self, method: VerificationMethod, adapter: Box<dyn VerificationAdapter>) {
        self.adapters.insert(method, adapter);
    }

    /// When an adapter is looked up by method, `get` shall return it if registered.
    pub fn get(&self, method: VerificationMethod) -> Option<&dyn VerificationAdapter> {
        self.adapters.get(&method).map(Box::as_ref)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanExecutionReport {
    pub plan_id: ArtifactId,
    pub snapshot_id: ContentHash,
    pub runs: Vec<VerificationRun>,
    pub evidence_ids: Vec<String>,
    pub missing_required_evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VerificationOrchestrator;

fn resolve_plan_targets(snapshot: &GraphSnapshot, plan: &VerificationPlan) -> Vec<(ArtifactId, Vec<ArtifactId>)> {
    let mut target_claims: Vec<&VerificationClaim> = Vec::new();
    let mut seen_claims: BTreeSet<&str> = BTreeSet::new();
    let mut property_order: Vec<ArtifactId> = Vec::new();
    let mut seen_props: BTreeSet<&str> = BTreeSet::new();

    for target_id in &plan.targets {
        if let Some(prop) = snapshot.property(target_id.as_str()) {
            if seen_props.insert(prop.id.as_str()) {
                property_order.push(prop.id.clone());
            }
        }
        if let Some(claim) = snapshot.claim(target_id.as_str()) {
            if seen_claims.insert(claim.id.as_str()) {
                target_claims.push(claim);
            }
        }
    }
    for claim in &target_claims {
        for prop_id in &claim.properties {
            if snapshot.property(prop_id.as_str()).is_some() && seen_props.insert(prop_id.as_str()) {
                property_order.push(prop_id.clone());
            }
        }
    }
    property_order
        .into_iter()
        .map(|prop_id: ArtifactId| {
            let claim_ids: Vec<ArtifactId> = target_claims
                .iter()
                .filter(|claim: &&&VerificationClaim| claim.properties.contains(&prop_id))
                .map(|claim: &&VerificationClaim| claim.id.clone())
                .collect();
            (prop_id, claim_ids)
        })
        .collect()
}

fn mismatch(run_id: &str, why: String) -> OrchestratorError {
    OrchestratorError::EnvelopeMismatch {
        run_id: run_id.to_string(),
        why,
    }
}

fn is_conclusive_outcome(outcome: &VerificationOutcome) -> bool {
    match outcome {
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied | EvaluationResult::Violated)
        | VerificationOutcome::Formal(
            FormalOutcome::Proved | FormalOutcome::Refuted | FormalOutcome::BoundedSuccess { .. } | FormalOutcome::BoundedFailure { .. },
        ) => true,
        VerificationOutcome::Evaluation(EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated | EvaluationResult::InvalidEvaluation)
        | VerificationOutcome::Formal(FormalOutcome::Unknown | FormalOutcome::Timeout | FormalOutcome::Unsupported) => false,
    }
}

fn is_positive_outcome(outcome: &VerificationOutcome) -> bool {
    match outcome {
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied)
        | VerificationOutcome::Formal(FormalOutcome::Proved | FormalOutcome::BoundedSuccess { .. }) => true,
        VerificationOutcome::Evaluation(
            EvaluationResult::Violated | EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated | EvaluationResult::InvalidEvaluation,
        )
        | VerificationOutcome::Formal(
            FormalOutcome::Refuted | FormalOutcome::BoundedFailure { .. } | FormalOutcome::Unknown | FormalOutcome::Timeout | FormalOutcome::Unsupported,
        ) => false,
    }
}

fn validate_run_envelope(envelope: &RunEnvelope, run: &VerificationRun, snapshot: &GraphSnapshot) -> Result<(), OrchestratorError> {
    if envelope.property_id != run.property_id {
        return Err(mismatch(&run.run_id, format!("property {} != {}", envelope.property_id, run.property_id)));
    }
    if envelope.snapshot_id != snapshot.snapshot_id {
        return Err(mismatch(&run.run_id, format!("snapshot {} != {}", envelope.snapshot_id, snapshot.snapshot_id)));
    }
    if envelope.run_id != run.run_id {
        return Err(mismatch(&run.run_id, format!("run_id \"{}\" != \"{}\"", envelope.run_id, run.run_id)));
    }
    if envelope.method != run.method {
        return Err(mismatch(&run.run_id, format!("method {} != {}", envelope.method.as_str(), run.method.as_str())));
    }
    if envelope.activity_id != run.activity_id.as_str() {
        return Err(mismatch(
            &run.run_id,
            format!("activity_id \"{}\" != \"{}\"", envelope.activity_id, run.activity_id),
        ));
    }
    if envelope.claim_ids != run.claim_ids {
        return Err(mismatch(&run.run_id, format!("claim_ids {:?} != {:?}", envelope.claim_ids, run.claim_ids)));
    }
    match envelope.run_state {
        RunState::Completed => {
            if !is_conclusive_outcome(&envelope.outcome) {
                return Err(mismatch(
                    &run.run_id,
                    format!("completed run state paired with non-conclusive outcome {:?}", envelope.outcome),
                ));
            }
            Ok(())
        }
        RunState::Failed | RunState::Cancelled | RunState::Inconclusive => Ok(()),
        RunState::Requested | RunState::Started => Err(mismatch(&run.run_id, format!("non-terminal run state {:?}", envelope.run_state))),
    }
}

fn evidence_kind_for(method: VerificationMethod) -> EvidenceKind {
    match method {
        VerificationMethod::Formal => EvidenceKind::FormalCheck,
        VerificationMethod::ExampleBased => EvidenceKind::StoryRun,
        VerificationMethod::ModelBased | VerificationMethod::PropertyBased | VerificationMethod::DeterministicSimulation => EvidenceKind::PropertyEvaluation,
    }
}

fn next_evidence_id(taken: &BTreeSet<String>, plan: &ArtifactId, act: &ArtifactId, prop: &ArtifactId) -> String {
    let base: String = format!("EV-{plan}-{act}-{prop}");
    if !taken.contains(&base) {
        return base;
    }
    for suffix in 2..=taken.len().saturating_add(1) {
        let candidate: String = format!("{base}-{suffix}");
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    format!("{base}-{}", taken.len().saturating_add(2))
}

fn run_matches_requirement(run: &VerificationRun, required: &str) -> bool {
    let trimmed: &str = required.trim();
    let method: &str = run.method.as_str();
    let qualified: String = format!("{method}:{}", run.property_id);
    trimmed == method || trimmed == run.activity_id.as_str() || trimmed == run.property_id.as_str() || trimmed == qualified
}

fn run_satisfies_positive_evidence(run: &VerificationRun, ledger: &EvidenceLedger) -> bool {
    let completed: bool = match run.state {
        RunState::Completed => true,
        RunState::Requested | RunState::Started | RunState::Failed | RunState::Cancelled | RunState::Inconclusive => false,
    };
    if !completed {
        return false;
    }
    let Some(evidence_id): Option<&str> = run.evidence_id.as_deref() else {
        return false;
    };
    let Some(record): Option<&EvidenceRecord> = ledger.get(evidence_id) else {
        return false;
    };
    record.verify_digest() && is_positive_outcome(&record.outcome)
}

fn requirement_satisfied(runs: &[VerificationRun], ledger: &EvidenceLedger, required: &str) -> bool {
    let matching: Vec<&VerificationRun> = runs.iter().filter(|run: &&VerificationRun| run_matches_requirement(run, required)).collect();
    !matching.is_empty() && matching.iter().all(|run: &&VerificationRun| run_satisfies_positive_evidence(run, ledger))
}

impl VerificationOrchestrator {
    /// When a verification plan is prepared against a snapshot, `prepare_plan` shall return requested runs bound to `snapshot.snapshot_id` and reject unknown plans or plans without target properties or activities.
    pub fn prepare_plan(snapshot: &GraphSnapshot, plan_id: &str) -> Result<Vec<VerificationRun>, OrchestratorError> {
        let plan: &VerificationPlan = snapshot.plan(plan_id).ok_or_else(|| OrchestratorError::UnknownPlan(plan_id.to_string()))?;
        let targets: Vec<(ArtifactId, Vec<ArtifactId>)> = resolve_plan_targets(snapshot, plan);
        if targets.is_empty() {
            return Err(OrchestratorError::EmptyPlanTargets(plan.id.clone()));
        }
        if plan.activities.is_empty() {
            return Err(OrchestratorError::EmptyPlanActivities(plan.id.clone()));
        }
        let mut runs: Vec<VerificationRun> = Vec::new();
        for activity in &plan.activities {
            for (property_id, claim_ids) in &targets {
                runs.push(VerificationRun {
                    run_id: format!("RUN-{}-{}-{}", plan.id, activity.id, property_id),
                    plan_id: plan.id.clone(),
                    activity_id: activity.id.clone(),
                    property_id: property_id.clone(),
                    claim_ids: claim_ids.clone(),
                    method: activity.method,
                    snapshot_id: snapshot.snapshot_id.clone(),
                    state: RunState::Requested,
                    progress_current: 0,
                    progress_total: activity.budget,
                    started_at: None,
                    finished_at: None,
                    evidence_id: None,
                    error: None,
                });
            }
        }
        Ok(runs)
    }

    /// When a verification plan is executed, `execute_plan` shall run each activity through its registered adapter, fail activities without adapters, validate envelopes before sealing into the ledger, and report `required_evidence` unsatisfied unless every matching run completed with a positive outcome.
    pub fn execute_plan(
        snapshot: &GraphSnapshot,
        plan_id: &str,
        registry: &AdapterRegistry,
        ledger: &mut EvidenceLedger,
        timestamp: &str,
        seed: Option<u64>,
    ) -> Result<PlanExecutionReport, OrchestratorError> {
        require_timestamp(timestamp)?;
        let plan: &VerificationPlan = snapshot.plan(plan_id).ok_or_else(|| OrchestratorError::UnknownPlan(plan_id.to_string()))?;
        let mut runs: Vec<VerificationRun> = Self::prepare_plan(snapshot, plan_id)?;
        let mut staged: Vec<EvidenceRecord> = Vec::new();
        let mut taken_ids: BTreeSet<String> = ledger.records().iter().map(|record: &EvidenceRecord| record.id.clone()).collect();
        let mut evidence_ids: Vec<String> = Vec::new();

        for run in &mut runs {
            run.start(timestamp)?;
            let Some(adapter): Option<&dyn VerificationAdapter> = registry.get(run.method) else {
                run.fail(
                    timestamp,
                    format!("no verification adapter registered for method {}", run.method.as_str()),
                    None,
                )?;
                continue;
            };
            let request: VerificationRequest = VerificationRequest {
                run_id: run.run_id.clone(),
                activity_id: run.activity_id.as_str().to_string(),
                property_id: run.property_id.clone(),
                claim_ids: run.claim_ids.clone(),
                budget: run.progress_total,
                seed,
            };
            if let Err(error) = adapter.validate(snapshot, &request) {
                run.fail(timestamp, format!("adapter validation failed: {error}"), None)?;
                continue;
            }
            let envelope: RunEnvelope = match adapter.execute(snapshot, &request) {
                Ok(envelope) => envelope,
                Err(error) => {
                    run.fail(timestamp, format!("adapter execution failed: {error}"), None)?;
                    continue;
                }
            };
            validate_run_envelope(&envelope, run, snapshot)?;
            let ev_id: String = next_evidence_id(&taken_ids, &plan.id, &run.activity_id, &run.property_id);
            let record: EvidenceRecord = EvidenceRecord::from_envelope(
                ev_id.clone(),
                evidence_kind_for(envelope.method),
                timestamp.to_string(),
                snapshot,
                &envelope,
                BTreeMap::new(),
            )
            .map_err(OrchestratorError::Ledger)?;
            ledger.validate_new_record(&record).map_err(OrchestratorError::Ledger)?;
            taken_ids.insert(ev_id.clone());
            staged.push(record);
            evidence_ids.push(ev_id.clone());

            match envelope.run_state {
                RunState::Completed => {
                    run.advance_progress(run.progress_total)?;
                    run.complete(timestamp, Some(ev_id))?;
                }
                RunState::Failed => run.fail(timestamp, format!("adapter failed: {}", envelope.raw_status), Some(ev_id))?,
                RunState::Cancelled => run.cancel(timestamp, Some(ev_id), Some(format!("adapter cancelled: {}", envelope.raw_status)))?,
                RunState::Inconclusive => run.mark_inconclusive(timestamp, Some(ev_id), Some(format!("adapter inconclusive: {}", envelope.raw_status)))?,
                RunState::Requested | RunState::Started => return Err(mismatch(&run.run_id, format!("non-terminal {:?}", envelope.run_state))),
            }
        }

        for record in staged {
            ledger.append_record(record).map_err(OrchestratorError::Ledger)?;
        }

        let missing_required_evidence: Vec<String> = plan
            .required_evidence
            .iter()
            .filter(|req: &&String| !requirement_satisfied(&runs, ledger, req))
            .cloned()
            .collect();

        Ok(PlanExecutionReport {
            plan_id: plan.id.clone(),
            snapshot_id: snapshot.snapshot_id.clone(),
            runs,
            evidence_ids,
            missing_required_evidence,
        })
    }
}

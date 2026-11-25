use crate::assurance::{ClaimAssessment, EvidenceEvaluationEntry};
use crate::corpus::{CorpusError, CounterexampleRegistry};
use crate::graph::{ArtifactId, ContentHash, GraphSnapshot, Property, VerificationClaim, VerificationMethod};
use crate::ledger::LedgerError;
use crate::ledger_fs::FsLedger;
use crate::planner::{ExplorationRecommendation, RecommendationReason};
use crate::verify::{ClaimStatus, Counterexample, EvaluationResult, FormalOutcome, VerificationOutcome};

fn read_counterexample_blob(fs_ledger: &FsLedger, digest: &ContentHash) -> Result<Counterexample, String> {
    let bytes: Vec<u8> = fs_ledger.read_blob(digest).map_err(|error: LedgerError| error.to_string())?;
    serde_json::from_slice::<Counterexample>(&bytes).map_err(|error: serde_json::Error| format!("corrupt counterexample blob {digest}: {error}"))
}

pub(crate) fn hydrate_corpus(snapshot: &GraphSnapshot, fs_ledger: &FsLedger) -> Result<CounterexampleRegistry, String> {
    let mut originals: Vec<Counterexample> = Vec::new();
    let mut minimized: Vec<Counterexample> = Vec::new();
    for record in fs_ledger.ledger().records() {
        for key in ["original_counterexample", "counterexample"] {
            if let Some(digest) = record.artifact_digests.get(key) {
                let counterexample: Counterexample = read_counterexample_blob(fs_ledger, digest)?;
                if counterexample.original_id.is_none() && counterexample.minimization.is_none() {
                    originals.push(counterexample);
                } else {
                    minimized.push(counterexample);
                }
            }
        }
    }
    let mut corpus: CounterexampleRegistry = CounterexampleRegistry::new();
    for original in originals {
        match corpus.register_original(original) {
            Ok(()) => {}
            Err(
                CorpusError::DuplicateId(_)
                | CorpusError::EmptyId
                | CorpusError::MissingOriginal(_)
                | CorpusError::InvalidMinimized(_)
                | CorpusError::ModelMismatch(_)
                | CorpusError::ReplayFailed(_),
            ) => {}
        }
    }
    for item in minimized {
        match corpus.register_minimized(snapshot, item) {
            Ok(()) => {}
            Err(
                CorpusError::DuplicateId(_)
                | CorpusError::EmptyId
                | CorpusError::MissingOriginal(_)
                | CorpusError::InvalidMinimized(_)
                | CorpusError::ModelMismatch(_)
                | CorpusError::ReplayFailed(_),
            ) => {}
        }
    }
    Ok(corpus)
}

fn is_negative_outcome(outcome: &VerificationOutcome) -> bool {
    match outcome {
        VerificationOutcome::Evaluation(EvaluationResult::Violated)
        | VerificationOutcome::Formal(FormalOutcome::Refuted | FormalOutcome::BoundedFailure { .. }) => true,
        VerificationOutcome::Evaluation(
            EvaluationResult::Satisfied | EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated | EvaluationResult::InvalidEvaluation,
        )
        | VerificationOutcome::Formal(
            FormalOutcome::Proved | FormalOutcome::BoundedSuccess { .. } | FormalOutcome::Unknown | FormalOutcome::Timeout | FormalOutcome::Unsupported,
        ) => false,
    }
}

fn preferred_method(snapshot: &GraphSnapshot, claim_id: &ArtifactId) -> VerificationMethod {
    if let Some(claim) = snapshot.claim(claim_id.as_str()) {
        for prop_id in &claim.properties {
            if let Some(prop) = snapshot.property(prop_id.as_str()) {
                let property: &Property = prop;
                if let Some(method) = property.methods.first().copied() {
                    return method;
                }
            }
        }
    }
    VerificationMethod::PropertyBased
}

pub(crate) fn append_refuted_fallbacks(snapshot: &GraphSnapshot, assessments: &[ClaimAssessment], recommendations: &mut Vec<ExplorationRecommendation>) {
    let mut refuted: Vec<&ClaimAssessment> = assessments
        .iter()
        .filter(|assessment: &&ClaimAssessment| match assessment.status {
            ClaimStatus::Refuted => true,
            ClaimStatus::Supported | ClaimStatus::Inconclusive | ClaimStatus::RequiresReview | ClaimStatus::Stale | ClaimStatus::Invalid => false,
        })
        .collect();
    refuted.sort_by(|left: &&ClaimAssessment, right: &&ClaimAssessment| left.claim_id.cmp(&right.claim_id));

    for assessment in refuted {
        let claim_props: Vec<ArtifactId> = snapshot
            .claim(assessment.claim_id.as_str())
            .map(|claim: &VerificationClaim| claim.properties.clone())
            .unwrap_or_default();
        let covered: bool = recommendations.iter().any(|rec: &ExplorationRecommendation| {
            let is_regression: bool = match rec.reason {
                RecommendationReason::RegressionCounterexample => true,
                RecommendationReason::UncoveredStates
                | RecommendationReason::UncoveredTransitions
                | RecommendationReason::UncoveredTransitionPairs
                | RecommendationReason::StaleEvidence
                | RecommendationReason::UnresolvedContradiction
                | RecommendationReason::UnverifiedClaim => false,
            };
            is_regression && (rec.target_id == assessment.claim_id || claim_props.contains(&rec.target_id))
        });
        if covered {
            continue;
        }
        let mut items: Vec<String> = assessment
            .evaluated
            .iter()
            .filter(|entry: &&EvidenceEvaluationEntry| is_negative_outcome(&entry.outcome))
            .map(|entry: &EvidenceEvaluationEntry| entry.evidence_id.clone())
            .collect();
        items.sort();
        items.dedup();
        if items.is_empty() {
            items.push(assessment.claim_id.to_string());
        }
        let next_index: usize = recommendations.len() + 1;
        let rationale: String = format!(
            "claim {} is refuted by current evidence ({}) and requires counterexample investigation and regression verification",
            assessment.claim_id,
            items.join(", ")
        );
        recommendations.push(ExplorationRecommendation {
            id: format!("REC-{next_index:03}"),
            method: preferred_method(snapshot, &assessment.claim_id),
            target_id: assessment.claim_id.clone(),
            reason: RecommendationReason::RegressionCounterexample,
            rationale,
            items,
        });
    }
}

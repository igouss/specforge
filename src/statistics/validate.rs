use std::collections::BTreeSet;

use crate::assurance::{freshness_of, EvidenceFreshness};
use crate::graph::{GraphSnapshot, VerificationClaim, VerificationMethod};
use crate::ledger::{EvidenceLedger, EvidenceRecord};
use crate::verify::{EvaluationResult, FormalOutcome, RunState, VerificationOutcome};

use super::{SamplingModel, StatisticalError, StatisticalMethod, StatisticalSample};

pub(super) fn validate_model(model: &SamplingModel, claim: &VerificationClaim) -> Result<(), StatisticalError> {
    if model.hypothesis.trim().is_empty() {
        return Err(StatisticalError::MissingAssumptions("hypothesis must not be empty".to_string()));
    }
    if model.sampling_population.trim().is_empty() {
        return Err(StatisticalError::MissingAssumptions("sampling_population must not be empty".to_string()));
    }
    if model.sampling_process.trim().is_empty() {
        return Err(StatisticalError::MissingAssumptions("sampling_process must not be empty".to_string()));
    }
    if model.scope.trim().is_empty() {
        return Err(StatisticalError::MissingAssumptions("scope must not be empty".to_string()));
    }
    if model.independence_assumptions.is_empty() || model.independence_assumptions.iter().any(|assumption: &String| assumption.trim().is_empty()) {
        return Err(StatisticalError::MissingAssumptions(
            "independence_assumptions must be non-empty and contain no blank entries".to_string(),
        ));
    }
    if model.limitations.is_empty() || model.limitations.iter().any(|item: &String| item.trim().is_empty()) {
        return Err(StatisticalError::MissingAssumptions(
            "limitations must be non-empty and contain no blank entries".to_string(),
        ));
    }
    if model.scope != claim.scope {
        return Err(StatisticalError::ScopeMismatch {
            model_scope: model.scope.clone(),
            claim_scope: claim.scope.clone(),
        });
    }
    match model.method {
        StatisticalMethod::WilsonScore { confidence: _ } => Ok(()),
        StatisticalMethod::BetaBinomial {
            prior_alpha,
            prior_beta,
            confidence: _,
        } => {
            if !prior_alpha.is_finite() || prior_alpha <= 0.0 {
                return Err(StatisticalError::InvalidModel(
                    "BetaBinomial prior_alpha must be finite and strictly positive".to_string(),
                ));
            }
            if !prior_beta.is_finite() || prior_beta <= 0.0 {
                return Err(StatisticalError::InvalidModel(
                    "BetaBinomial prior_beta must be finite and strictly positive".to_string(),
                ));
            }
            if !(prior_alpha + prior_beta).is_finite() {
                return Err(StatisticalError::InvalidModel(
                    "BetaBinomial prior parameters overflow finite arithmetic".to_string(),
                ));
            }
            Ok(())
        }
    }
}

pub(super) fn validate_sample(sample: &StatisticalSample) -> Result<(), StatisticalError> {
    if sample.sample_size == 0 {
        return Err(StatisticalError::InvalidSample("sample_size must be greater than zero".to_string()));
    }
    if sample.observed_successes.checked_add(sample.observed_failures) != Some(sample.sample_size) {
        return Err(StatisticalError::InvalidSample(
            "observed_successes + observed_failures must equal sample_size".to_string(),
        ));
    }
    if sample.evidence_ids.is_empty() {
        return Err(StatisticalError::InvalidSample(
            "evidence_ids must reference at least one evidence record".to_string(),
        ));
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for evidence_id in &sample.evidence_ids {
        if evidence_id.trim().is_empty() {
            return Err(StatisticalError::InvalidSample("evidence_ids must not contain blank entries".to_string()));
        }
        if !seen.insert(evidence_id.as_str()) {
            return Err(StatisticalError::InvalidSample(format!(
                "duplicate evidence identifier \"{evidence_id}\" in sample"
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_evidence(
    snapshot: &GraphSnapshot,
    ledger: &EvidenceLedger,
    claim: &VerificationClaim,
    sample: &StatisticalSample,
) -> Result<(), StatisticalError> {
    let mut expected_method: Option<VerificationMethod> = None;
    for evidence_id in &sample.evidence_ids {
        let record: &EvidenceRecord = ledger
            .get(evidence_id)
            .ok_or_else(|| StatisticalError::UnverifiedEvidence(format!("evidence record {evidence_id} not found in ledger")))?;
        if !record.verify_digest() {
            return Err(StatisticalError::UnverifiedEvidence(format!(
                "evidence record {} failed content digest verification",
                record.id
            )));
        }
        match freshness_of(record, snapshot, ledger) {
            EvidenceFreshness::Current => {}
            EvidenceFreshness::Stale { changed_dependencies } => {
                return Err(StatisticalError::UnverifiedEvidence(format!(
                    "evidence record {} is stale (changed dependencies: {})",
                    record.id,
                    changed_dependencies.join(", ")
                )));
            }
            EvidenceFreshness::Invalidated { by } => {
                return Err(StatisticalError::UnverifiedEvidence(format!(
                    "evidence record {} was invalidated by {by}",
                    record.id
                )));
            }
            EvidenceFreshness::Superseded { by } => {
                return Err(StatisticalError::UnverifiedEvidence(format!(
                    "evidence record {} was superseded by {by}",
                    record.id
                )));
            }
        }
        if !record.claim_ids.contains(&claim.id) {
            return Err(StatisticalError::UnverifiedEvidence(format!(
                "evidence record {} is not linked to claim {}",
                record.id, claim.id
            )));
        }
        if record.scope != claim.scope {
            return Err(StatisticalError::UnverifiedEvidence(format!(
                "evidence record {} scope \"{}\" does not match claim scope \"{}\"",
                record.id, record.scope, claim.scope
            )));
        }
        for assumption in &record.assumptions {
            if !claim.assumptions.contains(assumption) {
                return Err(StatisticalError::UnverifiedEvidence(format!(
                    "evidence record {} relies on undeclared assumption {} not declared on claim {}",
                    record.id, assumption, claim.id
                )));
            }
        }
        if record.source_revisions.is_empty() {
            return Err(StatisticalError::UnverifiedEvidence(format!(
                "evidence record {} is missing source_revisions provenance",
                record.id
            )));
        }
        match record.run_state {
            RunState::Completed => {}
            RunState::Requested | RunState::Started | RunState::Failed | RunState::Cancelled | RunState::Inconclusive => {
                return Err(StatisticalError::UnverifiedEvidence(format!(
                    "evidence record {} has non-completed run state {:?}",
                    record.id, record.run_state
                )));
            }
        }
        let formal_method: bool = match record.method {
            VerificationMethod::Formal => true,
            VerificationMethod::ExampleBased
            | VerificationMethod::ModelBased
            | VerificationMethod::PropertyBased
            | VerificationMethod::DeterministicSimulation => false,
        };
        if formal_method {
            return Err(StatisticalError::FormalProofDisallowed(format!(
                "evidence record {} uses formal verification and cannot be converted into a probability",
                record.id
            )));
        }
        match &record.outcome {
            VerificationOutcome::Formal(
                FormalOutcome::Proved
                | FormalOutcome::Refuted
                | FormalOutcome::BoundedSuccess { .. }
                | FormalOutcome::BoundedFailure { .. }
                | FormalOutcome::Unknown
                | FormalOutcome::Timeout
                | FormalOutcome::Unsupported,
            ) => {
                return Err(StatisticalError::FormalProofDisallowed(format!(
                    "evidence record {} carries formal verification results and cannot be converted into a probability",
                    record.id
                )));
            }
            VerificationOutcome::Evaluation(EvaluationResult::Satisfied) => {}
            VerificationOutcome::Evaluation(EvaluationResult::Violated) => {
                if sample.observed_failures == 0 {
                    return Err(StatisticalError::InvalidSample(format!(
                        "evidence record {} recorded a violation while sample declares zero observed failures",
                        record.id
                    )));
                }
            }
            VerificationOutcome::Evaluation(
                result @ (EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated | EvaluationResult::InvalidEvaluation),
            ) => {
                return Err(StatisticalError::UnverifiedEvidence(format!(
                    "evidence record {} has non-conclusive evaluation outcome {result}",
                    record.id
                )));
            }
        }
        match expected_method {
            None => {
                expected_method = Some(record.method);
            }
            Some(previous) => {
                if record.method != previous {
                    return Err(StatisticalError::HeterogeneousEvidenceDisallowed(format!(
                        "evidence record {} uses {:?} while prior evidence uses {:?}",
                        record.id, record.method, previous
                    )));
                }
            }
        }
    }
    Ok(())
}

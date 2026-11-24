use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::assurance::{freshness_of, ClaimAssessment, Contradiction, EvidenceEvaluationEntry, EvidenceFreshness};
use crate::corpus::CounterexampleRegistry;
use crate::coverage::{CoverageRecord, TransitionPair};
use crate::graph::{ArtifactId, GraphSnapshot, Property, VerificationClaim, VerificationMethod};
use crate::ledger::{EvidenceKind, EvidenceLedger, EvidenceRecord};
use crate::verify::{ClaimStatus, Counterexample, EvaluationResult, FormalOutcome, RunState, VerificationOutcome};

/// When the exploration planner identifies a verification gap or follow-up need, `RecommendationReason` shall classify why an activity is recommended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationReason {
    UncoveredStates,
    UncoveredTransitions,
    UncoveredTransitionPairs,
    StaleEvidence,
    UnresolvedContradiction,
    UnverifiedClaim,
    RegressionCounterexample,
}

impl fmt::Display for RecommendationReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label: &str = match self {
            RecommendationReason::UncoveredStates => "uncovered_states",
            RecommendationReason::UncoveredTransitions => "uncovered_transitions",
            RecommendationReason::UncoveredTransitionPairs => "uncovered_transition_pairs",
            RecommendationReason::StaleEvidence => "stale_evidence",
            RecommendationReason::UnresolvedContradiction => "unresolved_contradiction",
            RecommendationReason::UnverifiedClaim => "unverified_claim",
            RecommendationReason::RegressionCounterexample => "regression_counterexample",
        };
        formatter.write_str(label)
    }
}

/// When the exploration planner suggests a verification activity, `ExplorationRecommendation` shall specify its identifier, method, target artifact, reason, explanation, and affected items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplorationRecommendation {
    pub id: String,
    pub method: VerificationMethod,
    pub target_id: ArtifactId,
    pub reason: RecommendationReason,
    pub rationale: String,
    pub items: Vec<String>,
}

fn preferred_method_for_claim(snapshot: &GraphSnapshot, claim_id: &ArtifactId) -> VerificationMethod {
    let Some(claim): Option<&VerificationClaim> = snapshot.claim(claim_id.as_str()) else {
        return VerificationMethod::PropertyBased;
    };
    for property_id in &claim.properties {
        if let Some(property) = snapshot.property(property_id.as_str()) {
            let prop: &Property = property;
            if let Some(method) = prop.methods.first().copied() {
                return method;
            }
        }
    }
    VerificationMethod::PropertyBased
}

fn stale_evidence_ids(assessment: &ClaimAssessment) -> Vec<String> {
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for entry in &assessment.evaluated {
        match entry.freshness {
            EvidenceFreshness::Stale { .. } => {
                ids.insert(entry.evidence_id.clone());
            }
            EvidenceFreshness::Current | EvidenceFreshness::Invalidated { .. } | EvidenceFreshness::Superseded { .. } => {}
        }
    }
    ids.into_iter().collect()
}

fn unverified_items(snapshot: &GraphSnapshot, assessment: &ClaimAssessment) -> Vec<String> {
    let mut items: BTreeSet<String> = BTreeSet::new();
    if let Some(claim) = snapshot.claim(assessment.claim_id.as_str()) {
        for property_id in &claim.properties {
            items.insert(property_id.to_string());
        }
    }
    for entry in &assessment.evaluated {
        let entry_ref: &EvidenceEvaluationEntry = entry;
        match &entry_ref.outcome {
            VerificationOutcome::Evaluation(EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated | EvaluationResult::InvalidEvaluation)
            | VerificationOutcome::Formal(FormalOutcome::Unknown | FormalOutcome::Timeout | FormalOutcome::Unsupported) => {
                items.insert(entry_ref.evidence_id.clone());
            }
            VerificationOutcome::Evaluation(EvaluationResult::Satisfied | EvaluationResult::Violated)
            | VerificationOutcome::Formal(
                FormalOutcome::Proved | FormalOutcome::Refuted | FormalOutcome::BoundedSuccess { .. } | FormalOutcome::BoundedFailure { .. },
            ) => {}
        }
    }
    if items.is_empty() {
        items.insert(assessment.claim_id.to_string());
    }
    items.into_iter().collect()
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

fn is_counterexample_resolved(target: &Counterexample, snapshot: &GraphSnapshot, ledger: &EvidenceLedger) -> bool {
    let expected_activity: String = format!("replay-{}", target.id);
    let mut saw_valid_current: bool = false;
    let mut latest_is_positive: bool = false;

    for record in ledger.records() {
        let rec: &EvidenceRecord = record;
        let is_replay: bool = match rec.kind {
            EvidenceKind::Replay => true,
            EvidenceKind::PropertyEvaluation | EvidenceKind::Counterexample | EvidenceKind::StoryRun | EvidenceKind::FormalCheck => false,
        };
        if !is_replay {
            continue;
        }
        let matches_target: bool = rec.activity_id == expected_activity || rec.counterexample_id.as_deref() == Some(target.id.as_str());
        if !matches_target || rec.property_id.as_ref() != Some(&target.property_id) {
            continue;
        }
        if !rec.verify_digest() {
            return false;
        }
        let current: bool = match freshness_of(rec, snapshot, ledger) {
            EvidenceFreshness::Current => true,
            EvidenceFreshness::Stale { .. } | EvidenceFreshness::Invalidated { .. } | EvidenceFreshness::Superseded { .. } => false,
        };
        if !current {
            continue;
        }
        if is_negative_outcome(&rec.outcome) {
            return false;
        }
        let completed: bool = match rec.run_state {
            RunState::Completed => true,
            RunState::Requested | RunState::Started | RunState::Failed | RunState::Cancelled | RunState::Inconclusive => false,
        };
        saw_valid_current = true;
        latest_is_positive = completed && is_positive_outcome(&rec.outcome);
    }

    saw_valid_current && latest_is_positive
}

fn claim_status_rank(status: ClaimStatus) -> u8 {
    match status {
        ClaimStatus::Supported => 0,
        ClaimStatus::Refuted => 1,
        ClaimStatus::Inconclusive => 2,
        ClaimStatus::RequiresReview => 3,
        ClaimStatus::Stale => 4,
        ClaimStatus::Invalid => 5,
    }
}

fn push_recommendation(
    recommendations: &mut Vec<ExplorationRecommendation>,
    method: VerificationMethod,
    target_id: ArtifactId,
    reason: RecommendationReason,
    rationale: String,
    items: Vec<String>,
) {
    let next_index: usize = recommendations.len() + 1;
    recommendations.push(ExplorationRecommendation {
        id: format!("REC-{next_index:03}"),
        method,
        target_id,
        reason,
        rationale,
        items,
    });
}

/// When coverage records, claim assessments, and a counterexample corpus are inspected, `recommend_activities` shall deterministically emit read-only, explained verification recommendations without mutating the snapshot, ledger, or corpus.
pub fn recommend_activities(
    snapshot: &GraphSnapshot,
    ledger: &EvidenceLedger,
    coverage: &[CoverageRecord],
    assessments: &[ClaimAssessment],
    corpus: &CounterexampleRegistry,
) -> Vec<ExplorationRecommendation> {
    let mut recommendations: Vec<ExplorationRecommendation> = Vec::new();

    let mut sorted_coverage: Vec<&CoverageRecord> = coverage.iter().collect();
    sorted_coverage
        .sort_by(|left: &&CoverageRecord, right: &&CoverageRecord| left.model_id.cmp(&right.model_id).then_with(|| left.model_digest.cmp(&right.model_digest)));

    for record in sorted_coverage {
        if snapshot.state_machine(record.model_id.as_str()).is_none() {
            continue;
        }
        if snapshot.artifact_digest(record.model_id.as_str()) != Some(&record.model_digest) {
            let stale_digest: String = record.model_digest.to_string();
            let rationale: String = format!(
                "coverage record for state machine {} references stale model digest {} and must be recomputed against the current snapshot",
                record.model_id, stale_digest
            );
            push_recommendation(
                &mut recommendations,
                VerificationMethod::ModelBased,
                record.model_id.clone(),
                RecommendationReason::StaleEvidence,
                rationale,
                vec![stale_digest],
            );
            continue;
        }
        if !record.uncovered_states.is_empty() {
            let mut items: Vec<String> = record.uncovered_states.clone();
            items.sort();
            items.dedup();
            let rationale: String = format!("state machine {} has {} uncovered state(s): {}", record.model_id, items.len(), items.join(", "));
            push_recommendation(
                &mut recommendations,
                VerificationMethod::ModelBased,
                record.model_id.clone(),
                RecommendationReason::UncoveredStates,
                rationale,
                items,
            );
        }
        if !record.uncovered_transitions.is_empty() {
            let mut items: Vec<String> = record.uncovered_transitions.clone();
            items.sort();
            items.dedup();
            let rationale: String = format!(
                "state machine {} has {} uncovered transition(s): {}",
                record.model_id,
                items.len(),
                items.join(", ")
            );
            push_recommendation(
                &mut recommendations,
                VerificationMethod::ModelBased,
                record.model_id.clone(),
                RecommendationReason::UncoveredTransitions,
                rationale,
                items,
            );
        }
        if !record.uncovered_transition_pairs.is_empty() {
            let mut pairs: Vec<TransitionPair> = record.uncovered_transition_pairs.clone();
            pairs.sort();
            pairs.dedup();
            let items: Vec<String> = pairs
                .into_iter()
                .map(|pair: TransitionPair| format!("{} => {}", pair.first, pair.second))
                .collect();
            let rationale: String = format!(
                "state machine {} has {} uncovered transition pair(s): {}",
                record.model_id,
                items.len(),
                items.join(", ")
            );
            push_recommendation(
                &mut recommendations,
                VerificationMethod::ModelBased,
                record.model_id.clone(),
                RecommendationReason::UncoveredTransitionPairs,
                rationale,
                items,
            );
        }
    }

    let mut sorted_assessments: Vec<&ClaimAssessment> = assessments.iter().collect();
    sorted_assessments.sort_by(|left: &&ClaimAssessment, right: &&ClaimAssessment| {
        left.claim_id
            .cmp(&right.claim_id)
            .then_with(|| claim_status_rank(left.status).cmp(&claim_status_rank(right.status)))
            .then_with(|| left.rationale.cmp(&right.rationale))
    });

    for assessment in sorted_assessments {
        let method: VerificationMethod = preferred_method_for_claim(snapshot, &assessment.claim_id);
        match assessment.status {
            ClaimStatus::Stale => {
                let mut items: Vec<String> = stale_evidence_ids(assessment);
                let rationale: String = if items.is_empty() {
                    items.push(assessment.claim_id.to_string());
                    format!("claim {} is stale and requires fresh verification", assessment.claim_id)
                } else {
                    format!(
                        "claim {} is stale and requires re-verification for evidence: {}",
                        assessment.claim_id,
                        items.join(", ")
                    )
                };
                push_recommendation(
                    &mut recommendations,
                    method,
                    assessment.claim_id.clone(),
                    RecommendationReason::StaleEvidence,
                    rationale,
                    items,
                );
            }
            ClaimStatus::RequiresReview => {
                let mut items: Vec<String> = assessment
                    .contradictions
                    .iter()
                    .map(|item: &Contradiction| format!("{} vs {}", item.supporting_id, item.contradicting_id))
                    .collect();
                items.sort();
                items.dedup();
                let rationale: String = if items.is_empty() {
                    items.push(assessment.claim_id.to_string());
                    format!("claim {} requires review due to unresolved evidence conflicts", assessment.claim_id)
                } else {
                    format!(
                        "claim {} has {} unresolved evidence contradiction(s): {}",
                        assessment.claim_id,
                        items.len(),
                        items.join(", ")
                    )
                };
                push_recommendation(
                    &mut recommendations,
                    method,
                    assessment.claim_id.clone(),
                    RecommendationReason::UnresolvedContradiction,
                    rationale,
                    items,
                );
            }
            ClaimStatus::Inconclusive | ClaimStatus::Invalid => {
                let items: Vec<String> = unverified_items(snapshot, assessment);
                let rationale: String = format!(
                    "claim {} is {} and needs current verification covering: {}",
                    assessment.claim_id,
                    assessment.status,
                    items.join(", ")
                );
                push_recommendation(
                    &mut recommendations,
                    method,
                    assessment.claim_id.clone(),
                    RecommendationReason::UnverifiedClaim,
                    rationale,
                    items,
                );
            }
            ClaimStatus::Supported | ClaimStatus::Refuted => {}
        }
    }

    let mut originals: Vec<&Counterexample> = corpus.originals();
    originals.sort_by(|left: &&Counterexample, right: &&Counterexample| left.id.cmp(&right.id).then_with(|| left.property_id.cmp(&right.property_id)));

    for original in originals {
        let minimized_children: Vec<&Counterexample> = corpus.minimized_for(&original.id);
        let target: &Counterexample = minimized_children.last().copied().unwrap_or(original);
        if is_counterexample_resolved(target, snapshot, ledger) {
            continue;
        }
        let rationale: String = if target.replay.replayable {
            format!("replay active regression counterexample {} against property {}", target.id, target.property_id)
        } else {
            format!(
                "investigate active non-replayable counterexample {} against property {} and restore deterministic replay",
                target.id, target.property_id
            )
        };
        push_recommendation(
            &mut recommendations,
            VerificationMethod::PropertyBased,
            target.property_id.clone(),
            RecommendationReason::RegressionCounterexample,
            rationale,
            vec![target.id.clone()],
        );
    }

    recommendations
}

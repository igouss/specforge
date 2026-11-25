use std::collections::BTreeSet;

use super::{EvidenceFreshness, ImpactSet};
use crate::graph::{ArtifactId, ContentHash, DependencyPin, GraphSnapshot, VerificationClaim};
use crate::ledger::{EvidenceLedger, EvidenceLink, EvidenceRecord, EvidenceRelation};
use crate::verify::{EvaluationResult, FormalOutcome, RunState, VerificationOutcome};

pub(super) fn is_proved_outcome(outcome: &VerificationOutcome) -> bool {
    match outcome {
        VerificationOutcome::Formal(FormalOutcome::Proved) => true,
        VerificationOutcome::Evaluation(
            EvaluationResult::Satisfied
            | EvaluationResult::Violated
            | EvaluationResult::Inconclusive
            | EvaluationResult::NotEvaluated
            | EvaluationResult::InvalidEvaluation,
        )
        | VerificationOutcome::Formal(
            FormalOutcome::Refuted
            | FormalOutcome::BoundedSuccess { .. }
            | FormalOutcome::BoundedFailure { .. }
            | FormalOutcome::Unknown
            | FormalOutcome::Timeout
            | FormalOutcome::Unsupported,
        ) => false,
    }
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

fn is_inconclusive_or_invalid_outcome(outcome: &VerificationOutcome) -> bool {
    match outcome {
        VerificationOutcome::Evaluation(EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated | EvaluationResult::InvalidEvaluation)
        | VerificationOutcome::Formal(FormalOutcome::Unknown | FormalOutcome::Timeout | FormalOutcome::Unsupported) => true,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied | EvaluationResult::Violated)
        | VerificationOutcome::Formal(
            FormalOutcome::Proved | FormalOutcome::Refuted | FormalOutcome::BoundedSuccess { .. } | FormalOutcome::BoundedFailure { .. },
        ) => false,
    }
}

fn link_can_override(link: &EvidenceLink, target: &EvidenceRecord, ledger: &EvidenceLedger) -> bool {
    if link.from_id == target.id || !link.verify_digest() {
        return false;
    }
    let Some(source): Option<&EvidenceRecord> = ledger.get(&link.from_id) else {
        return false;
    };
    if !source.verify_digest() {
        return false;
    }
    let completed: bool = match source.run_state {
        RunState::Completed => true,
        RunState::Requested | RunState::Started | RunState::Failed | RunState::Cancelled | RunState::Inconclusive => false,
    };
    if is_negative_outcome(&target.outcome) && (!completed || is_inconclusive_or_invalid_outcome(&source.outcome)) {
        return false;
    }
    true
}

pub(super) fn missing_pins(record: &EvidenceRecord, required: &[DependencyPin]) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    for pin in required {
        let covered: bool = record
            .dependencies
            .iter()
            .any(|recorded: &DependencyPin| recorded.artifact_id == pin.artifact_id && recorded.digest == pin.digest);
        if !covered {
            missing.push(pin.artifact_id.clone());
        }
    }
    missing
}

pub(super) fn is_claim_link_relation(relation: EvidenceRelation) -> bool {
    match relation {
        EvidenceRelation::Supports | EvidenceRelation::Contradicts => true,
        EvidenceRelation::Corroborates | EvidenceRelation::DerivedFrom | EvidenceRelation::Supersedes | EvidenceRelation::Invalidates => false,
    }
}

pub(super) fn relevant_records<'a>(claim: &VerificationClaim, ledger: &'a EvidenceLedger) -> Vec<&'a EvidenceRecord> {
    let linked_ids: BTreeSet<&str> = ledger
        .links()
        .iter()
        .filter(|link: &&EvidenceLink| link.to_id == claim.id.as_str() && is_claim_link_relation(link.relation))
        .map(|link: &EvidenceLink| link.from_id.as_str())
        .collect();
    ledger
        .records()
        .iter()
        .filter(|record: &&EvidenceRecord| record.claim_ids.contains(&claim.id) || linked_ids.contains(record.id.as_str()))
        .collect()
}

/// When an evidence record's freshness is checked against a snapshot and ledger links, `freshness_of` shall distinguish invalidated, superseded, stale, and current evidence.
pub fn freshness_of(record: &EvidenceRecord, snapshot: &GraphSnapshot, ledger: &EvidenceLedger) -> EvidenceFreshness {
    if let Some(link) = ledger
        .links()
        .iter()
        .find(|link: &&EvidenceLink| link.to_id == record.id && link.relation == EvidenceRelation::Invalidates && link_can_override(link, record, ledger))
    {
        return EvidenceFreshness::Invalidated { by: link.from_id.clone() };
    }
    if let Some(link) = ledger
        .links()
        .iter()
        .find(|link: &&EvidenceLink| link.to_id == record.id && link.relation == EvidenceRelation::Supersedes && link_can_override(link, record, ledger))
    {
        return EvidenceFreshness::Superseded { by: link.from_id.clone() };
    }
    if record.dependencies.is_empty() {
        return EvidenceFreshness::Stale {
            changed_dependencies: vec!["<unpinned>".to_string()],
        };
    }
    let mut changed: Vec<String> = Vec::new();
    for pin in &record.dependencies {
        let matches: bool = snapshot
            .artifact_digest(&pin.artifact_id)
            .is_some_and(|current: &ContentHash| *current == pin.digest);
        if !matches {
            changed.push(pin.artifact_id.clone());
        }
    }
    for claim_id in &record.claim_ids {
        let required: Vec<DependencyPin> = snapshot.dependency_pins_for_claim(claim_id.as_str());
        changed.extend(missing_pins(record, &required));
    }
    if let Some(property_id) = &record.property_id {
        let required: Vec<DependencyPin> = snapshot.dependency_pins_for_property(property_id.as_str());
        changed.extend(missing_pins(record, &required));
    }
    if changed.is_empty() {
        EvidenceFreshness::Current
    } else {
        changed.sort();
        changed.dedup();
        EvidenceFreshness::Stale { changed_dependencies: changed }
    }
}

/// When a snapshot changes from `previous` to `current`, `impact_set` shall report the changed artifacts, stale evidence records, and affected claims while preserving historical records.
pub fn impact_set(previous: &GraphSnapshot, current: &GraphSnapshot, ledger: &EvidenceLedger) -> ImpactSet {
    let mut all_ids: BTreeSet<String> = BTreeSet::new();
    for id in previous.digests.keys() {
        all_ids.insert(id.clone());
    }
    for id in current.digests.keys() {
        all_ids.insert(id.clone());
    }
    let changed_artifacts: Vec<String> = all_ids
        .into_iter()
        .filter(|id: &String| previous.digests.get(id) != current.digests.get(id))
        .collect();
    let changed_set: BTreeSet<&str> = changed_artifacts.iter().map(String::as_str).collect();

    let mut stale_evidence_ids: Vec<String> = Vec::new();
    let mut affected_claims: BTreeSet<ArtifactId> = BTreeSet::new();
    for record in ledger.records() {
        let freshness: EvidenceFreshness = freshness_of(record, current, ledger);
        let stale_due_to_change: bool = match &freshness {
            EvidenceFreshness::Stale { changed_dependencies } => {
                record
                    .dependencies
                    .iter()
                    .any(|pin: &DependencyPin| changed_set.contains(pin.artifact_id.as_str()))
                    || changed_dependencies.iter().any(|id: &String| changed_set.contains(id.as_str()))
            }
            EvidenceFreshness::Current | EvidenceFreshness::Invalidated { .. } | EvidenceFreshness::Superseded { .. } => false,
        };
        if stale_due_to_change {
            stale_evidence_ids.push(record.id.clone());
            for claim_id in &record.claim_ids {
                affected_claims.insert(claim_id.clone());
            }
            for link in ledger.links() {
                if link.from_id == record.id && is_claim_link_relation(link.relation) {
                    if let Some(claim_id) = ArtifactId::parse(&link.to_id) {
                        if current.claim(claim_id.as_str()).is_some() || previous.claim(claim_id.as_str()).is_some() {
                            affected_claims.insert(claim_id);
                        }
                    }
                }
            }
        }
    }

    for claim in &current.claims {
        let self_changed: bool = changed_set.contains(claim.id.as_str());
        let deps_changed: bool = current
            .dependency_pins_for_claim(claim.id.as_str())
            .iter()
            .any(|pin: &DependencyPin| changed_set.contains(pin.artifact_id.as_str()));
        if self_changed || deps_changed {
            affected_claims.insert(claim.id.clone());
        }
    }
    for claim in &previous.claims {
        let self_changed: bool = changed_set.contains(claim.id.as_str());
        let deps_changed: bool = previous
            .dependency_pins_for_claim(claim.id.as_str())
            .iter()
            .any(|pin: &DependencyPin| changed_set.contains(pin.artifact_id.as_str()));
        if self_changed || deps_changed {
            affected_claims.insert(claim.id.clone());
        }
    }

    ImpactSet {
        changed_artifacts,
        stale_evidence_ids,
        affected_claim_ids: affected_claims.into_iter().collect(),
    }
}

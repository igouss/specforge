mod freshness;

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::graph::{ArtifactId, DependencyPin, GraphSnapshot, VerificationClaim, VerificationMethod};
use crate::ledger::{EvidenceLedger, EvidenceRecord, EvidenceRelation};
use crate::verify::{ClaimStatus, EvaluationResult, FormalOutcome, RunState, VerificationOutcome};

pub use freshness::{freshness_of, impact_set};

use freshness::{is_claim_link_relation, missing_pins, relevant_records};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EvidenceFreshness {
    Current,
    Stale { changed_dependencies: Vec<String> },
    Invalidated { by: String },
    Superseded { by: String },
}

impl fmt::Display for EvidenceFreshness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label: &str = match self {
            EvidenceFreshness::Current => "current",
            EvidenceFreshness::Stale { .. } => "stale",
            EvidenceFreshness::Invalidated { .. } => "invalidated",
            EvidenceFreshness::Superseded { .. } => "superseded",
        };
        formatter.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceEvaluationEntry {
    pub evidence_id: String,
    pub freshness: EvidenceFreshness,
    pub outcome: VerificationOutcome,
    pub method: VerificationMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contradiction {
    pub supporting_id: String,
    pub contradicting_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimAssessment {
    pub claim_id: ArtifactId,
    pub status: ClaimStatus,
    pub unbounded_proof: bool,
    pub evaluated: Vec<EvidenceEvaluationEntry>,
    pub contradictions: Vec<Contradiction>,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactSet {
    pub changed_artifacts: Vec<String>,
    pub stale_evidence_ids: Vec<String>,
    pub affected_claim_ids: Vec<ArtifactId>,
}

fn is_proved_outcome(outcome: &VerificationOutcome) -> bool {
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

pub(crate) fn supporting_disqualifications(record: &EvidenceRecord, claim: &VerificationClaim) -> Vec<String> {
    let mut reasons: Vec<String> = Vec::new();
    let completed: bool = match record.run_state {
        RunState::Completed => true,
        RunState::Requested | RunState::Started | RunState::Failed | RunState::Cancelled | RunState::Inconclusive => false,
    };
    if !completed {
        reasons.push(format!(
            "evidence {} disqualified from supporting claim {}: run state is {:?} rather than Completed",
            record.id, claim.id, record.run_state
        ));
    }
    if record.scope != claim.scope {
        reasons.push(format!(
            "evidence {} disqualified from supporting claim {}: scope \"{}\" does not match claim scope \"{}\"",
            record.id, claim.id, record.scope, claim.scope
        ));
    }
    for assumption in &record.assumptions {
        if !claim.assumptions.contains(assumption) {
            reasons.push(format!(
                "evidence {} disqualified from supporting claim {}: relies on undeclared assumption {}",
                record.id, claim.id, assumption
            ));
        }
    }
    if record.producer.name.trim().is_empty()
        || record.adapter.name.trim().is_empty()
        || record.timestamp.trim().is_empty()
        || record.run_id.trim().is_empty()
        || record.source_revisions.is_empty()
    {
        reasons.push(format!(
            "evidence {} disqualified from supporting claim {}: incomplete provenance metadata",
            record.id, claim.id
        ));
    }
    reasons
}

/// When a verification claim is assessed against a snapshot and ledger, `assess_claim` shall return the qualified claim status, freshness per record, contradictions, and explicit rationale SF-009 describes.
pub fn assess_claim(claim: &VerificationClaim, snapshot: &GraphSnapshot, ledger: &EvidenceLedger) -> ClaimAssessment {
    let relevant: Vec<&EvidenceRecord> = relevant_records(claim, ledger);
    let claim_pins: Vec<DependencyPin> = snapshot.dependency_pins_for_claim(claim.id.as_str());
    let mut evaluated: Vec<EvidenceEvaluationEntry> = Vec::new();
    let mut evaluated_ids: BTreeSet<String> = BTreeSet::new();
    let mut rationale: Vec<String> = Vec::new();
    let mut current_supporting: Vec<&EvidenceRecord> = Vec::new();
    let mut current_contradicting: Vec<&EvidenceRecord> = Vec::new();
    let mut current_inconclusive: Vec<&EvidenceRecord> = Vec::new();
    let mut has_invalid: bool = false;
    let mut has_stale: bool = false;

    let mut claim_contradict_ids: BTreeSet<&str> = BTreeSet::new();
    let mut claim_support_link_ids: BTreeSet<&str> = BTreeSet::new();
    for link in ledger.links() {
        if link.to_id == claim.id.as_str() && is_claim_link_relation(link.relation) {
            if !link.verify_digest() {
                has_invalid = true;
                rationale.push(format!("link {} -> {} failed content digest verification", link.from_id, link.to_id));
                continue;
            }
            match link.relation {
                EvidenceRelation::Contradicts => {
                    claim_contradict_ids.insert(link.from_id.as_str());
                }
                EvidenceRelation::Supports => {
                    claim_support_link_ids.insert(link.from_id.as_str());
                }
                EvidenceRelation::Corroborates | EvidenceRelation::DerivedFrom | EvidenceRelation::Supersedes | EvidenceRelation::Invalidates => {}
            }
        }
    }

    for record in relevant {
        if !record.verify_digest() {
            has_invalid = true;
            rationale.push(format!("evidence {} failed content digest verification", record.id));
            continue;
        }
        let mut freshness: EvidenceFreshness = freshness_of(record, snapshot, ledger);
        let expects_claim_pins: bool = record.claim_ids.contains(&claim.id) || claim_support_link_ids.contains(record.id.as_str());
        if expects_claim_pins {
            let mut missing: Vec<String> = missing_pins(record, &claim_pins);
            if !missing.is_empty() {
                match freshness {
                    EvidenceFreshness::Current => {
                        missing.sort();
                        missing.dedup();
                        freshness = EvidenceFreshness::Stale { changed_dependencies: missing };
                    }
                    EvidenceFreshness::Stale { mut changed_dependencies } => {
                        changed_dependencies.extend(missing);
                        changed_dependencies.sort();
                        changed_dependencies.dedup();
                        freshness = EvidenceFreshness::Stale { changed_dependencies };
                    }
                    EvidenceFreshness::Invalidated { .. } | EvidenceFreshness::Superseded { .. } => {}
                }
            }
        }
        evaluated_ids.insert(record.id.clone());
        evaluated.push(EvidenceEvaluationEntry {
            evidence_id: record.id.clone(),
            freshness: freshness.clone(),
            outcome: record.outcome.clone(),
            method: record.method,
        });
        match &freshness {
            EvidenceFreshness::Stale { changed_dependencies } => {
                has_stale = true;
                rationale.push(format!(
                    "evidence {} is stale because dependencies changed: {}",
                    record.id,
                    changed_dependencies.join(", ")
                ));
                continue;
            }
            EvidenceFreshness::Invalidated { by } => {
                rationale.push(format!("evidence {} was invalidated by {by}", record.id));
                continue;
            }
            EvidenceFreshness::Superseded { by } => {
                rationale.push(format!("evidence {} was superseded by {by}", record.id));
                continue;
            }
            EvidenceFreshness::Current => {}
        }
        if claim_contradict_ids.contains(record.id.as_str()) {
            let is_invalid_eval: bool = match &record.outcome {
                VerificationOutcome::Evaluation(EvaluationResult::InvalidEvaluation) => true,
                VerificationOutcome::Evaluation(
                    EvaluationResult::Satisfied | EvaluationResult::Violated | EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated,
                )
                | VerificationOutcome::Formal(
                    FormalOutcome::Proved
                    | FormalOutcome::Refuted
                    | FormalOutcome::BoundedSuccess { .. }
                    | FormalOutcome::BoundedFailure { .. }
                    | FormalOutcome::Unknown
                    | FormalOutcome::Timeout
                    | FormalOutcome::Unsupported,
                ) => false,
            };
            if is_invalid_eval {
                has_invalid = true;
            }
            current_contradicting.push(record);
            rationale.push(format!("evidence {} explicitly contradicts claim {} via ledger link", record.id, claim.id));
            continue;
        }
        match &record.outcome {
            VerificationOutcome::Evaluation(EvaluationResult::Satisfied) => {
                let disqualifications: Vec<String> = supporting_disqualifications(record, claim);
                if disqualifications.is_empty() {
                    current_supporting.push(record);
                    rationale.push(format!(
                        "evidence {} ({:?}) satisfied within scope \"{}\" (finite check, not universal proof)",
                        record.id, record.method, record.scope
                    ));
                } else {
                    current_inconclusive.push(record);
                    rationale.extend(disqualifications);
                }
            }
            VerificationOutcome::Evaluation(EvaluationResult::Violated) => {
                current_contradicting.push(record);
                rationale.push(format!("evidence {} ({:?}) recorded a violation", record.id, record.method));
            }
            VerificationOutcome::Evaluation(EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated) => {
                current_inconclusive.push(record);
                rationale.push(format!("evidence {} ({:?}) was inconclusive", record.id, record.method));
            }
            VerificationOutcome::Evaluation(EvaluationResult::InvalidEvaluation) => {
                has_invalid = true;
                rationale.push(format!("evidence {} had an invalid evaluation", record.id));
            }
            VerificationOutcome::Formal(FormalOutcome::Proved) => {
                let disqualifications: Vec<String> = supporting_disqualifications(record, claim);
                if disqualifications.is_empty() {
                    current_supporting.push(record);
                    rationale.push(format!("evidence {} formally proved proposition under scope \"{}\"", record.id, record.scope));
                } else {
                    current_inconclusive.push(record);
                    rationale.extend(disqualifications);
                }
            }
            VerificationOutcome::Formal(FormalOutcome::BoundedSuccess { bound }) => {
                let disqualifications: Vec<String> = supporting_disqualifications(record, claim);
                if disqualifications.is_empty() {
                    current_supporting.push(record);
                    rationale.push(format!(
                        "evidence {} established bounded formal success within bound \"{bound}\" (not an unbounded proof)",
                        record.id
                    ));
                } else {
                    current_inconclusive.push(record);
                    rationale.extend(disqualifications);
                }
            }
            VerificationOutcome::Formal(FormalOutcome::Refuted) => {
                current_contradicting.push(record);
                rationale.push(format!("evidence {} formally refuted proposition", record.id));
            }
            VerificationOutcome::Formal(FormalOutcome::BoundedFailure { bound }) => {
                current_contradicting.push(record);
                rationale.push(format!("evidence {} found a formal counterexample within bound \"{bound}\"", record.id));
            }
            VerificationOutcome::Formal(FormalOutcome::Unknown | FormalOutcome::Timeout | FormalOutcome::Unsupported) => {
                current_inconclusive.push(record);
                rationale.push(format!("evidence {} returned an inconclusive formal outcome", record.id));
            }
        }
    }

    let mut contradictions: Vec<Contradiction> = Vec::new();
    let mut contradiction_pairs: BTreeSet<(String, String)> = BTreeSet::new();
    for support in &current_supporting {
        for contra in &current_contradicting {
            let pair: (String, String) = (support.id.clone(), contra.id.clone());
            if contradiction_pairs.insert(pair) {
                contradictions.push(Contradiction {
                    supporting_id: support.id.clone(),
                    contradicting_id: contra.id.clone(),
                    reason: format!("current evidence {} supports claim {} while {} contradicts it", support.id, claim.id, contra.id),
                });
            }
        }
    }

    let supporting_ids: BTreeSet<&str> = current_supporting.iter().map(|record: &&EvidenceRecord| record.id.as_str()).collect();
    for link in ledger.links() {
        if link.relation != EvidenceRelation::Contradicts || !supporting_ids.contains(link.to_id.as_str()) {
            continue;
        }
        if !link.verify_digest() {
            has_invalid = true;
            rationale.push(format!("link {} -> {} failed content digest verification", link.from_id, link.to_id));
            continue;
        }
        let Some(contra): Option<&EvidenceRecord> = ledger.get(&link.from_id) else {
            continue;
        };
        if !contra.verify_digest() {
            has_invalid = true;
            rationale.push(format!("contradicting evidence {} failed content digest verification", contra.id));
            continue;
        }
        let contra_freshness: EvidenceFreshness = freshness_of(contra, snapshot, ledger);
        if evaluated_ids.insert(contra.id.clone()) {
            evaluated.push(EvidenceEvaluationEntry {
                evidence_id: contra.id.clone(),
                freshness: contra_freshness.clone(),
                outcome: contra.outcome.clone(),
                method: contra.method,
            });
        }
        let is_current: bool = match contra_freshness {
            EvidenceFreshness::Current => true,
            EvidenceFreshness::Stale { .. } | EvidenceFreshness::Invalidated { .. } | EvidenceFreshness::Superseded { .. } => false,
        };
        if is_current {
            let pair: (String, String) = (link.to_id.clone(), contra.id.clone());
            if contradiction_pairs.insert(pair) {
                contradictions.push(Contradiction {
                    supporting_id: link.to_id.clone(),
                    contradicting_id: contra.id.clone(),
                    reason: format!(
                        "current evidence {} explicitly contradicts supporting evidence {} via ledger link",
                        contra.id, link.to_id
                    ),
                });
                rationale.push(format!("evidence {} contradicts supporting evidence {} via ledger link", contra.id, link.to_id));
            }
        }
    }

    let mut all_properties_supported: bool = true;
    for property_id in &claim.properties {
        let covered: bool = current_supporting
            .iter()
            .any(|record: &&EvidenceRecord| record.property_id.as_ref() == Some(property_id));
        if !covered {
            all_properties_supported = false;
            rationale.push(format!(
                "claim {} lacks current supporting evidence for required property {}",
                claim.id, property_id
            ));
        }
    }

    let status: ClaimStatus = if has_invalid {
        ClaimStatus::Invalid
    } else if !contradictions.is_empty() {
        rationale.push("unresolved contradiction between current evidence requires explicit review".to_string());
        ClaimStatus::RequiresReview
    } else if !current_contradicting.is_empty() {
        ClaimStatus::Refuted
    } else if !current_supporting.is_empty() && all_properties_supported {
        ClaimStatus::Supported
    } else if has_stale {
        ClaimStatus::Stale
    } else if !current_inconclusive.is_empty() || !current_supporting.is_empty() {
        ClaimStatus::Inconclusive
    } else {
        rationale.push("no current evidence linked to claim".to_string());
        ClaimStatus::Inconclusive
    };

    let all_properties_proved: bool = if claim.properties.is_empty() {
        current_supporting.iter().any(|record: &&EvidenceRecord| is_proved_outcome(&record.outcome))
    } else {
        claim.properties.iter().all(|property_id: &ArtifactId| {
            current_supporting
                .iter()
                .any(|record: &&EvidenceRecord| record.property_id.as_ref() == Some(property_id) && is_proved_outcome(&record.outcome))
        })
    };
    let unbounded_proof: bool = status == ClaimStatus::Supported && all_properties_proved;

    ClaimAssessment {
        claim_id: claim.id.clone(),
        status,
        unbounded_proof,
        evaluated,
        contradictions,
        rationale,
    }
}

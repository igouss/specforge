mod export;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub use self::export::{export_argument_json, export_argument_markdown};
use crate::assurance::{assess_claim, supporting_disqualifications, ClaimAssessment, EvidenceEvaluationEntry, EvidenceFreshness};
use crate::graph::{ArtifactId, ContentHash, GraphSnapshot, VerificationClaim, VerificationMethod};
use crate::ledger::{EvidenceLedger, EvidenceLink, EvidenceRecord, EvidenceRelation};
use crate::verify::{ClaimStatus, EvaluationResult, FormalOutcome, VerificationOutcome};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgumentEvidenceItem {
    pub evidence_id: String,
    pub method: VerificationMethod,
    pub freshness: EvidenceFreshness,
    pub outcome: VerificationOutcome,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgumentDefeater {
    pub kind: String,
    pub source_id: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubclaimArgument {
    pub claim_id: ArtifactId,
    pub proposition: String,
    pub scope: String,
    pub assumptions: Vec<ArtifactId>,
    pub status: ClaimStatus,
    pub unbounded_proof: bool,
    pub evidence: Vec<ArgumentEvidenceItem>,
    pub defeaters: Vec<ArgumentDefeater>,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssuranceArgument {
    pub claim_id: ArtifactId,
    pub proposition: String,
    pub snapshot_id: ContentHash,
    pub scope: String,
    pub assumptions: Vec<ArtifactId>,
    pub status: ClaimStatus,
    pub unbounded_proof: bool,
    pub evidence: Vec<ArgumentEvidenceItem>,
    pub subclaims: Vec<SubclaimArgument>,
    pub defeaters: Vec<ArgumentDefeater>,
    pub rationale: Vec<String>,
}

fn status_severity(status: ClaimStatus) -> u8 {
    match status {
        ClaimStatus::Supported => 0,
        ClaimStatus::Inconclusive => 1,
        ClaimStatus::Stale => 2,
        ClaimStatus::Refuted => 3,
        ClaimStatus::RequiresReview => 4,
        ClaimStatus::Invalid => 5,
    }
}

fn worse_status(left: ClaimStatus, right: ClaimStatus) -> ClaimStatus {
    std::cmp::max_by_key(left, right, |status: &ClaimStatus| status_severity(*status))
}

fn collect_evidence_and_defeaters(
    claim: &VerificationClaim,
    assessment: &ClaimAssessment,
    ledger: &EvidenceLedger,
) -> (Vec<ArgumentEvidenceItem>, Vec<ArgumentDefeater>) {
    let mut evidence: Vec<ArgumentEvidenceItem> = Vec::new();
    let mut defeaters: Vec<ArgumentDefeater> = Vec::new();
    let claim_contradict_ids: BTreeSet<&str> = ledger
        .links()
        .iter()
        .filter(|link: &&EvidenceLink| link.to_id == claim.id.as_str() && link.relation == EvidenceRelation::Contradicts)
        .map(|link: &EvidenceLink| link.from_id.as_str())
        .collect();

    let mut contradicted_evidence_ids: BTreeSet<&str> = BTreeSet::new();
    for contradiction in &assessment.contradictions {
        contradicted_evidence_ids.insert(contradiction.contradicting_id.as_str());
        defeaters.push(ArgumentDefeater {
            kind: "contradiction".to_string(),
            source_id: format!("{}<->{}", contradiction.supporting_id, contradiction.contradicting_id),
            detail: contradiction.reason.clone(),
        });
    }

    let mut covered_properties: BTreeSet<&ArtifactId> = BTreeSet::new();
    for entry in &assessment.evaluated {
        let record_opt: Option<&EvidenceRecord> = ledger.get(&entry.evidence_id);
        let limitations: Vec<String> = record_opt.map_or_else(Vec::new, |record: &EvidenceRecord| record.limitations.clone());
        let item: ArgumentEvidenceItem = ArgumentEvidenceItem {
            evidence_id: entry.evidence_id.clone(),
            method: entry.method,
            freshness: entry.freshness.clone(),
            outcome: entry.outcome.clone(),
            limitations,
        };
        let is_qualified_support: bool = collect_entry_defeater(claim, entry, record_opt, &claim_contradict_ids, &contradicted_evidence_ids, &mut defeaters);
        if is_qualified_support {
            if let Some(record) = record_opt {
                if let Some(property_id) = &record.property_id {
                    covered_properties.insert(property_id);
                }
            }
        }
        evidence.push(item);
    }

    for property_id in &claim.properties {
        if !covered_properties.contains(property_id) {
            defeaters.push(ArgumentDefeater {
                kind: "evidence_gap".to_string(),
                source_id: property_id.as_str().to_string(),
                detail: format!("claim {} lacks current supporting evidence for required property {}", claim.id, property_id),
            });
        }
    }

    if assessment.status != ClaimStatus::Supported && defeaters.is_empty() {
        defeaters.push(ArgumentDefeater {
            kind: "evidence_gap".to_string(),
            source_id: claim.id.as_str().to_string(),
            detail: format!("claim {} has no current supporting evidence", claim.id),
        });
    }

    (evidence, defeaters)
}

fn collect_entry_defeater(
    claim: &VerificationClaim,
    entry: &EvidenceEvaluationEntry,
    record_opt: Option<&EvidenceRecord>,
    claim_contradict_ids: &BTreeSet<&str>,
    contradicted_evidence_ids: &BTreeSet<&str>,
    defeaters: &mut Vec<ArgumentDefeater>,
) -> bool {
    match &entry.freshness {
        EvidenceFreshness::Stale { changed_dependencies } => {
            defeaters.push(ArgumentDefeater {
                kind: "stale_evidence".to_string(),
                source_id: entry.evidence_id.clone(),
                detail: format!("stale because dependencies changed: {}", changed_dependencies.join(", ")),
            });
            return false;
        }
        EvidenceFreshness::Invalidated { by } => {
            defeaters.push(ArgumentDefeater {
                kind: "invalidated_evidence".to_string(),
                source_id: entry.evidence_id.clone(),
                detail: format!("invalidated by {by}"),
            });
            return false;
        }
        EvidenceFreshness::Superseded { .. } => {
            return false;
        }
        EvidenceFreshness::Current => {}
    }
    if claim_contradict_ids.contains(entry.evidence_id.as_str()) {
        if !contradicted_evidence_ids.contains(entry.evidence_id.as_str()) {
            defeaters.push(ArgumentDefeater {
                kind: "contradiction".to_string(),
                source_id: format!("{}->{}", entry.evidence_id, claim.id),
                detail: format!("evidence {} explicitly contradicts claim {} via ledger link", entry.evidence_id, claim.id),
            });
        }
        let is_invalid_eval: bool = match &entry.outcome {
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
            defeaters.push(ArgumentDefeater {
                kind: "invalid_evidence".to_string(),
                source_id: entry.evidence_id.clone(),
                detail: format!("evidence {} had an invalid evaluation", entry.evidence_id),
            });
        }
        return false;
    }
    match &entry.outcome {
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied)
        | VerificationOutcome::Formal(FormalOutcome::Proved | FormalOutcome::BoundedSuccess { .. }) => {
            let Some(record): Option<&EvidenceRecord> = record_opt else {
                return false;
            };
            let disqualifications: Vec<String> = supporting_disqualifications(record, claim);
            if !disqualifications.is_empty() {
                defeaters.push(ArgumentDefeater {
                    kind: "disqualified_evidence".to_string(),
                    source_id: entry.evidence_id.clone(),
                    detail: disqualifications.join("; "),
                });
                return false;
            }
            true
        }
        VerificationOutcome::Evaluation(EvaluationResult::Violated) => {
            defeaters.push(ArgumentDefeater {
                kind: "refuting_evidence".to_string(),
                source_id: entry.evidence_id.clone(),
                detail: format!("evidence {} recorded a violation", entry.evidence_id),
            });
            false
        }
        VerificationOutcome::Evaluation(EvaluationResult::InvalidEvaluation) => {
            defeaters.push(ArgumentDefeater {
                kind: "invalid_evidence".to_string(),
                source_id: entry.evidence_id.clone(),
                detail: format!("evidence {} had an invalid evaluation", entry.evidence_id),
            });
            false
        }
        VerificationOutcome::Evaluation(EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated) => {
            defeaters.push(ArgumentDefeater {
                kind: "inconclusive_evidence".to_string(),
                source_id: entry.evidence_id.clone(),
                detail: format!("evidence {} was inconclusive", entry.evidence_id),
            });
            false
        }
        VerificationOutcome::Formal(FormalOutcome::Refuted) => {
            defeaters.push(ArgumentDefeater {
                kind: "refuting_evidence".to_string(),
                source_id: entry.evidence_id.clone(),
                detail: format!("evidence {} formally refuted the property", entry.evidence_id),
            });
            false
        }
        VerificationOutcome::Formal(FormalOutcome::BoundedFailure { bound }) => {
            defeaters.push(ArgumentDefeater {
                kind: "refuting_evidence".to_string(),
                source_id: entry.evidence_id.clone(),
                detail: format!("evidence {} found a counterexample within bound \"{bound}\"", entry.evidence_id),
            });
            false
        }
        VerificationOutcome::Formal(FormalOutcome::Unknown | FormalOutcome::Timeout | FormalOutcome::Unsupported) => {
            defeaters.push(ArgumentDefeater {
                kind: "inconclusive_evidence".to_string(),
                source_id: entry.evidence_id.clone(),
                detail: format!("evidence {} returned an inconclusive formal outcome", entry.evidence_id),
            });
            false
        }
    }
}

/// When an assurance argument is constructed for a root claim and optional subclaims, `build_assurance_argument` shall assess each claim against the snapshot and ledger, preserve all defeaters and limitations, withhold unbounded proof whenever any subclaim is not unboundedly proved, and reject unknown, duplicate, or self-referencing claim ids.
pub fn build_assurance_argument(
    snapshot: &GraphSnapshot,
    ledger: &EvidenceLedger,
    claim_id: &str,
    subclaim_ids: &[String],
) -> Result<AssuranceArgument, String> {
    let root_claim: &VerificationClaim = snapshot.claim(claim_id).ok_or_else(|| format!("unknown claim {claim_id}"))?;
    let mut seen_subclaims: BTreeSet<&str> = BTreeSet::new();
    let mut subclaim_refs: Vec<&VerificationClaim> = Vec::new();
    for sub_id in subclaim_ids {
        if sub_id == claim_id {
            return Err(format!("self-referencing subclaim {sub_id} is forbidden"));
        }
        if !seen_subclaims.insert(sub_id.as_str()) {
            return Err(format!("duplicate subclaim {sub_id}"));
        }
        let sub_claim: &VerificationClaim = snapshot.claim(sub_id.as_str()).ok_or_else(|| format!("unknown claim {sub_id}"))?;
        subclaim_refs.push(sub_claim);
    }

    let root_assessment: ClaimAssessment = assess_claim(root_claim, snapshot, ledger);
    let (evidence, mut defeaters): (Vec<ArgumentEvidenceItem>, Vec<ArgumentDefeater>) = collect_evidence_and_defeaters(root_claim, &root_assessment, ledger);
    let mut rationale: Vec<String> = root_assessment.rationale.clone();
    let mut status: ClaimStatus = root_assessment.status;
    let mut unbounded_proof: bool = root_assessment.unbounded_proof;
    let mut subclaims: Vec<SubclaimArgument> = Vec::new();

    for sub_claim in subclaim_refs {
        let sub_assessment: ClaimAssessment = assess_claim(sub_claim, snapshot, ledger);
        let (sub_evidence, sub_defeaters): (Vec<ArgumentEvidenceItem>, Vec<ArgumentDefeater>) =
            collect_evidence_and_defeaters(sub_claim, &sub_assessment, ledger);
        match sub_assessment.status {
            ClaimStatus::Supported => {
                if !sub_assessment.unbounded_proof {
                    unbounded_proof = false;
                    rationale.push(format!(
                        "subclaim {} is supported without an unbounded proof; root argument does not claim unbounded proof",
                        sub_claim.id
                    ));
                }
            }
            ClaimStatus::Refuted | ClaimStatus::RequiresReview | ClaimStatus::Stale | ClaimStatus::Invalid | ClaimStatus::Inconclusive => {
                unbounded_proof = false;
                status = worse_status(status, sub_assessment.status);
                defeaters.push(ArgumentDefeater {
                    kind: "subclaim_defeater".to_string(),
                    source_id: sub_claim.id.as_str().to_string(),
                    detail: format!("subclaim {} has status {}", sub_claim.id, sub_assessment.status),
                });
                rationale.push(format!(
                    "subclaim {} has status {} and defeats root support",
                    sub_claim.id, sub_assessment.status
                ));
            }
        }
        subclaims.push(SubclaimArgument {
            claim_id: sub_claim.id.clone(),
            proposition: sub_claim.proposition.clone(),
            scope: sub_claim.scope.clone(),
            assumptions: sub_claim.assumptions.clone(),
            status: sub_assessment.status,
            unbounded_proof: sub_assessment.unbounded_proof,
            evidence: sub_evidence,
            defeaters: sub_defeaters,
            rationale: sub_assessment.rationale,
        });
    }

    if status != ClaimStatus::Supported {
        unbounded_proof = false;
    }

    Ok(AssuranceArgument {
        claim_id: root_claim.id.clone(),
        proposition: root_claim.proposition.clone(),
        snapshot_id: snapshot.snapshot_id.clone(),
        scope: root_claim.scope.clone(),
        assumptions: root_claim.assumptions.clone(),
        status,
        unbounded_proof,
        evidence,
        subclaims,
        defeaters,
        rationale,
    })
}

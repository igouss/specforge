use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::LedgerError;
use crate::graph::{ArtifactId, ContentHash, DependencyPin, GraphSnapshot, SourceRevision, VerificationMethod};
use crate::verify::{Counterexample, EvaluationResult, FormalOutcome, ReplayDescriptor, RunEnvelope, RunState, VerificationOutcome};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProducerMeta {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    PropertyEvaluation,
    Counterexample,
    Replay,
    StoryRun,
    FormalCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidencePayload {
    pub id: String,
    pub kind: EvidenceKind,
    pub producer: ProducerMeta,
    pub adapter: ProducerMeta,
    pub timestamp: String,
    pub snapshot_id: ContentHash,
    pub source_revisions: Vec<SourceRevision>,
    pub dependencies: Vec<DependencyPin>,
    pub run_id: String,
    pub activity_id: String,
    pub method: VerificationMethod,
    pub property_id: Option<ArtifactId>,
    pub claim_ids: Vec<ArtifactId>,
    pub scope: String,
    pub assumptions: Vec<ArtifactId>,
    pub run_state: RunState,
    pub outcome: VerificationOutcome,
    pub counterexample_id: Option<String>,
    pub replay: Option<ReplayDescriptor>,
    pub artifact_digests: BTreeMap<String, ContentHash>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: String,
    pub kind: EvidenceKind,
    pub producer: ProducerMeta,
    pub adapter: ProducerMeta,
    pub timestamp: String,
    pub snapshot_id: ContentHash,
    pub source_revisions: Vec<SourceRevision>,
    pub dependencies: Vec<DependencyPin>,
    pub run_id: String,
    pub activity_id: String,
    pub method: VerificationMethod,
    pub property_id: Option<ArtifactId>,
    pub claim_ids: Vec<ArtifactId>,
    pub scope: String,
    pub assumptions: Vec<ArtifactId>,
    pub run_state: RunState,
    pub outcome: VerificationOutcome,
    pub counterexample_id: Option<String>,
    pub replay: Option<ReplayDescriptor>,
    pub artifact_digests: BTreeMap<String, ContentHash>,
    pub limitations: Vec<String>,
    pub content_digest: ContentHash,
}

#[allow(clippy::too_many_arguments)]
fn validate_record_fields(
    id: &str,
    producer: &ProducerMeta,
    adapter: &ProducerMeta,
    timestamp: &str,
    run_id: &str,
    scope: &str,
    run_state: RunState,
    outcome: &VerificationOutcome,
    replay: Option<&ReplayDescriptor>,
) -> Result<(), LedgerError> {
    if id.trim().is_empty() {
        return Err(LedgerError::EmptyId);
    }
    if producer.name.trim().is_empty() {
        return Err(LedgerError::InvalidRecord {
            id: id.to_string(),
            why: "producer.name must not be empty".to_string(),
        });
    }
    if adapter.name.trim().is_empty() {
        return Err(LedgerError::InvalidRecord {
            id: id.to_string(),
            why: "adapter.name must not be empty".to_string(),
        });
    }
    if timestamp.trim().is_empty() {
        return Err(LedgerError::InvalidRecord {
            id: id.to_string(),
            why: "timestamp must not be empty".to_string(),
        });
    }
    if run_id.trim().is_empty() {
        return Err(LedgerError::InvalidRecord {
            id: id.to_string(),
            why: "run_id must not be empty".to_string(),
        });
    }
    if scope.trim().is_empty() {
        return Err(LedgerError::InvalidRecord {
            id: id.to_string(),
            why: "scope must not be empty".to_string(),
        });
    }
    let positive_outcome: bool = match outcome {
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied)
        | VerificationOutcome::Formal(FormalOutcome::Proved | FormalOutcome::BoundedSuccess { .. }) => true,
        VerificationOutcome::Evaluation(
            EvaluationResult::Violated | EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated | EvaluationResult::InvalidEvaluation,
        )
        | VerificationOutcome::Formal(
            FormalOutcome::Refuted | FormalOutcome::BoundedFailure { .. } | FormalOutcome::Unknown | FormalOutcome::Timeout | FormalOutcome::Unsupported,
        ) => false,
    };
    let completed: bool = match run_state {
        RunState::Completed => true,
        RunState::Requested | RunState::Started | RunState::Failed | RunState::Cancelled | RunState::Inconclusive => false,
    };
    if positive_outcome && !completed {
        return Err(LedgerError::InvalidRecord {
            id: id.to_string(),
            why: format!("positive outcome cannot be paired with run state {run_state:?}"),
        });
    }
    if let Some(descriptor) = replay {
        if descriptor.replayable {
            if descriptor.engine.trim().is_empty() {
                return Err(LedgerError::InvalidRecord {
                    id: id.to_string(),
                    why: "replayable descriptor must declare a non-empty engine".to_string(),
                });
            }
            if descriptor.seed.is_none() && descriptor.events.is_empty() && descriptor.replay_token.is_none() {
                return Err(LedgerError::InvalidRecord {
                    id: id.to_string(),
                    why: "replayable descriptor must provide a seed, events, or replay token".to_string(),
                });
            }
        }
    }
    Ok(())
}

impl EvidenceRecord {
    /// When an evidence payload is sealed, `seal` shall validate required fields and compute its canonical JSON SHA-256 content digest, returning an immutable record.
    pub fn seal(payload: EvidencePayload) -> Result<EvidenceRecord, LedgerError> {
        validate_record_fields(
            &payload.id,
            &payload.producer,
            &payload.adapter,
            &payload.timestamp,
            &payload.run_id,
            &payload.scope,
            payload.run_state,
            &payload.outcome,
            payload.replay.as_ref(),
        )?;
        let digest: ContentHash = ContentHash::of_canonical_json(&payload).map_err(LedgerError::Serialization)?;
        Ok(EvidenceRecord {
            id: payload.id,
            kind: payload.kind,
            producer: payload.producer,
            adapter: payload.adapter,
            timestamp: payload.timestamp,
            snapshot_id: payload.snapshot_id,
            source_revisions: payload.source_revisions,
            dependencies: payload.dependencies,
            run_id: payload.run_id,
            activity_id: payload.activity_id,
            method: payload.method,
            property_id: payload.property_id,
            claim_ids: payload.claim_ids,
            scope: payload.scope,
            assumptions: payload.assumptions,
            run_state: payload.run_state,
            outcome: payload.outcome,
            counterexample_id: payload.counterexample_id,
            replay: payload.replay,
            artifact_digests: payload.artifact_digests,
            limitations: payload.limitations,
            content_digest: digest,
        })
    }

    pub(crate) fn validate_fields(&self) -> Result<(), LedgerError> {
        validate_record_fields(
            &self.id,
            &self.producer,
            &self.adapter,
            &self.timestamp,
            &self.run_id,
            &self.scope,
            self.run_state,
            &self.outcome,
            self.replay.as_ref(),
        )
    }

    /// When a run envelope is converted into evidence, `from_envelope` shall capture its dependency pins from the snapshot and seal its digest.
    pub fn from_envelope(
        id: String,
        kind: EvidenceKind,
        timestamp: String,
        snapshot: &GraphSnapshot,
        envelope: &RunEnvelope,
        artifact_digests: BTreeMap<String, ContentHash>,
    ) -> Result<EvidenceRecord, LedgerError> {
        let mut pins_map: BTreeMap<String, ContentHash> = BTreeMap::new();
        for pin in snapshot.dependency_pins_for_property(envelope.property_id.as_str()) {
            pins_map.insert(pin.artifact_id, pin.digest);
        }
        for claim_id in &envelope.claim_ids {
            for pin in snapshot.dependency_pins_for_claim(claim_id.as_str()) {
                pins_map.insert(pin.artifact_id, pin.digest);
            }
        }
        let dependencies: Vec<DependencyPin> = pins_map
            .into_iter()
            .map(|(artifact_id, digest): (String, ContentHash)| DependencyPin { artifact_id, digest })
            .collect();
        let counterexample_id: Option<String> = envelope
            .minimized_counterexample
            .as_ref()
            .or(envelope.original_counterexample.as_ref())
            .map(|ce: &Counterexample| ce.id.clone());
        let payload: EvidencePayload = EvidencePayload {
            id,
            kind,
            producer: ProducerMeta {
                name: envelope.engine_name.clone(),
                version: envelope.engine_version.clone(),
            },
            adapter: ProducerMeta {
                name: envelope.adapter_id.clone(),
                version: envelope.adapter_version.clone(),
            },
            timestamp,
            snapshot_id: envelope.snapshot_id.clone(),
            source_revisions: envelope.source_revisions.clone(),
            dependencies,
            run_id: envelope.run_id.clone(),
            activity_id: envelope.activity_id.clone(),
            method: envelope.method,
            property_id: Some(envelope.property_id.clone()),
            claim_ids: envelope.claim_ids.clone(),
            scope: envelope.scope.clone(),
            assumptions: envelope.assumptions.clone(),
            run_state: envelope.run_state,
            outcome: envelope.outcome.clone(),
            counterexample_id,
            replay: envelope.replay.clone(),
            artifact_digests,
            limitations: envelope.limitations.clone(),
        };
        EvidenceRecord::seal(payload)
    }

    /// When an evidence record's integrity is verified, `verify_digest` shall return true only if its recomputed canonical SHA-256 digest matches `content_digest`.
    pub fn verify_digest(&self) -> bool {
        let payload: EvidencePayload = EvidencePayload {
            id: self.id.clone(),
            kind: self.kind,
            producer: self.producer.clone(),
            adapter: self.adapter.clone(),
            timestamp: self.timestamp.clone(),
            snapshot_id: self.snapshot_id.clone(),
            source_revisions: self.source_revisions.clone(),
            dependencies: self.dependencies.clone(),
            run_id: self.run_id.clone(),
            activity_id: self.activity_id.clone(),
            method: self.method,
            property_id: self.property_id.clone(),
            claim_ids: self.claim_ids.clone(),
            scope: self.scope.clone(),
            assumptions: self.assumptions.clone(),
            run_state: self.run_state,
            outcome: self.outcome.clone(),
            counterexample_id: self.counterexample_id.clone(),
            replay: self.replay.clone(),
            artifact_digests: self.artifact_digests.clone(),
            limitations: self.limitations.clone(),
        };
        ContentHash::of_canonical_json(&payload).is_ok_and(|computed: ContentHash| computed == self.content_digest)
    }
}

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{CorpusError, CounterexampleRegistry};
use crate::graph::GraphSnapshot;
use crate::ledger::{EvidenceKind, EvidenceLedger, EvidenceRecord, LedgerError};
use crate::verify::{AdapterError, Counterexample, EvaluationResult, FormalOutcome, RunEnvelope, RunState, VerificationAdapter, VerificationOutcome};

/// When a registered counterexample is replayed during regression verification, `CorpusReplayOutcome` shall distinguish reproduced violations, resolved checks, inconclusive runs, and skipped non-replayable entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusReplayOutcome {
    Reproduced,
    Resolved,
    Inconclusive,
    SkippedNotReplayable(String),
}

impl fmt::Display for CorpusReplayOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CorpusReplayOutcome::Reproduced => formatter.write_str("reproduced"),
            CorpusReplayOutcome::Resolved => formatter.write_str("resolved"),
            CorpusReplayOutcome::Inconclusive => formatter.write_str("inconclusive"),
            CorpusReplayOutcome::SkippedNotReplayable(_) => formatter.write_str("skipped_not_replayable"),
        }
    }
}

/// When a corpus replay processes one counterexample, `CorpusReplayEntry` shall record its identifier, emitted replay evidence identifier when executed, and replay outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusReplayEntry {
    pub counterexample_id: String,
    pub evidence_id: Option<String>,
    pub outcome: CorpusReplayOutcome,
}

/// When a counterexample corpus finishes replaying, `CorpusReplaySummary` shall list every processed counterexample outcome in deterministic order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusReplaySummary {
    pub entries: Vec<CorpusReplayEntry>,
}

fn outcome_from_envelope(envelope: &RunEnvelope) -> CorpusReplayOutcome {
    let completed: bool = match envelope.run_state {
        RunState::Completed => true,
        RunState::Requested | RunState::Started | RunState::Failed | RunState::Cancelled | RunState::Inconclusive => false,
    };
    match &envelope.outcome {
        VerificationOutcome::Evaluation(EvaluationResult::Violated)
        | VerificationOutcome::Formal(FormalOutcome::Refuted | FormalOutcome::BoundedFailure { .. }) => CorpusReplayOutcome::Reproduced,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied)
        | VerificationOutcome::Formal(FormalOutcome::Proved | FormalOutcome::BoundedSuccess { .. }) => {
            if completed {
                CorpusReplayOutcome::Resolved
            } else {
                CorpusReplayOutcome::Inconclusive
            }
        }
        VerificationOutcome::Evaluation(EvaluationResult::Inconclusive | EvaluationResult::NotEvaluated | EvaluationResult::InvalidEvaluation)
        | VerificationOutcome::Formal(FormalOutcome::Unknown | FormalOutcome::Timeout | FormalOutcome::Unsupported) => CorpusReplayOutcome::Inconclusive,
    }
}

fn validate_replay_envelope(envelope: &RunEnvelope, target: &Counterexample, snapshot: &GraphSnapshot, run_id: &str) -> Result<(), CorpusError> {
    if envelope.property_id != target.property_id {
        return Err(CorpusError::ReplayFailed(format!(
            "replay envelope property {} does not match counterexample {} property {}",
            envelope.property_id, target.id, target.property_id
        )));
    }
    if envelope.snapshot_id != snapshot.snapshot_id {
        return Err(CorpusError::ReplayFailed(format!(
            "replay envelope snapshot {} does not match current snapshot {}",
            envelope.snapshot_id, snapshot.snapshot_id
        )));
    }
    if envelope.run_id != run_id {
        return Err(CorpusError::ReplayFailed(format!(
            "replay envelope run_id \"{}\" does not match requested run_id \"{}\"",
            envelope.run_id, run_id
        )));
    }
    Ok(())
}

/// When a counterexample registry is replayed against an adapter, `replay_corpus` shall replay each original's latest minimized child (or the original itself), verify returned envelope identity, skip non-replayable entries without reporting them resolved, and atomically append sealed replay evidence records to the ledger only if every replay succeeds.
pub fn replay_corpus<A: VerificationAdapter + ?Sized>(
    registry: &CounterexampleRegistry,
    snapshot: &GraphSnapshot,
    adapter: &A,
    ledger: &mut EvidenceLedger,
    run_prefix: &str,
    timestamp: &str,
) -> Result<CorpusReplaySummary, CorpusError> {
    if run_prefix.trim().is_empty() {
        return Err(CorpusError::ReplayFailed("run_prefix must not be empty".to_string()));
    }
    if timestamp.trim().is_empty() {
        return Err(CorpusError::ReplayFailed("timestamp must not be empty".to_string()));
    }

    let mut entries: Vec<CorpusReplayEntry> = Vec::new();
    let mut staged_records: Vec<EvidenceRecord> = Vec::new();
    let mut staged_ids: BTreeSet<String> = BTreeSet::new();

    for (index, original) in registry.originals().into_iter().enumerate() {
        let minimized_children: Vec<&Counterexample> = registry.minimized_for(&original.id);
        let target: &Counterexample = minimized_children.last().copied().unwrap_or(original);

        if !target.replay.replayable {
            entries.push(CorpusReplayEntry {
                counterexample_id: target.id.clone(),
                evidence_id: None,
                outcome: CorpusReplayOutcome::SkippedNotReplayable(format!("counterexample {} is not replayable", target.id)),
            });
            continue;
        }

        let run_id: String = format!("{run_prefix}-{:03}", index + 1);
        let envelope: RunEnvelope = match adapter.replay(snapshot, target, &run_id) {
            Ok(envelope) => envelope,
            Err(AdapterError::NotReplayable(reason)) => {
                entries.push(CorpusReplayEntry {
                    counterexample_id: target.id.clone(),
                    evidence_id: None,
                    outcome: CorpusReplayOutcome::SkippedNotReplayable(reason),
                });
                continue;
            }
            Err(AdapterError::Unsupported(why)) => return Err(CorpusError::ReplayFailed(why)),
            Err(AdapterError::InvalidRequest(why)) => return Err(CorpusError::ReplayFailed(why)),
            Err(AdapterError::Execution(why)) => return Err(CorpusError::ReplayFailed(why)),
        };

        validate_replay_envelope(&envelope, target, snapshot, &run_id)?;

        let evidence_id: String = format!("EV-{run_id}");
        if ledger.get(&evidence_id).is_some() || !staged_ids.insert(evidence_id.clone()) {
            return Err(CorpusError::ReplayFailed(format!("replay evidence id {evidence_id} already exists in ledger")));
        }

        let record: EvidenceRecord = EvidenceRecord::from_envelope(
            evidence_id.clone(),
            EvidenceKind::Replay,
            timestamp.to_string(),
            snapshot,
            &envelope,
            BTreeMap::new(),
        )
        .map_err(|err: LedgerError| CorpusError::ReplayFailed(err.to_string()))?;
        staged_records.push(record);

        let outcome: CorpusReplayOutcome = outcome_from_envelope(&envelope);
        entries.push(CorpusReplayEntry {
            counterexample_id: target.id.clone(),
            evidence_id: Some(evidence_id),
            outcome,
        });
    }

    for record in staged_records {
        ledger
            .append_record(record)
            .map_err(|err: LedgerError| CorpusError::ReplayFailed(err.to_string()))?;
    }

    Ok(CorpusReplaySummary { entries })
}

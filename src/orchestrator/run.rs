use std::fmt;

use serde::{Deserialize, Serialize};

use crate::graph::{ArtifactId, ContentHash, VerificationMethod};
use crate::ledger::LedgerError;
use crate::verify::RunState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorError {
    UnknownPlan(String),
    EmptyPlanTargets(ArtifactId),
    EmptyPlanActivities(ArtifactId),
    InvalidTimestamp,
    InvalidTransition { run_id: String, from: RunState, to: RunState },
    EnvelopeMismatch { run_id: String, why: String },
    Ledger(LedgerError),
}

impl fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrchestratorError::UnknownPlan(id) => write!(f, "unknown verification plan {id}"),
            OrchestratorError::EmptyPlanTargets(id) => write!(f, "verification plan {id} resolves to no target properties"),
            OrchestratorError::EmptyPlanActivities(id) => write!(f, "verification plan {id} declares no activities"),
            OrchestratorError::InvalidTimestamp => f.write_str("orchestrator timestamp must not be empty"),
            OrchestratorError::InvalidTransition { run_id, from, to } => {
                write!(f, "invalid run state transition on {run_id}: {from:?} -> {to:?}")
            }
            OrchestratorError::EnvelopeMismatch { run_id, why } => write!(f, "adapter envelope mismatch on {run_id}: {why}"),
            OrchestratorError::Ledger(error) => write!(f, "evidence ledger error: {error}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRun {
    pub run_id: String,
    pub plan_id: ArtifactId,
    pub activity_id: ArtifactId,
    pub property_id: ArtifactId,
    pub claim_ids: Vec<ArtifactId>,
    pub method: VerificationMethod,
    pub snapshot_id: ContentHash,
    pub state: RunState,
    pub progress_current: usize,
    pub progress_total: usize,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub evidence_id: Option<String>,
    pub error: Option<String>,
}

pub(super) fn require_timestamp(timestamp: &str) -> Result<(), OrchestratorError> {
    if timestamp.trim().is_empty() {
        Err(OrchestratorError::InvalidTimestamp)
    } else {
        Ok(())
    }
}

fn is_legal_transition(from: RunState, to: RunState) -> bool {
    match (from, to) {
        (RunState::Requested, RunState::Started | RunState::Cancelled) => true,
        (RunState::Started, RunState::Completed | RunState::Failed | RunState::Cancelled | RunState::Inconclusive) => true,
        (RunState::Requested, RunState::Requested | RunState::Completed | RunState::Failed | RunState::Inconclusive)
        | (RunState::Started, RunState::Requested | RunState::Started)
        | (
            RunState::Completed | RunState::Failed | RunState::Cancelled | RunState::Inconclusive,
            RunState::Requested | RunState::Started | RunState::Completed | RunState::Failed | RunState::Cancelled | RunState::Inconclusive,
        ) => false,
    }
}

impl VerificationRun {
    /// When a run's status is inspected, `is_terminal` shall return true for `Completed`, `Failed`, `Cancelled`, and `Inconclusive`.
    pub fn is_terminal(&self) -> bool {
        match self.state {
            RunState::Requested | RunState::Started => false,
            RunState::Completed | RunState::Failed | RunState::Cancelled | RunState::Inconclusive => true,
        }
    }

    fn ensure_transition(&self, to: RunState) -> Result<(), OrchestratorError> {
        if is_legal_transition(self.state, to) {
            Ok(())
        } else {
            Err(OrchestratorError::InvalidTransition {
                run_id: self.run_id.clone(),
                from: self.state,
                to,
            })
        }
    }

    fn finish(&mut self, to: RunState, timestamp: &str, evidence_id: Option<String>, error: Option<String>) -> Result<(), OrchestratorError> {
        require_timestamp(timestamp)?;
        self.ensure_transition(to)?;
        self.state = to;
        self.finished_at = Some(timestamp.to_string());
        self.evidence_id = evidence_id;
        self.error = error;
        Ok(())
    }

    /// When a requested run begins execution, `start` shall transition `Requested -> Started` and record `started_at`.
    pub fn start(&mut self, timestamp: &str) -> Result<(), OrchestratorError> {
        require_timestamp(timestamp)?;
        self.ensure_transition(RunState::Started)?;
        self.state = RunState::Started;
        self.started_at = Some(timestamp.to_string());
        Ok(())
    }

    /// When a started run reports progress, `advance_progress` shall update `progress_current` monotonically and reject non-started states or backwards progress.
    pub fn advance_progress(&mut self, current: usize) -> Result<(), OrchestratorError> {
        match self.state {
            RunState::Started => {
                if current < self.progress_current {
                    return Err(OrchestratorError::InvalidTransition {
                        run_id: self.run_id.clone(),
                        from: self.state,
                        to: RunState::Started,
                    });
                }
                self.progress_current = if self.progress_total > 0 { current.min(self.progress_total) } else { current };
                Ok(())
            }
            RunState::Requested | RunState::Completed | RunState::Failed | RunState::Cancelled | RunState::Inconclusive => {
                Err(OrchestratorError::InvalidTransition {
                    run_id: self.run_id.clone(),
                    from: self.state,
                    to: RunState::Started,
                })
            }
        }
    }

    /// When a started run completes, `complete` shall transition `Started -> Completed` and record `finished_at` and `evidence_id`.
    pub fn complete(&mut self, timestamp: &str, evidence_id: Option<String>) -> Result<(), OrchestratorError> {
        self.finish(RunState::Completed, timestamp, evidence_id, None)?;
        self.progress_current = self.progress_total;
        Ok(())
    }

    /// When a started run fails, `fail` shall transition `Started -> Failed` and record `finished_at`, `error`, and optional `evidence_id`.
    pub fn fail(&mut self, timestamp: &str, error: String, evidence_id: Option<String>) -> Result<(), OrchestratorError> {
        self.finish(RunState::Failed, timestamp, evidence_id, Some(error))
    }

    /// When a requested or started run is cancelled, `cancel` shall transition to `Cancelled`, record optional `evidence_id` and `reason`, and reject terminal runs.
    pub fn cancel(&mut self, timestamp: &str, evidence_id: Option<String>, reason: Option<String>) -> Result<(), OrchestratorError> {
        self.finish(RunState::Cancelled, timestamp, evidence_id, reason)
    }

    /// When a started run is inconclusive, `mark_inconclusive` shall transition `Started -> Inconclusive`.
    pub fn mark_inconclusive(&mut self, timestamp: &str, evidence_id: Option<String>, reason: Option<String>) -> Result<(), OrchestratorError> {
        self.finish(RunState::Inconclusive, timestamp, evidence_id, reason)
    }
}

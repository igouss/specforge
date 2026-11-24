use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::graph::{ArtifactId, ContentHash, GraphSnapshot, SourceRevision, VerificationMethod};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Requested,
    Started,
    Completed,
    Failed,
    Cancelled,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationResult {
    Satisfied,
    Violated,
    Inconclusive,
    NotEvaluated,
    InvalidEvaluation,
}

impl fmt::Display for EvaluationResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label: &str = match self {
            EvaluationResult::Satisfied => "satisfied",
            EvaluationResult::Violated => "violated",
            EvaluationResult::Inconclusive => "inconclusive",
            EvaluationResult::NotEvaluated => "not_evaluated",
            EvaluationResult::InvalidEvaluation => "invalid_evaluation",
        };
        formatter.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FormalOutcome {
    Proved,
    Refuted,
    BoundedSuccess { bound: String },
    BoundedFailure { bound: String },
    Unknown,
    Timeout,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "category", content = "value", rename_all = "snake_case")]
pub enum VerificationOutcome {
    Evaluation(EvaluationResult),
    Formal(FormalOutcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Supported,
    Refuted,
    Inconclusive,
    RequiresReview,
    Stale,
    Invalid,
}

impl fmt::Display for ClaimStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label: &str = match self {
            ClaimStatus::Supported => "supported",
            ClaimStatus::Refuted => "refuted",
            ClaimStatus::Inconclusive => "inconclusive",
            ClaimStatus::RequiresReview => "requires_review",
            ClaimStatus::Stale => "stale",
            ClaimStatus::Invalid => "invalid",
        };
        formatter.write_str(label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootCause {
    Unresolved,
    Implementation,
    Specification,
    BehavioralModel,
    EnvironmentOrAssumption,
    Verification,
    ExpectedBehavior,
}

impl fmt::Display for RootCause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label: &str = match self {
            RootCause::Unresolved => "unresolved",
            RootCause::Implementation => "implementation",
            RootCause::Specification => "specification",
            RootCause::BehavioralModel => "behavioral_model",
            RootCause::EnvironmentOrAssumption => "environment_or_assumption",
            RootCause::Verification => "verification",
            RootCause::ExpectedBehavior => "expected_behavior",
        };
        formatter.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub step: usize,
    pub event: String,
    pub from_state: String,
    pub to_state: String,
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
    #[serde(default)]
    pub fault: Option<String>,
    #[serde(default)]
    pub virtual_time: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub initial_state: String,
    pub events: Vec<TraceEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViolationDetail {
    pub step: usize,
    pub observed: String,
    pub expected: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyEvaluation {
    pub property_id: ArtifactId,
    pub property_digest: ContentHash,
    pub evaluator_id: String,
    pub evaluator_version: String,
    pub result: EvaluationResult,
    pub violation: Option<ViolationDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingRecord {
    pub id: String,
    pub property_id: ArtifactId,
    pub run_id: String,
    pub root_cause: RootCause,
    pub counterexample_id: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayDescriptor {
    pub engine: String,
    pub engine_version: String,
    pub seed: Option<u64>,
    pub replay_token: Option<String>,
    pub events: Vec<String>,
    pub determinism: String,
    pub replayable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinimizationMeta {
    pub method: String,
    pub version: String,
    pub attempts: usize,
    pub original_len: usize,
    pub minimized_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counterexample {
    pub id: String,
    pub property_id: ArtifactId,
    pub property_digest: ContentHash,
    pub finding_id: String,
    pub run_id: String,
    pub snapshot_id: ContentHash,
    pub initial_state: String,
    pub events: Vec<TraceEvent>,
    pub violation: ViolationDetail,
    pub replay: ReplayDescriptor,
    pub original_id: Option<String>,
    pub minimization: Option<MinimizationMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationDecision {
    pub id: String,
    pub finding_id: String,
    pub evidence_ids: Vec<String>,
    pub diagnosis: RootCause,
    pub rationale: String,
    pub disposition: String,
    pub approver: String,
    pub authorized_change: Option<String>,
}

impl VerificationDecision {
    /// When a finding is dispositioned, `dispose` shall validate required fields, refuse authorized changes for unresolved diagnoses, and create an immutable decision record without modifying any graph artifact.
    #[allow(clippy::too_many_arguments)]
    pub fn dispose(
        id: String,
        finding: &FindingRecord,
        evidence_ids: Vec<String>,
        diagnosis: RootCause,
        rationale: String,
        disposition: String,
        approver: String,
        authorized_change: Option<String>,
    ) -> Result<VerificationDecision, String> {
        if id.trim().is_empty() {
            return Err("decision id must not be empty".to_string());
        }
        if rationale.trim().is_empty() {
            return Err("decision rationale must not be empty".to_string());
        }
        if disposition.trim().is_empty() {
            return Err("decision disposition must not be empty".to_string());
        }
        if approver.trim().is_empty() {
            return Err("decision approver must not be empty".to_string());
        }
        if diagnosis == RootCause::Unresolved && authorized_change.is_some() {
            return Err("cannot authorize a change while diagnosis is unresolved".to_string());
        }
        Ok(VerificationDecision {
            id,
            finding_id: finding.id.clone(),
            evidence_ids,
            diagnosis,
            rationale,
            disposition,
            approver,
            authorized_change,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    pub adapter_id: String,
    pub adapter_version: String,
    pub engine_name: String,
    pub engine_version: String,
    pub methods: Vec<VerificationMethod>,
    pub supports_replay: bool,
    pub supports_shrinking: bool,
    pub supports_coverage: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRequest {
    pub run_id: String,
    pub activity_id: String,
    pub property_id: ArtifactId,
    pub claim_ids: Vec<ArtifactId>,
    pub budget: usize,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEnvelope {
    pub run_id: String,
    pub activity_id: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub engine_name: String,
    pub engine_version: String,
    pub snapshot_id: ContentHash,
    pub source_revisions: Vec<SourceRevision>,
    pub property_id: ArtifactId,
    pub claim_ids: Vec<ArtifactId>,
    pub method: VerificationMethod,
    pub scope: String,
    pub assumptions: Vec<ArtifactId>,
    pub run_state: RunState,
    pub outcome: VerificationOutcome,
    pub evaluation: Option<PropertyEvaluation>,
    pub finding: Option<FindingRecord>,
    pub original_counterexample: Option<Counterexample>,
    pub minimized_counterexample: Option<Counterexample>,
    pub replay: Option<ReplayDescriptor>,
    pub limitations: Vec<String>,
    pub raw_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    Unsupported(String),
    InvalidRequest(String),
    NotReplayable(String),
    Execution(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterError::Unsupported(why) => write!(formatter, "unsupported adapter capability: {why}"),
            AdapterError::InvalidRequest(why) => write!(formatter, "invalid verification request: {why}"),
            AdapterError::NotReplayable(why) => write!(formatter, "counterexample is not replayable: {why}"),
            AdapterError::Execution(why) => write!(formatter, "adapter execution failed: {why}"),
        }
    }
}

/// When an external verification engine integrates with SpecForge, `VerificationAdapter` shall define its capability, validation, execution, and replay contract.
pub trait VerificationAdapter {
    fn capabilities(&self) -> AdapterCapabilities;
    fn validate(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<(), AdapterError>;
    fn execute(&self, snapshot: &GraphSnapshot, request: &VerificationRequest) -> Result<RunEnvelope, AdapterError>;
    fn replay(&self, snapshot: &GraphSnapshot, counterexample: &Counterexample, run_id: &str) -> Result<RunEnvelope, AdapterError>;
}

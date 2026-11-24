mod argument;
mod dst;
mod dst_checks;
mod formal;
mod formal_checks;

use std::collections::BTreeMap;

use specforge::argument::AssuranceArgument;
use specforge::dst::DstReplayMetadata;
use specforge::graph::ContentHash;
use specforge::verify::{AdapterError, RunEnvelope};

pub type FormalNormalization = Result<(RunEnvelope, BTreeMap<String, ContentHash>), AdapterError>;

#[derive(Debug, Default)]
pub struct ExternalAssuranceState {
    pub dst_result: Option<Result<(RunEnvelope, ContentHash), AdapterError>>,
    pub dst_replay_metadata: Option<DstReplayMetadata>,
    pub dst_replay_envelope: Option<RunEnvelope>,
    pub formal_result: Option<FormalNormalization>,
    pub argument_result: Option<Result<AssuranceArgument, String>>,
    pub evidence_counter: usize,
}

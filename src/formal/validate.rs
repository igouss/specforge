use std::collections::BTreeMap;

use super::{FormalBounds, FormalCheckReport};
use crate::graph::{ContentHash, GraphSnapshot, Property, VerificationClaim};
use crate::verify::{AdapterError, ExecutionTrace};

pub(super) const FORMAL_PROPERTY_REF_KEY: &str = "formal_property_ref";

pub(super) struct ValidatedFormalInputs<'a> {
    pub(super) property: &'a Property,
    pub(super) property_digest: ContentHash,
    pub(super) claims: Vec<&'a VerificationClaim>,
    pub(super) artifact_digests: BTreeMap<String, ContentHash>,
}

pub(super) fn validate_formal_inputs<'a>(
    snapshot: &'a GraphSnapshot,
    request: &crate::verify::VerificationRequest,
    report: &FormalCheckReport,
) -> Result<ValidatedFormalInputs<'a>, AdapterError> {
    if request.run_id.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("run_id must not be empty".to_string()));
    }
    if request.activity_id.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("activity_id must not be empty".to_string()));
    }
    if report.adapter_id.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("adapter_id must not be empty".to_string()));
    }
    if report.adapter_version.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("adapter_version must not be empty".to_string()));
    }
    if report.engine_name.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("engine_name must not be empty".to_string()));
    }
    if report.engine_version.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("engine_version must not be empty".to_string()));
    }
    if report.raw_status.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("raw_status must not be empty".to_string()));
    }
    if report.property_ref.property_id != request.property_id {
        return Err(AdapterError::InvalidRequest(format!(
            "formal property_ref {} does not match requested property {}",
            report.property_ref.property_id, request.property_id
        )));
    }
    if report.property_ref.external_tool.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("property_ref.external_tool must not be empty".to_string()));
    }
    if report.property_ref.module.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("property_ref.module must not be empty".to_string()));
    }
    if report.property_ref.symbol.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("property_ref.symbol must not be empty".to_string()));
    }
    let property: &Property = snapshot
        .property(request.property_id.as_str())
        .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown property {}", request.property_id)))?;
    let property_digest: ContentHash = snapshot
        .artifact_digest(property.id.as_str())
        .cloned()
        .ok_or_else(|| AdapterError::InvalidRequest(format!("missing digest for property {}", property.id)))?;
    if let Some(ref declared_digest) = report.property_ref.digest {
        if *declared_digest != property_digest {
            return Err(AdapterError::InvalidRequest(format!(
                "formal property_ref digest {} does not match snapshot digest {} for {}",
                declared_digest, property_digest, property.id
            )));
        }
    }
    let mut claims: Vec<&'a VerificationClaim> = Vec::new();
    for claim_id in &request.claim_ids {
        let claim: &'a VerificationClaim = snapshot
            .claim(claim_id.as_str())
            .ok_or_else(|| AdapterError::InvalidRequest(format!("unknown claim {claim_id}")))?;
        claims.push(claim);
    }
    for assumption_id in &report.assumptions {
        if snapshot.assumption(assumption_id.as_str()).is_none() {
            return Err(AdapterError::InvalidRequest(format!("unknown assumption {assumption_id}")));
        }
    }
    let mut artifact_digests: BTreeMap<String, ContentHash> = BTreeMap::new();
    for artifact in &report.artifacts {
        if artifact.name.trim().is_empty() {
            return Err(AdapterError::InvalidRequest("formal artifact name must not be empty".to_string()));
        }
        if artifact.name == FORMAL_PROPERTY_REF_KEY {
            return Err(AdapterError::InvalidRequest(format!(
                "formal artifact name \"{FORMAL_PROPERTY_REF_KEY}\" is reserved for property reference provenance"
            )));
        }
        if artifact.kind.trim().is_empty() {
            return Err(AdapterError::InvalidRequest(format!(
                "formal artifact \"{}\" must declare a non-empty kind",
                artifact.name
            )));
        }
        if artifact_digests.insert(artifact.name.clone(), artifact.digest.clone()).is_some() {
            return Err(AdapterError::InvalidRequest(format!("duplicate formal artifact name \"{}\"", artifact.name)));
        }
    }
    Ok(ValidatedFormalInputs {
        property,
        property_digest,
        claims,
        artifact_digests,
    })
}

pub(super) fn validate_positive_assumptions(
    report: &FormalCheckReport,
    property: &Property,
    claims: &[&VerificationClaim],
    outcome_label: &str,
) -> Result<(), AdapterError> {
    for assumption_id in &report.assumptions {
        if !property.assumptions.contains(assumption_id) {
            return Err(AdapterError::InvalidRequest(format!(
                "{outcome_label} formal report relies on assumption {assumption_id} not declared on property {}",
                property.id
            )));
        }
        for claim in claims {
            if !claim.assumptions.contains(assumption_id) {
                return Err(AdapterError::InvalidRequest(format!(
                    "{outcome_label} formal report relies on assumption {assumption_id} not declared on claim {}",
                    claim.id
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_counterexample_trace(trace: &ExecutionTrace) -> Result<(), AdapterError> {
    if trace.initial_state.trim().is_empty() {
        return Err(AdapterError::InvalidRequest("counterexample trace initial_state must not be empty".to_string()));
    }
    let mut previous_state: &str = trace.initial_state.as_str();
    for event in &trace.events {
        if event.event.trim().is_empty() {
            return Err(AdapterError::InvalidRequest(format!(
                "counterexample trace event at step {} must have a non-empty event name",
                event.step
            )));
        }
        if event.from_state.trim().is_empty() {
            return Err(AdapterError::InvalidRequest(format!(
                "counterexample trace event at step {} must have a non-empty from_state",
                event.step
            )));
        }
        if event.to_state.trim().is_empty() {
            return Err(AdapterError::InvalidRequest(format!(
                "counterexample trace event at step {} must have a non-empty to_state",
                event.step
            )));
        }
        if event.from_state != previous_state {
            return Err(AdapterError::InvalidRequest(format!(
                "counterexample trace is discontinuous at step {}: expected from_state \"{previous_state}\", got \"{}\"",
                event.step, event.from_state
            )));
        }
        previous_state = event.to_state.as_str();
    }
    Ok(())
}

pub(super) fn format_bounds_limitation(bound: &str, bounds: Option<&FormalBounds>, is_success: bool) -> String {
    let mut parts: Vec<String> = vec![format!("bound={bound}")];
    if let Some(declared) = bounds {
        if let Some(depth) = declared.max_depth {
            parts.push(format!("max_depth={depth}"));
        }
        if let Some(states) = declared.max_states {
            parts.push(format!("max_states={states}"));
        }
        if let Some(timeout) = declared.timeout_seconds {
            parts.push(format!("timeout_seconds={timeout}"));
        }
        if !declared.description.trim().is_empty() {
            parts.push(declared.description.trim().to_string());
        }
    }
    let joined: String = parts.join("; ");
    if is_success {
        format!("bounded formal verification succeeded within {joined}; does not constitute an unbounded proof")
    } else {
        format!("bounded formal verification found a counterexample within {joined}")
    }
}

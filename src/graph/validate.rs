use std::collections::BTreeSet;

use super::{ArtifactId, Assumption, GraphDefect, Property, StateMachine, VerificationClaim};
use crate::trace::RequirementId;

pub(super) struct SpecBlock {
    pub(super) document: usize,
    pub(super) path: String,
    pub(super) line: usize,
    pub(super) text: String,
}

pub(super) struct OrderedDefect {
    pub(super) document: usize,
    pub(super) line: usize,
    pub(super) defect: GraphDefect,
}

pub(super) struct RefCheck {
    pub(super) document: usize,
    pub(super) path: String,
    pub(super) line: usize,
    pub(super) owner: String,
    pub(super) referenced: String,
}

pub(super) struct SubjectCheck {
    pub(super) document: usize,
    pub(super) path: String,
    pub(super) line: usize,
    pub(super) property: ArtifactId,
    pub(super) referenced: String,
}

pub(super) struct PropertyCheck {
    pub(super) document: usize,
    pub(super) path: String,
    pub(super) line: usize,
    pub(super) claim: ArtifactId,
    pub(super) referenced: String,
}

pub(super) struct TargetCheck {
    pub(super) document: usize,
    pub(super) path: String,
    pub(super) line: usize,
    pub(super) plan: ArtifactId,
    pub(super) referenced: String,
}

#[derive(Default)]
pub(super) struct RawRefs {
    pub(super) assumption_checks: Vec<RefCheck>,
    pub(super) subject_checks: Vec<SubjectCheck>,
    pub(super) property_checks: Vec<PropertyCheck>,
    pub(super) target_checks: Vec<TargetCheck>,
}

pub(super) fn parse_artifact_id(raw: &str, block: &SpecBlock, defects: &mut Vec<OrderedDefect>) -> Option<ArtifactId> {
    match ArtifactId::parse(raw) {
        Some(id) => Some(id),
        None => {
            defects.push(OrderedDefect {
                document: block.document,
                line: block.line,
                defect: GraphDefect::InvalidId {
                    path: block.path.clone(),
                    line: block.line,
                    raw: raw.to_string(),
                },
            });
            None
        }
    }
}

pub(super) fn parse_req_ids(
    raw_ids: &[String],
    owner: &str,
    block: &SpecBlock,
    known_reqs: &BTreeSet<String>,
    defects: &mut Vec<OrderedDefect>,
) -> Vec<RequirementId> {
    let mut parsed: Vec<RequirementId> = Vec::new();
    for raw in raw_ids {
        match RequirementId::parse(raw) {
            Some(id) if known_reqs.contains(id.as_str()) => parsed.push(id),
            Some(_) | None => defects.push(OrderedDefect {
                document: block.document,
                line: block.line,
                defect: GraphDefect::UnknownRequirement {
                    path: block.path.clone(),
                    line: block.line,
                    artifact: owner.to_string(),
                    id: raw.clone(),
                },
            }),
        }
    }
    parsed
}

pub(super) fn register_id(
    id: &ArtifactId,
    block: &SpecBlock,
    known_reqs: &BTreeSet<String>,
    seen: &mut BTreeSet<ArtifactId>,
    defects: &mut Vec<OrderedDefect>,
) -> bool {
    if !known_reqs.contains(id.as_str()) && seen.insert(id.clone()) {
        true
    } else {
        defects.push(OrderedDefect {
            document: block.document,
            line: block.line,
            defect: GraphDefect::DuplicateArtifact {
                path: block.path.clone(),
                line: block.line,
                id: id.clone(),
            },
        });
        false
    }
}

pub(super) fn require_non_empty(value: &str, field: &str, block: &SpecBlock, defects: &mut Vec<OrderedDefect>) {
    if value.trim().is_empty() {
        defects.push(OrderedDefect {
            document: block.document,
            line: block.line,
            defect: GraphDefect::InvalidBlock {
                path: block.path.clone(),
                line: block.line,
                why: format!("field \"{field}\" must not be empty"),
            },
        });
    }
}

pub(super) fn validate_cross_references(
    assumptions: &[Assumption],
    state_machines: &[StateMachine],
    properties: &[Property],
    claims: &[VerificationClaim],
    raw_refs: &RawRefs,
    defects: &mut Vec<OrderedDefect>,
) {
    let assumption_ids: BTreeSet<&str> = assumptions.iter().map(|item: &Assumption| item.id.as_str()).collect();
    let machine_ids: BTreeSet<&str> = state_machines.iter().map(|item: &StateMachine| item.id.as_str()).collect();
    let property_ids: BTreeSet<&str> = properties.iter().map(|item: &Property| item.id.as_str()).collect();
    let claim_ids: BTreeSet<&str> = claims.iter().map(|item: &VerificationClaim| item.id.as_str()).collect();

    for check in &raw_refs.assumption_checks {
        if !assumption_ids.contains(check.referenced.as_str()) {
            defects.push(OrderedDefect {
                document: check.document,
                line: check.line,
                defect: GraphDefect::UnknownAssumption {
                    path: check.path.clone(),
                    line: check.line,
                    artifact: check.owner.clone(),
                    id: check.referenced.clone(),
                },
            });
        }
    }
    for check in &raw_refs.subject_checks {
        if !machine_ids.contains(check.referenced.as_str()) {
            defects.push(OrderedDefect {
                document: check.document,
                line: check.line,
                defect: GraphDefect::UnknownSubject {
                    path: check.path.clone(),
                    line: check.line,
                    property: check.property.clone(),
                    subject: check.referenced.clone(),
                },
            });
        }
    }
    for check in &raw_refs.property_checks {
        if !property_ids.contains(check.referenced.as_str()) {
            defects.push(OrderedDefect {
                document: check.document,
                line: check.line,
                defect: GraphDefect::UnknownProperty {
                    path: check.path.clone(),
                    line: check.line,
                    claim: check.claim.clone(),
                    property: check.referenced.clone(),
                },
            });
        }
    }
    for check in &raw_refs.target_checks {
        let exists: bool = property_ids.contains(check.referenced.as_str()) || claim_ids.contains(check.referenced.as_str());
        if !exists {
            defects.push(OrderedDefect {
                document: check.document,
                line: check.line,
                defect: GraphDefect::UnknownTarget {
                    path: check.path.clone(),
                    line: check.line,
                    plan: check.plan.clone(),
                    target: check.referenced.clone(),
                },
            });
        }
    }
}

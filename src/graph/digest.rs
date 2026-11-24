use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    ArtifactId, Assumption, PlanActivity, Predicate, Property, PropertyKind, SourceRevision, StateMachine, Transition, VerificationClaim, VerificationMethod,
    VerificationPlan,
};
use crate::trace::{RequirementId, Story, Trace};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ContentHash(String);

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D>(deserializer: D) -> Result<ContentHash, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: String = String::deserialize(deserializer)?;
        ContentHash::from_hex(&raw).ok_or_else(|| serde::de::Error::custom(format!("invalid SHA-256 hex digest \"{raw}\"")))
    }
}

impl ContentHash {
    /// When bytes are hashed, `of_bytes` shall return their lowercase hex SHA-256 digest.
    pub fn of_bytes(bytes: &[u8]) -> ContentHash {
        let mut hasher: Sha256 = Sha256::new();
        hasher.update(bytes);
        ContentHash(hex::encode(hasher.finalize()))
    }

    /// When a serializable value is hashed, `of_canonical_json` shall return the SHA-256 digest of its deterministic JSON representation.
    pub fn of_canonical_json<T: Serialize>(value: &T) -> Result<ContentHash, String> {
        let canonical: String = canonical_json(value)?;
        Ok(ContentHash::of_bytes(canonical.as_bytes()))
    }

    /// When a hex digest string is wrapped without hashing, `from_hex` shall accept only 64 lowercase hexadecimal characters.
    pub fn from_hex(text: &str) -> Option<ContentHash> {
        let valid: bool = text.len() == 64 && text.chars().all(|c: char| c.is_ascii_digit() || ('a'..='f').contains(&c));
        valid.then(|| ContentHash(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn canonical_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Null => serde_json::Value::Null,
        serde_json::Value::Bool(flag) => serde_json::Value::Bool(flag),
        serde_json::Value::Number(number) => serde_json::Value::Number(number),
        serde_json::Value::String(text) => serde_json::Value::String(text),
        serde_json::Value::Array(items) => serde_json::Value::Array(items.into_iter().map(canonical_value).collect()),
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> = map
                .into_iter()
                .map(|(key, item): (String, serde_json::Value)| (key, canonical_value(item)))
                .collect();
            let mut out: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
            for (key, item) in sorted {
                out.insert(key, item);
            }
            serde_json::Value::Object(out)
        }
    }
}

/// When a value is serialized canonically, `canonical_json` shall sort all object keys recursively and emit compact deterministic JSON.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<String, String> {
    let raw: serde_json::Value = serde_json::to_value(value).map_err(|error: serde_json::Error| error.to_string())?;
    let sorted: serde_json::Value = canonical_value(raw);
    serde_json::to_string(&sorted).map_err(|error: serde_json::Error| error.to_string())
}

#[derive(Serialize)]
struct RequirementSemantic<'a> {
    id: &'a str,
    version: &'a str,
    title: &'a str,
    statement: &'a str,
    stories: Vec<(&'a str, &'a str)>,
}

#[derive(Serialize)]
struct AssumptionSemantic<'a> {
    id: &'a str,
    statement: &'a str,
    scope: &'a str,
}

#[derive(Serialize)]
struct StateMachineSemantic<'a> {
    id: &'a str,
    requirements: Vec<&'a str>,
    initial: &'a str,
    states: &'a [String],
    transitions: &'a [Transition],
}

#[derive(Serialize)]
struct PropertySemantic<'a> {
    id: &'a str,
    name: &'a str,
    property_kind: PropertyKind,
    subject: &'a str,
    predicate: &'a Predicate,
    scope: &'a str,
    assumptions: Vec<&'a str>,
    requirements: Vec<&'a str>,
    methods: &'a [VerificationMethod],
}

#[derive(Serialize)]
struct ClaimSemantic<'a> {
    id: &'a str,
    proposition: &'a str,
    scope: &'a str,
    assumptions: Vec<&'a str>,
    properties: Vec<&'a str>,
    requirements: Vec<&'a str>,
}

#[derive(Serialize)]
struct PlanSemantic<'a> {
    id: &'a str,
    targets: Vec<&'a str>,
    scope: &'a str,
    assumptions: Vec<&'a str>,
    activities: &'a [PlanActivity],
}

#[derive(Serialize)]
struct SnapshotManifest<'a> {
    sources: &'a [SourceRevision],
    digests: &'a BTreeMap<String, ContentHash>,
}

pub(crate) fn semantic_digests(
    trace: &Trace,
    assumptions: &[Assumption],
    state_machines: &[StateMachine],
    properties: &[Property],
    claims: &[VerificationClaim],
    plans: &[VerificationPlan],
) -> Result<BTreeMap<String, ContentHash>, String> {
    let mut digests: BTreeMap<String, ContentHash> = BTreeMap::new();
    for requirement in &trace.requirements {
        let semantic: RequirementSemantic<'_> = RequirementSemantic {
            id: requirement.id.as_str(),
            version: &requirement.version,
            title: &requirement.title,
            statement: &requirement.statement,
            stories: requirement
                .stories
                .iter()
                .map(|story: &Story| (story.name.as_str(), story.text.as_str()))
                .collect(),
        };
        digests.insert(requirement.id.as_str().to_string(), ContentHash::of_canonical_json(&semantic)?);
    }
    for assumption in assumptions {
        let semantic: AssumptionSemantic<'_> = AssumptionSemantic {
            id: assumption.id.as_str(),
            statement: &assumption.statement,
            scope: &assumption.scope,
        };
        digests.insert(assumption.id.as_str().to_string(), ContentHash::of_canonical_json(&semantic)?);
    }
    for machine in state_machines {
        let semantic: StateMachineSemantic<'_> = StateMachineSemantic {
            id: machine.id.as_str(),
            requirements: machine.requirements.iter().map(RequirementId::as_str).collect(),
            initial: &machine.initial,
            states: &machine.states,
            transitions: &machine.transitions,
        };
        digests.insert(machine.id.as_str().to_string(), ContentHash::of_canonical_json(&semantic)?);
    }
    for property in properties {
        let semantic: PropertySemantic<'_> = PropertySemantic {
            id: property.id.as_str(),
            name: &property.name,
            property_kind: property.property_kind,
            subject: property.subject.as_str(),
            predicate: &property.predicate,
            scope: &property.scope,
            assumptions: property.assumptions.iter().map(ArtifactId::as_str).collect(),
            requirements: property.requirements.iter().map(RequirementId::as_str).collect(),
            methods: &property.methods,
        };
        digests.insert(property.id.as_str().to_string(), ContentHash::of_canonical_json(&semantic)?);
    }
    for claim in claims {
        let semantic: ClaimSemantic<'_> = ClaimSemantic {
            id: claim.id.as_str(),
            proposition: &claim.proposition,
            scope: &claim.scope,
            assumptions: claim.assumptions.iter().map(ArtifactId::as_str).collect(),
            properties: claim.properties.iter().map(ArtifactId::as_str).collect(),
            requirements: claim.requirements.iter().map(RequirementId::as_str).collect(),
        };
        digests.insert(claim.id.as_str().to_string(), ContentHash::of_canonical_json(&semantic)?);
    }
    for plan in plans {
        let semantic: PlanSemantic<'_> = PlanSemantic {
            id: plan.id.as_str(),
            targets: plan.targets.iter().map(ArtifactId::as_str).collect(),
            scope: &plan.scope,
            assumptions: plan.assumptions.iter().map(ArtifactId::as_str).collect(),
            activities: &plan.activities,
        };
        digests.insert(plan.id.as_str().to_string(), ContentHash::of_canonical_json(&semantic)?);
    }
    Ok(digests)
}

pub(super) fn snapshot_digest(sources: &[SourceRevision], digests: &BTreeMap<String, ContentHash>) -> Result<ContentHash, String> {
    let manifest: SnapshotManifest<'_> = SnapshotManifest { sources, digests };
    ContentHash::of_canonical_json(&manifest)
}

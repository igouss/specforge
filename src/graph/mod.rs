mod compile;
mod digest;
mod validate;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::trace::{Defect, RequirementId, Trace};

pub use compile::compile_graph;
pub use digest::{canonical_json, ContentHash};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ArtifactId(String);

impl<'de> Deserialize<'de> for ArtifactId {
    fn deserialize<D>(deserializer: D) -> Result<ArtifactId, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: String = String::deserialize(deserializer)?;
        ArtifactId::parse(&raw).ok_or_else(|| serde::de::Error::custom(format!("invalid artifact id \"{raw}\"")))
    }
}

impl ArtifactId {
    /// When a text consists of one or more uppercase ASCII segments separated by hyphens followed by a hyphen and digits, `parse` shall return it as an artifact id.
    /// If the text has any other form, then `parse` shall return nothing.
    pub fn parse(text: &str) -> Option<ArtifactId> {
        let parts: Vec<&str> = text.split('-').collect();
        if parts.len() < 2 {
            return None;
        }
        let (last, prefix): (&str, &[&str]) = match parts.split_last() {
            Some((last, prefix)) => (*last, prefix),
            None => return None,
        };
        let digits_fit: bool = !last.is_empty() && last.chars().all(|c: char| c.is_ascii_digit());
        let prefix_fit: bool = prefix
            .iter()
            .all(|segment: &&str| !segment.is_empty() && segment.chars().all(|c: char| c.is_ascii_uppercase()));
        (digits_fit && prefix_fit).then(|| ArtifactId(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRevision {
    pub path: String,
    pub digest: ContentHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assumption {
    pub id: ArtifactId,
    pub statement: String,
    pub scope: String,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub name: String,
    pub from: String,
    pub to: String,
    pub event: String,
    #[serde(default)]
    pub guard: Option<String>,
    #[serde(default)]
    pub effect: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateMachine {
    pub id: ArtifactId,
    pub requirements: Vec<RequirementId>,
    pub initial: String,
    pub states: Vec<String>,
    pub transitions: Vec<Transition>,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyKind {
    Safety,
    Liveness,
    BoundedResponse,
    StateInvariant,
    Relational,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Predicate {
    pub language: String,
    pub expression: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    ExampleBased,
    ModelBased,
    PropertyBased,
    DeterministicSimulation,
    Formal,
}

impl VerificationMethod {
    /// When a verification method is formatted as a canonical label, `as_str` shall return its snake_case identifier.
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationMethod::ExampleBased => "example_based",
            VerificationMethod::ModelBased => "model_based",
            VerificationMethod::PropertyBased => "property_based",
            VerificationMethod::DeterministicSimulation => "deterministic_simulation",
            VerificationMethod::Formal => "formal",
        }
    }
}

impl fmt::Display for VerificationMethod {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub id: ArtifactId,
    pub name: String,
    pub property_kind: PropertyKind,
    pub subject: ArtifactId,
    pub predicate: Predicate,
    pub scope: String,
    pub assumptions: Vec<ArtifactId>,
    pub requirements: Vec<RequirementId>,
    pub methods: Vec<VerificationMethod>,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationClaim {
    pub id: ArtifactId,
    pub proposition: String,
    pub scope: String,
    pub assumptions: Vec<ArtifactId>,
    pub properties: Vec<ArtifactId>,
    pub requirements: Vec<RequirementId>,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanActivity {
    pub id: ArtifactId,
    pub method: VerificationMethod,
    pub budget: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationPlan {
    pub id: ArtifactId,
    pub targets: Vec<ArtifactId>,
    pub scope: String,
    pub assumptions: Vec<ArtifactId>,
    pub activities: Vec<PlanActivity>,
    #[serde(default)]
    pub required_evidence: Vec<String>,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyPin {
    pub artifact_id: String,
    pub digest: ContentHash,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphSnapshot {
    pub snapshot_id: ContentHash,
    pub sources: Vec<SourceRevision>,
    pub trace: Trace,
    pub assumptions: Vec<Assumption>,
    pub state_machines: Vec<StateMachine>,
    pub properties: Vec<Property>,
    pub claims: Vec<VerificationClaim>,
    pub plans: Vec<VerificationPlan>,
    pub digests: BTreeMap<String, ContentHash>,
}

impl GraphSnapshot {
    /// When an artifact or requirement id is queried, `artifact_digest` shall return its semantic content digest if present.
    pub fn artifact_digest(&self, id: &str) -> Option<&ContentHash> {
        self.digests.get(id)
    }

    pub fn assumption(&self, id: &str) -> Option<&Assumption> {
        self.assumptions.iter().find(|item: &&Assumption| item.id.as_str() == id)
    }

    pub fn state_machine(&self, id: &str) -> Option<&StateMachine> {
        self.state_machines.iter().find(|item: &&StateMachine| item.id.as_str() == id)
    }

    pub fn property(&self, id: &str) -> Option<&Property> {
        self.properties.iter().find(|item: &&Property| item.id.as_str() == id)
    }

    pub fn claim(&self, id: &str) -> Option<&VerificationClaim> {
        self.claims.iter().find(|item: &&VerificationClaim| item.id.as_str() == id)
    }

    pub fn plan(&self, id: &str) -> Option<&VerificationPlan> {
        self.plans.iter().find(|item: &&VerificationPlan| item.id.as_str() == id)
    }

    /// When verification artifact count is queried, `artifact_count` shall return the total number of assumptions, state machines, properties, claims, and plans.
    pub fn artifact_count(&self) -> usize {
        self.assumptions.len() + self.state_machines.len() + self.properties.len() + self.claims.len() + self.plans.len()
    }

    /// When dependency pins for a property are requested, `dependency_pins_for_property` shall return sorted digest pins for the property, its subject, linked assumptions, and linked requirements.
    pub fn dependency_pins_for_property(&self, property_id: &str) -> Vec<DependencyPin> {
        let Some(property): Option<&Property> = self.property(property_id) else {
            return Vec::new();
        };
        let mut ids: BTreeSet<String> = BTreeSet::new();
        ids.insert(property.id.as_str().to_string());
        ids.insert(property.subject.as_str().to_string());
        if let Some(machine) = self.state_machine(property.subject.as_str()) {
            for requirement in &machine.requirements {
                ids.insert(requirement.as_str().to_string());
            }
        }
        for assumption in &property.assumptions {
            ids.insert(assumption.as_str().to_string());
        }
        for requirement in &property.requirements {
            ids.insert(requirement.as_str().to_string());
        }
        pins_for_ids(self, &ids)
    }

    /// When dependency pins for a claim are requested, `dependency_pins_for_claim` shall return sorted digest pins for the claim, its linked assumptions, requirements, properties, and each property's dependencies.
    pub fn dependency_pins_for_claim(&self, claim_id: &str) -> Vec<DependencyPin> {
        let Some(claim): Option<&VerificationClaim> = self.claim(claim_id) else {
            return Vec::new();
        };
        let mut ids: BTreeSet<String> = BTreeSet::new();
        ids.insert(claim.id.as_str().to_string());
        for assumption in &claim.assumptions {
            ids.insert(assumption.as_str().to_string());
        }
        for requirement in &claim.requirements {
            ids.insert(requirement.as_str().to_string());
        }
        for property in &claim.properties {
            ids.insert(property.as_str().to_string());
            for pin in self.dependency_pins_for_property(property.as_str()) {
                ids.insert(pin.artifact_id);
            }
        }
        pins_for_ids(self, &ids)
    }
}

fn pins_for_ids(snapshot: &GraphSnapshot, ids: &BTreeSet<String>) -> Vec<DependencyPin> {
    ids.iter()
        .filter_map(|id: &String| {
            snapshot.digests.get(id).map(|digest: &ContentHash| DependencyPin {
                artifact_id: id.clone(),
                digest: digest.clone(),
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphDefect {
    Trace(Defect),
    InvalidBlock {
        path: String,
        line: usize,
        why: String,
    },
    InvalidId {
        path: String,
        line: usize,
        raw: String,
    },
    DuplicateArtifact {
        path: String,
        line: usize,
        id: ArtifactId,
    },
    UnknownState {
        path: String,
        line: usize,
        machine: ArtifactId,
        state: String,
    },
    UnknownRequirement {
        path: String,
        line: usize,
        artifact: String,
        id: String,
    },
    UnknownAssumption {
        path: String,
        line: usize,
        artifact: String,
        id: String,
    },
    UnknownSubject {
        path: String,
        line: usize,
        property: ArtifactId,
        subject: String,
    },
    UnknownProperty {
        path: String,
        line: usize,
        claim: ArtifactId,
        property: String,
    },
    UnknownTarget {
        path: String,
        line: usize,
        plan: ArtifactId,
        target: String,
    },
}

impl fmt::Display for GraphDefect {
    /// When a graph defect is displayed, `fmt` shall write `<path>:<line>: ` and the sentence SF-007 pins for its kind.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphDefect::Trace(defect) => fmt::Display::fmt(defect, formatter),
            GraphDefect::InvalidBlock { path, line, why } => write!(formatter, "{path}:{line}: the specforge block cannot be parsed: {why}"),
            GraphDefect::InvalidId { path, line, raw } => write!(formatter, "{path}:{line}: \"{raw}\" is not a valid artifact identifier"),
            GraphDefect::DuplicateArtifact { path, line, id } => write!(formatter, "{path}:{line}: artifact {id} is declared a second time"),
            GraphDefect::UnknownState { path, line, machine, state } => {
                write!(formatter, "{path}:{line}: state machine {machine} references unknown state \"{state}\"")
            }
            GraphDefect::UnknownRequirement { path, line, artifact, id } => {
                write!(formatter, "{path}:{line}: {artifact} references unknown requirement {id}")
            }
            GraphDefect::UnknownAssumption { path, line, artifact, id } => {
                write!(formatter, "{path}:{line}: {artifact} references unknown assumption {id}")
            }
            GraphDefect::UnknownSubject { path, line, property, subject } => {
                write!(formatter, "{path}:{line}: property {property} references unknown subject {subject}")
            }
            GraphDefect::UnknownProperty { path, line, claim, property } => {
                write!(formatter, "{path}:{line}: claim {claim} references unknown property {property}")
            }
            GraphDefect::UnknownTarget { path, line, plan, target } => {
                write!(formatter, "{path}:{line}: plan {plan} references unknown target {target}")
            }
        }
    }
}

use std::collections::{BTreeMap, BTreeSet};

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use serde::Deserialize;

use super::digest::{semantic_digests, snapshot_digest, ContentHash};
use super::validate::{
    parse_artifact_id, parse_req_ids, register_id, require_non_empty, validate_cross_references, OrderedDefect, PropertyCheck, RawRefs, RefCheck, SpecBlock,
    SubjectCheck, TargetCheck,
};
use super::{
    ArtifactId, Assumption, GraphDefect, GraphSnapshot, PlanActivity, Predicate, Property, PropertyKind, SourceRevision, StateMachine, Transition,
    VerificationClaim, VerificationMethod, VerificationPlan,
};
use crate::trace::{self, Requirement, RequirementId, Source, Trace};

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RawBlock {
    Assumption {
        id: String,
        statement: String,
        scope: String,
    },
    StateMachine {
        id: String,
        #[serde(default)]
        requirements: Vec<String>,
        initial: String,
        states: Vec<String>,
        #[serde(default)]
        transitions: Vec<Transition>,
    },
    Property {
        id: String,
        name: String,
        property_kind: PropertyKind,
        subject: String,
        predicate: Predicate,
        scope: String,
        #[serde(default)]
        assumptions: Vec<String>,
        #[serde(default)]
        requirements: Vec<String>,
        #[serde(default)]
        methods: Vec<VerificationMethod>,
    },
    Claim {
        id: String,
        proposition: String,
        scope: String,
        #[serde(default)]
        assumptions: Vec<String>,
        #[serde(default)]
        properties: Vec<String>,
        #[serde(default)]
        requirements: Vec<String>,
    },
    Plan {
        id: String,
        #[serde(default)]
        targets: Vec<String>,
        scope: String,
        #[serde(default)]
        assumptions: Vec<String>,
        #[serde(default)]
        activities: Vec<RawPlanActivity>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlanActivity {
    id: String,
    method: VerificationMethod,
    budget: usize,
}

fn line_of(text: &str, offset: usize) -> usize {
    text.get(..offset).map_or(1, |before: &str| before.matches('\n').count() + 1)
}

fn extract_blocks(document: usize, source: &Source, out: &mut Vec<SpecBlock>) {
    let mut active: Option<SpecBlock> = None;
    for (event, range) in Parser::new(&source.text).into_offset_iter() {
        if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = event {
            if info.split_whitespace().next() == Some("specforge") {
                active = Some(SpecBlock {
                    document,
                    path: source.path.clone(),
                    line: line_of(&source.text, range.start),
                    text: String::new(),
                });
            }
        } else if let Event::Text(text) = event {
            if let Some(block) = active.as_mut() {
                block.text.push_str(&text);
            }
        } else if let Event::End(TagEnd::CodeBlock) = event {
            out.extend(active.take());
        }
    }
}

/// When specification documents are compiled into a graph snapshot, `compile_graph` shall yield the hashed snapshot SF-007 describes, or every defect in document then line order.
pub fn compile_graph(sources: &[Source]) -> Result<GraphSnapshot, Vec<GraphDefect>> {
    let trace: Trace = match trace::compile(sources) {
        Ok(trace) => trace,
        Err(trace_defects) => return Err(trace_defects.into_iter().map(GraphDefect::Trace).collect()),
    };
    let known_reqs: BTreeSet<String> = trace
        .requirements
        .iter()
        .map(|requirement: &Requirement| requirement.id.as_str().to_string())
        .collect();
    let mut blocks: Vec<SpecBlock> = Vec::new();
    for (document, source) in sources.iter().enumerate() {
        extract_blocks(document, source, &mut blocks);
    }

    let mut assumptions: Vec<Assumption> = Vec::new();
    let mut state_machines: Vec<StateMachine> = Vec::new();
    let mut properties: Vec<Property> = Vec::new();
    let mut claims: Vec<VerificationClaim> = Vec::new();
    let mut plans: Vec<VerificationPlan> = Vec::new();
    let mut seen: BTreeSet<ArtifactId> = BTreeSet::new();
    let mut defects: Vec<OrderedDefect> = Vec::new();
    let mut raw_refs: RawRefs = RawRefs::default();

    for block in &blocks {
        let parsed: RawBlock = match serde_norway::from_str::<RawBlock>(&block.text) {
            Ok(parsed) => parsed,
            Err(error) => {
                defects.push(OrderedDefect {
                    document: block.document,
                    line: block.line,
                    defect: GraphDefect::InvalidBlock {
                        path: block.path.clone(),
                        line: block.line,
                        why: error.to_string(),
                    },
                });
                continue;
            }
        };
        match parsed {
            RawBlock::Assumption { id, statement, scope } => {
                require_non_empty(&statement, "statement", block, &mut defects);
                require_non_empty(&scope, "scope", block, &mut defects);
                if let Some(id) = parse_artifact_id(&id, block, &mut defects) {
                    if register_id(&id, block, &known_reqs, &mut seen, &mut defects) {
                        assumptions.push(Assumption {
                            id,
                            statement,
                            scope,
                            path: block.path.clone(),
                            line: block.line,
                        });
                    }
                }
            }
            RawBlock::StateMachine {
                id,
                requirements,
                initial,
                states,
                transitions,
            } => {
                require_non_empty(&initial, "initial", block, &mut defects);
                if let Some(id) = parse_artifact_id(&id, block, &mut defects) {
                    let owner: String = format!("state machine {id}");
                    let reqs: Vec<RequirementId> = parse_req_ids(&requirements, &owner, block, &known_reqs, &mut defects);
                    let state_set: BTreeSet<&str> = states.iter().map(String::as_str).collect();
                    if !state_set.contains(initial.as_str()) {
                        defects.push(OrderedDefect {
                            document: block.document,
                            line: block.line,
                            defect: GraphDefect::UnknownState {
                                path: block.path.clone(),
                                line: block.line,
                                machine: id.clone(),
                                state: initial.clone(),
                            },
                        });
                    }
                    for transition in &transitions {
                        require_non_empty(&transition.name, "transition.name", block, &mut defects);
                        require_non_empty(&transition.event, "transition.event", block, &mut defects);
                        for state_name in [&transition.from, &transition.to] {
                            if !state_set.contains(state_name.as_str()) {
                                defects.push(OrderedDefect {
                                    document: block.document,
                                    line: block.line,
                                    defect: GraphDefect::UnknownState {
                                        path: block.path.clone(),
                                        line: block.line,
                                        machine: id.clone(),
                                        state: state_name.clone(),
                                    },
                                });
                            }
                        }
                    }
                    if register_id(&id, block, &known_reqs, &mut seen, &mut defects) {
                        state_machines.push(StateMachine {
                            id,
                            requirements: reqs,
                            initial,
                            states,
                            transitions,
                            path: block.path.clone(),
                            line: block.line,
                        });
                    }
                }
            }
            RawBlock::Property {
                id,
                name,
                property_kind,
                subject,
                predicate,
                scope,
                assumptions: raw_assumptions,
                requirements: raw_requirements,
                methods,
            } => {
                require_non_empty(&name, "name", block, &mut defects);
                require_non_empty(&scope, "scope", block, &mut defects);
                require_non_empty(&predicate.language, "predicate.language", block, &mut defects);
                require_non_empty(&predicate.expression, "predicate.expression", block, &mut defects);
                if let Some(id) = parse_artifact_id(&id, block, &mut defects) {
                    let owner: String = format!("property {id}");
                    let reqs: Vec<RequirementId> = parse_req_ids(&raw_requirements, &owner, block, &known_reqs, &mut defects);
                    raw_refs.subject_checks.push(SubjectCheck {
                        document: block.document,
                        path: block.path.clone(),
                        line: block.line,
                        property: id.clone(),
                        referenced: subject.clone(),
                    });
                    let mut parsed_assumptions: Vec<ArtifactId> = Vec::new();
                    for raw_assumption in raw_assumptions {
                        raw_refs.assumption_checks.push(RefCheck {
                            document: block.document,
                            path: block.path.clone(),
                            line: block.line,
                            owner: owner.clone(),
                            referenced: raw_assumption.clone(),
                        });
                        if let Some(parsed_id) = ArtifactId::parse(&raw_assumption) {
                            parsed_assumptions.push(parsed_id);
                        }
                    }
                    let registered: bool = register_id(&id, block, &known_reqs, &mut seen, &mut defects);
                    if let Some(subject_id) = ArtifactId::parse(&subject) {
                        if registered {
                            properties.push(Property {
                                id,
                                name,
                                property_kind,
                                subject: subject_id,
                                predicate,
                                scope,
                                assumptions: parsed_assumptions,
                                requirements: reqs,
                                methods,
                                path: block.path.clone(),
                                line: block.line,
                            });
                        }
                    }
                }
            }
            RawBlock::Claim {
                id,
                proposition,
                scope,
                assumptions: raw_assumptions,
                properties: raw_properties,
                requirements: raw_requirements,
            } => {
                require_non_empty(&proposition, "proposition", block, &mut defects);
                require_non_empty(&scope, "scope", block, &mut defects);
                if let Some(id) = parse_artifact_id(&id, block, &mut defects) {
                    let owner: String = format!("claim {id}");
                    let reqs: Vec<RequirementId> = parse_req_ids(&raw_requirements, &owner, block, &known_reqs, &mut defects);
                    let mut parsed_assumptions: Vec<ArtifactId> = Vec::new();
                    for raw_assumption in raw_assumptions {
                        raw_refs.assumption_checks.push(RefCheck {
                            document: block.document,
                            path: block.path.clone(),
                            line: block.line,
                            owner: owner.clone(),
                            referenced: raw_assumption.clone(),
                        });
                        if let Some(parsed_id) = ArtifactId::parse(&raw_assumption) {
                            parsed_assumptions.push(parsed_id);
                        }
                    }
                    let mut parsed_properties: Vec<ArtifactId> = Vec::new();
                    for raw_property in raw_properties {
                        raw_refs.property_checks.push(PropertyCheck {
                            document: block.document,
                            path: block.path.clone(),
                            line: block.line,
                            claim: id.clone(),
                            referenced: raw_property.clone(),
                        });
                        if let Some(parsed_id) = ArtifactId::parse(&raw_property) {
                            parsed_properties.push(parsed_id);
                        }
                    }
                    if register_id(&id, block, &known_reqs, &mut seen, &mut defects) {
                        claims.push(VerificationClaim {
                            id,
                            proposition,
                            scope,
                            assumptions: parsed_assumptions,
                            properties: parsed_properties,
                            requirements: reqs,
                            path: block.path.clone(),
                            line: block.line,
                        });
                    }
                }
            }
            RawBlock::Plan {
                id,
                targets,
                scope,
                assumptions: raw_assumptions,
                activities,
            } => {
                require_non_empty(&scope, "scope", block, &mut defects);
                if let Some(id) = parse_artifact_id(&id, block, &mut defects) {
                    let owner: String = format!("plan {id}");
                    let mut parsed_targets: Vec<ArtifactId> = Vec::new();
                    for raw_target in targets {
                        raw_refs.target_checks.push(TargetCheck {
                            document: block.document,
                            path: block.path.clone(),
                            line: block.line,
                            plan: id.clone(),
                            referenced: raw_target.clone(),
                        });
                        if let Some(parsed_id) = ArtifactId::parse(&raw_target) {
                            parsed_targets.push(parsed_id);
                        }
                    }
                    let mut parsed_assumptions: Vec<ArtifactId> = Vec::new();
                    for raw_assumption in raw_assumptions {
                        raw_refs.assumption_checks.push(RefCheck {
                            document: block.document,
                            path: block.path.clone(),
                            line: block.line,
                            owner: owner.clone(),
                            referenced: raw_assumption.clone(),
                        });
                        if let Some(parsed_id) = ArtifactId::parse(&raw_assumption) {
                            parsed_assumptions.push(parsed_id);
                        }
                    }
                    let registered: bool = register_id(&id, block, &known_reqs, &mut seen, &mut defects);
                    let mut parsed_activities: Vec<PlanActivity> = Vec::new();
                    for raw_activity in activities {
                        if let Some(activity_id) = parse_artifact_id(&raw_activity.id, block, &mut defects) {
                            if register_id(&activity_id, block, &known_reqs, &mut seen, &mut defects) {
                                parsed_activities.push(PlanActivity {
                                    id: activity_id,
                                    method: raw_activity.method,
                                    budget: raw_activity.budget,
                                });
                            }
                        }
                    }
                    if registered {
                        plans.push(VerificationPlan {
                            id,
                            targets: parsed_targets,
                            scope,
                            assumptions: parsed_assumptions,
                            activities: parsed_activities,
                            path: block.path.clone(),
                            line: block.line,
                        });
                    }
                }
            }
        }
    }

    validate_cross_references(&assumptions, &state_machines, &properties, &claims, &raw_refs, &mut defects);

    if !defects.is_empty() {
        defects.sort_by_key(|item: &OrderedDefect| (item.document, item.line));
        return Err(defects.into_iter().map(|item: OrderedDefect| item.defect).collect());
    }

    let source_revisions: Vec<SourceRevision> = sources
        .iter()
        .map(|source: &Source| SourceRevision {
            path: source.path.clone(),
            digest: ContentHash::of_bytes(source.text.as_bytes()),
        })
        .collect();
    let digests: BTreeMap<String, ContentHash> = match semantic_digests(&trace, &assumptions, &state_machines, &properties, &claims, &plans) {
        Ok(digests) => digests,
        Err(why) => {
            return Err(vec![GraphDefect::InvalidBlock {
                path: String::new(),
                line: 1,
                why,
            }]);
        }
    };
    let snapshot_id: ContentHash = match snapshot_digest(&source_revisions, &digests) {
        Ok(digest) => digest,
        Err(why) => {
            return Err(vec![GraphDefect::InvalidBlock {
                path: String::new(),
                line: 1,
                why,
            }]);
        }
    };

    Ok(GraphSnapshot {
        snapshot_id,
        sources: source_revisions,
        trace,
        assumptions,
        state_machines,
        properties,
        claims,
        plans,
        digests,
    })
}

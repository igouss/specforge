use std::fmt;

use crate::trace::{Requirement, RequirementId, Scenario, Story, Trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    Bound,
    Unbound,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub requirement: RequirementId,
    pub scenario: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Executable,
    Pending,
    Unbound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    NotStarted,
    Partial,
    Bound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stood {
    pub requirement: RequirementId,
    pub scenario: String,
    pub standing: Standing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub requirement: RequirementId,
    pub title: String,
    pub progress: Progress,
    pub scenarios: usize,
    pub executable: usize,
    pub pending: usize,
    pub monitored: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    StalePending {
        requirement: RequirementId,
        scenario: String,
    },
    PendingExecutable {
        requirement: RequirementId,
        scenario: String,
    },
    UnboundStep {
        requirement: RequirementId,
        scenario: String,
        step: String,
    },
    AmbiguousStep {
        requirement: RequirementId,
        scenario: String,
        step: String,
    },
    UnknownMonitored {
        id: String,
    },
}

impl fmt::Display for Finding {
    /// When a finding is displayed, `fmt` shall write the sentence SF-004 pins for its kind.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Finding::StalePending { requirement, scenario } => write!(formatter, "pending entry {requirement} \"{scenario}\" names no scenario"),
            Finding::PendingExecutable { requirement, scenario } => {
                write!(
                    formatter,
                    "pending entry {requirement} \"{scenario}\" names a scenario whose every step is bound"
                )
            }
            Finding::UnboundStep { requirement, scenario, step } => write!(formatter, "{requirement} \"{scenario}\" has the unbound step \"{step}\""),
            Finding::AmbiguousStep { requirement, scenario, step } => write!(formatter, "{requirement} \"{scenario}\" has the ambiguous step \"{step}\""),
            Finding::UnknownMonitored { id } => write!(formatter, "monitored requirement {id} is not in the trace"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Assessment {
    pub rows: Vec<Row>,
    pub scenarios: Vec<Stood>,
    pub findings: Vec<Finding>,
}

impl Assessment {
    /// While an assessment holds no finding, `fit` shall return true.
    pub fn fit(&self) -> bool {
        self.findings.is_empty()
    }
}

/// When a pending table is read, `pending` shall return one entry for each table row whose first cell is a requirement id and which has three cells: requirement, scenario, reason.
pub fn pending(markdown: &str) -> Vec<Pending> {
    markdown
        .lines()
        .filter_map(|line: &str| {
            let row: &str = line.trim().strip_prefix('|')?.strip_suffix('|')?;
            let cells: Vec<&str> = row.split('|').map(str::trim).collect();
            let [requirement, scenario, reason] = cells.as_slice() else {
                return None;
            };
            Some(Pending {
                requirement: RequirementId::parse(requirement)?,
                scenario: scenario.to_string(),
                reason: reason.to_string(),
            })
        })
        .collect()
}

fn shown(step: &gherkin::Step) -> String {
    format!("{} {}", step.keyword.trim(), step.value)
}

fn names(pending: &Pending, requirement: &Requirement, scenario: &Scenario) -> bool {
    pending.requirement == requirement.id && pending.scenario == scenario.name
}

struct Judged<'a> {
    scenario: &'a Scenario,
    standing: Standing,
    bindings: Vec<Binding>,
}

fn judged<'a>(requirement: &Requirement, scenario: &'a Scenario, binding: &dyn Fn(&gherkin::Step) -> Binding, pending: &[Pending]) -> Judged<'a> {
    let bindings: Vec<Binding> = scenario.steps.iter().map(binding).collect();
    let standing: Standing = if pending.iter().any(|entry: &Pending| names(entry, requirement, scenario)) {
        Standing::Pending
    } else if bindings.iter().all(|bound: &Binding| *bound == Binding::Bound) {
        Standing::Executable
    } else {
        Standing::Unbound
    };
    Judged { scenario, standing, bindings }
}

fn findings(requirement: &Requirement, judged: &[Judged<'_>], progress: Progress) -> Vec<Finding> {
    let mut found: Vec<Finding> = Vec::new();
    for judgement in judged {
        let scenario: String = judgement.scenario.name.clone();
        let every_bound: bool = judgement.bindings.iter().all(|bound: &Binding| *bound == Binding::Bound);
        if judgement.standing == Standing::Pending && every_bound {
            found.push(Finding::PendingExecutable {
                requirement: requirement.id.clone(),
                scenario: scenario.clone(),
            });
        }
        for (step, bound) in judgement.scenario.steps.iter().zip(&judgement.bindings) {
            let (requirement, scenario, step): (RequirementId, String, String) = (requirement.id.clone(), scenario.clone(), shown(step));
            match bound {
                Binding::Ambiguous => found.push(Finding::AmbiguousStep { requirement, scenario, step }),
                Binding::Unbound if progress == Progress::Partial && judgement.standing == Standing::Unbound => {
                    found.push(Finding::UnboundStep { requirement, scenario, step })
                }
                Binding::Unbound | Binding::Bound => {}
            }
        }
    }
    found
}

/// When a trace is assessed, `assess` shall return the rows, standings and findings SF-003 and SF-004 describe, in trace order.
pub fn assess(trace: &Trace, binding: &dyn Fn(&gherkin::Step) -> Binding, pending: &[Pending], monitored: &[String]) -> Assessment {
    let mut assessment: Assessment = Assessment::default();
    for requirement in &trace.requirements {
        let judged: Vec<Judged<'_>> = requirement
            .stories
            .iter()
            .flat_map(|story: &Story| &story.scenarios)
            .map(|scenario: &Scenario| judged(requirement, scenario, binding, pending))
            .collect();
        let count = |standing: Standing| judged.iter().filter(|judgement: &&Judged<'_>| judgement.standing == standing).count();
        let (executable, held): (usize, usize) = (count(Standing::Executable), count(Standing::Pending));
        let progress: Progress = match (executable, executable + held == judged.len()) {
            (0, _) => Progress::NotStarted,
            (_, true) => Progress::Bound,
            (_, false) => Progress::Partial,
        };
        assessment.findings.extend(findings(requirement, &judged, progress));
        assessment.scenarios.extend(judged.iter().map(|judgement: &Judged<'_>| Stood {
            requirement: requirement.id.clone(),
            scenario: judgement.scenario.name.clone(),
            standing: judgement.standing,
        }));
        assessment.rows.push(Row {
            requirement: requirement.id.clone(),
            title: requirement.title.clone(),
            progress,
            scenarios: judged.len(),
            executable,
            pending: held,
            monitored: monitored.iter().any(|id: &String| id == requirement.id.as_str()),
        });
    }
    let stale: Vec<Finding> = pending
        .iter()
        .filter(|entry: &&Pending| {
            !trace.requirements.iter().any(|requirement: &Requirement| {
                requirement
                    .stories
                    .iter()
                    .flat_map(|story: &Story| &story.scenarios)
                    .any(|scenario: &Scenario| names(entry, requirement, scenario))
            })
        })
        .map(|entry: &Pending| Finding::StalePending {
            requirement: entry.requirement.clone(),
            scenario: entry.scenario.clone(),
        })
        .collect();
    let unknown: Vec<Finding> = monitored
        .iter()
        .filter(|id: &&String| {
            !trace
                .requirements
                .iter()
                .any(|requirement: &Requirement| requirement.id.as_str() == id.as_str())
        })
        .map(|id: &String| Finding::UnknownMonitored { id: id.clone() })
        .collect();
    assessment.findings.extend(stale);
    assessment.findings.extend(unknown);
    assessment
}

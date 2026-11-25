use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use cucumber::codegen::WorldInventory;
use cucumber::writer::{Basic, FailOnSkipped, Normalize, Summarize};

use crate::assess::{self, Assessment, Binding, Finding, Progress, Standing, Stood};
use crate::graph::{self, ArtifactId, DependencyPin, GraphSnapshot, VerificationMethod};
use crate::ledger::{EvidenceKind, EvidenceLedger, EvidencePayload, EvidenceRecord, LedgerError, ProducerMeta};
use crate::trace::{self, Defect, Source, Trace};
use crate::verify::{EvaluationResult, RunState, VerificationOutcome};
use crate::view;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub specifications: PathBuf,
    pub pending: PathBuf,
    pub view: PathBuf,
    pub scratch: PathBuf,
    pub monitored: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Verify,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tally {
    pub scenarios: usize,
    pub executable: usize,
    pub ran: usize,
    pub passed: usize,
    pub pending: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Unread(String),
    Defective(Vec<Defect>),
    Unfit(Vec<Finding>),
    Stale(PathBuf),
    Drawn(PathBuf),
    Ran(Tally),
}

impl Outcome {
    /// While the outcome is a drawn view, or a run in which at least one scenario ran, every executable scenario ran and every scenario that ran passed, `passed` shall return true.
    pub fn passed(&self) -> bool {
        match self {
            Outcome::Drawn(_) => true,
            Outcome::Ran(tally) => tally.ran > 0 && tally.ran == tally.executable && tally.passed == tally.ran,
            Outcome::Unread(_) | Outcome::Defective(_) | Outcome::Unfit(_) | Outcome::Stale(_) => false,
        }
    }
}

fn lines<T: fmt::Display>(formatter: &mut fmt::Formatter<'_>, items: &[T]) -> fmt::Result {
    let shown: Vec<String> = items.iter().map(|item: &T| item.to_string()).collect();
    formatter.write_str(&shown.join("\n"))
}

impl fmt::Display for Outcome {
    /// When an outcome is displayed, `fmt` shall write the lines SF-006 pins for its kind.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Unread(why) => formatter.write_str(why),
            Outcome::Defective(defects) => lines(formatter, defects),
            Outcome::Unfit(findings) => lines(formatter, findings),
            Outcome::Stale(_) => formatter.write_str("the committed view is stale: draw it again"),
            Outcome::Drawn(path) => write!(formatter, "the view is drawn to {}", path.display()),
            Outcome::Ran(tally) => write!(
                formatter,
                "stories: {} of {} ran, {} passed, {} pending",
                tally.ran, tally.scenarios, tally.passed, tally.pending
            ),
        }
    }
}

/// When a step is looked up, the returned function shall answer whether exactly one, none or several of the world's step definitions match it.
pub fn binding<W: cucumber::World + fmt::Debug + WorldInventory>() -> impl Fn(&gherkin::Step) -> Binding {
    let collection: cucumber::step::Collection<W> = W::collection();
    move |step: &gherkin::Step| match collection.find(step) {
        Ok(Some(_)) => Binding::Bound,
        Ok(None) => Binding::Unbound,
        Err(_) => Binding::Ambiguous,
    }
}

fn unread(path: &Path, error: &std::io::Error) -> Outcome {
    Outcome::Unread(format!("{}: {error}", path.display()))
}

fn sources(specifications: &Path) -> Result<Vec<Source>, Outcome> {
    let entries: fs::ReadDir = fs::read_dir(specifications).map_err(|error: std::io::Error| unread(specifications, &error))?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let path: PathBuf = entry.map_err(|error: std::io::Error| unread(specifications, &error))?.path();
        if !path.is_dir() && path.extension().is_some_and(|extension: &std::ffi::OsStr| extension == "md") {
            paths.push(path);
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path: PathBuf| {
            let text: String = fs::read_to_string(&path).map_err(|error: std::io::Error| unread(&path, &error))?;
            let name: String = path
                .file_name()
                .map_or_else(String::new, |name: &std::ffi::OsStr| name.to_string_lossy().into_owned());
            Ok(Source { path: name, text })
        })
        .collect()
}

fn written(scratch: &Path, trace: &Trace, assessment: &Assessment) -> Result<(), Outcome> {
    fs::create_dir_all(scratch).map_err(|error: std::io::Error| unread(scratch, &error))?;
    let entries: fs::ReadDir = fs::read_dir(scratch).map_err(|error: std::io::Error| unread(scratch, &error))?;
    let mut features: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let path: PathBuf = entry.map_err(|error: std::io::Error| unread(scratch, &error))?.path();
        if !(path.is_file() && path.extension().is_some_and(|extension: &std::ffi::OsStr| extension == "feature")) {
            return Err(Outcome::Unread(format!(
                "{}: the scratch directory holds something other than feature files",
                path.display()
            )));
        }
        features.push(path);
    }
    for path in features {
        fs::remove_file(&path).map_err(|error: std::io::Error| unread(&path, &error))?;
    }
    for (requirement, row) in trace.requirements.iter().zip(&assessment.rows) {
        if row.progress == Progress::NotStarted {
            continue;
        }
        for (index, story) in requirement.stories.iter().enumerate() {
            let name: String = match requirement.stories.len() {
                1 => format!("{}.feature", requirement.id),
                _ => format!("{}-{}.feature", requirement.id, index + 1),
            };
            let path: PathBuf = scratch.join(name);
            fs::write(&path, &story.text).map_err(|error: std::io::Error| unread(&path, &error))?;
        }
    }
    Ok(())
}

async fn ran<W: cucumber::World + fmt::Debug + WorldInventory>(scratch: &Path, assessment: &Assessment) -> (usize, usize) {
    let executable: Vec<(String, String)> = assessment
        .scenarios
        .iter()
        .filter(|stood: &&Stood| stood.standing == Standing::Executable)
        .map(|stood: &Stood| (stood.requirement.to_string(), stood.scenario.clone()))
        .collect();
    if executable.is_empty() {
        return (0, 0);
    }
    let finished: FailOnSkipped<Summarize<Normalize<W, Basic>>> = W::cucumber()
        .with_default_cli()
        .fail_on_skipped()
        .filter_run(
            scratch.to_path_buf(),
            move |feature: &gherkin::Feature, _: Option<&gherkin::Rule>, scenario: &gherkin::Scenario| {
                executable
                    .iter()
                    .any(|(id, name): &(String, String)| feature.tags.contains(id) && *name == scenario.name)
            },
        )
        .await;
    (finished.scenarios_stats().total(), finished.scenarios_stats().passed)
}

async fn stories_from_sources<W: cucumber::World + fmt::Debug + WorldInventory>(project: &Project, documents: &[Source], mode: Mode) -> Outcome {
    let table: String = match fs::read_to_string(&project.pending) {
        Ok(table) => table,
        Err(error) => return unread(&project.pending, &error),
    };
    let trace: Trace = match trace::compile(documents) {
        Ok(trace) => trace,
        Err(defects) => return Outcome::Defective(defects),
    };
    let assessment: Assessment = assess::assess(&trace, &binding::<W>(), &assess::pending(&table), &project.monitored);
    if !assessment.fit() {
        return Outcome::Unfit(assessment.findings);
    }
    let drawn: String = view::drawn(&assessment);
    match mode {
        Mode::Draw => {
            let made: std::io::Result<()> = match project.view.parent() {
                Some(parent) => fs::create_dir_all(parent),
                None => Ok(()),
            };
            return match made.and_then(|()| fs::write(&project.view, drawn)) {
                Ok(()) => Outcome::Drawn(project.view.clone()),
                Err(error) => unread(&project.view, &error),
            };
        }
        Mode::Verify => {
            if fs::read_to_string(&project.view).ok().as_deref() != Some(drawn.as_str()) {
                return Outcome::Stale(project.view.clone());
            }
        }
    }
    if let Err(outcome) = written(&project.scratch, &trace, &assessment) {
        return outcome;
    }
    let (ran, passed): (usize, usize) = ran::<W>(&project.scratch, &assessment).await;
    let scenarios: usize = assessment
        .rows
        .iter()
        .filter(|row: &&assess::Row| row.progress != Progress::NotStarted)
        .map(|row: &assess::Row| row.scenarios)
        .sum();
    let pending: usize = assessment.rows.iter().map(|row: &assess::Row| row.pending).sum();
    let executable: usize = assessment.rows.iter().map(|row: &assess::Row| row.executable).sum();
    Outcome::Ran(Tally {
        scenarios,
        executable,
        ran,
        passed,
        pending,
    })
}

/// When a project's stories are verified or drawn, `stories` shall behave as SF-006 describes.
pub async fn stories<W: cucumber::World + fmt::Debug + WorldInventory>(project: &Project, mode: Mode) -> Outcome {
    let documents: Vec<Source> = match sources(&project.specifications) {
        Ok(documents) => documents,
        Err(outcome) => return outcome,
    };
    stories_from_sources::<W>(project, &documents, mode).await
}

/// When a project story run is recorded as evidence, `record_stories_evidence` shall seal an immutable evidence record bound to the graph snapshot and append it to the ledger.
pub fn record_stories_evidence(
    evidence_id: String,
    run_id: String,
    timestamp: String,
    snapshot: &GraphSnapshot,
    outcome: &Outcome,
    claim_ids: Vec<ArtifactId>,
    ledger: &mut EvidenceLedger,
) -> Result<EvidenceRecord, LedgerError> {
    for claim_id in &claim_ids {
        if snapshot.claim(claim_id.as_str()).is_none() {
            return Err(LedgerError::InvalidRecord {
                id: evidence_id,
                why: format!("unknown claim {claim_id} in snapshot"),
            });
        }
    }
    let (run_state, result, limitation): (RunState, EvaluationResult, String) = match outcome {
        Outcome::Ran(tally) if outcome.passed() => (
            RunState::Completed,
            EvaluationResult::Satisfied,
            format!(
                "finite example-based cucumber run ({} of {} executable scenarios passed, {} pending); does not prove universal correctness",
                tally.passed, tally.executable, tally.pending
            ),
        ),
        Outcome::Ran(tally) if tally.passed < tally.ran => (
            RunState::Completed,
            EvaluationResult::Violated,
            format!("cucumber run failed ({} of {} ran passed, {} pending)", tally.passed, tally.ran, tally.pending),
        ),
        Outcome::Ran(tally) => (
            RunState::Inconclusive,
            EvaluationResult::Inconclusive,
            format!(
                "incomplete cucumber run ({} of {} executable scenarios ran, {} pending)",
                tally.ran, tally.executable, tally.pending
            ),
        ),
        Outcome::Drawn(_) => (
            RunState::Completed,
            EvaluationResult::NotEvaluated,
            "view drawn without executing scenarios".to_string(),
        ),
        Outcome::Unread(_) | Outcome::Defective(_) | Outcome::Unfit(_) | Outcome::Stale(_) => (
            RunState::Failed,
            EvaluationResult::InvalidEvaluation,
            format!("pre-execution gate failed: {outcome}"),
        ),
    };
    let dependencies: Vec<DependencyPin> = snapshot
        .digests
        .iter()
        .map(|(artifact_id, digest): (&String, &graph::ContentHash)| DependencyPin {
            artifact_id: artifact_id.clone(),
            digest: digest.clone(),
        })
        .collect();
    let payload: EvidencePayload = EvidencePayload {
        id: evidence_id,
        kind: EvidenceKind::StoryRun,
        producer: ProducerMeta {
            name: "cucumber".to_string(),
            version: "0.23".to_string(),
        },
        adapter: ProducerMeta {
            name: "specforge-cucumber-adapter".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        timestamp,
        snapshot_id: snapshot.snapshot_id.clone(),
        source_revisions: snapshot.sources.clone(),
        dependencies,
        run_id,
        activity_id: "cucumber-stories".to_string(),
        method: VerificationMethod::ExampleBased,
        property_id: None,
        claim_ids,
        scope: "project-stories".to_string(),
        assumptions: Vec::new(),
        run_state,
        outcome: VerificationOutcome::Evaluation(result),
        counterexample_id: None,
        replay: None,
        artifact_digests: BTreeMap::new(),
        limitations: vec![limitation],
    };
    let record: EvidenceRecord = EvidenceRecord::seal(payload)?;
    ledger.append_record(record.clone())?;
    Ok(record)
}

/// When a project's stories are verified through the Cucumber adapter, `stories_with_evidence` shall run the project, compile its graph snapshot from the same sources, and append an immutable evidence record to the ledger.
pub async fn stories_with_evidence<W: cucumber::World + fmt::Debug + WorldInventory>(
    project: &Project,
    evidence_id: String,
    run_id: String,
    timestamp: String,
    claim_ids: Vec<ArtifactId>,
    ledger: &mut EvidenceLedger,
) -> Result<(Outcome, EvidenceRecord), String> {
    let documents: Vec<Source> = sources(&project.specifications).map_err(|outcome: Outcome| outcome.to_string())?;
    let snapshot: GraphSnapshot = graph::compile_graph(&documents).map_err(|defects: Vec<graph::GraphDefect>| {
        defects
            .into_iter()
            .map(|defect: graph::GraphDefect| defect.to_string())
            .collect::<Vec<String>>()
            .join("\n")
    })?;
    for claim_id in &claim_ids {
        if snapshot.claim(claim_id.as_str()).is_none() {
            return Err(format!("unknown claim {claim_id} in snapshot"));
        }
    }
    let outcome: Outcome = stories_from_sources::<W>(project, &documents, Mode::Verify).await;
    let record: EvidenceRecord =
        record_stories_evidence(evidence_id, run_id, timestamp, &snapshot, &outcome, claim_ids, ledger).map_err(|error: LedgerError| error.to_string())?;
    Ok((outcome, record))
}

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use cucumber::codegen::WorldInventory;
use cucumber::writer::{Basic, FailOnSkipped, Normalize, Summarize};

use crate::assess::{self, Assessment, Binding, Finding, Progress, Standing, Stood};
use crate::trace::{self, Defect, Source, Trace};
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
    /// While the outcome is a drawn view, or a run in which at least one scenario ran and every scenario that ran passed, `passed` shall return true.
    pub fn passed(&self) -> bool {
        match self {
            Outcome::Drawn(_) => true,
            Outcome::Ran(tally) => tally.ran > 0 && tally.passed == tally.ran,
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
        if path.is_file() && path.extension().is_some_and(|extension: &std::ffi::OsStr| extension == "md") {
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
    for entry in entries {
        let path: PathBuf = entry.map_err(|error: std::io::Error| unread(scratch, &error))?.path();
        if path.extension().is_some_and(|extension: &std::ffi::OsStr| extension == "feature") {
            fs::remove_file(&path).map_err(|error: std::io::Error| unread(&path, &error))?;
        }
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

/// When a project's stories are verified or drawn, `stories` shall behave as SF-006 describes.
pub async fn stories<W: cucumber::World + fmt::Debug + WorldInventory>(project: &Project, mode: Mode) -> Outcome {
    let documents: Vec<Source> = match sources(&project.specifications) {
        Ok(documents) => documents,
        Err(outcome) => return outcome,
    };
    let table: String = match fs::read_to_string(&project.pending) {
        Ok(table) => table,
        Err(error) => return unread(&project.pending, &error),
    };
    let trace: Trace = match trace::compile(&documents) {
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
    Outcome::Ran(Tally {
        scenarios,
        ran,
        passed,
        pending,
    })
}

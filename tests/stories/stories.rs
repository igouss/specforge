use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use cucumber::{given, then, when};
use specforge::stories::{stories, Mode, Outcome, Project};

use crate::ledger::{Ledger, RAN};
use crate::{fixture, fixture_text, listed, root, unescaped, Forge};

static PROJECTS: AtomicUsize = AtomicUsize::new(0);

fn copied(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("the directory is made");
    for entry in fs::read_dir(from).expect("the project is read") {
        let path: PathBuf = entry.expect("an entry").path();
        let into: PathBuf = to.join(path.file_name().expect("a file name"));
        match path.is_dir() {
            true => copied(&path, &into),
            false => drop(fs::copy(&path, &into).expect("the file is copied")),
        }
    }
}

fn project(world: &Forge) -> &Project {
    world.project.as_ref().expect("a project is given")
}

fn outcome(world: &Forge) -> &Outcome {
    world.outcome.as_ref().expect("the project ran")
}

async fn run(world: &mut Forge, mode: Mode) {
    let project: Project = project(world).clone();
    let (outcome, ran): (Outcome, usize) = RAN
        .scope(Cell::new(0), async {
            let outcome: Outcome = stories::<Ledger>(&project, mode).await;
            (outcome, RAN.with(Cell::get))
        })
        .await;
    world.outcome = Some(outcome);
    world.ran = ran;
}

#[given(expr = "the project {string}")]
fn given_project(world: &mut Forge, name: String) {
    let copy: PathBuf = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("projects")
        .join(PROJECTS.fetch_add(1, Ordering::SeqCst).to_string());
    if copy.exists() {
        fs::remove_dir_all(&copy).expect("the stale copy is removed");
    }
    copied(&root().join("tests/projects").join(name), &copy);
    world.project = Some(Project {
        specifications: copy.join("specs"),
        pending: copy.join("pending.md"),
        view: copy.join("view.md"),
        scratch: copy.join("scratch"),
        monitored: Vec::new(),
    });
}

#[given(expr = "its specification {string} is replaced by the document {string}")]
fn replaced(world: &mut Forge, specification: String, document: String) {
    let specifications: &Path = &project(world).specifications;
    fs::remove_file(specifications.join(specification)).expect("the specification is removed");
    fs::copy(fixture(&document), specifications.join(&document)).expect("the document is copied");
}

#[given("its pending table is emptied")]
fn emptied(world: &mut Forge) {
    fs::write(&project(world).pending, "").expect("the pending table is emptied");
}

#[given("its view is drawn")]
#[when("its view is drawn")]
async fn drawn(world: &mut Forge) {
    run(world, Mode::Draw).await;
}

#[given(expr = "its committed view is replaced by the file {string}")]
fn view_replaced(world: &mut Forge, name: String) {
    fs::write(&project(world).view, fixture_text(&name)).expect("the view is replaced");
}

#[given("its committed view is deleted")]
fn view_deleted(world: &mut Forge) {
    fs::remove_file(&project(world).view).expect("the view is deleted");
}

#[given(expr = "its specification directory holds a link {string} to a document that does not exist")]
fn dangling(world: &mut Forge, name: String) {
    let specifications: &Path = &project(world).specifications;
    std::os::unix::fs::symlink(specifications.join("absent-target.md"), specifications.join(name)).expect("the link is made");
}

#[given(expr = "its specification directory holds a directory {string}")]
fn spec_directory_holds(world: &mut Forge, name: String) {
    fs::create_dir_all(project(world).specifications.join(name)).expect("the directory is made");
}

#[given(expr = "its scratch directory holds the file {string}")]
fn scratch_holds(world: &mut Forge, name: String) {
    let scratch: &Path = &project(world).scratch;
    fs::create_dir_all(scratch).expect("the scratch directory is made");
    fs::write(scratch.join(name), "").expect("the file is written");
}

#[given(expr = "it monitors the requirements {string}")]
fn monitors(world: &mut Forge, ids: String) {
    let monitored: Vec<String> = listed(&ids, ", ");
    world.project.as_mut().expect("a project is given").monitored = monitored;
}

#[when("its stories are verified")]
async fn verified(world: &mut Forge) {
    run(world, Mode::Verify).await;
}

#[then(expr = "the outcome reads {string}")]
fn reads(world: &mut Forge, expected: String) {
    assert_eq!(outcome(world).to_string(), unescaped(&expected));
}

#[then(expr = "the outcome names {string}")]
fn names(world: &mut Forge, expected: String) {
    let shown: String = outcome(world).to_string();
    assert!(shown.contains(&unescaped(&expected)), "{shown}");
}

#[then("the verification passes")]
fn passes(world: &mut Forge) {
    assert!(outcome(world).passed(), "{}", outcome(world));
}

#[then("the verification fails")]
fn fails(world: &mut Forge) {
    assert!(!outcome(world).passed(), "{}", outcome(world));
}

#[then("no feature file lies in its specification directory")]
fn no_feature_file(world: &mut Forge) {
    let features: Vec<PathBuf> = fs::read_dir(&project(world).specifications)
        .expect("the specifications are read")
        .map(|entry: std::io::Result<fs::DirEntry>| entry.expect("an entry").path())
        .filter(|path: &PathBuf| path.extension().is_some_and(|extension: &std::ffi::OsStr| extension == "feature"))
        .collect();
    assert_eq!(features, Vec::<PathBuf>::new());
}

#[then("no scenario ran")]
fn none_ran(world: &mut Forge) {
    assert_eq!(world.ran, 0);
}

#[then(expr = "its committed view is the file {string}")]
fn view_is(world: &mut Forge, name: String) {
    assert_eq!(fs::read_to_string(&project(world).view).expect("the view is read"), fixture_text(&name));
}

#[then("its committed view does not exist")]
fn view_absent(world: &mut Forge) {
    assert!(!project(world).view.exists());
}

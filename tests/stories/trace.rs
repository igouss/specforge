use cucumber::{given, then, when};
use specforge::trace::{compile, Defect, Requirement, Scenario, Source, Trace};

use crate::{fixture_text, listed, unescaped, Forge};

fn trace(world: &Forge) -> &Trace {
    match &world.compiled {
        Some(Ok(trace)) => trace,
        Some(Err(defects)) => panic!("the documents are defective: {defects:?}"),
        None => panic!("the documents were not compiled"),
    }
}

fn defects(world: &Forge) -> Vec<String> {
    match &world.compiled {
        Some(Err(defects)) => defects.iter().map(Defect::to_string).collect(),
        Some(Ok(_)) => Vec::new(),
        None => panic!("the documents were not compiled"),
    }
}

fn requirement<'a>(world: &'a Forge, id: &str) -> &'a Requirement {
    trace(world)
        .requirements
        .iter()
        .find(|requirement: &&Requirement| requirement.id.as_str() == id)
        .expect("the requirement is in the trace")
}

fn scenario<'a>(world: &'a Forge, name: &str, id: &str) -> &'a Scenario {
    requirement(world, id)
        .stories
        .iter()
        .flat_map(|story: &specforge::trace::Story| &story.scenarios)
        .find(|scenario: &&Scenario| scenario.name == name)
        .expect("the scenario is in the requirement")
}

#[given(expr = "the document {string}")]
fn document(world: &mut Forge, name: String) {
    world.documents.push(name);
}

#[when("the documents are compiled")]
fn compiled(world: &mut Forge) {
    let sources: Vec<Source> = world
        .documents
        .iter()
        .map(|name: &String| Source {
            path: name.clone(),
            text: fixture_text(name),
        })
        .collect();
    world.compiled = Some(compile(&sources));
}

#[then("the trace holds no requirements")]
fn no_requirements(world: &mut Forge) {
    assert_eq!(trace(world).requirements, Vec::new());
}

#[then("there are no defects")]
fn no_defects(world: &mut Forge) {
    assert_eq!(defects(world), Vec::<String>::new());
}

#[then(expr = "the trace holds the requirements {string}")]
fn holds(world: &mut Forge, ids: String) {
    let held: Vec<String> = trace(world)
        .requirements
        .iter()
        .map(|requirement: &Requirement| requirement.id.to_string())
        .collect();
    assert_eq!(held, listed(&ids, ", "));
}

#[then(expr = "requirement {string} has the version {string}")]
fn has_version(world: &mut Forge, id: String, version: String) {
    assert_eq!(requirement(world, &id).version, version);
}

#[then(expr = "requirement {string} has the title {string}")]
fn has_title(world: &mut Forge, id: String, title: String) {
    assert_eq!(requirement(world, &id).title, title);
}

#[then(expr = "requirement {string} has the statement {string}")]
fn has_statement(world: &mut Forge, id: String, statement: String) {
    assert_eq!(requirement(world, &id).statement, unescaped(&statement));
}

#[then(expr = "requirement {string} is declared at {string} line {int}")]
fn declared(world: &mut Forge, id: String, path: String, line: usize) {
    let found: &Requirement = requirement(world, &id);
    assert_eq!((found.path.as_str(), found.line), (path.as_str(), line));
}

#[then(expr = "requirement {string} has no scenarios")]
fn no_scenarios(world: &mut Forge, id: String) {
    assert_eq!(requirement(world, &id).stories, Vec::new());
}

#[then(expr = "requirement {string} has the scenarios {string}")]
fn has_scenarios(world: &mut Forge, id: String, names: String) {
    let held: Vec<String> = requirement(world, &id)
        .stories
        .iter()
        .flat_map(|story: &specforge::trace::Story| &story.scenarios)
        .map(|scenario: &Scenario| scenario.name.clone())
        .collect();
    assert_eq!(held, listed(&names, "; "));
}

#[then(expr = "the scenario {string} of {string} has the steps {string}")]
fn has_steps(world: &mut Forge, name: String, id: String, steps: String) {
    let held: Vec<String> = scenario(world, &name, &id)
        .steps
        .iter()
        .map(|step: &specforge::gherkin::Step| format!("{} {}", step.keyword.trim(), step.value))
        .collect();
    assert_eq!(held, listed(&steps, "; "));
}

#[then(expr = "the scenario {string} of {string} has the tags {string}")]
fn has_tags(world: &mut Forge, name: String, id: String, tags: String) {
    assert_eq!(scenario(world, &name, &id).tags, listed(&tags, ", "));
}

#[then(expr = "the story of {string} opens at line {int}")]
fn opens(world: &mut Forge, id: String, line: usize) {
    let lines: Vec<usize> = requirement(world, &id)
        .stories
        .iter()
        .map(|story: &specforge::trace::Story| story.line)
        .collect();
    assert_eq!(lines, vec![line]);
}

#[then("there is no trace")]
fn no_trace(world: &mut Forge) {
    assert!(matches!(world.compiled, Some(Err(_))), "a trace was compiled");
}

#[then(expr = "the defects are {string}")]
fn defects_are(world: &mut Forge, expected: String) {
    assert_eq!(defects(world), listed(&expected, "; "));
}

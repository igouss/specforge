use cucumber::{given, then, when};
use specforge::assess::{assess, pending, Assessment, Binding, Finding, Pending, Progress, Row, Standing, Stood};
use specforge::trace::{compile, RequirementId, Source, Trace};
use specforge::view::drawn;

use crate::{fixture_text, listed, unescaped, Forge};

fn assessment(world: &Forge) -> &Assessment {
    world.assessment.as_ref().expect("the trace was assessed")
}

fn row<'a>(world: &'a Forge, id: &str) -> &'a Row {
    assessment(world)
        .rows
        .iter()
        .find(|row: &&Row| row.requirement.as_str() == id)
        .expect("the requirement has a row")
}

fn progress(progress: Progress) -> &'static str {
    match progress {
        Progress::NotStarted => "not started",
        Progress::Partial => "partial",
        Progress::Bound => "bound",
    }
}

fn standing(standing: Standing) -> &'static str {
    match standing {
        Standing::Executable => "executable",
        Standing::Pending => "pending",
        Standing::Unbound => "unbound",
    }
}

fn findings(world: &Forge) -> Vec<String> {
    assessment(world).findings.iter().map(Finding::to_string).collect()
}

#[given("no step is bound")]
fn nothing_bound(world: &mut Forge) {
    world.bound.clear();
    world.twice.clear();
}

#[given(expr = "the steps {string} are bound")]
fn steps_bound(world: &mut Forge, steps: String) {
    world.bound.extend(listed(&steps, "; "));
}

#[given(expr = "the step {string} is bound twice")]
fn bound_twice(world: &mut Forge, text: String) {
    world.twice.push(unescaped(&text));
}

#[given(expr = "the pending table names {string} {string}")]
fn pending_names(world: &mut Forge, id: String, scenario: String) {
    let requirement: RequirementId = RequirementId::parse(&id).expect("a requirement id");
    world.pending.push(Pending {
        requirement,
        scenario: unescaped(&scenario),
        reason: String::new(),
    });
}

#[given(expr = "the pending table is the file {string}")]
fn pending_file(world: &mut Forge, name: String) {
    world.pending.extend(pending(&fixture_text(&name)));
}

#[given(expr = "the requirements {string} are monitored")]
fn are_monitored(world: &mut Forge, ids: String) {
    world.monitored.extend(listed(&ids, ", "));
}

#[when("the trace is assessed")]
fn assessed(world: &mut Forge) {
    let sources: Vec<Source> = world
        .documents
        .iter()
        .map(|name: &String| Source {
            path: name.clone(),
            text: fixture_text(name),
        })
        .collect();
    let trace: Trace = compile(&sources).expect("the documents compile");
    let binding = |step: &specforge::gherkin::Step| match (world.twice.contains(&step.value), world.bound.contains(&step.value)) {
        (true, _) => Binding::Ambiguous,
        (false, true) => Binding::Bound,
        (false, false) => Binding::Unbound,
    };
    let assessment: Assessment = assess(&trace, &binding, &world.pending, &world.monitored);
    world.assessment = Some(assessment);
}

#[then(expr = "requirement {string} is {string}")]
fn is(world: &mut Forge, id: String, expected: String) {
    assert_eq!(progress(row(world, &id).progress), expected);
}

#[then(expr = "requirement {string} is {string} with {int} of {int} scenarios executable and {int} pending")]
fn is_with(world: &mut Forge, id: String, expected: String, executable: usize, scenarios: usize, pending: usize) {
    let found: &Row = row(world, &id);
    assert_eq!(
        (progress(found.progress), found.executable, found.scenarios, found.pending),
        (expected.as_str(), executable, scenarios, pending)
    );
}

#[then(expr = "the scenario {string} of {string} stands {string}")]
fn stands(world: &mut Forge, name: String, id: String, expected: String) {
    let stood: &Stood = assessment(world)
        .scenarios
        .iter()
        .find(|stood: &&Stood| stood.requirement.as_str() == id && stood.scenario == name)
        .expect("the scenario stands");
    assert_eq!(standing(stood.standing), expected);
}

#[then("there are no findings")]
fn no_findings(world: &mut Forge) {
    assert_eq!(findings(world), Vec::<String>::new());
}

#[then(expr = "the findings are {string}")]
fn findings_are(world: &mut Forge, expected: String) {
    assert_eq!(findings(world), listed(&expected, "; "));
}

#[then("the assessment is fit")]
fn fit(world: &mut Forge) {
    assert!(assessment(world).fit());
}

#[then("the assessment is unfit")]
fn unfit(world: &mut Forge) {
    assert!(!assessment(world).fit());
}

#[then(expr = "requirement {string} is monitored")]
fn is_monitored(world: &mut Forge, id: String) {
    assert!(row(world, &id).monitored);
}

#[then(expr = "requirement {string} is not monitored")]
fn is_not_monitored(world: &mut Forge, id: String) {
    assert!(!row(world, &id).monitored);
}

#[then(expr = "the view is the file {string}")]
fn view_is_file(world: &mut Forge, name: String) {
    assert_eq!(drawn(assessment(world)), fixture_text(&name));
}

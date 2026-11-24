use cucumber::{given, then, when};
use specforge::graph::{compile_graph, ContentHash, GraphDefect, GraphSnapshot};
use specforge::trace::Source;

use crate::{fixture_text, unescaped, Forge};

pub fn compile_fixture(name: &str) -> Result<GraphSnapshot, Vec<GraphDefect>> {
    let source: Source = Source {
        path: name.to_string(),
        text: fixture_text(name),
    };
    compile_graph(&[source])
}

pub fn current_snapshot(world: &Forge) -> &GraphSnapshot {
    match &world.graph_compiled {
        Some(Ok(snapshot)) => snapshot,
        Some(Err(defects)) => panic!("graph compilation failed: {defects:?}"),
        None => panic!("no graph snapshot compiled"),
    }
}

#[given(expr = "the graph document {string}")]
fn graph_document(world: &mut Forge, name: String) {
    world.graph_documents.push(name);
}

#[given(expr = "a compiled snapshot from {string}")]
fn compiled_snapshot_from(world: &mut Forge, name: String) {
    let snapshot: GraphSnapshot = compile_fixture(&name).expect("fixture compiles");
    world.baseline_snapshot = Some(snapshot.clone());
    world.graph_compiled = Some(Ok(snapshot));
}

#[when("the graph snapshot is compiled")]
fn compile_snapshot(world: &mut Forge) {
    let sources: Vec<Source> = world
        .graph_documents
        .iter()
        .map(|name: &String| Source {
            path: name.clone(),
            text: fixture_text(name),
        })
        .collect();
    world.graph_compiled = Some(compile_graph(&sources));
}

#[when("the graph snapshot is compiled as baseline")]
fn compile_baseline(world: &mut Forge) {
    compile_snapshot(world);
    world.baseline_snapshot = Some(current_snapshot(world).clone());
}

#[when(expr = "the graph document {string} is compiled as revised")]
fn compile_revised(world: &mut Forge, name: String) {
    let snapshot: GraphSnapshot = compile_fixture(&name).expect("revised fixture compiles");
    world.revised_snapshot = Some(snapshot);
}

#[then("the snapshot compiles with no defects")]
fn snapshot_no_defects(world: &mut Forge) {
    assert!(matches!(world.graph_compiled, Some(Ok(_))), "{:?}", world.graph_compiled);
}

#[then(expr = "the snapshot holds assumption {string}, state machine {string}, property {string}, claim {string}, and plan {string}")]
fn snapshot_holds_all(world: &mut Forge, assumption: String, machine: String, property: String, claim: String, plan: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    assert!(snapshot.assumption(&assumption).is_some(), "missing assumption {assumption}");
    assert!(snapshot.state_machine(&machine).is_some(), "missing state machine {machine}");
    assert!(snapshot.property(&property).is_some(), "missing property {property}");
    assert!(snapshot.claim(&claim).is_some(), "missing claim {claim}");
    assert!(snapshot.plan(&plan).is_some(), "missing plan {plan}");
}

#[then("the snapshot digest is deterministic across repeated compilations")]
fn snapshot_digest_deterministic(world: &mut Forge) {
    let first: ContentHash = current_snapshot(world).snapshot_id.clone();
    let sources: Vec<Source> = world
        .graph_documents
        .iter()
        .map(|name: &String| Source {
            path: name.clone(),
            text: fixture_text(name),
        })
        .collect();
    let second: GraphSnapshot = compile_graph(&sources).expect("second compilation succeeds");
    assert_eq!(first, second.snapshot_id);
}

#[then(expr = "the snapshot holds {int} verification artifacts")]
fn snapshot_artifact_count(world: &mut Forge, count: usize) {
    assert_eq!(current_snapshot(world).artifact_count(), count);
}

#[then("the snapshot digest changes")]
fn snapshot_digest_changes(world: &mut Forge) {
    let baseline: &GraphSnapshot = world.baseline_snapshot.as_ref().expect("baseline compiled");
    let revised: &GraphSnapshot = world.revised_snapshot.as_ref().expect("revised compiled");
    assert_ne!(baseline.snapshot_id, revised.snapshot_id);
}

#[then(expr = "the digest of property {string} is unchanged")]
fn property_digest_unchanged(world: &mut Forge, id: String) {
    let baseline: &GraphSnapshot = world.baseline_snapshot.as_ref().expect("baseline compiled");
    let revised: &GraphSnapshot = world.revised_snapshot.as_ref().expect("revised compiled");
    assert_eq!(baseline.artifact_digest(&id), revised.artifact_digest(&id));
}

#[then(expr = "the digest of state machine {string} is unchanged")]
fn machine_digest_unchanged(world: &mut Forge, id: String) {
    let baseline: &GraphSnapshot = world.baseline_snapshot.as_ref().expect("baseline compiled");
    let revised: &GraphSnapshot = world.revised_snapshot.as_ref().expect("revised compiled");
    assert_eq!(baseline.artifact_digest(&id), revised.artifact_digest(&id));
}

#[then("there is no snapshot")]
fn no_snapshot(world: &mut Forge) {
    assert!(matches!(world.graph_compiled, Some(Err(_))), "expected defects");
}

#[then(expr = "the graph defects include {string}")]
fn graph_defects_include(world: &mut Forge, expected: String) {
    let wanted: String = unescaped(&expected);
    let found: Vec<String> = match &world.graph_compiled {
        Some(Err(defects)) => defects.iter().map(GraphDefect::to_string).collect(),
        other => panic!("expected graph defects, got {other:?}"),
    };
    assert!(found.contains(&wanted), "wanted {wanted:?} in {found:?}");
}

#[then(expr = "a graph defect mentions {string}")]
fn graph_defect_mentions(world: &mut Forge, needle: String) {
    let found: Vec<String> = match &world.graph_compiled {
        Some(Err(defects)) => defects.iter().map(GraphDefect::to_string).collect(),
        other => panic!("expected graph defects, got {other:?}"),
    };
    assert!(
        found.iter().any(|defect: &String| defect.contains(&needle)),
        "wanted defect mentioning {needle:?} in {found:?}"
    );
}

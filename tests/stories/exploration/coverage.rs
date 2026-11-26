use std::collections::BTreeMap;

use cucumber::{then, when};
use specforge::coverage::{compute_coverage, validate_imported_scenario, CoverageError, CoverageRecord, ImportedScenario, MbtAdapter};
use specforge::graph::{ArtifactId, ContentHash, GraphSnapshot, StateMachine};
use specforge::model::{execute_machine, GeneratedScenario};
use specforge::verify::{ExecutionTrace, TraceEvent};

use crate::graph::current_snapshot;
use crate::{listed, Forge};

struct StubMbtAdapter;

impl MbtAdapter for StubMbtAdapter {
    fn import_scenarios(&self, snapshot: &GraphSnapshot, machine_id: &str) -> Result<Vec<ImportedScenario>, CoverageError> {
        let machine: &StateMachine = snapshot
            .state_machine(machine_id)
            .ok_or_else(|| CoverageError::UnknownMachine(machine_id.to_string()))?;
        let digest: ContentHash = snapshot
            .artifact_digest(machine_id)
            .cloned()
            .ok_or_else(|| CoverageError::MissingDigest(machine_id.to_string()))?;
        Ok(vec![
            ImportedScenario {
                id: format!("IMP-{machine_id}-001"),
                model_id: machine.id.clone(),
                model_digest: digest.clone(),
                strategy: "external_mbt_bfs".to_string(),
                coverage_objective: "deliver_state".to_string(),
                seed: Some(7),
                events: vec!["deliver".to_string()],
                importer: "stub-mbt".to_string(),
                importer_version: "1.0.0".to_string(),
            },
            ImportedScenario {
                id: format!("IMP-{machine_id}-002"),
                model_id: machine.id.clone(),
                model_digest: digest,
                strategy: "external_mbt_bfs".to_string(),
                coverage_objective: "acknowledge_path".to_string(),
                seed: Some(7),
                events: vec!["deliver".to_string(), "ack".to_string()],
                importer: "stub-mbt".to_string(),
                importer_version: "1.0.0".to_string(),
            },
        ])
    }
}

#[when(expr = "an MBT adapter imports valid scenarios for state machine {string}")]
fn import_valid_mbt(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let adapter: StubMbtAdapter = StubMbtAdapter;
    let imported: Vec<ImportedScenario> = adapter.import_scenarios(snapshot, &machine_id).expect("adapter imports scenarios");
    let mut validated: Vec<GeneratedScenario> = Vec::new();
    for scenario in &imported {
        let generated: GeneratedScenario = validate_imported_scenario(snapshot, scenario).expect("imported scenario validates");
        validated.push(generated);
    }
    world.exploration.validated_imports = validated;
}

#[then(expr = "{int} imported scenarios are validated for {string}")]
fn imported_count(world: &mut Forge, count: usize, machine_id: String) {
    assert_eq!(world.exploration.validated_imports.len(), count);
    for scenario in &world.exploration.validated_imports {
        assert_eq!(scenario.model_id.as_str(), machine_id);
    }
}

#[then("every validated imported scenario records its strategy and source model digest")]
fn imported_provenance(world: &mut Forge) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    for scenario in &world.exploration.validated_imports {
        assert!(!scenario.strategy.is_empty());
        assert_eq!(Some(&scenario.model_digest), snapshot.artifact_digest(scenario.model_id.as_str()));
    }
}

#[then(expr = "the validated imported scenarios cover states {string}")]
fn imported_cover_states(world: &mut Forge, states: String) {
    let covered: Vec<String> = world
        .exploration
        .validated_imports
        .iter()
        .flat_map(|scenario: &GeneratedScenario| scenario.covered_states.clone())
        .collect();
    for expected in listed(&states, ", ") {
        assert!(covered.contains(&expected), "missing state {expected} in {covered:?}");
    }
}

#[then(expr = "the validated imported scenarios cover transitions {string}")]
fn imported_cover_transitions(world: &mut Forge, transitions: String) {
    let covered: Vec<String> = world
        .exploration
        .validated_imports
        .iter()
        .flat_map(|scenario: &GeneratedScenario| scenario.covered_transitions.clone())
        .collect();
    for expected in listed(&transitions, ", ") {
        assert!(covered.contains(&expected), "missing transition {expected} in {covered:?}");
    }
}

#[when(expr = "an imported scenario with a stale model digest is validated for {string}")]
fn validate_stale_import(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let stale_digest: ContentHash = ContentHash::of_bytes(b"stale-model-revision");
    let scenario: ImportedScenario = ImportedScenario {
        id: "IMP-STALE-001".to_string(),
        model_id: ArtifactId::parse(&machine_id).expect("valid machine id"),
        model_digest: stale_digest,
        strategy: "external_mbt_bfs".to_string(),
        coverage_objective: "deliver_state".to_string(),
        seed: None,
        events: vec!["deliver".to_string()],
        importer: "stub-mbt".to_string(),
        importer_version: "1.0.0".to_string(),
    };
    world.exploration.import_result = Some(validate_imported_scenario(snapshot, &scenario));
}

#[then("the scenario import fails with a digest mismatch error")]
fn import_fails_digest(world: &mut Forge) {
    assert!(
        matches!(world.exploration.import_result, Some(Err(CoverageError::DigestMismatch { .. }))),
        "{:?}",
        world.exploration.import_result
    );
}

#[then(expr = "the import error message mentions {string}")]
fn import_error_mentions(world: &mut Forge, needle: String) {
    let message: String = match &world.exploration.import_result {
        Some(Err(error)) => error.to_string(),
        other => panic!("expected import error, got {other:?}"),
    };
    assert!(message.contains(&needle), "expected \"{needle}\" in import error \"{message}\"");
}

#[when(expr = "an imported scenario with an impossible transition sequence is validated for {string}")]
fn validate_impossible_import(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let digest: ContentHash = snapshot.artifact_digest(&machine_id).cloned().expect("digest exists");
    let scenario: ImportedScenario = ImportedScenario {
        id: "IMP-BAD-001".to_string(),
        model_id: ArtifactId::parse(&machine_id).expect("valid machine id"),
        model_digest: digest,
        strategy: "external_mbt_bfs".to_string(),
        coverage_objective: "shortcut".to_string(),
        seed: None,
        events: vec!["redeliver".to_string()],
        importer: "stub-mbt".to_string(),
        importer_version: "1.0.0".to_string(),
    };
    world.exploration.import_result = Some(validate_imported_scenario(snapshot, &scenario));
}

#[then("the scenario import fails with an invalid scenario error")]
fn import_fails_invalid(world: &mut Forge) {
    assert!(
        matches!(world.exploration.import_result, Some(Err(CoverageError::InvalidScenario(_)))),
        "{:?}",
        world.exploration.import_result
    );
}

#[when(expr = "an imported scenario with empty events is validated for {string}")]
fn validate_empty_events_import(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let digest: ContentHash = snapshot.artifact_digest(&machine_id).cloned().expect("digest exists");
    let scenario: ImportedScenario = ImportedScenario {
        id: "IMP-EMPTY-001".to_string(),
        model_id: ArtifactId::parse(&machine_id).expect("valid machine id"),
        model_digest: digest,
        strategy: "external_mbt_bfs".to_string(),
        coverage_objective: "empty".to_string(),
        seed: None,
        events: Vec::new(),
        importer: "stub-mbt".to_string(),
        importer_version: "1.0.0".to_string(),
    };
    world.exploration.import_result = Some(validate_imported_scenario(snapshot, &scenario));
}

#[when(expr = "an imported scenario with an empty importer version is validated for {string}")]
fn validate_empty_version_import(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let digest: ContentHash = snapshot.artifact_digest(&machine_id).cloned().expect("digest exists");
    let scenario: ImportedScenario = ImportedScenario {
        id: "IMP-NOVER-001".to_string(),
        model_id: ArtifactId::parse(&machine_id).expect("valid machine id"),
        model_digest: digest,
        strategy: "external_mbt_bfs".to_string(),
        coverage_objective: "deliver_state".to_string(),
        seed: None,
        events: vec!["deliver".to_string()],
        importer: "stub-mbt".to_string(),
        importer_version: "  ".to_string(),
    };
    world.exploration.import_result = Some(validate_imported_scenario(snapshot, &scenario));
}

#[when(expr = "coverage is computed for {string} over a zero-transition trace")]
fn coverage_zero_transition(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let trace: ExecutionTrace = ExecutionTrace {
        initial_state: "queued".to_string(),
        events: Vec::new(),
    };
    world.exploration.coverage_result = Some(compute_coverage(snapshot, &machine_id, &[trace]));
}

#[when(expr = "coverage is computed for {string} over the safe message trace")]
fn coverage_safe_trace(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let machine: &StateMachine = snapshot.state_machine(&machine_id).expect("machine exists");
    let events: Vec<String> = vec!["deliver".to_string(), "ack".to_string()];
    let trace: ExecutionTrace = execute_machine(machine, &events);
    world.exploration.coverage_result = Some(compute_coverage(snapshot, &machine_id, &[trace]));
}

fn current_coverage(world: &Forge) -> &CoverageRecord {
    match &world.exploration.coverage_result {
        Some(Ok(record)) => record,
        other => panic!("expected coverage record, got {other:?}"),
    }
}

#[then(expr = "state coverage reports {int} of {int} states covered and uncovered states {string}")]
fn assert_state_coverage(world: &mut Forge, covered: usize, total: usize, uncovered: String) {
    let record: &CoverageRecord = current_coverage(world);
    assert_eq!(record.states_covered.len(), covered);
    assert_eq!(record.states_total, total);
    assert_eq!(record.uncovered_states, listed(&uncovered, ", "));
}

#[then(expr = "transition coverage reports {int} of {int} transitions covered and uncovered transitions {string}")]
fn assert_transition_coverage(world: &mut Forge, covered: usize, total: usize, uncovered: String) {
    let record: &CoverageRecord = current_coverage(world);
    assert_eq!(record.transitions_covered.len(), covered);
    assert_eq!(record.transitions_total, total);
    assert_eq!(record.uncovered_transitions, listed(&uncovered, ", "));
}

#[then(expr = "transition pair coverage reports {int} of {int} pairs covered and {int} uncovered pair")]
fn assert_pair_coverage(world: &mut Forge, covered: usize, total: usize, uncovered: usize) {
    let record: &CoverageRecord = current_coverage(world);
    assert_eq!(record.transition_pairs_covered.len(), covered);
    assert_eq!(record.transition_pairs_total, total);
    assert_eq!(record.uncovered_transition_pairs.len(), uncovered);
}

#[then(expr = "requirement coverage reports {int} of {int} linked requirements covered")]
fn assert_req_coverage(world: &mut Forge, covered: usize, total: usize) {
    let record: &CoverageRecord = current_coverage(world);
    assert_eq!(record.requirements_covered.len(), covered);
    assert_eq!(record.requirements_total, total);
}

#[when(expr = "coverage is computed for {string} over a discontinuous trace")]
fn coverage_discontinuous_trace(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let trace: ExecutionTrace = ExecutionTrace {
        initial_state: "queued".to_string(),
        events: vec![TraceEvent {
            step: 1,
            event: "ack".to_string(),
            from_state: "delivered".to_string(),
            to_state: "acknowledged".to_string(),
            inputs: BTreeMap::new(),
            fault: None,
            virtual_time: Some(1),
        }],
    };
    world.exploration.coverage_result = Some(compute_coverage(snapshot, &machine_id, &[trace]));
}

#[when(expr = "coverage is computed for {string} over a continuous undeclared transition trace")]
fn coverage_undeclared_continuous(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let trace: ExecutionTrace = ExecutionTrace {
        initial_state: "queued".to_string(),
        events: vec![TraceEvent {
            step: 1,
            event: "ack".to_string(),
            from_state: "queued".to_string(),
            to_state: "queued".to_string(),
            inputs: BTreeMap::new(),
            fault: None,
            virtual_time: Some(1),
        }],
    };
    world.exploration.coverage_result = Some(compute_coverage(snapshot, &machine_id, &[trace]));
}

#[when(expr = "coverage is computed for {string} over a trace landing in the wrong target state")]
fn coverage_wrong_target_state(world: &mut Forge, machine_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let trace: ExecutionTrace = ExecutionTrace {
        initial_state: "queued".to_string(),
        events: vec![TraceEvent {
            step: 1,
            event: "deliver".to_string(),
            from_state: "queued".to_string(),
            to_state: "acknowledged".to_string(),
            inputs: BTreeMap::new(),
            fault: None,
            virtual_time: Some(1),
        }],
    };
    world.exploration.coverage_result = Some(compute_coverage(snapshot, &machine_id, &[trace]));
}

#[then("coverage computation fails with an invalid trace error")]
fn coverage_fails_trace(world: &mut Forge) {
    assert!(
        matches!(world.exploration.coverage_result, Some(Err(CoverageError::InvalidTrace(_)))),
        "{:?}",
        world.exploration.coverage_result
    );
}

#[then(expr = "the coverage error message mentions {string}")]
fn coverage_error_mentions(world: &mut Forge, needle: String) {
    let message: String = match &world.exploration.coverage_result {
        Some(Err(error)) => error.to_string(),
        other => panic!("expected coverage error, got {other:?}"),
    };
    assert!(message.contains(&needle), "expected \"{needle}\" in coverage error \"{message}\"");
}

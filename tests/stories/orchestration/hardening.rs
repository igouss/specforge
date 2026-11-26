use cucumber::then;
use specforge::ledger::EvidenceRecord;
use specforge::orchestrator::{OrchestratorError, PlanExecutionReport, VerificationRun};

use super::current_report;
use crate::{listed, Forge};

#[then("the adapter registry debug summary lists the registered methods")]
fn registry_debug_lists_methods(world: &mut Forge) {
    let summary: String = format!("{:?}", world.orchestration.registry);
    assert!(summary.contains("AdapterRegistry"), "unexpected debug summary: {summary}");
    assert!(summary.contains("PropertyBased"), "missing PropertyBased in {summary}");
    assert!(summary.contains("DeterministicSimulation"), "missing DeterministicSimulation in {summary}");
}

#[then("a started run allows idempotent progress updates, clamps progress to a positive budget, and allows unbounded progress when budget is zero")]
fn progress_idempotent_clamped_and_unbounded(world: &mut Forge) {
    let mut bounded: VerificationRun = world.orchestration.prepared[0].clone();
    bounded.start("2026-06-01T12:00:00Z").expect("start bounded");
    bounded.advance_progress(6).expect("advance to 6");
    bounded.advance_progress(6).expect("idempotent progress 6");
    assert_eq!(bounded.progress_current, 6);
    let over_budget: usize = bounded.progress_total + 10;
    bounded.advance_progress(over_budget).expect("clamp to total");
    assert_eq!(bounded.progress_current, bounded.progress_total);

    let mut unbounded: VerificationRun = world.orchestration.prepared[0].clone();
    unbounded.progress_total = 0;
    unbounded.start("2026-06-01T12:00:00Z").expect("start unbounded");
    unbounded.advance_progress(5).expect("advance unbounded");
    assert_eq!(unbounded.progress_current, 5);
}

#[then("starting a run with a blank timestamp is rejected")]
fn blank_timestamp_start_rejected(world: &mut Forge) {
    let mut run: VerificationRun = world.orchestration.prepared[0].clone();
    let result: Result<(), OrchestratorError> = run.start("   ");
    assert_eq!(result, Err(OrchestratorError::InvalidTimestamp));
}

#[then(expr = "the plan execution report evidence ids are {string}")]
fn report_evidence_ids_match(world: &mut Forge, expected: String) {
    let report: &PlanExecutionReport = current_report(world);
    assert_eq!(report.evidence_ids, listed(&expected, ", "));
}

#[then(expr = "the ledger holds {int} evidence records with sequential suffixes for {string}")]
fn ledger_holds_sequential_suffixes(world: &mut Forge, count: usize, plan_id: String) {
    let actual: Vec<String> = world
        .evidence_ledger
        .records()
        .iter()
        .map(|record: &EvidenceRecord| record.id.clone())
        .collect();
    assert_eq!(actual.len(), count, "records: {actual:?}");
    let expected: Vec<String> = vec![
        format!("EV-{plan_id}-ACT-PBT-004-PROP-MSG-001"),
        format!("EV-{plan_id}-ACT-DST-004-PROP-MSG-001"),
        format!("EV-{plan_id}-ACT-PBT-004-PROP-MSG-001-2"),
        format!("EV-{plan_id}-ACT-DST-004-PROP-MSG-001-2"),
        format!("EV-{plan_id}-ACT-PBT-004-PROP-MSG-001-3"),
        format!("EV-{plan_id}-ACT-DST-004-PROP-MSG-001-3"),
    ];
    assert_eq!(actual, expected);
}

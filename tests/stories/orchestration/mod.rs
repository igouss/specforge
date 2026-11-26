mod hardening;
mod stubs;

use cucumber::{given, then, when};
use specforge::assurance::{assess_claim, ClaimAssessment, CorrelationNote};
use specforge::graph::{compile_graph, GraphDefect, GraphSnapshot, VerificationClaim, VerificationMethod};
use specforge::orchestrator::{AdapterRegistry, OrchestratorError, PlanExecutionReport, VerificationOrchestrator, VerificationRun};
use specforge::proptest_adapter::ProptestAdapter;
use specforge::trace::Source;
use specforge::verify::RunState;

use self::stubs::{append_pair, assert_invalid, safe_trace, state_label, StubAdapter, StubFault};
use crate::graph::current_snapshot;
use crate::{listed, Forge};

#[derive(Debug, Default)]
pub struct OrchestrationState {
    pub registry: AdapterRegistry,
    pub prepared: Vec<VerificationRun>,
    pub report: Option<PlanExecutionReport>,
    pub last_error: Option<OrchestratorError>,
    pub blank_compile_result: Option<Result<GraphSnapshot, Vec<GraphDefect>>>,
    pub assessment: Option<ClaimAssessment>,
}

pub(super) fn current_report(world: &Forge) -> &PlanExecutionReport {
    world.orchestration.report.as_ref().expect("plan report present")
}

fn run_for_activity<'a>(world: &'a Forge, activity_id: &str) -> &'a VerificationRun {
    current_report(world)
        .runs
        .iter()
        .find(|r: &&VerificationRun| r.activity_id.as_str() == activity_id)
        .expect("activity run")
}

#[given("an adapter registry with safe property-based and deterministic-simulation adapters")]
fn registry_pbt_and_dst(world: &mut Forge) {
    let mut reg: AdapterRegistry = AdapterRegistry::new();
    reg.register(VerificationMethod::PropertyBased, Box::new(ProptestAdapter::with_runner(safe_trace, 6)));
    reg.register(
        VerificationMethod::DeterministicSimulation,
        Box::new(StubAdapter {
            method: VerificationMethod::DeterministicSimulation,
            fault: StubFault::None,
        }),
    );
    world.orchestration.registry = reg;
}

#[given("an adapter registry with violating property-based and safe deterministic-simulation adapters")]
fn registry_violating_pbt_and_safe_dst(world: &mut Forge) {
    let mut reg: AdapterRegistry = AdapterRegistry::new();
    reg.register(VerificationMethod::PropertyBased, Box::new(ProptestAdapter::for_machine(8)));
    reg.register(
        VerificationMethod::DeterministicSimulation,
        Box::new(StubAdapter {
            method: VerificationMethod::DeterministicSimulation,
            fault: StubFault::None,
        }),
    );
    world.orchestration.registry = reg;
}

#[given("an adapter registry with only a safe property-based adapter")]
fn registry_only_pbt(world: &mut Forge) {
    let mut reg: AdapterRegistry = AdapterRegistry::new();
    reg.register(VerificationMethod::PropertyBased, Box::new(ProptestAdapter::with_runner(safe_trace, 6)));
    world.orchestration.registry = reg;
}

#[when(expr = "verification plan {string} is executed with seed {int} at {string}")]
fn execute_plan_step(world: &mut Forge, plan_id: String, seed: u64, timestamp: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    match VerificationOrchestrator::execute_plan(
        &snapshot,
        &plan_id,
        &world.orchestration.registry,
        &mut world.evidence_ledger,
        &timestamp,
        Some(seed),
    ) {
        Ok(report) => {
            world.orchestration.report = Some(report);
            world.orchestration.last_error = None;
        }
        Err(error) => world.orchestration.last_error = Some(error),
    }
}

#[when(expr = "verification plan {string} is executed again with seed {int} at {string}")]
fn execute_plan_again_step(world: &mut Forge, plan_id: String, seed: u64, timestamp: String) {
    execute_plan_step(world, plan_id, seed, timestamp);
}

#[then(expr = "the plan execution report has {int} completed runs with distinct methods {string}")]
fn report_completed_methods(world: &mut Forge, count: usize, methods: String) {
    let report: &PlanExecutionReport = current_report(world);
    assert_eq!(report.runs.len(), count);
    let actual: Vec<String> = report
        .runs
        .iter()
        .map(|run: &VerificationRun| {
            assert_eq!(run.state, RunState::Completed);
            assert!(run.started_at.is_some() && run.finished_at.is_some() && run.progress_current == run.progress_total);
            run.method.as_str().to_string()
        })
        .collect();
    assert_eq!(actual, listed(&methods, ", "));
}

#[then(expr = "the plan execution report recorded {int} immutable evidence records in the ledger")]
fn report_recorded_evidence(world: &mut Forge, count: usize) {
    let report: &PlanExecutionReport = current_report(world);
    assert_eq!(report.evidence_ids.len(), count);
    assert_eq!(world.evidence_ledger.records().len(), count);
    world.evidence_ledger.verify_integrity().expect("ledger integrity holds");
    for id in &report.evidence_ids {
        assert!(world.evidence_ledger.get(id).expect("evidence present").verify_digest());
    }
}

#[then("the plan execution report has no missing required evidence")]
fn report_no_missing_required(world: &mut Forge) {
    assert!(current_report(world).missing_required_evidence.is_empty());
}

#[then(expr = "assessing claim {string} is {string} with {int} correlation notes")]
fn assess_claim_with_correlation_count(world: &mut Forge, claim_id: String, status: String, expected_notes: usize) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let claim: &VerificationClaim = snapshot.claim(&claim_id).expect("claim exists");
    let assessment: ClaimAssessment = assess_claim(claim, snapshot, &world.evidence_ledger);
    assert_eq!(assessment.status.to_string(), status);
    assert_eq!(assessment.correlations.len(), expected_notes, "{:?}", assessment.correlations);
    world.orchestration.assessment = Some(assessment);
}

#[when(expr = "verification plan {string} is prepared")]
fn prepare_plan_step(world: &mut Forge, plan_id: String) {
    world.orchestration.prepared = VerificationOrchestrator::prepare_plan(current_snapshot(world), &plan_id).expect("plan prepared");
}

#[then(expr = "{int} runs are prepared in state {string} bound to the snapshot")]
fn prepared_runs_bound(world: &mut Forge, count: usize, state: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    assert_eq!(world.orchestration.prepared.len(), count);
    for run in &world.orchestration.prepared {
        assert_eq!(state_label(run.state), state);
        assert_eq!(run.snapshot_id, snapshot.snapshot_id);
        assert!(!run.is_terminal());
    }
}

#[then("a prepared run transitions cleanly through requested, started, progress, and completed")]
fn run_transitions_completed(world: &mut Forge) {
    let mut run: VerificationRun = world.orchestration.prepared[0].clone();
    run.start("2026-06-01T12:00:00Z").expect("start");
    run.advance_progress(4).expect("progress");
    assert_eq!((run.state, run.progress_current), (RunState::Started, 4));
    run.complete("2026-06-01T12:00:05Z", Some("EV-TEST-1".to_string())).expect("complete");
    assert!(run.state == RunState::Completed && run.is_terminal() && run.progress_current == run.progress_total);
}

#[then("a started run rejects backwards progress advancement")]
fn started_run_rejects_backwards_progress(world: &mut Forge) {
    let mut run: VerificationRun = world.orchestration.prepared[0].clone();
    run.start("2026-06-01T12:00:00Z").expect("start");
    run.advance_progress(6).expect("advance to 6");
    assert_invalid(run.advance_progress(3));
    assert_eq!(run.progress_current, 6);
}

#[then("a completed run rejects further start, progress, fail, cancel, and inconclusive transitions")]
fn completed_run_rejects_mutations(world: &mut Forge) {
    let mut run: VerificationRun = world.orchestration.prepared[0].clone();
    run.start("2026-06-01T12:00:00Z").expect("start");
    run.complete("2026-06-01T12:00:05Z", Some("EV-TEST-1".to_string())).expect("complete");
    assert_invalid(run.start("2026-06-01T12:00:06Z"));
    assert_invalid(run.advance_progress(1));
    assert_invalid(run.fail("2026-06-01T12:00:06Z", "late".to_string(), None));
    assert_invalid(run.cancel("2026-06-01T12:00:06Z", None, None));
    assert_invalid(run.mark_inconclusive("2026-06-01T12:00:06Z", None, None));
}

#[then("a prepared run can cancel directly from requested and rejects transitions after cancelled")]
fn cancel_from_requested(world: &mut Forge) {
    let mut run: VerificationRun = world.orchestration.prepared[1].clone();
    run.cancel("2026-06-01T12:00:01Z", None, Some("cancelled".to_string())).expect("cancel");
    assert!(run.state == RunState::Cancelled && run.is_terminal());
    assert_invalid(run.start("2026-06-01T12:00:02Z"));
    assert_invalid(run.complete("2026-06-01T12:00:02Z", None));
}

#[then("a prepared run can transition from started to inconclusive and rejects transitions after inconclusive")]
fn started_to_inconclusive(world: &mut Forge) {
    let mut run: VerificationRun = world.orchestration.prepared[0].clone();
    run.start("2026-06-01T12:00:00Z").expect("start");
    run.mark_inconclusive("2026-06-01T12:00:03Z", None, Some("budget exhausted".to_string()))
        .expect("inconclusive");
    assert!(run.state == RunState::Inconclusive && run.is_terminal());
    assert_invalid(run.complete("2026-06-01T12:00:04Z", None));
}

#[then("a prepared run rejects completing, failing, or advancing progress directly from requested")]
fn requested_rejects_terminal_or_progress(world: &mut Forge) {
    let mut run: VerificationRun = world.orchestration.prepared[0].clone();
    assert_invalid(run.complete("2026-06-01T12:00:01Z", None));
    assert_invalid(run.fail("2026-06-01T12:00:01Z", "early".to_string(), None));
    assert_invalid(run.advance_progress(1));
    assert_invalid(run.mark_inconclusive("2026-06-01T12:00:01Z", None, None));
}

#[then(expr = "activity {string} finishes in state {string} with an evidence record")]
fn activity_completed_with_evidence(world: &mut Forge, activity_id: String, state: String) {
    let run: &VerificationRun = run_for_activity(world, &activity_id);
    assert_eq!(state_label(run.state), state);
    assert!(run.evidence_id.is_some());
}

#[then(expr = "activity {string} finishes in state {string} with no evidence record and an explicit error")]
fn activity_failed_without_evidence(world: &mut Forge, activity_id: String, state: String) {
    let run: &VerificationRun = run_for_activity(world, &activity_id);
    assert_eq!(state_label(run.state), state);
    assert!(run.evidence_id.is_none());
    assert!(run.error.as_deref().is_some_and(|msg: &str| msg.contains("no verification adapter registered")));
}

#[then(expr = "the plan execution report reports missing required evidence {string}")]
fn report_missing_required(world: &mut Forge, expected: String) {
    assert_eq!(current_report(world).missing_required_evidence, listed(&expected, ", "));
}

#[when(expr = "unknown verification plan {string} is prepared")]
fn prepare_unknown_plan(world: &mut Forge, plan_id: String) {
    world.orchestration.last_error = VerificationOrchestrator::prepare_plan(current_snapshot(world), &plan_id).err();
}

#[when(expr = "empty-target verification plan {string} is prepared")]
fn prepare_empty_plan(world: &mut Forge, plan_id: String) {
    world.orchestration.last_error = VerificationOrchestrator::prepare_plan(current_snapshot(world), &plan_id).err();
}

#[then(expr = "orchestration fails with an error mentioning {string}")]
fn orchestration_fails_mentioning(world: &mut Forge, fragment: String) {
    let text: String = world.orchestration.last_error.as_ref().expect("error").to_string().to_lowercase();
    assert!(text.contains(&fragment.to_lowercase()), "\"{text}\" missing \"{fragment}\"");
}

#[when("a plan with a blank required_evidence entry is compiled")]
fn compile_blank_required_evidence(world: &mut Forge) {
    let text: String = format!(
        "{}\n\n```specforge\nkind: plan\nid: PLAN-BLANK-001\ntargets:\n  - PROP-MSG-001\nscope: single-broker\nassumptions:\n  - ASM-MSG-001\nactivities:\n  - id: ACT-BLANK-001\n    method: property_based\n    budget: 4\nrequired_evidence:\n  - \"   \"\n```\n",
        crate::fixture_text("sdv-orchestration.md")
    );
    world.orchestration.blank_compile_result = Some(compile_graph(&[Source {
        path: "blank-plan.md".to_string(),
        text,
    }]));
}

#[then(expr = "blank required_evidence compilation fails with a graph defect mentioning {string}")]
fn blank_required_evidence_rejected(world: &mut Forge, fragment: String) {
    let Some(Err(defects)) = world.orchestration.blank_compile_result.as_ref() else {
        panic!("expected defect");
    };
    let joined: String = defects.iter().map(ToString::to_string).collect::<Vec<String>>().join("\n");
    assert!(joined.contains(&fragment), "\"{joined}\" missing \"{fragment}\"");
}

fn run_with_stub_fault(world: &mut Forge, plan_id: &str, fault: StubFault) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let mut reg: AdapterRegistry = AdapterRegistry::new();
    reg.register(
        VerificationMethod::PropertyBased,
        Box::new(StubAdapter {
            method: VerificationMethod::PropertyBased,
            fault,
        }),
    );
    reg.register(
        VerificationMethod::DeterministicSimulation,
        Box::new(StubAdapter {
            method: VerificationMethod::DeterministicSimulation,
            fault: StubFault::None,
        }),
    );
    world.orchestration.last_error =
        VerificationOrchestrator::execute_plan(&snapshot, plan_id, &reg, &mut world.evidence_ledger, "2026-06-01T12:08:00Z", Some(1)).err();
}

#[when(expr = "verification plan {string} is executed against an adapter returning a mismatched envelope")]
fn execute_mismatched_envelope(world: &mut Forge, plan_id: String) {
    run_with_stub_fault(world, &plan_id, StubFault::WrongRunId);
}

#[when(expr = "verification plan {string} is executed against an adapter returning a mismatched method envelope")]
fn execute_mismatched_method_envelope(world: &mut Forge, plan_id: String) {
    run_with_stub_fault(world, &plan_id, StubFault::WrongMethod);
}

#[when(expr = "verification plan {string} is executed against an adapter returning a mismatched activity envelope")]
fn execute_mismatched_activity_envelope(world: &mut Forge, plan_id: String) {
    run_with_stub_fault(world, &plan_id, StubFault::WrongActivity);
}

#[when(expr = "verification plan {string} is executed against an adapter returning a mismatched claims envelope")]
fn execute_mismatched_claims_envelope(world: &mut Forge, plan_id: String) {
    run_with_stub_fault(world, &plan_id, StubFault::WrongClaims);
}

#[when(expr = "verification plan {string} is executed against an adapter returning a completed inconclusive envelope")]
fn execute_completed_inconclusive_envelope(world: &mut Forge, plan_id: String) {
    run_with_stub_fault(world, &plan_id, StubFault::CompletedInconclusive);
}

#[then(expr = "orchestration fails with an error mentioning {string} and leaves the ledger empty")]
fn orchestration_fails_and_empty_ledger(world: &mut Forge, fragment: String) {
    orchestration_fails_mentioning(world, fragment);
    assert!(world.evidence_ledger.records().is_empty());
}

#[when(expr = "claim {string} is assessed after orchestration")]
fn assess_claim_after_orchestration(world: &mut Forge, claim_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let claim: &VerificationClaim = snapshot.claim(&claim_id).expect("claim exists");
    world.orchestration.assessment = Some(assess_claim(claim, snapshot, &world.evidence_ledger));
}

#[then(expr = "the orchestrated claim assessment discloses correlation notes mentioning {string} and {string}")]
fn assessment_discloses_two_fragments(world: &mut Forge, first: String, second: String) {
    let assessment: &ClaimAssessment = world.orchestration.assessment.as_ref().expect("assessment");
    let ok: bool = assessment
        .correlations
        .iter()
        .any(|n: &CorrelationNote| n.evidence_ids.len() >= 2 && n.reason.contains(&first) && n.reason.contains(&second));
    assert!(ok, "missing \"{first}\" & \"{second}\" in {:?}", assessment.correlations);
}

#[then("the orchestrated claim assessment rationale discloses the correlation")]
fn assessment_rationale_discloses_correlation(world: &mut Forge) {
    let assessment: &ClaimAssessment = world.orchestration.assessment.as_ref().expect("assessment");
    assert!(assessment.rationale.iter().any(|line: &String| line.contains("correlated supporting evidence")));
}

#[when(expr = "two supporting records share identical replay token {string} for {string} and claim {string}")]
fn two_records_same_token(world: &mut Forge, token: String, prop_id: String, claim_id: String) {
    append_pair(world, "EV-TOK", &prop_id, &claim_id, Some(token), Vec::new());
}

#[when(expr = "two supporting records share identical replay events {string} for {string} and claim {string}")]
fn two_records_same_events(world: &mut Forge, events: String, prop_id: String, claim_id: String) {
    append_pair(world, "EV-EVT", &prop_id, &claim_id, None, listed(&events, ", "));
}

#[then(expr = "the orchestrated claim assessment discloses a correlation note mentioning {string}")]
fn assessment_discloses_single_fragment(world: &mut Forge, fragment: String) {
    let assessment: &ClaimAssessment = world.orchestration.assessment.as_ref().expect("assessment");
    let ok: bool = assessment
        .correlations
        .iter()
        .any(|n: &CorrelationNote| n.evidence_ids.len() >= 2 && n.reason.contains(&fragment));
    assert!(ok, "missing \"{fragment}\" in {:?}", assessment.correlations);
}

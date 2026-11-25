use cucumber::{given, then, when};
use specforge::graph::{ArtifactId, SourceRevision, VerificationMethod};
use specforge::ledger::EvidenceKind;
use specforge::statistics::{
    ConfidenceLevel, QualifiedEstimate, SamplingModel, StatisticalError, StatisticalMethod, StatisticalSample, STATISTICAL_DISCLAIMER,
};
use specforge::verify::{EvaluationResult, FormalOutcome, RunState, VerificationOutcome};

use super::{append_custom_record, append_record, confidence_from_pct, default_model, require_estimate, run_estimate, sample_for, wilson_95_model};
use crate::graph::current_snapshot;
use crate::Forge;

#[given(expr = "a completed DST evidence record {string} is appended for property {string} and claim {string}")]
fn append_dst_record(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    append_record(
        world,
        &id,
        &property_id,
        &claim_id,
        EvidenceKind::PropertyEvaluation,
        VerificationMethod::DeterministicSimulation,
        RunState::Completed,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
    );
}

#[given(expr = "a proved formal evidence record {string} is appended for property {string} and claim {string}")]
fn append_proved_formal_record(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    append_record(
        world,
        &id,
        &property_id,
        &claim_id,
        EvidenceKind::FormalCheck,
        VerificationMethod::Formal,
        RunState::Completed,
        VerificationOutcome::Formal(FormalOutcome::Proved),
    );
}

#[given(expr = "an incomplete run evidence record {string} is appended for property {string} and claim {string}")]
fn append_incomplete_run_record(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    append_record(
        world,
        &id,
        &property_id,
        &claim_id,
        EvidenceKind::PropertyEvaluation,
        VerificationMethod::PropertyBased,
        RunState::Inconclusive,
        VerificationOutcome::Evaluation(EvaluationResult::Inconclusive),
    );
}

#[given(expr = "an evidence record {string} with undeclared assumption {string} is appended for property {string} and claim {string}")]
fn append_extra_assumption_record(world: &mut Forge, id: String, assumption: String, property_id: String, claim_id: String) {
    let revisions: Vec<SourceRevision> = current_snapshot(world).sources.clone();
    let assumptions: Vec<ArtifactId> = vec![
        ArtifactId::parse("ASM-MSG-001").expect("valid assumption id"),
        ArtifactId::parse(&assumption).expect("valid extra assumption id"),
    ];
    append_custom_record(
        world,
        &id,
        &property_id,
        &claim_id,
        EvidenceKind::PropertyEvaluation,
        VerificationMethod::PropertyBased,
        RunState::Completed,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        assumptions,
        revisions,
    );
}

#[given(expr = "an evidence record {string} with empty source revisions is appended for property {string} and claim {string}")]
fn append_empty_revisions_record(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    let assumptions: Vec<ArtifactId> = vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption id")];
    append_custom_record(
        world,
        &id,
        &property_id,
        &claim_id,
        EvidenceKind::PropertyEvaluation,
        VerificationMethod::PropertyBased,
        RunState::Completed,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        assumptions,
        Vec::new(),
    );
}

#[when(expr = "a statistical Wilson score estimate at {int} percent confidence is computed for claim {string} from {string} with {int} successes out of {int}")]
fn compute_wilson(world: &mut Forge, pct: u8, claim_id: String, evidence_raw: String, successes: usize, total: usize) {
    let model: SamplingModel = default_model(StatisticalMethod::WilsonScore {
        confidence: confidence_from_pct(pct),
    });
    let sample: StatisticalSample = sample_for(&claim_id, &evidence_raw, successes, total);
    run_estimate(world, &model, &sample);
}

#[allow(clippy::too_many_arguments)]
#[when(
    expr = "a statistical Beta-Binomial estimate with prior alpha {float} and beta {float} at {int} percent confidence is computed for claim {string} from {string} with {int} successes out of {int}"
)]
fn compute_beta_binomial(
    world: &mut Forge,
    prior_alpha: f64,
    prior_beta: f64,
    pct: u8,
    claim_id: String,
    evidence_raw: String,
    successes: usize,
    total: usize,
) {
    let model: SamplingModel = default_model(StatisticalMethod::BetaBinomial {
        prior_alpha,
        prior_beta,
        confidence: confidence_from_pct(pct),
    });
    let sample: StatisticalSample = sample_for(&claim_id, &evidence_raw, successes, total);
    run_estimate(world, &model, &sample);
}

#[when(expr = "a statistical estimate is attempted for claim {string} from {string} with missing {string}")]
fn attempt_missing_field(world: &mut Forge, claim_id: String, evidence_raw: String, field: String) {
    let mut model: SamplingModel = wilson_95_model();
    match field.as_str() {
        "hypothesis" => model.hypothesis = "   ".to_string(),
        "sampling_population" => model.sampling_population = String::new(),
        "sampling_process" => model.sampling_process = "\t".to_string(),
        "scope" => model.scope = " ".to_string(),
        "independence_assumptions" => model.independence_assumptions.clear(),
        "blank_independence_assumption" => model.independence_assumptions.push("   ".to_string()),
        "limitations" => model.limitations.clear(),
        "blank_limitation" => model.limitations.push(String::new()),
        other => panic!("unexpected missing field case {other}"),
    }
    let sample: StatisticalSample = sample_for(&claim_id, &evidence_raw, 9, 10);
    run_estimate(world, &model, &sample);
}

#[when(expr = "a statistical estimate is attempted for claim {string} from {string} with model scope {string}")]
fn attempt_model_scope(world: &mut Forge, claim_id: String, evidence_raw: String, scope: String) {
    let mut model: SamplingModel = wilson_95_model();
    model.scope = scope;
    let sample: StatisticalSample = sample_for(&claim_id, &evidence_raw, 9, 10);
    run_estimate(world, &model, &sample);
}

#[when(expr = "a statistical estimate is attempted for unknown claim {string} from {string}")]
fn attempt_unknown_claim(world: &mut Forge, claim_id: String, evidence_raw: String) {
    let model: SamplingModel = wilson_95_model();
    let sample: StatisticalSample = sample_for(&claim_id, &evidence_raw, 9, 10);
    run_estimate(world, &model, &sample);
}

#[when(expr = "a statistical Beta-Binomial estimate with invalid prior {string} is attempted for claim {string} from {string}")]
fn attempt_invalid_prior(world: &mut Forge, kind: String, claim_id: String, evidence_raw: String) {
    let (prior_alpha, prior_beta): (f64, f64) = match kind.as_str() {
        "zero_alpha" => (0.0, 1.0),
        "negative_beta" => (1.0, -2.0),
        "nan_alpha" => (f64::NAN, 1.0),
        "infinite_beta" => (1.0, f64::INFINITY),
        other => panic!("unexpected invalid prior case {other}"),
    };
    let model: SamplingModel = default_model(StatisticalMethod::BetaBinomial {
        prior_alpha,
        prior_beta,
        confidence: ConfidenceLevel::NinetyFive,
    });
    let sample: StatisticalSample = sample_for(&claim_id, &evidence_raw, 9, 10);
    run_estimate(world, &model, &sample);
}

#[when(expr = "a statistical estimate is attempted for claim {string} with invalid sample {string}")]
fn attempt_invalid_sample(world: &mut Forge, claim_id: String, kind: String) {
    let model: SamplingModel = wilson_95_model();
    let mut sample: StatisticalSample = sample_for(&claim_id, "EV-STAT-1", 9, 10);
    match kind.as_str() {
        "zero_size" => {
            sample.sample_size = 0;
            sample.observed_successes = 0;
            sample.observed_failures = 0;
        }
        "mismatched_counts" => {
            sample.observed_successes = 6;
            sample.observed_failures = 5;
        }
        "empty_evidence" => sample.evidence_ids.clear(),
        "duplicate_evidence" => sample.evidence_ids.push("EV-STAT-1".to_string()),
        other => panic!("unexpected invalid sample case {other}"),
    }
    run_estimate(world, &model, &sample);
}

#[then(expr = "the statistical estimate succeeds for claim {string} in scope {string}")]
fn estimate_succeeds(world: &mut Forge, claim_id: String, scope: String) {
    let estimate: &QualifiedEstimate = require_estimate(world);
    assert_eq!(estimate.claim_id.as_str(), claim_id);
    assert_eq!(estimate.scope, scope);
    assert!(estimate.point_estimate.is_finite() && estimate.lower_bound.is_finite() && estimate.upper_bound.is_finite());
    assert!(estimate.lower_bound <= estimate.point_estimate && estimate.point_estimate <= estimate.upper_bound);
    assert!(estimate.separate_from_formal_proof);
}

#[then(expr = "the statistical estimate records Wilson score at {int} percent confidence with sample size {int}, {int} successes, and {int} failures")]
fn estimate_records_wilson(world: &mut Forge, pct: u8, total: usize, successes: usize, failures: usize) {
    let estimate: &QualifiedEstimate = require_estimate(world);
    assert_eq!(
        estimate.method,
        StatisticalMethod::WilsonScore {
            confidence: confidence_from_pct(pct),
        }
    );
    assert_eq!(estimate.confidence_level_pct, pct);
    assert_eq!(estimate.sample_size, total);
    assert_eq!(estimate.observed_successes, successes);
    assert_eq!(estimate.observed_failures, failures);
}

#[then(expr = "the statistical estimate records Beta-Binomial prior {float} and {float} at {int} percent confidence")]
fn estimate_records_beta_binomial(world: &mut Forge, alpha: f64, beta: f64, pct: u8) {
    let estimate: &QualifiedEstimate = require_estimate(world);
    assert_eq!(
        estimate.method,
        StatisticalMethod::BetaBinomial {
            prior_alpha: alpha,
            prior_beta: beta,
            confidence: confidence_from_pct(pct),
        }
    );
    assert_eq!(estimate.confidence_level_pct, pct);
}

#[then(expr = "the statistical estimate point estimate is within {float} of {float} with finite ordered bounds strictly inside {float} to {float}")]
fn estimate_bounds_in_range(world: &mut Forge, tolerance: f64, expected_point: f64, min_bound: f64, max_bound: f64) {
    let estimate: &QualifiedEstimate = require_estimate(world);
    assert!((estimate.point_estimate - expected_point).abs() <= tolerance);
    assert!(estimate.lower_bound > min_bound && estimate.lower_bound < estimate.point_estimate);
    assert!(estimate.point_estimate < estimate.upper_bound && estimate.upper_bound < max_bound);
}

#[then("the statistical estimate preserves sampling metadata, disclaimer, and separation from formal proof")]
fn estimate_preserves_metadata(world: &mut Forge) {
    let estimate: &QualifiedEstimate = require_estimate(world);
    assert!(!estimate.hypothesis.trim().is_empty());
    assert!(!estimate.sampling_population.trim().is_empty());
    assert!(!estimate.sampling_process.trim().is_empty());
    assert_eq!(estimate.independence_assumptions.len(), 2);
    assert!(!estimate.evidence_ids.is_empty());
    assert!(estimate.limitations.iter().any(|item: &String| item == STATISTICAL_DISCLAIMER));
    assert!(estimate.separate_from_formal_proof);
}

#[then(expr = "the statistical estimate limitations include {string}")]
fn estimate_limitations_include(world: &mut Forge, expected: String) {
    let estimate: &QualifiedEstimate = require_estimate(world);
    assert!(
        estimate.limitations.iter().any(|item: &String| item == &expected),
        "limitations {:?} missing \"{expected}\"",
        estimate.limitations
    );
}

#[then("the statistical estimate round-trips through JSON losslessly")]
fn estimate_json_round_trip(world: &mut Forge) {
    let estimate: &QualifiedEstimate = require_estimate(world);
    let serialized: String = serde_json::to_string(estimate).expect("estimate serializes to JSON");
    let decoded: QualifiedEstimate = serde_json::from_str(&serialized).expect("estimate deserializes from JSON");
    assert_eq!(estimate.claim_id, decoded.claim_id);
    assert_eq!(estimate.hypothesis, decoded.hypothesis);
    assert_eq!(estimate.scope, decoded.scope);
    assert_eq!(estimate.method, decoded.method);
    assert_eq!(estimate.sampling_population, decoded.sampling_population);
    assert_eq!(estimate.sampling_process, decoded.sampling_process);
    assert_eq!(estimate.independence_assumptions, decoded.independence_assumptions);
    assert_eq!(estimate.sample_size, decoded.sample_size);
    assert_eq!(estimate.observed_successes, decoded.observed_successes);
    assert_eq!(estimate.observed_failures, decoded.observed_failures);
    assert_eq!(estimate.confidence_level_pct, decoded.confidence_level_pct);
    assert_eq!(estimate.limitations, decoded.limitations);
    assert_eq!(estimate.evidence_ids, decoded.evidence_ids);
    assert_eq!(estimate.separate_from_formal_proof, decoded.separate_from_formal_proof);
    assert!((estimate.point_estimate - decoded.point_estimate).abs() < 1e-15);
    assert!((estimate.lower_bound - decoded.lower_bound).abs() < 1e-15);
    assert!((estimate.upper_bound - decoded.upper_bound).abs() < 1e-15);
}

#[then("the statistical estimate has lower bound below 1.0 and upper bound equal to 1.0")]
fn estimate_all_success_bounds(world: &mut Forge) {
    let estimate: &QualifiedEstimate = require_estimate(world);
    assert!((estimate.point_estimate - 1.0).abs() < 1e-12);
    assert!(estimate.lower_bound < 1.0);
    assert!((estimate.upper_bound - 1.0).abs() < 1e-12);
}

#[then("the statistical estimate has lower bound equal to 0.0 and upper bound above 0.0")]
fn estimate_all_failure_bounds(world: &mut Forge) {
    let estimate: &QualifiedEstimate = require_estimate(world);
    assert!(estimate.point_estimate.abs() < 1e-12);
    assert!(estimate.lower_bound.abs() < 1e-12);
    assert!(estimate.upper_bound > 0.0);
}

#[then(expr = "the statistical estimate fails with error kind {string}")]
fn estimate_fails_with_kind(world: &mut Forge, expected_kind: String) {
    let error: &StatisticalError = match &world.statistics.last_result {
        Some(Err(err)) => err,
        Some(Ok(estimate)) => panic!("expected error {expected_kind}, got estimate: {estimate:?}"),
        None => panic!("no statistical estimate attempted"),
    };
    assert!(!error.to_string().trim().is_empty());
    let actual_kind: &str = match error {
        StatisticalError::UnknownClaim(_) => "unknown_claim",
        StatisticalError::MissingAssumptions(_) => "missing_assumptions",
        StatisticalError::InvalidModel(_) => "invalid_model",
        StatisticalError::InvalidSample(_) => "invalid_sample",
        StatisticalError::UnverifiedEvidence(_) => "unverified_evidence",
        StatisticalError::FormalProofDisallowed(_) => "formal_proof_disallowed",
        StatisticalError::HeterogeneousEvidenceDisallowed(_) => "heterogeneous_evidence_disallowed",
        StatisticalError::ScopeMismatch { .. } => "scope_mismatch",
    };
    assert_eq!(actual_kind, expected_kind, "unexpected error: {error}");
}

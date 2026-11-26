mod validate;

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::graph::{ArtifactId, GraphSnapshot, VerificationClaim};
use crate::ledger::EvidenceLedger;

use validate::{validate_evidence, validate_model, validate_sample};

pub const STATISTICAL_DISCLAIMER: &str = "statistical estimate under declared sampling assumptions; not a formal proof of universal correctness";

const BETA_NORMAL_APPROX_LIMITATION: &str = "normal approximation to Beta posterior credible interval";
const Z_NINETY: f64 = 1.6448536269514722;
const Z_NINETY_FIVE: f64 = 1.959963984540054;
const Z_NINETY_NINE: f64 = 2.5758293035489004;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    Ninety,
    NinetyFive,
    NinetyNine,
}

impl ConfidenceLevel {
    /// When a confidence level's percentage is queried, `percentage` shall return 90, 95, or 99.
    pub fn percentage(&self) -> u8 {
        match self {
            ConfidenceLevel::Ninety => 90,
            ConfidenceLevel::NinetyFive => 95,
            ConfidenceLevel::NinetyNine => 99,
        }
    }

    /// When a confidence level's two-sided normal quantile is queried, `z_score` shall return the exact z-score for 90%, 95%, or 99%.
    pub fn z_score(&self) -> f64 {
        match self {
            ConfidenceLevel::Ninety => Z_NINETY,
            ConfidenceLevel::NinetyFive => Z_NINETY_FIVE,
            ConfidenceLevel::NinetyNine => Z_NINETY_NINE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StatisticalMethod {
    WilsonScore {
        confidence: ConfidenceLevel,
    },
    BetaBinomial {
        prior_alpha: f64,
        prior_beta: f64,
        confidence: ConfidenceLevel,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamplingModel {
    pub hypothesis: String,
    pub sampling_population: String,
    pub sampling_process: String,
    pub independence_assumptions: Vec<String>,
    pub scope: String,
    pub method: StatisticalMethod,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatisticalSample {
    pub claim_id: ArtifactId,
    pub sample_size: usize,
    pub observed_successes: usize,
    pub observed_failures: usize,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatisticalError {
    UnknownClaim(ArtifactId),
    MissingAssumptions(String),
    InvalidModel(String),
    InvalidSample(String),
    UnverifiedEvidence(String),
    FormalProofDisallowed(String),
    HeterogeneousEvidenceDisallowed(String),
    ScopeMismatch { model_scope: String, claim_scope: String },
}

impl fmt::Display for StatisticalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatisticalError::UnknownClaim(id) => write!(formatter, "unknown verification claim {id}"),
            StatisticalError::MissingAssumptions(why) => write!(formatter, "missing statistical assumptions: {why}"),
            StatisticalError::InvalidModel(why) => write!(formatter, "invalid statistical model: {why}"),
            StatisticalError::InvalidSample(why) => write!(formatter, "invalid statistical sample: {why}"),
            StatisticalError::UnverifiedEvidence(why) => write!(formatter, "unverified evidence for statistical estimate: {why}"),
            StatisticalError::FormalProofDisallowed(why) => write!(formatter, "formal proof evidence disallowed in statistical estimate: {why}"),
            StatisticalError::HeterogeneousEvidenceDisallowed(why) => {
                write!(formatter, "heterogeneous verification methods disallowed in statistical estimate: {why}")
            }
            StatisticalError::ScopeMismatch { model_scope, claim_scope } => {
                write!(formatter, "sampling model scope \"{model_scope}\" does not match claim scope \"{claim_scope}\"")
            }
        }
    }
}

impl Error for StatisticalError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualifiedEstimate {
    pub claim_id: ArtifactId,
    pub hypothesis: String,
    pub scope: String,
    pub method: StatisticalMethod,
    pub sampling_population: String,
    pub sampling_process: String,
    pub independence_assumptions: Vec<String>,
    pub sample_size: usize,
    pub observed_successes: usize,
    pub observed_failures: usize,
    pub point_estimate: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub confidence_level_pct: u8,
    pub limitations: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub separate_from_formal_proof: bool,
}

fn compute_bounds(method: StatisticalMethod, sample: &StatisticalSample) -> (f64, f64, f64, u8) {
    let n: f64 = sample.sample_size as f64;
    let k: f64 = sample.observed_successes as f64;
    let failures: f64 = sample.observed_failures as f64;
    match method {
        StatisticalMethod::WilsonScore { confidence } => {
            let z: f64 = confidence.z_score();
            let z2: f64 = z * z;
            let p_hat: f64 = k / n;
            let denom: f64 = 1.0 + z2 / n;
            let center: f64 = p_hat + z2 / (2.0 * n);
            let radicand: f64 = (p_hat * (1.0 - p_hat) + z2 / (4.0 * n)) / n;
            let half: f64 = z * radicand.max(0.0).sqrt();
            let raw_lower: f64 = ((center - half) / denom).clamp(0.0, 1.0);
            let raw_upper: f64 = ((center + half) / denom).clamp(0.0, 1.0);
            let lower: f64 = raw_lower.min(p_hat);
            let upper: f64 = raw_upper.max(p_hat);
            (p_hat, lower, upper, confidence.percentage())
        }
        StatisticalMethod::BetaBinomial {
            prior_alpha,
            prior_beta,
            confidence,
        } => {
            let posterior_alpha: f64 = prior_alpha + k;
            let posterior_beta: f64 = prior_beta + failures;
            let total: f64 = posterior_alpha + posterior_beta;
            let mean: f64 = posterior_alpha / total;
            let variance: f64 = (posterior_alpha / total) * (posterior_beta / total) / (total + 1.0);
            let std_dev: f64 = variance.max(0.0).sqrt();
            let z: f64 = confidence.z_score();
            let lower: f64 = (mean - z * std_dev).clamp(0.0, 1.0).min(mean);
            let upper: f64 = (mean + z * std_dev).clamp(0.0, 1.0).max(mean);
            (mean, lower, upper, confidence.percentage())
        }
    }
}

/// When a declared sampling model and homogeneous current evidence sample are evaluated for a claim, `compute_qualified_estimate` shall validate assumptions, scope, evidence integrity, provenance, failure consistency, and non-formal homogeneity, and compute deterministic uncertainty bounds kept strictly separate from formal proof status.
pub fn compute_qualified_estimate(
    snapshot: &GraphSnapshot,
    ledger: &EvidenceLedger,
    model: &SamplingModel,
    sample: &StatisticalSample,
) -> Result<QualifiedEstimate, StatisticalError> {
    let claim: &VerificationClaim = snapshot
        .claim(sample.claim_id.as_str())
        .ok_or_else(|| StatisticalError::UnknownClaim(sample.claim_id.clone()))?;
    validate_model(model, claim)?;
    validate_sample(sample)?;
    validate_evidence(snapshot, ledger, claim, sample)?;
    let (point_estimate, lower_bound, upper_bound, confidence_level_pct): (f64, f64, f64, u8) = compute_bounds(model.method, sample);
    let mut limitations: Vec<String> = model.limitations.clone();
    match model.method {
        StatisticalMethod::WilsonScore { confidence: _ } => {}
        StatisticalMethod::BetaBinomial {
            prior_alpha: _,
            prior_beta: _,
            confidence: _,
        } => {
            if !limitations.iter().any(|item: &String| item == BETA_NORMAL_APPROX_LIMITATION) {
                limitations.push(BETA_NORMAL_APPROX_LIMITATION.to_string());
            }
        }
    }
    if !limitations.iter().any(|item: &String| item == STATISTICAL_DISCLAIMER) {
        limitations.push(STATISTICAL_DISCLAIMER.to_string());
    }
    Ok(QualifiedEstimate {
        claim_id: claim.id.clone(),
        hypothesis: model.hypothesis.clone(),
        scope: model.scope.clone(),
        method: model.method,
        sampling_population: model.sampling_population.clone(),
        sampling_process: model.sampling_process.clone(),
        independence_assumptions: model.independence_assumptions.clone(),
        sample_size: sample.sample_size,
        observed_successes: sample.observed_successes,
        observed_failures: sample.observed_failures,
        point_estimate,
        lower_bound,
        upper_bound,
        confidence_level_pct,
        limitations,
        evidence_ids: sample.evidence_ids.clone(),
        separate_from_formal_proof: true,
    })
}

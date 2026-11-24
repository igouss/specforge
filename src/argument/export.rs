use std::fmt::Write as _;

use super::{ArgumentDefeater, ArgumentEvidenceItem, AssuranceArgument};
use crate::graph::{canonical_json, ArtifactId, VerificationMethod};
use crate::verify::{FormalOutcome, VerificationOutcome};

fn method_label(method: VerificationMethod) -> &'static str {
    match method {
        VerificationMethod::ExampleBased => "example_based",
        VerificationMethod::ModelBased => "model_based",
        VerificationMethod::PropertyBased => "property_based",
        VerificationMethod::DeterministicSimulation => "deterministic_simulation",
        VerificationMethod::Formal => "formal",
    }
}

fn outcome_label(outcome: &VerificationOutcome) -> String {
    match outcome {
        VerificationOutcome::Evaluation(result) => format!("evaluation:{result}"),
        VerificationOutcome::Formal(formal) => match formal {
            FormalOutcome::Proved => "formal:proved".to_string(),
            FormalOutcome::Refuted => "formal:refuted".to_string(),
            FormalOutcome::BoundedSuccess { bound } => format!("formal:bounded_success({bound})"),
            FormalOutcome::BoundedFailure { bound } => format!("formal:bounded_failure({bound})"),
            FormalOutcome::Unknown => "formal:unknown".to_string(),
            FormalOutcome::Timeout => "formal:timeout".to_string(),
            FormalOutcome::Unsupported => "formal:unsupported".to_string(),
        },
    }
}

/// When an assurance argument is exported as JSON, `export_argument_json` shall emit canonical deterministic JSON.
pub fn export_argument_json(argument: &AssuranceArgument) -> Result<String, String> {
    canonical_json(argument)
}

fn write_evidence_markdown(out: &mut String, evidence: &[ArgumentEvidenceItem], indent: &str) {
    if evidence.is_empty() {
        let _ = writeln!(out, "{indent}- None");
        return;
    }
    for item in evidence {
        let _ = writeln!(
            out,
            "{indent}- `{}` | method: `{}` | freshness: `{}` | outcome: `{}`",
            item.evidence_id,
            method_label(item.method),
            item.freshness,
            outcome_label(&item.outcome)
        );
        for limitation in &item.limitations {
            let _ = writeln!(out, "{indent}  - Limitation: {limitation}");
        }
    }
}

fn write_defeaters_markdown(out: &mut String, defeaters: &[ArgumentDefeater], indent: &str) {
    if defeaters.is_empty() {
        let _ = writeln!(out, "{indent}- None");
        return;
    }
    for defeater in defeaters {
        let _ = writeln!(out, "{indent}- `{}` (`{}`): {}", defeater.kind, defeater.source_id, defeater.detail);
    }
}

fn write_rationale_markdown(out: &mut String, rationale: &[String], indent: &str) {
    if rationale.is_empty() {
        let _ = writeln!(out, "{indent}- None");
        return;
    }
    for reason in rationale {
        let _ = writeln!(out, "{indent}- {reason}");
    }
}

fn join_assumptions(assumptions: &[ArtifactId]) -> String {
    if assumptions.is_empty() {
        "none".to_string()
    } else {
        assumptions.iter().map(ArtifactId::as_str).collect::<Vec<&str>>().join(", ")
    }
}

/// When an assurance argument is exported as Markdown, `export_argument_markdown` shall render a deterministic human-readable report disclosing scope, assumptions, evidence, limitations, defeaters, subclaims, and rationale.
pub fn export_argument_markdown(argument: &AssuranceArgument) -> String {
    let mut out: String = String::new();
    let _ = writeln!(out, "# Assurance Argument: {}", argument.claim_id);
    let _ = writeln!(out);
    let _ = writeln!(out, "- Proposition: {}", argument.proposition);
    let _ = writeln!(out, "- Snapshot: `{}`", argument.snapshot_id);
    let _ = writeln!(out, "- Scope: `{}`", argument.scope);
    let _ = writeln!(out, "- Assumptions: {}", join_assumptions(&argument.assumptions));
    let _ = writeln!(out, "- Status: `{}`", argument.status);
    let _ = writeln!(out, "- Unbounded Proof: `{}`", if argument.unbounded_proof { "yes" } else { "no" });
    let _ = writeln!(out);
    let _ = writeln!(out, "## Evidence");
    write_evidence_markdown(&mut out, &argument.evidence, "");
    let _ = writeln!(out);
    let _ = writeln!(out, "## Defeaters");
    write_defeaters_markdown(&mut out, &argument.defeaters, "");
    let _ = writeln!(out);
    let _ = writeln!(out, "## Subclaims");
    if argument.subclaims.is_empty() {
        let _ = writeln!(out, "- None");
    } else {
        for sub in &argument.subclaims {
            let _ = writeln!(out, "### Subclaim: {}", sub.claim_id);
            let _ = writeln!(out, "- Proposition: {}", sub.proposition);
            let _ = writeln!(out, "- Scope: `{}`", sub.scope);
            let _ = writeln!(out, "- Assumptions: {}", join_assumptions(&sub.assumptions));
            let _ = writeln!(out, "- Status: `{}`", sub.status);
            let _ = writeln!(out, "- Unbounded Proof: `{}`", if sub.unbounded_proof { "yes" } else { "no" });
            let _ = writeln!(out, "- Evidence:");
            write_evidence_markdown(&mut out, &sub.evidence, "  ");
            let _ = writeln!(out, "- Defeaters:");
            write_defeaters_markdown(&mut out, &sub.defeaters, "  ");
            let _ = writeln!(out, "- Rationale:");
            write_rationale_markdown(&mut out, &sub.rationale, "  ");
            let _ = writeln!(out);
        }
    }
    let _ = writeln!(out, "## Rationale");
    write_rationale_markdown(&mut out, &argument.rationale, "");
    out
}

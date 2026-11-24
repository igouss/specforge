use std::collections::BTreeMap;

use serde::Serialize;

use super::{CliOutput, EXIT_OK};
use crate::assurance::{ClaimAssessment, ImpactSet};
use crate::graph::{canonical_json, ArtifactId, ContentHash, GraphSnapshot, SourceRevision, VerificationMethod};
use crate::ledger::{EvidenceKind, EvidenceLink, EvidenceRecord, EvidenceRelation};
use crate::verify::{Counterexample, FormalOutcome, RunEnvelope, RunState, VerificationOutcome};

#[derive(Serialize)]
struct CompileReport<'a> {
    status: &'a str,
    snapshot_id: &'a ContentHash,
    sources: &'a [SourceRevision],
    requirements: usize,
    artifacts: usize,
    digests: &'a BTreeMap<String, ContentHash>,
}

#[derive(Serialize)]
struct AssessReport<'a> {
    snapshot_id: &'a ContentHash,
    passed: bool,
    claims: &'a [ClaimAssessment],
}

#[derive(Serialize)]
struct VerifyReport<'a> {
    snapshot_id: &'a ContentHash,
    property_id: &'a ArtifactId,
    claim_ids: &'a [ArtifactId],
    run_id: &'a str,
    seed: u64,
    budget: usize,
    run_state: RunState,
    outcome: String,
    raw_status: &'a str,
    evidence_id: Option<&'a str>,
    counterexample_blob_digest: Option<&'a ContentHash>,
    original_counterexample: Option<&'a Counterexample>,
    minimized_counterexample: Option<&'a Counterexample>,
}

#[derive(Serialize)]
struct LedgerVerifyReport<'a> {
    status: &'a str,
    records: usize,
    links: usize,
    blobs: usize,
}

#[derive(Serialize)]
struct LedgerRecordSummary<'a> {
    id: &'a str,
    kind: EvidenceKind,
    method: VerificationMethod,
    property_id: Option<&'a ArtifactId>,
    claim_ids: &'a [ArtifactId],
    run_state: RunState,
    outcome: String,
    snapshot_id: &'a ContentHash,
    content_digest: &'a ContentHash,
}

#[derive(Serialize)]
struct LedgerLinkSummary<'a> {
    from_id: &'a str,
    to_id: &'a str,
    relation: EvidenceRelation,
    reason: &'a str,
    content_digest: &'a ContentHash,
}

#[derive(Serialize)]
struct LedgerListReport<'a> {
    records: Vec<LedgerRecordSummary<'a>>,
    links: Vec<LedgerLinkSummary<'a>>,
}

#[derive(Serialize)]
struct ImpactReport<'a> {
    from_snapshot_id: &'a ContentHash,
    to_snapshot_id: &'a ContentHash,
    clean: bool,
    changed_artifacts: &'a [String],
    stale_evidence_ids: &'a [String],
    affected_claim_ids: &'a [ArtifactId],
}

fn encode_json<T: Serialize>(value: &T, json: bool) -> Result<String, CliOutput> {
    canonical_json(value).map_err(|error: String| CliOutput::error(error, json))
}

pub(crate) fn outcome_label(outcome: &VerificationOutcome) -> String {
    match outcome {
        VerificationOutcome::Evaluation(result) => result.to_string(),
        VerificationOutcome::Formal(formal) => match formal {
            FormalOutcome::Proved => "proved".to_string(),
            FormalOutcome::Refuted => "refuted".to_string(),
            FormalOutcome::BoundedSuccess { bound } => format!("bounded_success({bound})"),
            FormalOutcome::BoundedFailure { bound } => format!("bounded_failure({bound})"),
            FormalOutcome::Unknown => "unknown".to_string(),
            FormalOutcome::Timeout => "timeout".to_string(),
            FormalOutcome::Unsupported => "unsupported".to_string(),
        },
    }
}

fn kind_label(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::PropertyEvaluation => "property_evaluation",
        EvidenceKind::Counterexample => "counterexample",
        EvidenceKind::Replay => "replay",
        EvidenceKind::StoryRun => "story_run",
        EvidenceKind::FormalCheck => "formal_check",
    }
}

fn method_label(method: VerificationMethod) -> &'static str {
    match method {
        VerificationMethod::ExampleBased => "example_based",
        VerificationMethod::ModelBased => "model_based",
        VerificationMethod::PropertyBased => "property_based",
        VerificationMethod::DeterministicSimulation => "deterministic_simulation",
        VerificationMethod::Formal => "formal",
    }
}

fn relation_label(relation: EvidenceRelation) -> &'static str {
    match relation {
        EvidenceRelation::Supports => "supports",
        EvidenceRelation::Contradicts => "contradicts",
        EvidenceRelation::Corroborates => "corroborates",
        EvidenceRelation::DerivedFrom => "derived_from",
        EvidenceRelation::Supersedes => "supersedes",
        EvidenceRelation::Invalidates => "invalidates",
    }
}

pub(crate) fn format_compile(snapshot: &GraphSnapshot, json: bool) -> CliOutput {
    if json {
        let report: CompileReport<'_> = CompileReport {
            status: "ok",
            snapshot_id: &snapshot.snapshot_id,
            sources: &snapshot.sources,
            requirements: snapshot.trace.requirements.len(),
            artifacts: snapshot.artifact_count(),
            digests: &snapshot.digests,
        };
        return match encode_json(&report, true) {
            Ok(encoded) => CliOutput::ok(encoded),
            Err(output) => output,
        };
    }
    let mut lines: Vec<String> = vec![
        format!("snapshot_id: {}", snapshot.snapshot_id),
        format!("sources: {}", snapshot.sources.len()),
        format!("requirements: {}", snapshot.trace.requirements.len()),
        format!("artifacts: {}", snapshot.artifact_count()),
        "digests:".to_string(),
    ];
    for (id, digest) in &snapshot.digests {
        lines.push(format!("  {id}: {digest}"));
    }
    CliOutput::ok(lines.join("\n"))
}

pub(crate) fn format_assess(snapshot: &GraphSnapshot, assessments: &[ClaimAssessment], passed: bool, exit_code: u8, json: bool) -> CliOutput {
    if json {
        let report: AssessReport<'_> = AssessReport {
            snapshot_id: &snapshot.snapshot_id,
            passed,
            claims: assessments,
        };
        return match encode_json(&report, true) {
            Ok(encoded) => CliOutput::with_status(exit_code, encoded),
            Err(output) => output,
        };
    }
    let mut lines: Vec<String> = vec![format!("snapshot_id: {}", snapshot.snapshot_id), format!("assessed: {}", assessments.len())];
    if assessments.is_empty() {
        lines.push("no claims declared in snapshot".to_string());
    }
    for assessment in assessments {
        lines.push(format!(
            "{}: {} (unbounded_proof={}, evidence={}, contradictions={})",
            assessment.claim_id,
            assessment.status,
            assessment.unbounded_proof,
            assessment.evaluated.len(),
            assessment.contradictions.len()
        ));
        for reason in &assessment.rationale {
            lines.push(format!("  rationale: {reason}"));
        }
    }
    CliOutput::with_status(exit_code, lines.join("\n"))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn format_verify(
    envelope: &RunEnvelope,
    seed: u64,
    budget: usize,
    evidence_id: Option<&str>,
    counterexample_blob_digest: Option<&ContentHash>,
    exit_code: u8,
    json: bool,
) -> CliOutput {
    let outcome_text: String = outcome_label(&envelope.outcome);
    if json {
        let report: VerifyReport<'_> = VerifyReport {
            snapshot_id: &envelope.snapshot_id,
            property_id: &envelope.property_id,
            claim_ids: &envelope.claim_ids,
            run_id: &envelope.run_id,
            seed,
            budget,
            run_state: envelope.run_state,
            outcome: outcome_text,
            raw_status: &envelope.raw_status,
            evidence_id,
            counterexample_blob_digest,
            original_counterexample: envelope.original_counterexample.as_ref(),
            minimized_counterexample: envelope.minimized_counterexample.as_ref(),
        };
        return match encode_json(&report, true) {
            Ok(encoded) => CliOutput::with_status(exit_code, encoded),
            Err(output) => output,
        };
    }
    let mut lines: Vec<String> = vec![
        format!("snapshot_id: {}", envelope.snapshot_id),
        format!("property: {}", envelope.property_id),
        format!("outcome: {outcome_text}"),
        format!("status: {}", envelope.raw_status),
    ];
    if let Some(ev_id) = evidence_id {
        lines.push(format!("evidence_id: {ev_id}"));
    }
    if let Some(digest) = counterexample_blob_digest {
        lines.push(format!("counterexample_blob: {digest}"));
    }
    if let Some(ce) = envelope.minimized_counterexample.as_ref().or(envelope.original_counterexample.as_ref()) {
        lines.push(format!("counterexample_events: {}", ce.replay.events.join(", ")));
    }
    CliOutput::with_status(exit_code, lines.join("\n"))
}

pub(crate) fn format_ledger_verify(record_count: usize, link_count: usize, blob_count: usize, json: bool) -> CliOutput {
    if json {
        let report: LedgerVerifyReport<'_> = LedgerVerifyReport {
            status: "valid",
            records: record_count,
            links: link_count,
            blobs: blob_count,
        };
        return match encode_json(&report, true) {
            Ok(encoded) => CliOutput::with_status(EXIT_OK, encoded),
            Err(output) => output,
        };
    }
    CliOutput::ok(format!("ledger: valid\nrecords: {record_count}\nlinks: {link_count}\nblobs: {blob_count}"))
}

pub(crate) fn format_ledger_list(records: &[EvidenceRecord], links: &[EvidenceLink], json: bool) -> CliOutput {
    if json {
        let report: LedgerListReport<'_> = LedgerListReport {
            records: records
                .iter()
                .map(|record: &EvidenceRecord| LedgerRecordSummary {
                    id: &record.id,
                    kind: record.kind,
                    method: record.method,
                    property_id: record.property_id.as_ref(),
                    claim_ids: &record.claim_ids,
                    run_state: record.run_state,
                    outcome: outcome_label(&record.outcome),
                    snapshot_id: &record.snapshot_id,
                    content_digest: &record.content_digest,
                })
                .collect(),
            links: links
                .iter()
                .map(|link: &EvidenceLink| LedgerLinkSummary {
                    from_id: &link.from_id,
                    to_id: &link.to_id,
                    relation: link.relation,
                    reason: &link.reason,
                    content_digest: &link.content_digest,
                })
                .collect(),
        };
        return match encode_json(&report, true) {
            Ok(encoded) => CliOutput::ok(encoded),
            Err(output) => output,
        };
    }
    let mut lines: Vec<String> = vec![format!("records: {}", records.len())];
    for record in records {
        let claim_list: String = record.claim_ids.iter().map(ArtifactId::as_str).collect::<Vec<&str>>().join(",");
        lines.push(format!(
            "  {} | {} | {} | {} | claims=[{}]",
            record.id,
            kind_label(record.kind),
            method_label(record.method),
            outcome_label(&record.outcome),
            claim_list
        ));
    }
    lines.push(format!("links: {}", links.len()));
    for link in links {
        lines.push(format!("  {} --{}--> {}", link.from_id, relation_label(link.relation), link.to_id));
    }
    CliOutput::ok(lines.join("\n"))
}

pub(crate) fn format_impact(previous: &GraphSnapshot, current: &GraphSnapshot, impact: &ImpactSet, clean: bool, exit_code: u8, json: bool) -> CliOutput {
    if json {
        let report: ImpactReport<'_> = ImpactReport {
            from_snapshot_id: &previous.snapshot_id,
            to_snapshot_id: &current.snapshot_id,
            clean,
            changed_artifacts: &impact.changed_artifacts,
            stale_evidence_ids: &impact.stale_evidence_ids,
            affected_claim_ids: &impact.affected_claim_ids,
        };
        return match encode_json(&report, true) {
            Ok(encoded) => CliOutput::with_status(exit_code, encoded),
            Err(output) => output,
        };
    }
    let changed_text: String = if impact.changed_artifacts.is_empty() {
        "none".to_string()
    } else {
        impact.changed_artifacts.join(", ")
    };
    let stale_text: String = if impact.stale_evidence_ids.is_empty() {
        "none".to_string()
    } else {
        impact.stale_evidence_ids.join(", ")
    };
    let affected_text: String = if impact.affected_claim_ids.is_empty() {
        "none".to_string()
    } else {
        impact.affected_claim_ids.iter().map(ArtifactId::as_str).collect::<Vec<&str>>().join(", ")
    };
    let text: String = format!(
        "from_snapshot_id: {}\nto_snapshot_id: {}\nchanged_artifacts: {changed_text}\nstale_evidence_ids: {stale_text}\naffected_claim_ids: {affected_text}",
        previous.snapshot_id, current.snapshot_id
    );
    CliOutput::with_status(exit_code, text)
}

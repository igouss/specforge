use std::collections::BTreeMap;

use cucumber::{given, then, when};
use specforge::argument::{build_assurance_argument, export_argument_json, export_argument_markdown, ArgumentDefeater, AssuranceArgument};
use specforge::dst::{normalize_dst_report, DeterminismGuarantee, DstReplayMetadata, DstRunReport};
use specforge::formal::{normalize_formal_report, FormalArtifactRef, FormalBounds, FormalCheckReport, FormalPropertyRef};
use specforge::graph::{ArtifactId, ContentHash, GraphSnapshot, VerificationMethod};
use specforge::ledger::{EvidenceKind, EvidenceLink, EvidencePayload, EvidenceRecord, EvidenceRelation, ProducerMeta};
use specforge::verify::{EvaluationResult, ExecutionTrace, FormalOutcome, RunEnvelope, RunState, TraceEvent, VerificationOutcome, VerificationRequest};

use crate::graph::current_snapshot;
use crate::{listed, Forge};

pub(super) fn append_formal_evidence(world: &mut Forge, id: &str, property_id: &str, claim_id: &str, outcome: FormalOutcome, bounds: Option<FormalBounds>) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let prop_id: ArtifactId = ArtifactId::parse(property_id).expect("valid property id");
    let clm_id: ArtifactId = ArtifactId::parse(claim_id).expect("valid claim id");
    let request: VerificationRequest = VerificationRequest {
        run_id: format!("RUN-{id}"),
        activity_id: "ACT-ARG-FORMAL".to_string(),
        property_id: prop_id.clone(),
        claim_ids: vec![clm_id],
        budget: 1,
        seed: None,
    };
    let report: FormalCheckReport = FormalCheckReport {
        adapter_id: "arg-formal-adapter".to_string(),
        adapter_version: "0.3.0".to_string(),
        engine_name: "tlc".to_string(),
        engine_version: "2.18.0".to_string(),
        property_ref: FormalPropertyRef {
            property_id: prop_id,
            external_tool: "TLC".to_string(),
            module: "BrokerSafety".to_string(),
            symbol: format!("Spec_{property_id}"),
            digest: snapshot.artifact_digest(property_id).cloned(),
        },
        outcome,
        bounds,
        assumptions: vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption")],
        artifacts: vec![FormalArtifactRef {
            name: "proof-script".to_string(),
            kind: "tla_module".to_string(),
            digest: ContentHash::of_bytes(format!("proof:{id}:{property_id}").as_bytes()),
            uri: Some(format!("formal/{property_id}.tla")),
        }],
        counterexample_trace: None,
        raw_status: "formal_completed".to_string(),
        limitations: Vec::new(),
    };
    let (envelope, digests): (RunEnvelope, BTreeMap<String, ContentHash>) =
        normalize_formal_report(snapshot, &request, &report).expect("formal report normalizes");
    let record: EvidenceRecord = EvidenceRecord::from_envelope(
        id.to_string(),
        EvidenceKind::FormalCheck,
        "2026-11-24T08:10:00Z".to_string(),
        snapshot,
        &envelope,
        digests,
    )
    .expect("formal evidence seals");
    world.evidence_ledger.append_record(record).expect("formal evidence appends");
}

#[allow(clippy::too_many_arguments)]
fn seal_custom_record(
    snapshot: &GraphSnapshot,
    id: &str,
    property_id: &str,
    claim_id: &str,
    scope: &str,
    run_state: RunState,
    outcome: VerificationOutcome,
) -> EvidenceRecord {
    let payload: EvidencePayload = EvidencePayload {
        id: id.to_string(),
        kind: EvidenceKind::FormalCheck,
        producer: ProducerMeta {
            name: "tlc".to_string(),
            version: "2.18.0".to_string(),
        },
        adapter: ProducerMeta {
            name: "arg-formal-adapter".to_string(),
            version: "0.3.0".to_string(),
        },
        timestamp: "2026-11-24T08:12:00Z".to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        source_revisions: snapshot.sources.clone(),
        dependencies: snapshot.dependency_pins_for_claim(claim_id),
        run_id: format!("RUN-{id}"),
        activity_id: "ACT-ARG-CUSTOM".to_string(),
        method: VerificationMethod::Formal,
        property_id: Some(ArtifactId::parse(property_id).expect("valid property id")),
        claim_ids: vec![ArtifactId::parse(claim_id).expect("valid claim id")],
        scope: scope.to_string(),
        assumptions: vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption")],
        run_state,
        outcome,
        counterexample_id: None,
        replay: None,
        artifact_digests: BTreeMap::new(),
        limitations: Vec::new(),
    };
    EvidenceRecord::seal(payload).expect("custom record seals")
}

#[given(expr = "formal proved evidence exists for {string} and claim {string}")]
fn given_formal_proved(world: &mut Forge, property_id: String, claim_id: String) {
    world.external_assurance.evidence_counter += 1;
    let id: String = format!("EV-PROVED-{}", world.external_assurance.evidence_counter);
    append_formal_evidence(world, &id, &property_id, &claim_id, FormalOutcome::Proved, None);
}

#[given(expr = "bounded formal evidence {string} exists for {string} and claim {string}")]
fn given_formal_bounded(world: &mut Forge, bound: String, property_id: String, claim_id: String) {
    world.external_assurance.evidence_counter += 1;
    let id: String = format!("EV-BOUNDED-{}", world.external_assurance.evidence_counter);
    let bounds: FormalBounds = FormalBounds {
        max_depth: Some(12),
        max_states: Some(10000),
        timeout_seconds: Some(30),
        description: format!("bounded check {bound}"),
    };
    append_formal_evidence(world, &id, &property_id, &claim_id, FormalOutcome::BoundedSuccess { bound }, Some(bounds));
}

#[given(expr = "refuted formal evidence exists for {string} and claim {string}")]
fn given_formal_refuted(world: &mut Forge, property_id: String, claim_id: String) {
    world.external_assurance.evidence_counter += 1;
    let id: String = format!("EV-REFUTED-{}", world.external_assurance.evidence_counter);
    append_formal_evidence(world, &id, &property_id, &claim_id, FormalOutcome::Refuted, None);
}

#[given(expr = "disqualified wrong-scope formal evidence exists for {string} and claim {string}")]
fn given_disqualified_scope(world: &mut Forge, property_id: String, claim_id: String) {
    let record: EvidenceRecord = seal_custom_record(
        current_snapshot(world),
        "EV-DISQ-1",
        &property_id,
        &claim_id,
        "multi-broker",
        RunState::Completed,
        VerificationOutcome::Formal(FormalOutcome::Proved),
    );
    world.evidence_ledger.append_record(record).expect("disqualified record appends");
}

#[given(expr = "claim-contradicting evidence link exists for {string} and claim {string}")]
fn given_claim_contradicting_link(world: &mut Forge, property_id: String, claim_id: String) {
    let record: EvidenceRecord = seal_custom_record(
        current_snapshot(world),
        "EV-LINK-CONTRA",
        &property_id,
        &claim_id,
        "single-broker",
        RunState::Completed,
        VerificationOutcome::Formal(FormalOutcome::Proved),
    );
    world.evidence_ledger.append_record(record).expect("contradicting record appends");
    let link: EvidenceLink = EvidenceLink::seal(
        "EV-LINK-CONTRA".to_string(),
        claim_id,
        EvidenceRelation::Contradicts,
        "external review contradicted claim".to_string(),
    )
    .expect("contradicts link seals");
    world.evidence_ledger.append_link(link).expect("contradicts link appends");
}

#[given(expr = "invalid-evaluation evidence exists for {string} and claim {string}")]
fn given_invalid_evaluation(world: &mut Forge, property_id: String, claim_id: String) {
    let record: EvidenceRecord = seal_custom_record(
        current_snapshot(world),
        "EV-INVALID-1",
        &property_id,
        &claim_id,
        "single-broker",
        RunState::Failed,
        VerificationOutcome::Evaluation(EvaluationResult::InvalidEvaluation),
    );
    world.evidence_ledger.append_record(record).expect("invalid record appends");
}

#[when(expr = "contradictory DST violation evidence is also appended for {string} and claim {string}")]
fn append_contradictory_dst(world: &mut Forge, property_id: String, claim_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let request: VerificationRequest = VerificationRequest {
        run_id: "RUN-DST-CONTRA".to_string(),
        activity_id: "ACT-DST-CONTRA".to_string(),
        property_id: ArtifactId::parse(&property_id).expect("valid property id"),
        claim_ids: vec![ArtifactId::parse(&claim_id).expect("valid claim id")],
        budget: 4,
        seed: Some(99),
    };
    let report: DstRunReport = DstRunReport {
        adapter_id: "arg-dst-adapter".to_string(),
        adapter_version: "0.2.0".to_string(),
        engine_name: "sim-kernel".to_string(),
        engine_version: "1.4.0".to_string(),
        run_state: RunState::Completed,
        trace: ExecutionTrace {
            initial_state: "queued".to_string(),
            events: vec![
                TraceEvent {
                    step: 1,
                    event: "deliver".to_string(),
                    from_state: "queued".to_string(),
                    to_state: "delivered".to_string(),
                    inputs: BTreeMap::new(),
                    fault: None,
                    virtual_time: Some(1),
                },
                TraceEvent {
                    step: 2,
                    event: "ack".to_string(),
                    from_state: "delivered".to_string(),
                    to_state: "acknowledged".to_string(),
                    inputs: BTreeMap::new(),
                    fault: None,
                    virtual_time: Some(2),
                },
                TraceEvent {
                    step: 3,
                    event: "redeliver".to_string(),
                    from_state: "acknowledged".to_string(),
                    to_state: "redelivered".to_string(),
                    inputs: BTreeMap::new(),
                    fault: Some("crash_restart".to_string()),
                    virtual_time: Some(3),
                },
            ],
        },
        replay_metadata: DstReplayMetadata {
            seed: Some(99),
            scheduler_decisions: Vec::new(),
            fault_injections: Vec::new(),
            runtime_config: BTreeMap::new(),
            determinism: DeterminismGuarantee {
                deterministic_rng: true,
                deterministic_scheduler: true,
                virtual_clock: true,
                deterministic_faults: true,
                notes: Vec::new(),
            },
        },
        raw_status: "dst:violation".to_string(),
        limitations: Vec::new(),
    };
    let (envelope, digest): (RunEnvelope, ContentHash) = normalize_dst_report(snapshot, &request, &report).expect("dst normalizes");
    let mut artifacts: BTreeMap<String, ContentHash> = BTreeMap::new();
    artifacts.insert("dst_replay_metadata".to_string(), digest);
    let record: EvidenceRecord = EvidenceRecord::from_envelope(
        "EV-DST-CONTRA".to_string(),
        EvidenceKind::PropertyEvaluation,
        "2026-11-24T08:11:00Z".to_string(),
        snapshot,
        &envelope,
        artifacts,
    )
    .expect("dst evidence seals");
    world.evidence_ledger.append_record(record).expect("dst evidence appends");
}

#[when(expr = "an assurance argument is built for claim {string} with subclaims {string}")]
fn build_argument_step(world: &mut Forge, claim_id: String, subclaims_raw: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let subclaims: Vec<String> = if subclaims_raw.trim().is_empty() {
        Vec::new()
    } else {
        listed(&subclaims_raw, ", ")
    };
    world.external_assurance.argument_result = Some(build_assurance_argument(snapshot, &world.evidence_ledger, &claim_id, &subclaims));
}

pub(super) fn built_argument(world: &Forge) -> &AssuranceArgument {
    world
        .external_assurance
        .argument_result
        .as_ref()
        .expect("argument built")
        .as_ref()
        .expect("argument succeeded")
}

#[then(expr = "the assurance argument status is {string} and unbounded proof is {word}")]
fn argument_status_and_unbounded(world: &mut Forge, expected_status: String, expected_unbounded: String) {
    let argument: &AssuranceArgument = built_argument(world);
    let expected_bool: bool = match expected_unbounded.as_str() {
        "true" => true,
        "false" => false,
        other => panic!("expected true or false, got {other}"),
    };
    assert_eq!(argument.status.to_string(), expected_status);
    assert_eq!(argument.unbounded_proof, expected_bool);
}

#[then(expr = "the assurance argument lists assumption {string} and scope {string}")]
fn argument_assumption_and_scope(world: &mut Forge, assumption: String, scope: String) {
    let argument: &AssuranceArgument = built_argument(world);
    assert_eq!(argument.scope, scope);
    assert!(
        argument.assumptions.iter().any(|item: &ArtifactId| item.as_str() == assumption),
        "missing assumption {assumption} in {:?}",
        argument.assumptions
    );
}

#[then(expr = "the assurance argument includes {int} evidence item and {int} subclaim")]
fn argument_counts(world: &mut Forge, evidence_count: usize, subclaim_count: usize) {
    let argument: &AssuranceArgument = built_argument(world);
    assert_eq!(argument.evidence.len(), evidence_count);
    assert_eq!(argument.subclaims.len(), subclaim_count);
}

#[then(expr = "the assurance argument JSON and Markdown exports are deterministic and disclose {string}")]
fn argument_exports_deterministic(world: &mut Forge, needle: String) {
    let argument: &AssuranceArgument = built_argument(world);
    let json_one: String = export_argument_json(argument).expect("json export succeeds");
    let json_two: String = export_argument_json(argument).expect("second json export succeeds");
    assert_eq!(json_one, json_two);
    assert!(json_one.contains(&needle), "JSON export missing {needle}: {json_one}");
    let roundtrip: AssuranceArgument = serde_json::from_str(&json_one).expect("argument json deserializes");
    assert_eq!(*argument, roundtrip);

    let md_one: String = export_argument_markdown(argument);
    let md_two: String = export_argument_markdown(argument);
    assert_eq!(md_one, md_two);
    assert!(md_one.contains(&needle), "Markdown export missing {needle}: {md_one}");
}

#[then(expr = "the exported Markdown includes indented subclaim rationale for {string}")]
fn exported_markdown_subclaim_rationale(world: &mut Forge, subclaim_id: String) {
    let argument: &AssuranceArgument = built_argument(world);
    let sub: &specforge::argument::SubclaimArgument = argument
        .subclaims
        .iter()
        .find(|item: &&specforge::argument::SubclaimArgument| item.claim_id.as_str() == subclaim_id)
        .expect("subclaim exists");
    assert!(!sub.rationale.is_empty(), "subclaim rationale must be non-empty");
    let markdown: String = export_argument_markdown(argument);
    assert!(markdown.contains("- Evidence:\n  - "), "subclaim evidence not indented:\n{markdown}");
    assert!(markdown.contains("- Defeaters:\n  - "), "subclaim defeaters not indented:\n{markdown}");
    let expected_rationale_line: String = format!("- Rationale:\n  - {}", sub.rationale[0]);
    assert!(
        markdown.contains(&expected_rationale_line),
        "expected indented subclaim rationale \"{expected_rationale_line}\" in:\n{markdown}"
    );
}

#[then(expr = "the assurance argument discloses a defeater of kind {string} naming {string}")]
fn argument_discloses_defeater(world: &mut Forge, kind: String, needle: String) {
    let argument: &AssuranceArgument = built_argument(world);
    assert!(
        argument
            .defeaters
            .iter()
            .any(|defeater: &ArgumentDefeater| defeater.kind == kind && (defeater.source_id.contains(&needle) || defeater.detail.contains(&needle))),
        "expected defeater kind={kind} naming={needle} in {:?}",
        argument.defeaters
    );
}

#[then(expr = "the exported Markdown includes {string} and {string}")]
fn exported_markdown_includes(world: &mut Forge, first: String, second: String) {
    let argument: &AssuranceArgument = built_argument(world);
    let markdown: String = export_argument_markdown(argument);
    assert!(markdown.contains(&first), "Markdown missing {first}: {markdown}");
    assert!(markdown.contains(&second), "Markdown missing {second}: {markdown}");
}

#[then(expr = "assurance argument construction fails with an error mentioning {string}")]
fn argument_construction_fails(world: &mut Forge, needle: String) {
    let result: &Result<AssuranceArgument, String> = world.external_assurance.argument_result.as_ref().expect("argument attempted");
    match result {
        Err(message) => assert!(message.contains(&needle), "expected error mentioning {needle}, got {message}"),
        Ok(argument) => panic!("expected error mentioning {needle}, got {argument:?}"),
    }
}

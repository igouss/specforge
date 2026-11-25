use std::collections::BTreeMap;

use cucumber::{then, when};
use serde_json::Value;
use specforge::graph::{ContentHash, GraphSnapshot, VerificationMethod};
use specforge::ledger::{EvidenceLedger, EvidenceLink, EvidencePayload, EvidenceRecord, EvidenceRelation, LedgerError};
use specforge::verify::{EvaluationResult, ReplayDescriptor, RunState, VerificationOutcome};

use super::{build_payload_with_scope, parse_relation};
use crate::graph::current_snapshot;
use crate::Forge;

fn apply_invalid_field(payload: &mut EvidencePayload, defect: &str) {
    match defect {
        "empty_id" => payload.id = "   ".to_string(),
        "empty_producer" => payload.producer.name = "  ".to_string(),
        "empty_adapter" => payload.adapter.name = String::new(),
        "empty_timestamp" => payload.timestamp = " ".to_string(),
        "empty_run_id" => payload.run_id = String::new(),
        "empty_scope" => payload.scope = " \t ".to_string(),
        "positive_failed_run" => {
            payload.run_state = RunState::Failed;
            payload.outcome = VerificationOutcome::Evaluation(EvaluationResult::Satisfied);
        }
        "replay_empty_engine" => {
            payload.replay = Some(ReplayDescriptor {
                engine: " ".to_string(),
                engine_version: "0.1.0".to_string(),
                seed: Some(7),
                replay_token: None,
                events: Vec::new(),
                determinism: "seeded".to_string(),
                replayable: true,
            });
        }
        "replay_no_determinism" => {
            payload.replay = Some(ReplayDescriptor {
                engine: "sim-engine".to_string(),
                engine_version: "0.1.0".to_string(),
                seed: None,
                replay_token: None,
                events: Vec::new(),
                determinism: "seeded".to_string(),
                replayable: true,
            });
        }
        other => panic!("unknown invalid field defect {other}"),
    }
}

fn forge_valid_digest_record(snapshot: &GraphSnapshot, id: &str, defect: &str) -> EvidenceRecord {
    let mut payload: EvidencePayload = build_payload_with_scope(
        snapshot,
        id,
        "PROP-MSG-001",
        "CLM-MSG-001",
        "single-broker",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    apply_invalid_field(&mut payload, defect);
    let content_digest: ContentHash = ContentHash::of_canonical_json(&payload).expect("canonical json hashes");
    let record: EvidenceRecord = EvidenceRecord {
        id: payload.id,
        kind: payload.kind,
        producer: payload.producer,
        adapter: payload.adapter,
        timestamp: payload.timestamp,
        snapshot_id: payload.snapshot_id,
        source_revisions: payload.source_revisions,
        dependencies: payload.dependencies,
        run_id: payload.run_id,
        activity_id: payload.activity_id,
        method: payload.method,
        property_id: payload.property_id,
        claim_ids: payload.claim_ids,
        scope: payload.scope,
        assumptions: payload.assumptions,
        run_state: payload.run_state,
        outcome: payload.outcome,
        counterexample_id: payload.counterexample_id,
        replay: payload.replay,
        artifact_digests: payload.artifact_digests,
        limitations: payload.limitations,
        content_digest,
    };
    assert!(record.verify_digest(), "forged record must carry a valid content digest");
    record
}

#[then(regex = r"^re-importing the exported ledger JSON preserves (\d+) records? and (\d+) links?$")]
fn reimport_exported_json(world: &mut Forge, expected_records: usize, expected_links: usize) {
    let exported: String = world
        .evidence_state
        .last_exported_json
        .clone()
        .unwrap_or_else(|| world.evidence_ledger.export_json().expect("export succeeds"));
    let reimported: EvidenceLedger = serde_json::from_str::<EvidenceLedger>(&exported).expect("re-import succeeds");
    assert_eq!(reimported, world.evidence_ledger);
    assert_eq!(reimported.records().len(), expected_records);
    assert_eq!(reimported.links().len(), expected_links);
}

#[when(expr = "an evidence payload with invalid field {string} is sealed")]
fn seal_invalid_payload(world: &mut Forge, defect: String) {
    let mut payload: EvidencePayload = build_payload_with_scope(
        current_snapshot(world),
        "EV-BAD-001",
        "PROP-MSG-001",
        "CLM-MSG-001",
        "single-broker",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    apply_invalid_field(&mut payload, &defect);
    world.evidence_state.last_record_seal = Some(EvidenceRecord::seal(payload));
}

#[then(expr = "sealing is refused with error mentioning {string}")]
fn sealing_refused_mentioning(world: &mut Forge, needle: String) {
    let error: &LedgerError = world
        .evidence_state
        .last_record_seal
        .as_ref()
        .expect("seal attempted")
        .as_ref()
        .expect_err("seal must fail");
    let rendered: String = error.to_string();
    assert!(rendered.contains(&needle), "expected {needle:?} in {rendered:?}");
}

#[when(expr = "a forged valid-digest evidence record {string} with invalid field {string} is appended")]
fn append_forged_invalid_record(world: &mut Forge, id: String, defect: String) {
    let record: EvidenceRecord = forge_valid_digest_record(current_snapshot(world), &id, &defect);
    world.last_append = Some(world.evidence_ledger.append_record(record));
}

#[then(expr = "the append is refused with error mentioning {string}")]
fn append_refused_mentioning(world: &mut Forge, needle: String) {
    let error: &LedgerError = world.last_append.as_ref().expect("append attempted").as_ref().expect_err("append must fail");
    let rendered: String = error.to_string();
    assert!(rendered.contains(&needle), "expected {needle:?} in {rendered:?}");
}

#[when(expr = "a link {string} is sealed from {string} to {string}")]
fn seal_link_attempt(world: &mut Forge, relation: String, from_id: String, to_id: String) {
    let relation: EvidenceRelation = parse_relation(&relation);
    world.evidence_state.last_link_seal = Some(EvidenceLink::seal(from_id, to_id, relation, "link test".to_string()));
}

#[then(expr = "link sealing is refused with error mentioning {string}")]
fn link_sealing_refused_mentioning(world: &mut Forge, needle: String) {
    let error: &LedgerError = world
        .evidence_state
        .last_link_seal
        .as_ref()
        .expect("link seal attempted")
        .as_ref()
        .expect_err("link seal must fail");
    assert!(matches!(error, LedgerError::InvalidLink { .. }), "{error:?}");
    let rendered: String = error.to_string();
    assert!(rendered.contains(&needle), "expected {needle:?} in {rendered:?}");
}

#[when(expr = "a tampered link {string} from {string} to {string} is appended")]
fn append_tampered_link(world: &mut Forge, relation: String, from_id: String, to_id: String) {
    let relation: EvidenceRelation = parse_relation(&relation);
    let mut link: EvidenceLink = EvidenceLink::seal(from_id, to_id, relation, "original reason".to_string()).expect("link seals");
    link.reason = "tampered reason".to_string();
    world.last_append = Some(world.evidence_ledger.append_link(link));
}

#[then(expr = "the append error message mentions {string}")]
fn append_error_mentions(world: &mut Forge, needle: String) {
    let error: &LedgerError = world.last_append.as_ref().expect("append attempted").as_ref().expect_err("append must fail");
    let rendered: String = error.to_string();
    assert!(rendered.contains(&needle), "expected {needle:?} in {rendered:?}");
}

#[when(expr = "an attempted link {string} is appended from {string} to {string}")]
fn append_attempted_link(world: &mut Forge, relation: String, from_id: String, to_id: String) {
    let relation: EvidenceRelation = parse_relation(&relation);
    let link: EvidenceLink = EvidenceLink::seal(from_id, to_id, relation, "attempted link".to_string()).expect("link seals");
    world.last_append = Some(world.evidence_ledger.append_link(link));
}

#[then(expr = "the append is refused as missing record mentioning {string}")]
fn append_refused_missing_record(world: &mut Forge, needle: String) {
    let error: &LedgerError = world.last_append.as_ref().expect("append attempted").as_ref().expect_err("append must fail");
    assert!(matches!(error, LedgerError::MissingRecord(_)), "{error:?}");
    let rendered: String = error.to_string();
    assert!(rendered.contains(&needle), "expected {needle:?} in {rendered:?}");
}

#[when(expr = "the exported ledger JSON is mutated with {string} and imported")]
fn mutate_and_import_json(world: &mut Forge, mutation: String) {
    let exported: String = world
        .evidence_state
        .last_exported_json
        .clone()
        .unwrap_or_else(|| world.evidence_ledger.export_json().expect("export succeeds"));
    let mut value: Value = serde_json::from_str::<Value>(&exported).expect("exported json parses");
    match mutation.as_str() {
        "tampered_record" => {
            let records: &mut Vec<Value> = value.get_mut("records").and_then(Value::as_array_mut).expect("records array");
            records[0]["scope"] = Value::String("tampered-scope".to_string());
        }
        "tampered_link" => {
            let links: &mut Vec<Value> = value.get_mut("links").and_then(Value::as_array_mut).expect("links array");
            let target: &mut Value = links
                .iter_mut()
                .find(|entry: &&mut Value| entry.get("from_id").and_then(Value::as_str) == Some("EV-002"))
                .expect("EV-002 link exists");
            target["reason"] = Value::String("tampered link reason".to_string());
        }
        "missing_link_target" => {
            let link: EvidenceLink = EvidenceLink::seal(
                "EV-002".to_string(),
                "EV-GHOST".to_string(),
                EvidenceRelation::Supersedes,
                "dangling target".to_string(),
            )
            .expect("dangling link seals");
            let links: &mut Vec<Value> = value.get_mut("links").and_then(Value::as_array_mut).expect("links array");
            links.push(serde_json::to_value(link).expect("link serializes"));
        }
        "missing_link_source" => {
            let link: EvidenceLink = EvidenceLink::seal(
                "EV-PHANTOM".to_string(),
                "EV-001".to_string(),
                EvidenceRelation::Supports,
                "dangling source".to_string(),
            )
            .expect("dangling source link seals");
            let links: &mut Vec<Value> = value.get_mut("links").and_then(Value::as_array_mut).expect("links array");
            links.push(serde_json::to_value(link).expect("link serializes"));
        }
        "forged_invalid_record" => {
            let forged: EvidenceRecord = forge_valid_digest_record(current_snapshot(world), "EV-FORGED", "positive_failed_run");
            let records: &mut Vec<Value> = value.get_mut("records").and_then(Value::as_array_mut).expect("records array");
            records.push(serde_json::to_value(forged).expect("forged record serializes"));
        }
        other => panic!("unknown json mutation {other}"),
    }
    let mutated: String = serde_json::to_string(&value).expect("mutated json encodes");
    let imported: Result<EvidenceLedger, String> = serde_json::from_str::<EvidenceLedger>(&mutated).map_err(|error: serde_json::Error| error.to_string());
    world.evidence_state.last_json_import = Some(imported);
}

#[then(expr = "ledger JSON import fails with error mentioning {string}")]
fn json_import_fails_mentioning(world: &mut Forge, needle: String) {
    let error: &String = world
        .evidence_state
        .last_json_import
        .as_ref()
        .expect("import attempted")
        .as_ref()
        .expect_err("import must fail");
    assert!(error.contains(&needle), "expected {needle:?} in {error:?}");
}

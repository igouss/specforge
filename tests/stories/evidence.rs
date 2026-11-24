use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use cucumber::{given, then, when};
use specforge::assurance::{assess_claim, impact_set, ClaimAssessment, EvidenceEvaluationEntry, ImpactSet};
use specforge::graph::{ArtifactId, ContentHash, DependencyPin, GraphSnapshot, VerificationClaim, VerificationMethod};
use specforge::ledger::{EvidenceKind, EvidenceLedger, EvidenceLink, EvidencePayload, EvidenceRecord, EvidenceRelation, LedgerError, ProducerMeta};
use specforge::ledger_fs::FsLedger;
use specforge::verify::{EvaluationResult, FormalOutcome, RunState, VerificationOutcome};

use crate::graph::{compile_fixture, current_snapshot};
use crate::Forge;

static LEDGER_DIRS: AtomicUsize = AtomicUsize::new(0);

#[allow(clippy::too_many_arguments)]
fn build_record_with_scope(
    snapshot: &GraphSnapshot,
    id: &str,
    property_id: &str,
    claim_id: &str,
    scope: &str,
    method: VerificationMethod,
    outcome: VerificationOutcome,
    artifact_digests: BTreeMap<String, ContentHash>,
) -> EvidenceRecord {
    let prop_id: ArtifactId = ArtifactId::parse(property_id).expect("valid property id");
    let clm_id: ArtifactId = ArtifactId::parse(claim_id).expect("valid claim id");
    let dependencies: Vec<DependencyPin> = snapshot.dependency_pins_for_claim(claim_id);
    let payload: EvidencePayload = EvidencePayload {
        id: id.to_string(),
        kind: EvidenceKind::PropertyEvaluation,
        producer: ProducerMeta {
            name: "specforge-test-engine".to_string(),
            version: "0.1.0".to_string(),
        },
        adapter: ProducerMeta {
            name: "specforge-test-adapter".to_string(),
            version: "0.1.0".to_string(),
        },
        timestamp: "2026-11-23T20:00:00Z".to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        source_revisions: snapshot.sources.clone(),
        dependencies,
        run_id: format!("RUN-{id}"),
        activity_id: "ACT-TEST-001".to_string(),
        method,
        property_id: Some(prop_id),
        claim_ids: vec![clm_id],
        scope: scope.to_string(),
        assumptions: vec![ArtifactId::parse("ASM-MSG-001").expect("valid assumption id")],
        run_state: RunState::Completed,
        outcome,
        counterexample_id: None,
        replay: None,
        artifact_digests,
        limitations: vec!["finite test run".to_string()],
    };
    EvidenceRecord::seal(payload).expect("evidence record seals")
}

fn build_record(
    snapshot: &GraphSnapshot,
    id: &str,
    property_id: &str,
    claim_id: &str,
    method: VerificationMethod,
    outcome: VerificationOutcome,
    artifact_digests: BTreeMap<String, ContentHash>,
) -> EvidenceRecord {
    build_record_with_scope(snapshot, id, property_id, claim_id, "single-broker", method, outcome, artifact_digests)
}

#[given("an empty evidence ledger")]
fn empty_ledger(world: &mut Forge) {
    world.evidence_ledger = EvidenceLedger::new();
}

#[given(expr = "a satisfied evidence record {string} is appended for property {string} and claim {string}")]
#[when(expr = "a satisfied evidence record {string} is appended for property {string} and claim {string}")]
fn append_satisfied(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    let record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a satisfied evidence record {string} in scope {string} is appended for property {string} and claim {string}")]
fn append_satisfied_in_scope(world: &mut Forge, id: String, scope: String, property_id: String, claim_id: String) {
    let record: EvidenceRecord = build_record_with_scope(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        &scope,
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a violated evidence record {string} is appended for property {string} and claim {string}")]
#[when(expr = "a violated evidence record {string} is appended for property {string} and claim {string}")]
fn append_violated(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    let record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Violated),
        BTreeMap::new(),
    );
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "an inconclusive evidence record {string} is appended for property {string} and claim {string}")]
fn append_inconclusive(world: &mut Forge, id: String, property_id: String, claim_id: String) {
    let record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Inconclusive),
        BTreeMap::new(),
    );
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[given(expr = "a bounded-success formal evidence record {string} with bound {string} is appended for property {string} and claim {string}")]
fn append_bounded_formal(world: &mut Forge, id: String, bound: String, property_id: String, claim_id: String) {
    let record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        &property_id,
        &claim_id,
        VerificationMethod::Formal,
        VerificationOutcome::Formal(FormalOutcome::BoundedSuccess { bound }),
        BTreeMap::new(),
    );
    world.evidence_ledger.append_record(record).expect("append succeeds");
}

#[when(expr = "a duplicate evidence record {string} is appended")]
fn append_duplicate(world: &mut Forge, id: String) {
    let record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        "PROP-MSG-001",
        "CLM-MSG-001",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    world.last_append = Some(world.evidence_ledger.append_record(record));
}

#[when(expr = "a tampered evidence record {string} is appended")]
fn append_tampered(world: &mut Forge, id: String) {
    let mut record: EvidenceRecord = build_record(
        current_snapshot(world),
        &id,
        "PROP-MSG-001",
        "CLM-MSG-001",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        BTreeMap::new(),
    );
    record.scope = "tampered-scope".to_string();
    world.last_append = Some(world.evidence_ledger.append_record(record));
}

#[then("the append is refused as duplicate")]
fn refused_duplicate(world: &mut Forge) {
    assert!(
        matches!(world.last_append, Some(Err(LedgerError::DuplicateRecord(_)))),
        "{:?}",
        world.last_append
    );
}

#[then("the append is refused as invalid digest")]
fn refused_invalid_digest(world: &mut Forge) {
    assert!(matches!(world.last_append, Some(Err(LedgerError::InvalidDigest(_)))), "{:?}", world.last_append);
}

#[when(expr = "a link {string} is appended from {string} to {string}")]
fn append_link(world: &mut Forge, relation: String, from_id: String, to_id: String) {
    let relation: EvidenceRelation = match relation.as_str() {
        "supports" => EvidenceRelation::Supports,
        "contradicts" => EvidenceRelation::Contradicts,
        "corroborates" => EvidenceRelation::Corroborates,
        "derived_from" => EvidenceRelation::DerivedFrom,
        "supersedes" => EvidenceRelation::Supersedes,
        "invalidates" => EvidenceRelation::Invalidates,
        other => panic!("unknown relation {other}"),
    };
    let link: EvidenceLink = EvidenceLink::seal(from_id, to_id, relation, "recorded in story".to_string()).expect("link seals");
    world.evidence_ledger.append_link(link).expect("link appends");
}

#[then(regex = r"^the ledger(?: still)? holds (\d+) evidence records?$")]
fn ledger_count(world: &mut Forge, expected: usize) {
    assert_eq!(world.evidence_ledger.records().len(), expected);
}

#[then("ledger integrity verification passes")]
fn ledger_integrity(world: &mut Forge) {
    world.evidence_ledger.verify_integrity().expect("integrity passes");
}

#[then("exporting the ledger as JSON is deterministic")]
fn ledger_export_deterministic(world: &mut Forge) {
    let first: String = world.evidence_ledger.export_json().expect("first export");
    let second: String = world.evidence_ledger.export_json().expect("second export");
    assert_eq!(first, second);
}

#[then(regex = r#"^record "([^"]+)"(?: still)? has evaluation result "([^"]+)"$"#)]
fn record_eval_result(world: &mut Forge, id: String, expected: String) {
    let record: &EvidenceRecord = world.evidence_ledger.get(&id).expect("record exists");
    match &record.outcome {
        VerificationOutcome::Evaluation(result) => assert_eq!(result.to_string(), expected),
        other => panic!("expected evaluation outcome, got {other:?}"),
    }
}

#[given("a temporary filesystem ledger")]
fn temp_fs_ledger(world: &mut Forge) {
    let dir: PathBuf = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ledgers")
        .join(LEDGER_DIRS.fetch_add(1, Ordering::SeqCst).to_string());
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("stale ledger dir removed");
    }
    let fs_ledger: FsLedger = FsLedger::open(dir.clone()).expect("fs ledger opens");
    world.fs_ledger_dir = Some(dir);
    world.fs_ledger = Some(fs_ledger);
}

#[given(expr = "a satisfied evidence record {string} with blob payload {string} is persisted")]
#[when(expr = "a satisfied evidence record {string} with blob payload {string} is persisted")]
fn persist_record_with_blob(world: &mut Forge, id: String, payload: String) {
    let snapshot: GraphSnapshot = current_snapshot(world).clone();
    let fs_ledger: &mut FsLedger = world.fs_ledger.as_mut().expect("fs ledger opened");
    let digest: ContentHash = fs_ledger.write_blob(payload.as_bytes()).expect("blob written");
    let mut digests: BTreeMap<String, ContentHash> = BTreeMap::new();
    digests.insert("trace".to_string(), digest.clone());
    let record: EvidenceRecord = build_record(
        &snapshot,
        &id,
        "PROP-MSG-001",
        "CLM-MSG-001",
        VerificationMethod::PropertyBased,
        VerificationOutcome::Evaluation(EvaluationResult::Satisfied),
        digests,
    );
    fs_ledger.append_record(record).expect("record persisted");
    world.blob_digests.insert(id, digest);
}

#[when("the filesystem ledger is reloaded")]
fn reload_fs_ledger(world: &mut Forge) {
    let dir: PathBuf = world.fs_ledger_dir.clone().expect("fs ledger dir set");
    world.fs_reload = Some(FsLedger::open(dir));
}

#[then(expr = "the reloaded ledger holds {int} evidence record")]
fn reloaded_count(world: &mut Forge, expected: usize) {
    let loaded: &FsLedger = world.fs_reload.as_ref().expect("reloaded").as_ref().expect("reload succeeded");
    assert_eq!(loaded.ledger().records().len(), expected);
}

#[then(expr = "the blob for {string} reads back as {string}")]
fn blob_reads_back(world: &mut Forge, id: String, expected: String) {
    let loaded: &FsLedger = world.fs_reload.as_ref().expect("reloaded").as_ref().expect("reload succeeded");
    let digest: &ContentHash = world.blob_digests.get(&id).expect("digest recorded");
    let bytes: Vec<u8> = loaded.read_blob(digest).expect("blob reads back");
    assert_eq!(String::from_utf8(bytes).expect("utf8"), expected);
}

#[when("the persisted ledger file is tampered with")]
fn tamper_fs_ledger(world: &mut Forge) {
    let dir: &Path = world.fs_ledger_dir.as_ref().expect("fs ledger dir set");
    let log_path: PathBuf = dir.join("ledger.jsonl");
    let content: String = fs::read_to_string(&log_path).expect("log read");
    let tampered: String = content.replace("single-broker", "tampered-broker");
    assert_ne!(content, tampered);
    fs::write(&log_path, tampered).expect("tampered log written");
}

#[then("reloading the filesystem ledger fails with an integrity error")]
fn reload_fails_integrity(world: &mut Forge) {
    assert!(matches!(world.fs_reload, Some(Err(LedgerError::InvalidDigest(_)))), "{:?}", world.fs_reload);
}

#[when(expr = "the stored blob for {string} is tampered with")]
fn tamper_stored_blob(world: &mut Forge, id: String) {
    let dir: &Path = world.fs_ledger_dir.as_ref().expect("fs ledger dir set");
    let digest: &ContentHash = world.blob_digests.get(&id).expect("digest recorded");
    let blob_path: PathBuf = dir.join("blobs").join(digest.as_str());
    fs::write(&blob_path, b"tampered-blob-bytes").expect("tampered blob written");
}

#[then("reloading the filesystem ledger fails with a blob integrity error")]
fn reload_fails_blob_integrity(world: &mut Forge) {
    assert!(matches!(world.fs_reload, Some(Err(LedgerError::BlobIntegrity { .. }))), "{:?}", world.fs_reload);
}

#[when(expr = "the snapshot changes to {string}")]
fn snapshot_changes_to(world: &mut Forge, name: String) {
    let previous: GraphSnapshot = current_snapshot(world).clone();
    let current: GraphSnapshot = compile_fixture(&name).expect("new snapshot compiles");
    world.impact = Some(impact_set(&previous, &current, &world.evidence_ledger));
    world.baseline_snapshot = Some(previous);
    world.graph_compiled = Some(Ok(current));
}

#[when(expr = "claim {string} is assessed against the current snapshot")]
fn assess_claim_step(world: &mut Forge, claim_id: String) {
    let snapshot: &GraphSnapshot = current_snapshot(world);
    let claim: &VerificationClaim = snapshot.claim(&claim_id).expect("claim exists");
    world.claim_assessment = Some(assess_claim(claim, snapshot, &world.evidence_ledger));
}

#[then(expr = "the claim status is {string}")]
fn claim_status_is(world: &mut Forge, expected: String) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    assert_eq!(assessment.status.to_string(), expected, "{:?}", assessment.rationale);
}

#[then(expr = "evidence {string} has freshness {string}")]
fn evidence_freshness_is(world: &mut Forge, id: String, expected: String) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    let entry: &EvidenceEvaluationEntry = assessment
        .evaluated
        .iter()
        .find(|entry: &&EvidenceEvaluationEntry| entry.evidence_id == id)
        .expect("evidence in assessment");
    assert_eq!(entry.freshness.to_string(), expected);
}

#[then(expr = "the impact set names stale evidence {string} and affected claim {string}")]
fn impact_set_names(world: &mut Forge, evidence_id: String, claim_id: String) {
    let impact: &ImpactSet = world.impact.as_ref().expect("impact computed");
    assert!(impact.stale_evidence_ids.contains(&evidence_id), "{impact:?}");
    assert!(impact.affected_claim_ids.iter().any(|id: &ArtifactId| id.as_str() == claim_id), "{impact:?}");
}

#[then(expr = "the claim assessment reports {int} contradiction")]
fn claim_contradiction_count(world: &mut Forge, expected: usize) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    assert_eq!(assessment.contradictions.len(), expected);
}

#[then(expr = "the claim assessment rationale mentions {string}")]
fn claim_rationale_mentions(world: &mut Forge, needle: String) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    assert!(
        assessment.rationale.iter().any(|line: &String| line.contains(&needle)),
        "{:?}",
        assessment.rationale
    );
}

#[then("the claim assessment is not an unbounded proof")]
fn claim_not_unbounded_proof(world: &mut Forge) {
    let assessment: &ClaimAssessment = world.claim_assessment.as_ref().expect("claim assessed");
    assert!(!assessment.unbounded_proof);
}

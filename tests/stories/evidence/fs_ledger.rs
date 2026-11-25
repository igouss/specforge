use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use cucumber::{given, then, when};
use specforge::graph::{ContentHash, GraphSnapshot, VerificationMethod};
use specforge::ledger::{EvidenceLink, EvidenceRecord, EvidenceRelation, LedgerError};
use specforge::ledger_fs::FsLedger;
use specforge::verify::{EvaluationResult, VerificationOutcome};

use super::{build_record, parse_relation};
use crate::graph::current_snapshot;
use crate::Forge;

static LEDGER_DIRS: AtomicUsize = AtomicUsize::new(0);

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

#[when(expr = "a link {string} from {string} to {string} is persisted to the filesystem ledger")]
fn persist_link_to_fs_ledger(world: &mut Forge, relation: String, from_id: String, to_id: String) {
    let relation: EvidenceRelation = parse_relation(&relation);
    let link: EvidenceLink = EvidenceLink::seal(from_id, to_id, relation, "persisted link".to_string()).expect("link seals");
    let fs_ledger: &mut FsLedger = world.fs_ledger.as_mut().expect("fs ledger opened");
    fs_ledger.append_link(link).expect("link persisted");
}

#[when("the filesystem ledger is reloaded")]
fn reload_fs_ledger(world: &mut Forge) {
    let dir: PathBuf = world.fs_ledger_dir.clone().expect("fs ledger dir set");
    world.fs_reload = Some(FsLedger::open(dir));
}

#[then(regex = r"^the reloaded ledger holds (\d+) evidence records?$")]
fn reloaded_count(world: &mut Forge, expected: usize) {
    let loaded: &FsLedger = world.fs_reload.as_ref().expect("reloaded").as_ref().expect("reload succeeded");
    assert_eq!(loaded.ledger().records().len(), expected);
}

#[then(expr = "the reloaded ledger holds {int} evidence link from {string} to {string}")]
fn reloaded_link_present(world: &mut Forge, expected: usize, from_id: String, to_id: String) {
    let loaded: &FsLedger = world.fs_reload.as_ref().expect("reloaded").as_ref().expect("reload succeeded");
    assert_eq!(loaded.ledger().links().len(), expected);
    assert!(
        loaded
            .ledger()
            .links()
            .iter()
            .any(|link: &EvidenceLink| link.from_id == from_id && link.to_id == to_id),
        "{:?}",
        loaded.ledger().links()
    );
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

#[when("the filesystem ledger is opened read-only")]
fn open_fs_ledger_read_only(world: &mut Forge) {
    let dir: PathBuf = world.fs_ledger_dir.clone().expect("fs ledger dir set");
    let read_only: FsLedger = FsLedger::open_read_only(dir).expect("read-only ledger opens");
    world.evidence_state.fs_read_only = Some(read_only);
}

#[then(expr = "the read-only ledger holds {int} evidence records and {int} link")]
fn read_only_ledger_holds(world: &mut Forge, records: usize, links: usize) {
    let ledger: &FsLedger = world.evidence_state.fs_read_only.as_ref().expect("read-only opened");
    assert_eq!(ledger.ledger().records().len(), records);
    assert_eq!(ledger.ledger().links().len(), links);
}

#[then("appending a link to the read-only filesystem ledger is refused")]
fn append_link_read_only_refused(world: &mut Forge) {
    let ledger: &mut FsLedger = world.evidence_state.fs_read_only.as_mut().expect("read-only opened");
    let link: EvidenceLink = EvidenceLink::seal(
        "EV-FS-2".to_string(),
        "CLM-MSG-001".to_string(),
        EvidenceRelation::Supports,
        "read-only refusal".to_string(),
    )
    .expect("link seals");
    let error: LedgerError = ledger.append_link(link).expect_err("read-only append must fail");
    assert!(matches!(error, LedgerError::Io(_)), "{error:?}");
    assert!(error.to_string().contains("read-only"), "{error}");
}

#[when(expr = "writing blob payload {string} to the filesystem ledger is attempted")]
fn attempt_blob_write(world: &mut Forge, payload: String) {
    let fs_ledger: &FsLedger = world.fs_ledger.as_ref().expect("fs ledger opened");
    world.evidence_state.fs_blob_write = Some(fs_ledger.write_blob(payload.as_bytes()));
}

#[then("the blob write fails with a blob integrity error mentioning the blob digest")]
fn blob_write_fails_integrity(world: &mut Forge) {
    let error: &LedgerError = world
        .evidence_state
        .fs_blob_write
        .as_ref()
        .expect("blob write attempted")
        .as_ref()
        .expect_err("blob write must fail");
    match error {
        LedgerError::BlobIntegrity { digest } => {
            let rendered: String = error.to_string();
            assert!(rendered.contains(digest.as_str()), "expected digest {} in {rendered:?}", digest.as_str());
        }
        LedgerError::EmptyId
        | LedgerError::DuplicateRecord(_)
        | LedgerError::InvalidDigest(_)
        | LedgerError::InvalidRecord { .. }
        | LedgerError::InvalidLink { .. }
        | LedgerError::MissingRecord(_)
        | LedgerError::Serialization(_)
        | LedgerError::Io(_)
        | LedgerError::CorruptEntry { .. } => panic!("expected BlobIntegrity, got {error:?}"),
    }
}

#[when("a malformed JSONL line is appended to the persisted ledger file")]
fn append_malformed_jsonl(world: &mut Forge) {
    let dir: &Path = world.fs_ledger_dir.as_ref().expect("fs ledger dir set");
    let log_path: PathBuf = dir.join("ledger.jsonl");
    let mut file: fs::File = OpenOptions::new().append(true).open(&log_path).expect("log opens for append");
    writeln!(file, "{{not-valid-json").expect("malformed line appended");
    file.flush().expect("flushed");
}

#[then(expr = "reloading the filesystem ledger fails with a corrupt entry error at line {int}")]
fn reload_fails_corrupt_line(world: &mut Forge, expected_line: usize) {
    let error: &LedgerError = world.fs_reload.as_ref().expect("reload attempted").as_ref().expect_err("reload must fail");
    match error {
        LedgerError::CorruptEntry { line, .. } => {
            assert_eq!(*line, expected_line, "{error:?}");
            let rendered: String = error.to_string();
            let expected_fragment: String = format!("line {expected_line}");
            assert!(rendered.contains(&expected_fragment), "expected {expected_fragment:?} in {rendered:?}");
        }
        LedgerError::EmptyId
        | LedgerError::DuplicateRecord(_)
        | LedgerError::InvalidDigest(_)
        | LedgerError::InvalidRecord { .. }
        | LedgerError::InvalidLink { .. }
        | LedgerError::MissingRecord(_)
        | LedgerError::Serialization(_)
        | LedgerError::Io(_)
        | LedgerError::BlobIntegrity { .. } => panic!("expected CorruptEntry, got {error:?}"),
    }
}

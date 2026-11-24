use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::graph::{canonical_json, ContentHash};
use crate::ledger::{EvidenceLedger, EvidenceLink, EvidenceRecord, LedgerError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "entry_kind", content = "entry", rename_all = "snake_case")]
enum LedgerEntry {
    Record(Box<EvidenceRecord>),
    Link(EvidenceLink),
}

#[derive(Debug, Clone)]
pub struct FsLedger {
    root: PathBuf,
    inner: EvidenceLedger,
}

impl FsLedger {
    /// When a filesystem ledger directory is opened, `open` shall replay its append-only JSONL log and reject any corrupt or tampered entry.
    pub fn open(root: PathBuf) -> Result<FsLedger, LedgerError> {
        fs::create_dir_all(root.join("blobs")).map_err(|error: std::io::Error| LedgerError::Io(error.to_string()))?;
        let log_path: PathBuf = root.join("ledger.jsonl");
        let mut inner: EvidenceLedger = EvidenceLedger::new();
        if log_path.exists() {
            let content: String = fs::read_to_string(&log_path).map_err(|error: std::io::Error| LedgerError::Io(error.to_string()))?;
            for (index, line) in content.lines().enumerate() {
                let trimmed: &str = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let entry: LedgerEntry = serde_json::from_str::<LedgerEntry>(trimmed).map_err(|error: serde_json::Error| LedgerError::CorruptEntry {
                    line: index + 1,
                    why: error.to_string(),
                })?;
                match entry {
                    LedgerEntry::Record(record) => inner.append_record(*record)?,
                    LedgerEntry::Link(link) => inner.append_link(link)?,
                }
            }
        }
        inner.verify_integrity()?;
        let ledger: FsLedger = FsLedger { root, inner };
        for record in ledger.inner.records() {
            for digest in record.artifact_digests.values() {
                let _: Vec<u8> = ledger.read_blob(digest)?;
            }
        }
        Ok(ledger)
    }

    /// When raw artifact bytes are stored, `write_blob` shall write them content-addressed by SHA-256 and verify any existing file with the same digest.
    pub fn write_blob(&self, bytes: &[u8]) -> Result<ContentHash, LedgerError> {
        let digest: ContentHash = ContentHash::of_bytes(bytes);
        let path: PathBuf = self.blob_path(&digest);
        if path.exists() {
            let existing: Vec<u8> = fs::read(&path).map_err(|error: std::io::Error| LedgerError::Io(error.to_string()))?;
            if ContentHash::of_bytes(&existing) != digest {
                return Err(LedgerError::BlobIntegrity { digest });
            }
            return Ok(digest);
        }
        fs::write(&path, bytes).map_err(|error: std::io::Error| LedgerError::Io(error.to_string()))?;
        Ok(digest)
    }

    /// When a stored blob is read, `read_blob` shall recompute its SHA-256 digest and reject tampered content.
    pub fn read_blob(&self, digest: &ContentHash) -> Result<Vec<u8>, LedgerError> {
        let path: PathBuf = self.blob_path(digest);
        let bytes: Vec<u8> = fs::read(&path).map_err(|error: std::io::Error| LedgerError::Io(error.to_string()))?;
        if ContentHash::of_bytes(&bytes) != *digest {
            return Err(LedgerError::BlobIntegrity { digest: digest.clone() });
        }
        Ok(bytes)
    }

    /// When an evidence record is appended, `append_record` shall validate it without mutating state, persist and flush one canonical JSONL line to `ledger.jsonl`, and only then update memory.
    pub fn append_record(&mut self, record: EvidenceRecord) -> Result<(), LedgerError> {
        self.inner.validate_new_record(&record)?;
        self.append_entry(&LedgerEntry::Record(Box::new(record.clone())))?;
        self.inner.append_record(record)
    }

    /// When an evidence link is appended, `append_link` shall validate it without mutating state, persist and flush one canonical JSONL line to `ledger.jsonl`, and only then update memory.
    pub fn append_link(&mut self, link: EvidenceLink) -> Result<(), LedgerError> {
        self.inner.validate_new_link(&link)?;
        self.append_entry(&LedgerEntry::Link(link.clone()))?;
        self.inner.append_link(link)
    }

    pub fn ledger(&self) -> &EvidenceLedger {
        &self.inner
    }

    pub fn log_path(&self) -> PathBuf {
        self.root.join("ledger.jsonl")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn blob_path(&self, digest: &ContentHash) -> PathBuf {
        self.root.join("blobs").join(digest.as_str())
    }

    fn append_entry(&self, entry: &LedgerEntry) -> Result<(), LedgerError> {
        let encoded: String = canonical_json(entry).map_err(LedgerError::Serialization)?;
        let mut file: fs::File = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path())
            .map_err(|error: std::io::Error| LedgerError::Io(error.to_string()))?;
        writeln!(file, "{encoded}").map_err(|error: std::io::Error| LedgerError::Io(error.to_string()))?;
        file.flush().map_err(|error: std::io::Error| LedgerError::Io(error.to_string()))
    }
}

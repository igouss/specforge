mod record;

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::graph::{canonical_json, ContentHash};

pub use record::{EvidenceKind, EvidencePayload, EvidenceRecord, ProducerMeta};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRelation {
    Supports,
    Contradicts,
    Corroborates,
    DerivedFrom,
    Supersedes,
    Invalidates,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct EvidenceLinkPayload<'a> {
    from_id: &'a str,
    to_id: &'a str,
    relation: EvidenceRelation,
    reason: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceLink {
    pub from_id: String,
    pub to_id: String,
    pub relation: EvidenceRelation,
    pub reason: String,
    pub content_digest: ContentHash,
}

fn validate_link_fields(from_id: &str, to_id: &str) -> Result<(), LedgerError> {
    if from_id.trim().is_empty() {
        return Err(LedgerError::InvalidLink {
            why: "link from_id must not be empty".to_string(),
        });
    }
    if to_id.trim().is_empty() {
        return Err(LedgerError::InvalidLink {
            why: "link to_id must not be empty".to_string(),
        });
    }
    if from_id == to_id {
        return Err(LedgerError::InvalidLink {
            why: format!("self-link on {from_id} is forbidden"),
        });
    }
    Ok(())
}

impl EvidenceLink {
    /// When an evidence link is sealed, `seal` shall validate its endpoints and compute its canonical JSON SHA-256 content digest.
    pub fn seal(from_id: String, to_id: String, relation: EvidenceRelation, reason: String) -> Result<EvidenceLink, LedgerError> {
        validate_link_fields(&from_id, &to_id)?;
        let payload: EvidenceLinkPayload<'_> = EvidenceLinkPayload {
            from_id: &from_id,
            to_id: &to_id,
            relation,
            reason: &reason,
        };
        let content_digest: ContentHash = ContentHash::of_canonical_json(&payload).map_err(LedgerError::Serialization)?;
        Ok(EvidenceLink {
            from_id,
            to_id,
            relation,
            reason,
            content_digest,
        })
    }

    /// When an evidence link's integrity is verified, `verify_digest` shall return true only if its recomputed canonical SHA-256 digest matches `content_digest`.
    pub fn verify_digest(&self) -> bool {
        let payload: EvidenceLinkPayload<'_> = EvidenceLinkPayload {
            from_id: &self.from_id,
            to_id: &self.to_id,
            relation: self.relation,
            reason: &self.reason,
        };
        ContentHash::of_canonical_json(&payload).is_ok_and(|computed: ContentHash| computed == self.content_digest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    EmptyId,
    DuplicateRecord(String),
    InvalidDigest(String),
    InvalidRecord { id: String, why: String },
    InvalidLink { why: String },
    MissingRecord(String),
    Serialization(String),
    Io(String),
    CorruptEntry { line: usize, why: String },
    BlobIntegrity { digest: ContentHash },
}

impl fmt::Display for LedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::EmptyId => formatter.write_str("evidence record id must not be empty"),
            LedgerError::DuplicateRecord(id) => write!(formatter, "evidence record {id} already exists in the ledger"),
            LedgerError::InvalidDigest(id) => write!(formatter, "evidence entry {id} failed content digest verification"),
            LedgerError::InvalidRecord { id, why } => write!(formatter, "evidence record {id} is invalid: {why}"),
            LedgerError::InvalidLink { why } => write!(formatter, "evidence link is invalid: {why}"),
            LedgerError::MissingRecord(id) => write!(formatter, "evidence link references unknown record {id}"),
            LedgerError::Serialization(why) => write!(formatter, "evidence serialization failed: {why}"),
            LedgerError::Io(why) => write!(formatter, "ledger I/O error: {why}"),
            LedgerError::CorruptEntry { line, why } => write!(formatter, "corrupt ledger entry at line {line}: {why}"),
            LedgerError::BlobIntegrity { digest } => write!(formatter, "blob {digest} failed SHA-256 integrity verification"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct EvidenceLedger {
    records: Vec<EvidenceRecord>,
    links: Vec<EvidenceLink>,
}

#[derive(Deserialize)]
struct RawEvidenceLedger {
    records: Vec<EvidenceRecord>,
    links: Vec<EvidenceLink>,
}

impl<'de> Deserialize<'de> for EvidenceLedger {
    fn deserialize<D>(deserializer: D) -> Result<EvidenceLedger, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: RawEvidenceLedger = RawEvidenceLedger::deserialize(deserializer)?;
        let ledger: EvidenceLedger = EvidenceLedger {
            records: raw.records,
            links: raw.links,
        };
        ledger.verify_integrity().map_err(serde::de::Error::custom)?;
        Ok(ledger)
    }
}

fn requires_target_record(relation: EvidenceRelation) -> bool {
    match relation {
        EvidenceRelation::DerivedFrom | EvidenceRelation::Supersedes | EvidenceRelation::Invalidates | EvidenceRelation::Corroborates => true,
        EvidenceRelation::Supports | EvidenceRelation::Contradicts => false,
    }
}

impl EvidenceLedger {
    /// When a new ledger is created, `new` shall return an empty append-only ledger.
    pub fn new() -> EvidenceLedger {
        EvidenceLedger::default()
    }

    pub(crate) fn validate_new_record(&self, record: &EvidenceRecord) -> Result<(), LedgerError> {
        if record.id.trim().is_empty() {
            return Err(LedgerError::EmptyId);
        }
        if self.records.iter().any(|existing: &EvidenceRecord| existing.id == record.id) {
            return Err(LedgerError::DuplicateRecord(record.id.clone()));
        }
        if !record.verify_digest() {
            return Err(LedgerError::InvalidDigest(record.id.clone()));
        }
        record.validate_fields()
    }

    pub(crate) fn validate_new_link(&self, link: &EvidenceLink) -> Result<(), LedgerError> {
        validate_link_fields(&link.from_id, &link.to_id)?;
        if !link.verify_digest() {
            return Err(LedgerError::InvalidDigest(format!("{}->{}", link.from_id, link.to_id)));
        }
        if !self.records.iter().any(|existing: &EvidenceRecord| existing.id == link.from_id) {
            return Err(LedgerError::MissingRecord(link.from_id.clone()));
        }
        if requires_target_record(link.relation) && !self.records.iter().any(|existing: &EvidenceRecord| existing.id == link.to_id) {
            return Err(LedgerError::MissingRecord(link.to_id.clone()));
        }
        Ok(())
    }

    /// When a record is appended, `append_record` shall refuse empty ids, duplicate ids, invalid fields, or invalid digests and otherwise append without mutating prior records.
    pub fn append_record(&mut self, record: EvidenceRecord) -> Result<(), LedgerError> {
        self.validate_new_record(&record)?;
        self.records.push(record);
        Ok(())
    }

    /// When a link is appended, `append_link` shall validate its endpoints, digest, and referenced records before appending.
    pub fn append_link(&mut self, link: EvidenceLink) -> Result<(), LedgerError> {
        self.validate_new_link(&link)?;
        self.links.push(link);
        Ok(())
    }

    pub fn records(&self) -> &[EvidenceRecord] {
        &self.records
    }

    pub fn links(&self) -> &[EvidenceLink] {
        &self.links
    }

    pub fn get(&self, id: &str) -> Option<&EvidenceRecord> {
        self.records.iter().find(|record: &&EvidenceRecord| record.id == id)
    }

    /// When ledger integrity is checked, `verify_integrity` shall confirm every record and link satisfies all schema, reference, and digest invariants.
    pub fn verify_integrity(&self) -> Result<(), LedgerError> {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for record in &self.records {
            if record.id.trim().is_empty() {
                return Err(LedgerError::EmptyId);
            }
            if !seen.insert(record.id.as_str()) {
                return Err(LedgerError::DuplicateRecord(record.id.clone()));
            }
            if !record.verify_digest() {
                return Err(LedgerError::InvalidDigest(record.id.clone()));
            }
            record.validate_fields()?;
        }
        for link in &self.links {
            validate_link_fields(&link.from_id, &link.to_id)?;
            if !link.verify_digest() {
                return Err(LedgerError::InvalidDigest(format!("{}->{}", link.from_id, link.to_id)));
            }
            if !seen.contains(link.from_id.as_str()) {
                return Err(LedgerError::MissingRecord(link.from_id.clone()));
            }
            if requires_target_record(link.relation) && !seen.contains(link.to_id.as_str()) {
                return Err(LedgerError::MissingRecord(link.to_id.clone()));
            }
        }
        Ok(())
    }

    /// When the ledger is exported, `export_json` shall emit canonical deterministic JSON after verifying integrity.
    pub fn export_json(&self) -> Result<String, LedgerError> {
        self.verify_integrity()?;
        canonical_json(self).map_err(LedgerError::Serialization)
    }
}

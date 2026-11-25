use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::graph::{ArtifactId, VerificationMethod};
use crate::ledger::EvidenceRecord;
use crate::verify::ReplayDescriptor;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationNote {
    pub evidence_ids: Vec<String>,
    pub reason: String,
}

fn push_group_reason(grouped: &mut BTreeMap<Vec<String>, Vec<String>>, mut ids: Vec<String>, cause: String) {
    ids.sort();
    ids.dedup();
    if ids.len() < 2 {
        return;
    }
    let causes: &mut Vec<String> = grouped.entry(ids).or_default();
    if !causes.contains(&cause) {
        causes.push(cause);
    }
}

type EventGroupKey<'a> = (&'a str, VerificationMethod, Option<&'a str>, &'a str, &'a [String]);

/// When two or more current supporting evidence records for the same claim share an engine/method/seed, replay token, or replay event sequence, `detect_correlations` shall emit deterministic correlation notes and append them to rationale.
pub(super) fn detect_correlations(current_supporting: &[&EvidenceRecord], rationale: &mut Vec<String>) -> Vec<CorrelationNote> {
    if current_supporting.len() < 2 {
        return Vec::new();
    }

    let mut by_seed: BTreeMap<(&str, VerificationMethod, u64), Vec<String>> = BTreeMap::new();
    let mut by_token: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut by_events: BTreeMap<EventGroupKey<'_>, Vec<String>> = BTreeMap::new();

    for record in current_supporting {
        let Some(replay): Option<&ReplayDescriptor> = record.replay.as_ref() else {
            continue;
        };
        if let Some(seed) = replay.seed {
            by_seed
                .entry((record.producer.name.as_str(), record.method, seed))
                .or_default()
                .push(record.id.clone());
        }
        if let Some(token) = replay.replay_token.as_deref() {
            let trimmed: &str = token.trim();
            if !trimmed.is_empty() {
                by_token.entry(trimmed).or_default().push(record.id.clone());
            }
        }
        if !replay.events.is_empty() {
            by_events
                .entry((
                    record.producer.name.as_str(),
                    record.method,
                    record.property_id.as_ref().map(ArtifactId::as_str),
                    record.snapshot_id.as_str(),
                    replay.events.as_slice(),
                ))
                .or_default()
                .push(record.id.clone());
        }
    }

    let mut reasons_by_group: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();

    for ((engine, method, seed), ids) in by_seed {
        push_group_reason(
            &mut reasons_by_group,
            ids,
            format!("shared engine \"{engine}\", method \"{}\", and seed {seed}", method.as_str()),
        );
    }
    for (token, ids) in by_token {
        push_group_reason(&mut reasons_by_group, ids, format!("shared replay token \"{token}\""));
    }
    for ((engine, method, property_id, snapshot_id, events), ids) in by_events {
        let prop_label: &str = property_id.unwrap_or("<none>");
        push_group_reason(
            &mut reasons_by_group,
            ids,
            format!(
                "identical replay events [{}] on engine \"{engine}\", method \"{}\", property {prop_label}, snapshot {snapshot_id}",
                events.join(", "),
                method.as_str()
            ),
        );
    }

    let mut notes: Vec<CorrelationNote> = Vec::new();
    for (evidence_ids, causes) in reasons_by_group {
        let reason: String = format!(
            "correlated supporting evidence ({}): {} (not independent corroboration)",
            evidence_ids.join(", "),
            causes.join("; ")
        );
        rationale.push(reason.clone());
        notes.push(CorrelationNote { evidence_ids, reason });
    }
    notes
}

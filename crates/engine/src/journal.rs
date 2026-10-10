//! The append-only journal: what was applied, in order, with payloads.
//!
//! One JSON object per line — inspectable by eye, appendable by any
//! surface, replayable by the engine. Replay is the session-restart
//! path: rebuild the document *and* the undo history from the entries,
//! so undo survives a restart (the M5 gate).

use doc::Document;

use crate::error::EngineError;
use crate::history::{Call, History, Step};
use crate::registry::CommandRegistry;

/// One applied command as recorded in the journal.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JournalEntry {
    /// 1-based application order; a replay refuses gaps.
    pub seq: u64,
    /// Catalog id of the applied command.
    pub name: String,
    /// The payload exactly as applied.
    pub payload: serde_json::Value,
    /// Transaction label when the application happened inside an open
    /// transaction; replay folds consecutive same-label entries into
    /// one undo step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub txn: Option<String>,
}

/// In-memory journal: entries in application order.
#[derive(Clone, Debug, Default)]
pub struct Journal {
    entries: Vec<JournalEntry>,
    next_seq: u64,
}

impl Journal {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_seq: 1,
        }
    }

    /// Records one application and returns the entry as written.
    pub fn record(
        &mut self,
        name: &str,
        payload: &serde_json::Value,
        txn: Option<&str>,
    ) -> JournalEntry {
        let entry = JournalEntry {
            seq: self.next_seq,
            name: name.to_owned(),
            payload: payload.clone(),
            txn: txn.map(str::to_owned),
        };
        self.next_seq += 1;
        self.entries.push(entry.clone());
        entry
    }

    /// Entries in application order.
    #[must_use]
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// Number of recorded entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when nothing has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The whole journal as JSON lines (one entry per line, newline
    /// terminated).
    #[must_use]
    pub fn to_lines(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            let Ok(mut line) = serde_json::to_string(entry) else {
                continue;
            };
            line.push('\n');
            out.push_str(&line);
        }
        out
    }

    /// Parses JSON lines. Blank lines are skipped; a malformed line or
    /// a sequence gap is an error.
    pub fn from_lines(text: &str) -> Result<Self, EngineError> {
        let mut journal = Self::new();
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: JournalEntry =
                serde_json::from_str(line).map_err(|error| EngineError::Journal {
                    line: index + 1,
                    detail: error.to_string(),
                })?;
            if entry.seq != journal.next_seq {
                return Err(EngineError::Journal {
                    line: index + 1,
                    detail: format!("expected seq {}, found {}", journal.next_seq, entry.seq),
                });
            }
            journal.next_seq += 1;
            journal.entries.push(entry);
        }
        Ok(journal)
    }
}

/// Replays a journal onto `document` through `registry`, returning the
/// rebuilt undo history (redo empty): a restarted session can undo
/// across the replay boundary.
///
/// Consecutive entries sharing a transaction label replay as one
/// transaction (one undo step), mirroring how they were recorded.
pub fn replay(
    document: &mut Document,
    registry: &CommandRegistry,
    entries: &[JournalEntry],
) -> Result<History, EngineError> {
    let mut history = History::new();
    let mut index = 0;
    while index < entries.len() {
        let entry = &entries[index];
        match &entry.txn {
            None => {
                let delta = apply(document, registry, entry)?;
                history.record(
                    Call {
                        name: entry.name.clone(),
                        payload: entry.payload.clone(),
                    },
                    delta,
                );
                index += 1;
            }
            Some(label) => {
                let mut calls = Vec::new();
                let mut deltas = Vec::new();
                while index < entries.len() && entries[index].txn.as_deref() == Some(label) {
                    deltas.push(apply(document, registry, &entries[index])?);
                    calls.push(Call {
                        name: entries[index].name.clone(),
                        payload: entries[index].payload.clone(),
                    });
                    index += 1;
                }
                history.commit_as(Step {
                    label: label.to_owned(),
                    calls,
                    delta: crate::delta::Delta::Group(deltas),
                });
            }
        }
    }
    Ok(history)
}

fn apply(
    document: &mut Document,
    registry: &CommandRegistry,
    entry: &JournalEntry,
) -> Result<crate::delta::Delta, EngineError> {
    registry.execute(document, &entry.name, entry.payload.clone())
}

//! Transactional undo/redo history.
//!
//! One undo step is one [`Step`]: a single command, or a whole open
//! transaction folded at commit. Undo replays the step's captured
//! delta; redo re-executes its calls through the registry. Redo is
//! cleared the moment new work lands on the undo stack.

use crate::delta::Delta;
use crate::error::EngineError;

/// One executable command call (catalog id + payload).
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    /// Catalog id.
    pub name: String,
    /// Payload as applied.
    pub payload: serde_json::Value,
}

/// One undo step: what was done ([`Step::calls`]) and how to take it
/// back ([`Step::delta`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// Display label — the catalog id, or the transaction label.
    pub label: String,
    /// The calls to re-execute on redo, in order.
    pub calls: Vec<Call>,
    /// The captured inverse.
    pub delta: Delta,
}

/// Undo/redo stacks.
#[derive(Default)]
pub struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
    open: Option<TransactionBuffer>,
}

#[derive(Default)]
struct TransactionBuffer {
    label: String,
    calls: Vec<Call>,
    deltas: Vec<Delta>,
}

impl History {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one applied command: into the open transaction when one
    /// is open, otherwise as its own undo step. Either way redo is
    /// dropped — new work invalidates it.
    pub fn record(&mut self, call: Call, delta: Delta) {
        self.redo.clear();
        match &mut self.open {
            Some(buffer) => {
                buffer.calls.push(call);
                buffer.deltas.push(delta);
            }
            None => {
                let label = call.name.clone();
                self.undo.push(Step {
                    label,
                    calls: vec![call],
                    delta,
                });
            }
        }
    }

    /// Opens a transaction. Nested transactions are refused — group at
    /// the surface, not inside the engine.
    pub fn begin_transaction(&mut self, label: &str) -> Result<(), EngineError> {
        if self.open.is_some() {
            return Err(EngineError::InvalidState {
                name: "transaction.begin".to_owned(),
                detail: "a transaction is already open".to_owned(),
            });
        }
        self.open = Some(TransactionBuffer {
            label: label.to_owned(),
            calls: Vec::new(),
            deltas: Vec::new(),
        });
        Ok(())
    }

    /// `true` while a transaction is open.
    #[must_use]
    pub fn transaction_open(&self) -> bool {
        self.open.is_some()
    }

    /// Folds the open transaction into one undo step. An empty
    /// transaction is refused — a group with no calls cannot undo, and
    /// silently vanishing steps hide bugs.
    pub fn commit_transaction(&mut self) -> Result<String, EngineError> {
        let buffer = self.open.take().ok_or(EngineError::InvalidState {
            name: "transaction.commit".to_owned(),
            detail: "no transaction is open".to_owned(),
        })?;
        if buffer.calls.is_empty() {
            return Err(EngineError::InvalidState {
                name: "transaction.commit".to_owned(),
                detail: format!("transaction {:?} has no commands", buffer.label),
            });
        }
        let label = buffer.label.clone();
        self.undo.push(Step {
            label: buffer.label,
            calls: buffer.calls,
            delta: Delta::Group(buffer.deltas),
        });
        Ok(label)
    }

    /// Installs a fully-formed step (the journal-replay path — replay
    /// re-derives deltas, it does not walk the interactive buffers).
    pub fn commit_as(&mut self, step: Step) {
        self.undo.push(step);
    }

    /// Aborts the open transaction: un-applies every buffered delta in
    /// reverse (the document returns to its pre-transaction state) and
    /// discards the buffer. Returns how many calls were rolled back.
    pub fn abort_transaction(
        &mut self,
        document: &mut doc::Document,
    ) -> Result<usize, EngineError> {
        let buffer = self.open.take().ok_or(EngineError::InvalidState {
            name: "transaction.abort".to_owned(),
            detail: "no transaction is open".to_owned(),
        })?;
        let rolled = buffer.calls.len();
        for delta in buffer.deltas.iter().rev() {
            delta.apply_inverse(document)?;
        }
        Ok(rolled)
    }

    /// Pops the top undo step and hands it over for inversion; the
    /// caller applies [`Step::delta`] to the document and then files
    /// the step with [`History::file_redo`].
    pub fn pop_undo(&mut self) -> Result<Step, EngineError> {
        self.undo.pop().ok_or(EngineError::NothingToUndo)
    }

    /// Files an undone step on the redo stack.
    pub fn file_redo(&mut self, step: Step) {
        self.redo.push(step);
    }

    /// Pops the top redo step for re-execution; the caller re-applies
    /// [`Step::calls`] and files the step with [`History::file_undo`].
    pub fn pop_redo(&mut self) -> Result<Step, EngineError> {
        self.redo.pop().ok_or(EngineError::NothingToRedo)
    }

    /// Files a re-executed step back on the undo stack.
    pub fn file_undo(&mut self, step: Step) {
        self.undo.push(step);
    }

    /// Depth of the undo stack.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// The undo stack's step labels, oldest first — the History
    /// panel's edit list, one entry per undoable step.
    #[must_use]
    pub fn undo_labels(&self) -> Vec<&str> {
        self.undo.iter().map(|step| step.label.as_str()).collect()
    }

    /// Rewinds the undo stack to `depth` steps: every step above it
    /// is undone (its captured inverse applied to the document) and
    /// dropped. Returns how many steps were undone.
    ///
    /// The dropped steps are *not* filed on the redo stack: a rewind
    /// discards them, so the redo stack is cleared — redoing after a
    /// rewind would replay a step against a document that no longer
    /// has the state it was captured in.
    ///
    /// A `depth` past the recorded steps is refused
    /// ([`EngineError::InvalidState`]) rather than clamped: a
    /// surface asking to rewind beyond the document's history is
    /// asking for something that does not exist.
    pub fn rewind_to(
        &mut self,
        document: &mut doc::Document,
        depth: usize,
    ) -> Result<usize, EngineError> {
        if depth > self.undo.len() {
            return Err(EngineError::InvalidState {
                name: "history.rewind".to_owned(),
                detail: format!(
                    "depth {depth} is past the {} recorded step(s)",
                    self.undo.len()
                ),
            });
        }
        let mut undone = 0usize;
        while self.undo.len() > depth {
            let Some(step) = self.undo.last() else {
                break;
            };
            step.delta.apply_inverse(document)?;
            self.undo.pop();
            undone += 1;
        }
        self.redo.clear();
        Ok(undone)
    }

    /// Depth of the redo stack.
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Drops all history (document closed or replaced).
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.open = None;
    }
}

impl std::fmt::Debug for History {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("History")
            .field("undo", &self.undo.len())
            .field("redo", &self.redo.len())
            .field("transaction_open", &self.open.is_some())
            .finish()
    }
}

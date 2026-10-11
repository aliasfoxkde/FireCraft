//! The editing session: document + registry + history + journal + dirty
//! flag, with transactions and restart-replay.
//!
//! `Session` is the only object a surface needs. It deliberately has
//! *no* mutable document accessor: every mutation goes through
//! [`Session::apply`], so UI, CLI, journal replay, and (M9) scripts
//! cannot diverge.

use doc::Document;

use crate::error::EngineError;
use crate::history::{Call, History};
use crate::journal::{self, Journal};
use crate::registry::CommandRegistry;

/// One open editing session over one document.
pub struct Session {
    registry: CommandRegistry,
    document: Document,
    history: History,
    journal: Journal,
    open_txn: Option<String>,
    dirty: bool,
}

impl Session {
    /// Opens a session over `document` with the built-in catalog
    /// installed.
    #[must_use]
    pub fn new(document: Document) -> Self {
        let mut registry = CommandRegistry::new();
        let _ = crate::catalog::install(&mut registry);
        Self::with_registry(document, registry)
    }

    /// Opens a session over a custom registry (test registries, later
    /// runtime-registered commands). Built-ins are installed first and
    /// a custom registration colliding with a catalog id is an error
    /// the registry itself already enforces.
    #[must_use]
    pub fn with_registry(document: Document, registry: CommandRegistry) -> Self {
        Self {
            registry,
            document,
            history: History::new(),
            journal: Journal::new(),
            open_txn: None,
            dirty: false,
        }
    }

    /// Applies one command. When a transaction is open the command
    /// *joins* it (the surface opened the group deliberately); undo and
    /// redo commit a pending group first so stacks never straddle one.
    pub fn apply(&mut self, name: &str, payload: serde_json::Value) -> Result<(), EngineError> {
        let delta = self
            .registry
            .execute(&mut self.document, name, payload.clone())?;
        let txn = self.open_txn.clone();
        self.journal.record(name, &payload, txn.as_deref());
        self.history.record(
            Call {
                name: name.to_owned(),
                payload,
            },
            delta,
        );
        self.dirty = true;
        Ok(())
    }

    /// Begins a transaction: subsequent applies (until
    /// [`Session::commit_transaction`]) fold into ONE undo step. An
    /// already-open transaction is committed first (it cannot nest).
    pub fn begin_transaction(&mut self, label: &str) -> Result<(), EngineError> {
        self.commit_open_transaction();
        self.history.begin_transaction(label)?;
        self.open_txn = Some(label.to_owned());
        Ok(())
    }

    /// Folds the open transaction into one undo step. An empty
    /// transaction is refused.
    pub fn commit_transaction(&mut self) -> Result<String, EngineError> {
        let label = self.history.commit_transaction()?;
        self.open_txn = None;
        Ok(label)
    }

    /// Aborts the open transaction: every command applied inside it is
    /// un-applied and its journal entries are rolled back — the
    /// document returns to its pre-transaction state.
    pub fn abort_transaction(&mut self) -> Result<(), EngineError> {
        let Some(label) = self.open_txn.take() else {
            return Ok(());
        };
        self.history.abort_transaction(&mut self.document)?;
        self.journal.rollback_txn(&label);
        Ok(())
    }

    /// `true` while a transaction is open.
    #[must_use]
    pub fn transaction_open(&self) -> bool {
        self.open_txn.is_some()
    }

    /// Undoes one step (a whole transaction counts as one).
    pub fn undo(&mut self) -> Result<String, EngineError> {
        self.commit_open_transaction();
        let step = self.history.pop_undo()?;
        step.delta.apply_inverse(&mut self.document)?;
        let label = step.label.clone();
        self.history.file_redo(step);
        self.dirty = true;
        Ok(label)
    }

    /// Redoes one step by re-applying its calls through the registry.
    pub fn redo(&mut self) -> Result<String, EngineError> {
        self.commit_open_transaction();
        let step = self.history.pop_redo()?;
        for call in &step.calls {
            self.registry
                .execute(&mut self.document, &call.name, call.payload.clone())?;
        }
        let label = step.label.clone();
        self.history.file_undo(step);
        self.dirty = true;
        Ok(label)
    }

    /// The document, for rendering and inspection only.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Registered catalog ids, sorted (menus, CLI help).
    #[must_use]
    pub fn command_names(&self) -> Vec<&'static str> {
        self.registry.names()
    }

    /// `true` when the catalog knows this id.
    #[must_use]
    pub fn has_command(&self, name: &str) -> bool {
        self.registry.contains(name)
    }

    /// The journal so far (JSON lines).
    #[must_use]
    pub fn journal_lines(&self) -> String {
        self.journal.to_lines()
    }

    /// Journal entry count.
    #[must_use]
    pub fn journal_len(&self) -> usize {
        self.journal.len()
    }

    /// Undo-stack depth.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.history.undo_len()
    }

    /// Redo-stack depth.
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.history.redo_len()
    }

    /// `true` when the document has edits not yet exported/saved.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clears the dirty flag (after a successful save/export).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// The session-restart path: fresh document + journal lines in,
    /// live session out. The replayed history is fully undoable — undo
    /// survives the restart (the M5 gate) — and the journal itself is
    /// retained so further edits APPEND to the replayed prefix; a
    /// truncated journal here would double-apply on the next restart.
    pub fn reopen(document: Document, lines: &str) -> Result<Self, EngineError> {
        let mut registry = CommandRegistry::new();
        crate::catalog::install(&mut registry)?;
        let journal = Journal::from_lines(lines)?;
        let mut session = Self::with_registry(document, registry);
        session.history =
            journal::replay(&mut session.document, &session.registry, journal.entries())?;
        session.journal = journal;
        session.dirty = !session.journal.is_empty();
        Ok(session)
    }

    fn commit_open_transaction(&mut self) {
        if self.open_txn.take().is_some() {
            // An implicit commit before new work: the group must have
            // content, and a contentless group is a surface bug — but
            // refusing here would leave the engine unusable, so the
            // empty group is dropped with the buffer.
            let _ = self.history.commit_transaction();
        }
    }
}

/// RAII handle over an open transaction: commits explicitly via
/// [`TransactionGuard::commit`], aborts on drop (document restored,
/// journal rolled back) — a surface that forgets to commit cannot
/// leave a half-applied group behind.
#[must_use = "dropping the guard aborts the transaction"]
pub struct TransactionGuard<'session> {
    session: &'session mut Session,
    armed: bool,
}

impl<'session> TransactionGuard<'session> {
    /// Begins a transaction on `session`.
    pub fn begin(session: &'session mut Session, label: &str) -> Result<Self, EngineError> {
        session.begin_transaction(label)?;
        Ok(Self {
            session,
            armed: true,
        })
    }

    /// Folds the transaction into one undo step and disarms the guard.
    pub fn commit(mut self) -> Result<String, EngineError> {
        self.armed = false;
        self.session.commit_transaction()
    }
}

impl std::ops::Deref for TransactionGuard<'_> {
    type Target = Session;

    fn deref(&self) -> &Session {
        self.session
    }
}

impl std::ops::DerefMut for TransactionGuard<'_> {
    fn deref_mut(&mut self) -> &mut Session {
        self.session
    }
}

impl Drop for TransactionGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            // Drop cannot report failure; an abort that cannot restore
            // (delta mismatch) leaves the mismatch for the next
            // operation to surface, and the buffer is gone either way.
            let _ = self.session.abort_transaction();
        }
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("pages", &self.document.pages.len())
            .field("objects", &self.document.registry.len())
            .field("undo", &self.history.undo_len())
            .field("redo", &self.history.redo_len())
            .field("journal", &self.journal.len())
            .field("transaction_open", &self.open_txn.is_some())
            .field("dirty", &self.dirty)
            .finish()
    }
}

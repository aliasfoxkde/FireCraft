//! Session-level behavior: the only mutation door, transaction
//! folding, dirty tracking, and reopen-through-MemoryFs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use support::{DocumentBytes, sample_document};

use engine::platform::FileSystem as _;
use engine::{Session, TransactionGuard};
use serde_json::json;

fn session() -> (Session, Vec<doc::ObjectId>) {
    let (document, ids) = sample_document();
    (Session::new(document), ids)
}

#[test]
fn apply_undo_redo_through_the_one_door() {
    let (mut session, ids) = session();
    let original = DocumentBytes::of(session.document());

    session
        .apply("object.setOpacity", json!({ "id": ids[0], "opacity": 400 }))
        .expect("apply");
    assert!(session.is_dirty());
    assert_eq!(session.undo_len(), 1);

    session.undo().expect("undo");
    assert_eq!(DocumentBytes::of(session.document()), original);

    session.redo().expect("redo");
    match session.document().registry.get(ids[0]) {
        Some(doc::ObjectData::Path(path)) => assert_eq!(path.opacity, 400),
        other => panic!("wrong kind: {other:?}"),
    }

    // Unknown catalog ids are refused at the door.
    let error = session
        .apply("object.transmogrify", json!({}))
        .expect_err("unknown refused");
    assert!(
        matches!(error, engine::EngineError::UnknownCommand { .. }),
        "{error}"
    );
}

#[test]
fn session_transactions_fold_into_one_undo_step() {
    let (mut session, _) = session();
    session.begin_transaction("toggle").expect("begin");
    assert!(session.transaction_open());
    session
        .apply(
            "cell.setVisible",
            json!({ "page": 0, "layer": 0, "cell": 0, "visible": false }),
        )
        .expect("apply 1");
    session
        .apply(
            "cell.setVisible",
            json!({ "page": 0, "layer": 0, "cell": 1, "visible": false }),
        )
        .expect("apply 2");
    let label = session.commit_transaction().expect("commit");
    assert_eq!(label, "toggle");
    assert_eq!(session.undo_len(), 1, "both applies are one undo step");
    // Journal carries the txn label so replay refolds identically.
    let lines = session.journal_lines();
    assert!(lines.contains("\"txn\":\"toggle\""), "{lines}");

    session.undo().expect("undo");
    assert_eq!(session.undo_len(), 0);
}

#[test]
fn undo_survives_session_restart_reopen() {
    let (document, ids) = sample_document();
    let mut session = Session::new(document);
    let original = DocumentBytes::of(session.document());
    session
        .apply("object.setOpacity", json!({ "id": ids[0], "opacity": 400 }))
        .expect("apply");
    session.begin_transaction("t").expect("begin");
    session
        .apply(
            "cell.setVisible",
            json!({ "page": 0, "layer": 1, "cell": 0, "visible": false }),
        )
        .expect("in-txn apply");
    session.commit_transaction().expect("commit");
    let edited = DocumentBytes::of(session.document());
    let lines = session.journal_lines();

    // --- restart ---
    let (fresh, _) = sample_document();
    let mut restarted = Session::reopen(fresh, &lines).expect("reopen");
    assert!(
        restarted.is_dirty(),
        "a replayed journal means unsaved edits"
    );
    assert_eq!(DocumentBytes::of(restarted.document()), edited);
    assert_eq!(restarted.undo_len(), 2);

    // Undo across the restart boundary, all the way to the original.
    restarted.undo().expect("undo");
    restarted.undo().expect("undo");
    assert_eq!(DocumentBytes::of(restarted.document()), original);
}

#[test]
fn journal_persists_through_injected_file_system() {
    let (document, ids) = sample_document();
    let mut session = Session::new(document);
    session
        .apply("object.setOpacity", json!({ "id": ids[0], "opacity": 333 }))
        .expect("apply");

    let fs = engine::platform::MemoryFs::new();
    let lines = session.journal_lines();
    fs.write_file("/tmp/session.fwj", lines.as_bytes())
        .expect("save journal");

    let (fresh, _) = sample_document();
    let saved = fs.read_file("/tmp/session.fwj").expect("load journal");
    let text = String::from_utf8(saved).expect("journal is utf-8");
    let restarted = Session::reopen(fresh, &text).expect("reopen");
    match restarted.document().registry.get(ids[0]) {
        Some(doc::ObjectData::Path(path)) => assert_eq!(path.opacity, 333),
        other => panic!("wrong kind: {other:?}"),
    }
    assert!(fs.exists("/tmp/session.fwj"));
    assert!(fs.read_file("/tmp/absent.fwj").is_err());
}

#[test]
fn mark_clean_round_trips() {
    let (mut session, ids) = session();
    assert!(!session.is_dirty());
    session
        .apply("object.setOpacity", json!({ "id": ids[0], "opacity": 100 }))
        .expect("apply");
    assert!(session.is_dirty());
    session.mark_clean();
    assert!(!session.is_dirty());
    session.undo().expect("undo");
    assert!(session.is_dirty(), "undo is also an edit");
}

#[test]
fn transaction_guard_closes_on_drop() {
    // TransactionGuard documents the RAII contract used by the shell:
    // on drop it commits when the closure succeeded, aborts (commits
    // nothing — the engine has no partial-undo abort yet) otherwise.
    let (mut session, _) = session();
    {
        let mut guard = TransactionGuard::begin(&mut session, "group").expect("begin");
        guard
            .apply(
                "cell.setVisible",
                json!({ "page": 0, "layer": 0, "cell": 0, "visible": false }),
            )
            .expect("apply");
        guard.commit().expect("commit");
    }
    assert!(!session.transaction_open());
    assert_eq!(session.undo_len(), 1);
}

#[test]
fn transaction_guard_aborts_on_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (mut session, _) = session();
    let outcome: Result<(), engine::EngineError> = {
        let mut guard = TransactionGuard::begin(&mut session, "doomed")
            .map_err(Box::<dyn std::error::Error>::from)?;
        guard
            .apply(
                "cell.setVisible",
                json!({ "page": 0, "layer": 0, "cell": 0, "visible": false }),
            )
            .expect("apply inside doomed txn");
        drop(guard);
        Err(engine::EngineError::NothingToUndo)
    };
    assert!(outcome.is_err());
    assert!(!session.transaction_open(), "guard aborted the transaction");
    assert_eq!(session.undo_len(), 0, "nothing was recorded");
    // And the document really went back: the cell is visible again.
    assert!(
        session.document().pages[0].layers.layers[0].cells[0].visible,
        "cell did not restore"
    );
    Ok(())
}

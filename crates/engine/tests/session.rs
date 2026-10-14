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
fn object_duplicate_clones_above_the_source() {
    let (mut session, ids) = session();
    let original = DocumentBytes::of(session.document());

    session
        .apply("object.duplicate", json!({ "id": ids[0] }))
        .expect("duplicate");

    // The copy sits directly above the source in the same cell and
    // carries its own live registry slot.
    let elements = &session.document().pages[0].layers.layers[0].cells[0].elements;
    assert_eq!(elements[0], ids[0], "source keeps its slot");
    let copy = elements[1];
    assert_ne!(copy, ids[0]);
    match (
        session.document().registry.get(ids[0]),
        session.document().registry.get(copy),
    ) {
        (Some(doc::ObjectData::Path(_)), Some(doc::ObjectData::Path(_))) => {}
        other => panic!("wrong kinds: {other:?}"),
    }

    // Undo releases the copy's slot and restores the document.
    session.undo().expect("undo");
    assert_eq!(DocumentBytes::of(session.document()), original);
    assert!(
        session.document().registry.get(copy).is_none(),
        "undo must release the duplicate's slot"
    );
}

#[test]
fn object_duplicate_deep_clones_groups() {
    let (mut session, ids) = session();
    session
        .apply(
            "object.group",
            json!({ "ids": [ids[0], ids[1]], "name": null }),
        )
        .expect("group");
    let group = session.document().pages[0].layers.layers[0].cells[0].elements[0];
    let original = DocumentBytes::of(session.document());

    session
        .apply("object.duplicate", json!({ "id": group }))
        .expect("duplicate");

    // The copy sits above the source, is a group, and owns fresh slots
    // for its members — editing the clone never touches the original's
    // children.
    let elements = &session.document().pages[0].layers.layers[0].cells[0].elements;
    assert_eq!(elements, &[group, elements[1]], "copy above source");
    let copy = elements[1];
    let (Some(doc::ObjectData::Group(source)), Some(doc::ObjectData::Group(clone))) = (
        session.document().registry.get(group),
        session.document().registry.get(copy),
    ) else {
        panic!("both sides are groups");
    };
    assert_eq!(
        source.children,
        vec![ids[0], ids[1]],
        "original keeps its members"
    );
    assert_ne!(source.children, clone.children, "the clone re-pointed");
    let cloned_children = clone.children.clone();
    for child in &cloned_children {
        assert!(matches!(
            session.document().registry.get(*child),
            Some(doc::ObjectData::Path(_))
        ));
        assert!(
            *child != ids[0] && *child != ids[1],
            "cloned children are fresh slots, not shared"
        );
    }

    // Undo retracts every minted slot and restores the exact bytes.
    session.undo().expect("undo");
    assert_eq!(DocumentBytes::of(session.document()), original);
    assert!(session.document().registry.get(copy).is_none());
    for child in cloned_children {
        assert!(
            session.document().registry.get(child).is_none(),
            "child slot {child:?} must retract with the group"
        );
    }
}

#[test]
fn object_translate_moves_a_path_and_refuses_an_empty_selection() {
    let (mut session, ids) = session();
    let original = DocumentBytes::of(session.document());

    // A lone path moves; undo restores the exact bytes.
    session
        .apply(
            "object.translate",
            json!({ "ids": [ids[0]], "dx": 15.0, "dy": -7.5 }),
        )
        .expect("translate");
    match session.document().registry.get(ids[0]) {
        Some(doc::ObjectData::Path(path)) => {
            let first = &path.contours[0].points[0].anchor;
            assert_eq!((first.x, first.y), (15.0, -7.5));
        }
        other => panic!("wrong kind: {other:?}"),
    }
    session.undo().expect("undo");
    assert_eq!(DocumentBytes::of(session.document()), original);
    session.redo().expect("redo");

    // A stale id refuses without mutating, but paired with a live id
    // the whole call fails atomically (validate before mutating is not
    // required — undo-absence is; here the caller sees the refusal).
    let error = session
        .apply(
            "object.translate",
            json!({ "ids": [], "dx": 1.0, "dy": 1.0 }),
        )
        .expect_err("empty ids refused");
    assert!(
        matches!(error, engine::EngineError::MissingTarget { .. }),
        "{error}"
    );
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

/// `path.setFillColor` over a gradient definition flattens it to the
/// solid family (decision 0018): the colour verb applies a solid, the
/// documented UI keeps gradients editable only per stop, and the
/// colour slot renders "if any" — never through a ramp.
#[test]
fn set_fill_color_collapses_a_gradient_fill_to_solid() {
    let (mut session, ids) = session();

    session
        .apply(
            "path.setFill",
            json!({
                "id": ids[0],
                "shape": "linear",
                "stops": [
                    { "position": 0.0, "color_raw": 0xFF18_1F2Bi64 },
                    { "position": 1.0, "color_raw": 0xFFE8_9B2Ci64 },
                ],
            }),
        )
        .expect("gradient");
    let gradient = match session.document().registry.get(ids[0]) {
        Some(doc::ObjectData::Path(path)) => path
            .paint
            .as_ref()
            .and_then(|paint| paint.fill.clone())
            .expect("gradient definition"),
        other => panic!("wrong kind: {other:?}"),
    };
    assert_eq!(gradient.shape, doc::FillShape::Linear);
    assert!(gradient.gradient.is_some());

    session
        .apply(
            "path.setFillColor",
            json!({ "id": ids[0], "color_raw": 0xFF00_FF00i64 }),
        )
        .expect("solid colour");

    {
        let path = match session.document().registry.get(ids[0]) {
            Some(doc::ObjectData::Path(path)) => path,
            other => panic!("wrong kind: {other:?}"),
        };
        let paint = path.paint.as_ref().expect("paint stays");
        assert_eq!(paint.fill_raw, 0xFF00_FF00);
        let fill = paint.fill.as_ref().expect("solid definition stays");
        assert_eq!(fill.shape, doc::FillShape::Solid);
        assert!(fill.gradient.is_none(), "the ramp clears");
        assert!(fill.dither.is_none());
        assert!(fill.shape_extended, "commands write the FSXi slot");
        assert!(
            path.fill_start.is_none() && path.fill_end.is_none(),
            "no handles were installed to begin with"
        );
    }

    // One undo step restores the whole gradient definition.
    session.undo().expect("undo");
    let restored = match session.document().registry.get(ids[0]) {
        Some(doc::ObjectData::Path(path)) => path.paint.clone().expect("paint"),
        other => panic!("wrong kind: {other:?}"),
    };
    assert_eq!(restored.fill, Some(gradient));
    assert_eq!(restored.fill_raw, 0xFF18_1F2B);

    session.redo().expect("redo");
    match session.document().registry.get(ids[0]) {
        Some(doc::ObjectData::Path(path)) => {
            let paint = path.paint.as_ref().expect("paint");
            assert_eq!(paint.fill_raw, 0xFF00_FF00);
            assert_eq!(
                paint.fill.as_ref().map(|fill| fill.shape),
                Some(doc::FillShape::Solid)
            );
        }
        other => panic!("wrong kind: {other:?}"),
    }
}

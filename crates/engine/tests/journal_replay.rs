//! Journal replay gate: apply → journal → "restart" (fresh document +
//! replay) reproduces the identical document, and undo/redo work across
//! the restart boundary. The M5 gate item in headless form — the
//! Session wrapper (next task) adds platform I/O around exactly these
//! calls.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use support::{Scratch, sample_document};

use engine::{Call, Delta, History, Journal};
use serde_json::json;

#[test]
fn replay_rebuilds_document_and_undo_history() {
    let (document, ids) = sample_document();
    let mut session = Scratch::open(document);
    let original = session.bytes();

    session.apply(
        "object.setOpacity",
        json!({ "id": ids[0], "opacity": 400 }),
        None,
    );
    session.apply(
        "layer.rename",
        json!({ "page": 0, "layer": 0, "name": "renamed" }),
        None,
    );
    // One transaction: two cell toggles land as ONE undo step.
    session.apply(
        "cell.setVisible",
        json!({ "page": 0, "layer": 1, "cell": 0, "visible": false }),
        Some("toggle"),
    );
    session.apply(
        "cell.setVisible",
        json!({ "page": 0, "layer": 1, "cell": 1, "visible": false }),
        Some("toggle"),
    );
    session.apply(
        "object.setOpacity",
        json!({ "id": ids[1], "opacity": 250 }),
        None,
    );
    let edited = session.bytes();
    assert_eq!(session.history.undo_len(), 4, "3 singles + 1 transaction");

    // --- restart: fresh document + history, replay the journal ---
    let mut restarted = session.restart();
    assert_eq!(restarted.bytes(), edited, "replay reproduces the document");
    assert_eq!(
        restarted.history.undo_len(),
        4,
        "history survives the restart"
    );
    assert_eq!(restarted.history.redo_len(), 0);

    // Undo all four steps across the restart boundary: the transaction
    // comes back as one step with both calls.
    assert_eq!(restarted.undo(), "object.setOpacity");
    assert_eq!(restarted.undo(), "toggle");
    assert_eq!(restarted.undo(), "layer.rename");
    assert_eq!(restarted.undo(), "object.setOpacity");
    assert_eq!(restarted.bytes(), original, "undo reaches the original");
    assert_eq!(restarted.history.redo_len(), 4);

    // Redo all four: back to `edited`, byte for byte.
    for _ in 0..4 {
        restarted.redo();
    }
    assert_eq!(
        restarted.bytes(),
        edited,
        "redo closes the loop after restart"
    );
}

#[test]
fn undo_after_restart_matches_live_undo() {
    let (document, ids) = sample_document();
    let mut live = Scratch::open(document);
    live.apply(
        "object.setOpacity",
        json!({ "id": ids[0], "opacity": 400 }),
        None,
    );
    live.apply(
        "cell.setVisible",
        json!({ "page": 0, "layer": 1, "cell": 0, "visible": false }),
        Some("t"),
    );
    live.apply(
        "cell.setVisible",
        json!({ "page": 0, "layer": 1, "cell": 1, "visible": false }),
        Some("t"),
    );
    let edited = live.bytes();

    live.undo();
    assert_eq!(live.undo(), "object.setOpacity");
    let undone = live.bytes();
    live.redo();
    live.redo();
    assert_eq!(live.bytes(), edited, "live redo closes the loop");

    // The restarted session's undo path lands on the same bytes as the
    // live session's undo path.
    let mut restarted = live.restart();
    restarted.undo();
    restarted.undo();
    assert_eq!(restarted.bytes(), undone);
}

#[test]
fn journal_round_trips_through_lines() {
    let mut journal = Journal::new();
    journal.record("object.setOpacity", &json!({ "id": 1 }), None);
    journal.record("cell.setVisible", &json!({ "visible": true }), Some("t"));
    let text = journal.to_lines();
    assert_eq!(text.lines().count(), 2);
    let parsed = Journal::from_lines(&text).expect("parses");
    assert_eq!(parsed.entries(), journal.entries());

    // Blank lines are skipped, gaps are refused: a truncated journal
    // cannot replay silently.
    let gapped = format!("{text}\n");
    assert_eq!(
        Journal::from_lines(&gapped).expect("blanks skipped").len(),
        2
    );
    let mut corrupted = journal.to_lines();
    corrupted.push_str("{\"seq\":9,\"name\":\"layer.rename\",\"payload\":{}}\n");
    let error = Journal::from_lines(&corrupted).expect_err("gap refused");
    assert!(
        matches!(error, engine::EngineError::Journal { .. }),
        "{error}"
    );
}

#[test]
fn float_payloads_replay_bit_exact() {
    // Journal lines are text; a payload's f64 fields must survive the
    // text round-trip bit-exactly or replay drifts a ULP from the live
    // session (found by panic_hunt seed 6018027439184454733: autoshape
    // geometry diverged because serde_json's default parser is not
    // correctly rounded — the `float_roundtrip` feature is mandatory).
    let (document, _ids) = sample_document();
    let mut session = Scratch::open(document);
    let start = 1.948_407_346_410_206_7_f64;
    session.apply(
        "autoshape.pie",
        json!({
            "centerX": -185.0, "centerY": -125.0,
            "outer": 119.0, "inner": 127.0,
            "start": start, "end": start - 1.0,
            "page": 1, "layer": 1, "cell": 0,
        }),
        None,
    );
    let live = session.bytes();
    let restarted = session.restart();
    assert_eq!(restarted.bytes(), live, "replay drifted from live bytes");
}

#[test]
fn empty_transaction_is_refused() {
    let mut history = History::new();
    history.begin_transaction("nothing").expect("opens");
    assert!(history.transaction_open());
    let error = history.commit_transaction().expect_err("empty refused");
    assert!(
        matches!(error, engine::EngineError::InvalidState { .. }),
        "{error}"
    );
    assert!(
        !history.transaction_open(),
        "buffer consumed by the failed commit"
    );
    history.begin_transaction("real").expect("opens again");
    history.record(
        Call {
            name: "layer.rename".into(),
            payload: json!({ "page": 0, "layer": 0, "name": "x" }),
        },
        Delta::Noop,
    );
    let label = history.commit_transaction().expect("commits");
    assert_eq!(label, "real");
    assert_eq!(history.undo_len(), 1);
}

#[test]
fn new_work_drops_redo() {
    let mut history = History::new();
    history.record(
        Call {
            name: "a".into(),
            payload: json!(null),
        },
        Delta::Noop,
    );
    let step = history.pop_undo().expect("undo");
    history.file_redo(step);
    assert_eq!(history.redo_len(), 1);
    history.record(
        Call {
            name: "b".into(),
            payload: json!(null),
        },
        Delta::Noop,
    );
    assert_eq!(history.redo_len(), 0, "redo invalidated by new work");
}

#[test]
fn nested_transaction_is_refused() {
    let mut history = History::new();
    history.begin_transaction("outer").expect("opens");
    let error = history
        .begin_transaction("inner")
        .expect_err("nested refused");
    assert!(
        matches!(error, engine::EngineError::InvalidState { .. }),
        "{error}"
    );
}

//! The decision-0001 protocol, inside the crate: an own-authored
//! handler script with the shipped scripts' shape (a top-level
//! `smartShape.operation` switch, `RegisterDragInsert(Insert…())`,
//! grow-by-read point lists) runs end to end against the prelude.
//! Real shipped scripts are exercised by the M9 task-5 corpus harness
//! (scratch-only — never referenced from committed code).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use scripting::{Limits, Outcome, ScriptEngine};

/// The spike protocol's script shape, own-authored (clean-room).
const HANDLER: &str = r#"
switch (smartShape.operation) {
    case "BeginDragInsert":
        RegisterDragInsert(InsertSmartShapeAt());
        break;
    default:
        throw new Error("unknown operation " + smartShape.operation);
}

function InsertSmartShapeAt() {
    var pts = __hostArray(function (i) {
        return { x: 0, y: 0, primary: null, secondary: null };
    });
    for (var i = 0; i < 4; i++) {
        pts[pts.length++] = { x: i, y: i * 2, primary: null, secondary: null };
    }
    return { connectedPts: pts, info: { name: "probe-shape" } };
}

function RegisterDragInsert(insert) {
    if (insert.connectedPts.length !== 4) {
        throw new Error("expected 4 points, got " + insert.connectedPts.length);
    }
    fw.__log("registered", insert.info.name);
}
"#;

#[test]
fn handler_script_runs_end_to_end() {
    let mut engine = ScriptEngine::new(Limits::default()).expect("engine");
    let report = engine.run("handler.jsf", "smartShape.operation = 'BeginDragInsert';\n");
    assert!(report.completed(), "{:?}", report.message);
    let report = engine.run("handler.jsf", HANDLER);
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["registered probe-shape"]);
    assert!(report.host_calls > 0);
}

#[test]
fn unknown_operation_throws_typed() {
    let mut engine = ScriptEngine::new(Limits::default()).expect("engine");
    let report = engine.run("handler.jsf", "smartShape.operation = 'Frobnicate';\n");
    assert!(report.completed());
    let report = engine.run("handler.jsf", HANDLER);
    assert_eq!(report.outcome, Outcome::Threw);
    let message = report.message.expect("message");
    assert!(
        message.contains("unknown operation Frobnicate"),
        "{message}"
    );
}

/// The host-array semantic: reading an unassigned index below
/// `length` materializes a default element (grow-by-read).
#[test]
fn host_array_materializes_on_read() {
    let mut engine = ScriptEngine::new(Limits::default()).expect("engine");
    let report = engine.run(
        "array.jsf",
        r#"
var pts = __hostArray(function (i) { return { x: i * 10, y: 0 }; });
pts.length = 3;
var a = pts[0];
var c = pts[2];
fw.__log(a.x + "," + c.x + "," + pts[1].x);
"#,
    );
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["0,20,10"]);
}

/// Engine reuse: a second run on the same context sees globals the
/// first run defined (the long-running-session posture).
#[test]
fn engine_is_reusable_across_runs() {
    let mut engine = ScriptEngine::new(Limits::default()).expect("engine");
    let report = engine.run("one.jsf", "globalThis.__marker = 41;");
    assert!(report.completed(), "{:?}", report.message);
    let report = engine.run("two.jsf", "fw.__log(__marker + 1);");
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["42"]);
}

/// Shipped-file normalization: UTF-8 BOM + CRLF.
#[test]
fn prepare_source_strips_bom_and_normalizes_crlf() {
    let raw = "\u{feff}var a = 1;\r\nvar b = 2;\r\n";
    let prepared = scripting::prepare_source(raw);
    assert!(prepared.starts_with("var a = 1;\nvar b = 2;\n"));
    // And it still runs.
    let mut engine = ScriptEngine::new(Limits::default()).expect("engine");
    let report = engine.run("bom.jsf", raw);
    assert!(report.completed(), "{:?}", report.message);
}

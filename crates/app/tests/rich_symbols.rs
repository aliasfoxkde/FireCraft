//! The M10 task 5 stopping condition: a rich symbol from a scripted
//! session — definition script installed, instance placed, parameter
//! changed, re-derived through the M9 host — stays editable, replays
//! byte-exactly through the journal (no JS engine on replay: the
//! re-derivation's effects ARE registry commands), and round-trips
//! the `.fw.png` stream including the `JSVv` script and the `DTAv`
//! parameter store.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::Editor;
use doc::ObjectData;
use engine::Session;
use scripting::{Limits, ScriptEngine, rederive_symbol};

/// The definition script, corpus-shaped: declare the parameter
/// defaults, write them into the named members when the host runs
/// opCode 1, mirror current values when it runs opCode 2.
const SCRIPT: &str = r##"
var defaults = [
    { name: "label", type: "text", value: "Click me" },
    { name: "color", type: "color", value: "#204a87" }
];
var values = Widget.elem.customData["currentValues"];
if (Widget.opCode === 1) {
    values = defaults;
    Widget.elem.customData["currentValues"] = defaults;
}
for (var i = 0; i < values.length; i++) {
    var v = values[i];
    if (v.name === "label") { Widget.GetObjectByName("label").textChars = v.value; }
    if (v.name === "color") { Widget.GetObjectByName("plate").pathAttributes.fillColor = v.value; }
}
"##;

/// The golden chain: draw two named objects, make them symbol "btn",
/// install the script, place a second instance whose defaults are
/// re-derived from scratch (opCode 1), then change the first
/// instance's parameter and re-derive (opCode 2). The definition is
/// shared content, so ordering matters: the apply pass runs LAST and
/// its values are what the members carry. Every step rides
/// `Editor::apply` or the host drain; the journal captures the whole
/// chain as commands.
fn rich_session() -> Editor {
    let mut editor = Editor::untitled();

    editor
        .apply(
            "object.createPath",
            serde_json::json!({
                "kind": "rect",
                "x": 0.0, "y": 0.0, "width": 40.0, "height": 16.0,
                "page": 0, "layer": 0, "cell": 0, "name": "plate",
            }),
        )
        .unwrap();
    editor
        .apply(
            "text.createText",
            serde_json::json!({
                "page": 0, "layer": 0, "cell": 0,
                "x": 4.0, "y": 4.0, "width": 32.0, "height": 8.0,
                "text": "label", "name": "label",
            }),
        )
        .unwrap();
    let members = editor.current_cell().unwrap().elements.clone();
    editor
        .apply(
            "symbol.create",
            serde_json::json!({
                "ids": members, "name": "btn", "symbol_type": "graphic",
            }),
        )
        .unwrap();
    editor
        .apply(
            "symbol.setScript",
            serde_json::json!({ "symbol": 0, "source": SCRIPT }),
        )
        .unwrap();

    let mut engine = ScriptEngine::new(Limits::default()).unwrap();

    // Instance 2: placed fresh, then defaults written (opCode 1).
    editor
        .place_symbol(0, geom::Point::new(120.0, 40.0))
        .unwrap();
    let second = *editor.current_cell().unwrap().elements.last().unwrap();
    let report = rederive_symbol(&mut editor.session, &mut engine, second, 1);
    assert!(
        report.completed(),
        "defaults pass failed: {:?}",
        report.message
    );

    // Instance 1: the one symbol.create left on the page. Change one
    // parameter, then apply current values (opCode 2) — last, so the
    // shared definition carries the changed value.
    let first = editor.current_cell().unwrap().elements[0];
    editor
        .apply(
            "symbol.setParam",
            serde_json::json!({
                "id": first, "name": "label", "kind": "text", "value": "Buy now",
            }),
        )
        .unwrap();
    let report = rederive_symbol(&mut editor.session, &mut engine, first, 2);
    assert!(
        report.completed(),
        "apply pass failed: {:?}",
        report.message
    );
    editor
}

fn member(editor: &Editor, name: &str) -> doc::ObjectId {
    for id in &editor.document().symbols[0].layers.layers[0].cells[0].elements {
        if let Some(data) = editor.document().registry.get(*id) {
            let named = match data {
                ObjectData::Path(path) => path.name.as_deref() == Some(name),
                ObjectData::Text(text) => text.name.as_deref() == Some(name),
                _ => false,
            };
            if named {
                return *id;
            }
        }
    }
    panic!("no member named {name}");
}

#[test]
fn rederivation_mutates_the_definition_and_stays_editable() {
    let mut editor = rich_session();

    // The definition's members carry the re-derived content.
    let label = member(&editor, "label");
    match editor.document().registry.get(label) {
        Some(doc::ObjectData::Text(text)) => assert_eq!(text.runs[0].text, "Buy now"),
        other => panic!("expected the label text, got {other:?}"),
    }
    let plate = member(&editor, "plate");
    match editor.document().registry.get(plate) {
        Some(doc::ObjectData::Path(path)) => {
            assert_eq!(
                path.paint.as_ref().unwrap().fill_raw,
                0xFF874A20,
                "#204a87 lands as raw ABGR"
            );
        }
        other => panic!("expected the plate path, got {other:?}"),
    }

    // Both instances kept (or gained) their parameter stores — the
    // opCode-1 pass wrote defaults into the second instance.
    let cells = &editor.document().pages[0].layers.layers[0].cells[0].elements;
    assert_eq!(cells.len(), 2, "two instances share the definition");
    for id in cells {
        match editor.document().registry.get(*id) {
            Some(doc::ObjectData::SymbolInstance(instance)) => {
                let values = instance
                    .custom_data
                    .get("currentValues")
                    .expect("currentValues on every instance");
                assert!(values.contains(r#""name":"label""#));
            }
            other => panic!("expected a symbol instance, got {other:?}"),
        }
    }

    // Still editable: a plain mutation on the definition (the shared
    // content model) goes through the normal command door.
    editor
        .apply(
            "text.setText",
            serde_json::json!({ "id": label, "text": "Hand-edited" }),
        )
        .unwrap();
    match editor.document().registry.get(label) {
        Some(doc::ObjectData::Text(text)) => assert_eq!(text.runs[0].text, "Hand-edited"),
        other => panic!("expected the edited text, got {other:?}"),
    }
}

#[test]
fn the_rich_golden_replays_and_round_trips() {
    let editor = rich_session();
    let steps = editor.steps_text();

    // Journal replay reproduces the whole chain — create, script
    // install, parameter set, re-derivation effects — with no JS
    // engine in the loop (decision 0011: derivation lands as
    // registry commands).
    let replayed = Session::reopen(doc::Document::default(), &steps).expect("replays");
    assert_eq!(
        serde_json::to_vec(replayed.document()).unwrap(),
        serde_json::to_vec(editor.session.document()).unwrap(),
        "replayed document must equal the original, byte for byte"
    );

    // The file boundary: the `JSVv` script and `DTAv` parameter
    // stores survive save → open → save byte-stably.
    let bytes = editor.save().unwrap();
    let reopened = Editor::open(&bytes).unwrap();
    assert_eq!(
        reopened.save().unwrap(),
        bytes,
        "the saved file re-emits identically"
    );
    let definition = &reopened.document().symbols[0];
    assert_eq!(
        definition.op_code.as_deref(),
        Some(SCRIPT),
        "the definition carries its script across the file boundary"
    );
    assert_eq!(
        reopened.document().pages[0].layers.layers[0].cells[0]
            .elements
            .len(),
        2,
        "both instances survive"
    );
}

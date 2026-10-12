//! The rich-symbol battery: a definition with an `op_code` script
//! re-derives through [`scripting::rederive_symbol`] in both modes —
//! opCode 1 writes defaults, opCode 2 applies the instance's current
//! values. Mutations land as registry commands, so scripted
//! re-derivation ≡ direct invocation ≡ journal replay (byte-equal),
//! and every refusal is typed with the document untouched.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use doc::{Canvas, Document, Layer, LayerList, ObjectData, Page, PageInfo, SymbolInstanceObject};
use engine::Session;
use scripting::{Limits, Outcome, ScriptEngine, rederive_symbol};

// --- fixtures -----------------------------------------------------------

fn page_info() -> PageInfo {
    PageInfo {
        version: None,
        revision: None,
        frame_count: 1,
        frame_count_duplicate: 1,
        request_id: 0,
        canvas: Canvas::default(),
        onion_before: 0,
        onion_after: 0,
        frame_durations: Vec::new(),
    }
}

/// 96x64 canvas, one page, two empty layers ("bottom", "top").
fn base_document() -> Document {
    let mut document = Document::default();
    document.info.canvas = Canvas {
        x: 0.0,
        y: 0.0,
        width: 96.0,
        height: 64.0,
        resolution: 72.0,
        resolution_unit: 0,
        background_raw: 0,
    };
    document.info.frame_count = 1;
    document.pages.push(Page {
        info: page_info(),
        name: Some("Page 1".to_owned()),
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList {
            layers: vec![
                Layer {
                    name: "bottom".to_owned(),
                    cells: vec![doc::Cell::default()],
                    ..Layer::default()
                },
                Layer {
                    name: "top".to_owned(),
                    cells: vec![doc::Cell::default()],
                    ..Layer::default()
                },
            ],
        },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    });
    document
}

fn bytes(document: &Document) -> Vec<u8> {
    serde_json::to_vec(document).expect("document serializes")
}

fn engine() -> ScriptEngine {
    ScriptEngine::new(Limits::default()).expect("engine")
}

/// Seeds the rich-symbol baseline through the registry: a named rect
/// "plate" and a named text "label" become symbol "btn"; a fresh
/// instance sits on `top`. `script: None` leaves the definition plain.
struct Rich {
    document: Document,
    /// The placed instance (there is exactly one).
    instance: doc::ObjectId,
}

fn rich_document(script: Option<&str>) -> Rich {
    let mut session = Session::new(base_document());
    session
        .apply(
            "object.createPath",
            serde_json::json!({
                "kind": "rect",
                "x": 0.0, "y": 0.0, "width": 40.0, "height": 16.0,
                "page": 0, "layer": 1, "cell": 0, "name": "plate",
            }),
        )
        .expect("plate");
    session
        .apply(
            "text.createText",
            serde_json::json!({
                "page": 0, "layer": 1, "cell": 0,
                "x": 4.0, "y": 4.0, "width": 32.0, "height": 8.0,
                "text": "label", "name": "label",
            }),
        )
        .expect("label");
    let members = session.document().pages[0].layers.layers[1].cells[0]
        .elements
        .clone();
    session
        .apply(
            "symbol.create",
            serde_json::json!({
                "ids": members,
                "name": "btn",
                "symbol_type": "graphic",
            }),
        )
        .expect("symbol.create");
    let number = session.document().symbols[0].symbol_number;
    if let Some(script) = script {
        session
            .apply(
                "symbol.setScript",
                serde_json::json!({ "symbol": number, "source": script }),
            )
            .expect("setScript");
    }
    session
        .apply(
            "symbol.createInstance",
            serde_json::json!({
                "symbol": number,
                "page": 0, "layer": 1, "cell": 0,
                "x": 8.0, "y": 8.0, "name": "btn 1",
            }),
        )
        .expect("createInstance");
    let cell = &session.document().pages[0].layers.layers[1].cells[0].elements;
    let instance = cell
        .iter()
        .copied()
        .find(|id| {
            matches!(
                session.document().registry.get(*id),
                Some(ObjectData::SymbolInstance(_))
            )
        })
        .expect("one instance placed");
    Rich {
        document: session.document().clone(),
        instance,
    }
}

/// The definition's named member, by registry walk.
fn member(document: &Document, name: &str) -> doc::ObjectId {
    for id in &document.symbols[0].layers.layers[0].cells[0].elements {
        if let Some(data) = document.registry.get(*id) {
            let matches = match data {
                ObjectData::Path(path) => path.name.as_deref() == Some(name),
                ObjectData::Text(text) => text.name.as_deref() == Some(name),
                _ => false,
            };
            if matches {
                return *id;
            }
        }
    }
    panic!("no member named {name}");
}

fn instance_data(document: &Document, id: doc::ObjectId) -> SymbolInstanceObject {
    match document.registry.get(id) {
        Some(ObjectData::SymbolInstance(instance)) => instance.clone(),
        other => panic!("expected a symbol instance, got {other:?}"),
    }
}

/// The defaults pass (the corpus script shape: declare defaults, write
/// them into the members when opCode says "set defaults").
const DEFAULTS_SCRIPT: &str = r##"
var defaults = [
    { name: "label", type: "text", value: "Click me" },
    { name: "color", type: "color", value: "#204a87" }
];
if (Widget.opCode === 1) {
    Widget.elem.customData["currentValues"] = defaults;
    Widget.GetObjectByName("label").textChars = "Click me";
    Widget.GetObjectByName("plate").pathAttributes.fillColor = "#204a87";
}
"##;

/// The apply pass: mirror each current value into its named member.
const APPLY_SCRIPT: &str = r##"
var values = Widget.elem.customData["currentValues"];
for (var i = 0; i < values.length; i++) {
    var v = values[i];
    if (v.name === "label") { Widget.GetObjectByName("label").textChars = v.value; }
    if (v.name === "color") { Widget.GetObjectByName("plate").pathAttributes.fillColor = v.value; }
}
"##;

// --- the two op codes -----------------------------------------------------

#[test]
fn defaults_pass_writes_members_and_current_values() {
    let rich = rich_document(Some(DEFAULTS_SCRIPT));
    let mut session = Session::new(rich.document);
    let mut script = engine();
    let report = rederive_symbol(&mut session, &mut script, rich.instance, 1);
    assert!(report.completed(), "{:?}", report.message);

    let label = member(session.document(), "label");
    let plate = member(session.document(), "plate");
    match session.document().registry.get(label) {
        Some(ObjectData::Text(text)) => assert_eq!(text.runs[0].text, "Click me"),
        other => panic!("expected text, got {other:?}"),
    }
    match session.document().registry.get(plate) {
        Some(ObjectData::Path(path)) => {
            // #204a87 in ABGR: a=ff, b=87, g=4a, r=20.
            assert_eq!(path.paint.as_ref().expect("paint").fill_raw, 0xFF874A20);
        }
        other => panic!("expected path, got {other:?}"),
    }
    // The parameter array marshals through JSON into customData.
    let data = instance_data(session.document(), rich.instance);
    let values = data
        .custom_data
        .get("currentValues")
        .expect("currentValues");
    assert_eq!(
        values,
        r##"[{"name":"label","type":"text","value":"Click me"},{"name":"color","type":"color","value":"#204a87"}]"##
    );
}

#[test]
fn apply_pass_mirrors_current_values_into_members() {
    let rich = rich_document(Some(APPLY_SCRIPT));
    let mut session = Session::new(rich.document);

    // A parameter change first: plain journaled mutation.
    session
        .apply(
            "symbol.setParam",
            serde_json::json!({
                "id": rich.instance,
                "name": "label", "kind": "text", "value": "Buy now",
            }),
        )
        .expect("setParam");
    session
        .apply(
            "symbol.setParam",
            serde_json::json!({
                "id": rich.instance,
                "name": "color", "kind": "color", "value": "#a40000",
            }),
        )
        .expect("setParam");

    let mut script = engine();
    let report = rederive_symbol(&mut session, &mut script, rich.instance, 2);
    assert!(report.completed(), "{:?}", report.message);

    let label = member(session.document(), "label");
    let plate = member(session.document(), "plate");
    match session.document().registry.get(label) {
        Some(ObjectData::Text(text)) => assert_eq!(text.runs[0].text, "Buy now"),
        other => panic!("expected text, got {other:?}"),
    }
    match session.document().registry.get(plate) {
        Some(ObjectData::Path(path)) => {
            assert_eq!(path.paint.as_ref().expect("paint").fill_raw, 0xFF0000A4);
        }
        other => panic!("expected path, got {other:?}"),
    }
}

// --- the contract: scripted ≡ direct ≡ replay -----------------------------

#[test]
fn rederivation_is_direct_and_replay_equivalent() {
    let rich = rich_document(Some(DEFAULTS_SCRIPT));

    // Scripted: one rederive call.
    let mut scripted = Session::new(rich.document.clone());
    let mut script_engine = engine();
    let report = rederive_symbol(&mut scripted, &mut script_engine, rich.instance, 1);
    assert!(report.completed(), "{:?}", report.message);

    // Direct: the same registry commands in the queue order the
    // script produced (customData first, then the member writes).
    let mut direct = Session::new(rich.document.clone());
    direct
        .apply(
            "symbol.setCustomData",
            serde_json::json!({
                "id": rich.instance,
                "key": "currentValues",
                "value": r##"[{"name":"label","type":"text","value":"Click me"},{"name":"color","type":"color","value":"#204a87"}]"##,
            }),
        )
        .expect("setCustomData");
    direct
        .apply(
            "text.setText",
            serde_json::json!({ "id": member(direct.document(), "label"), "text": "Click me" }),
        )
        .expect("setText");
    direct
        .apply(
            "object.setFill",
            serde_json::json!({
                "id": member(direct.document(), "plate"),
                "color_raw": 0xFF874A20i64,
            }),
        )
        .expect("setFill");

    assert_eq!(bytes(scripted.document()), bytes(direct.document()));
    let replay =
        Session::reopen(rich.document, &scripted.journal_lines()).expect("journal reopens");
    assert_eq!(bytes(scripted.document()), bytes(replay.document()));
}

// --- propString -------------------------------------------------------------

#[test]
fn prop_string_round_trips_the_parameter_array() {
    // The exercise rides INSIDE the definition script: the propString
    // write queues through customData and drains with the run.
    let source = &format!(
        "{APPLY_SCRIPT}\nfw.__log(Widget.propString); \
         Widget.propString = 'label&|&Hola&|&text'; fw.__log(Widget.propString);"
    );
    let rich = rich_document(Some(source));
    let mut session = Session::new(rich.document);

    // One seeded parameter, so the getter has an array to encode.
    session
        .apply(
            "symbol.setParam",
            serde_json::json!({
                "id": rich.instance,
                "name": "label", "kind": "text", "value": "Buy now",
            }),
        )
        .expect("setParam");

    let mut script = engine();
    let report = rederive_symbol(&mut session, &mut script, rich.instance, 2);
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(
        report.notes,
        vec!["label&|&Buy now&|&text", "label&|&Hola&|&text"]
    );
    // The write landed on the instance through the queue.
    let data = instance_data(session.document(), rich.instance);
    assert_eq!(
        data.custom_data
            .get("currentValues")
            .expect("currentValues"),
        r#"[{"name":"label","value":"Hola","type":"text"}]"#
    );
}

// --- the guarantees -------------------------------------------------------

#[test]
fn refusals_are_typed_and_leave_the_document_untouched() {
    // A plain symbol: no script, nothing to run.
    let rich = rich_document(None);
    let mut session = Session::new(rich.document.clone());
    let mut script = engine();
    let report = rederive_symbol(&mut session, &mut script, rich.instance, 1);
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(
        report
            .message
            .expect("message")
            .contains("carries no rich-symbol script")
    );
    assert_eq!(bytes(session.document()), bytes(&rich.document));
    assert_eq!(session.journal_len(), 0);

    // Out-of-contract op code.
    let mut session = Session::new(rich.document.clone());
    let mut script = engine();
    let report = rederive_symbol(&mut session, &mut script, rich.instance, 3);
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(report.message.expect("message").contains("out of contract"));
    assert_eq!(bytes(session.document()), bytes(&rich.document));

    // A non-instance target: the definition's "plate" path.
    let plate = member(session.document(), "plate");
    let mut script = engine();
    let report = rederive_symbol(&mut session, &mut script, plate, 1);
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(
        report
            .message
            .expect("message")
            .contains("not a symbol instance")
    );
}

#[test]
fn throwing_script_queues_but_never_applies() {
    // The script queues a mutation, then dies: the document is
    // untouched and the journal is empty (the run-completed gate).
    let source = r#"
Widget.GetObjectByName("label").textChars = "ghost";
throw new Error("aborted");
"#;
    let rich = rich_document(Some(source));
    let mut session = Session::new(rich.document.clone());
    let mut script = engine();
    let report = rederive_symbol(&mut session, &mut script, rich.instance, 1);
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(report.message.expect("message").contains("aborted"));
    assert_eq!(bytes(session.document()), bytes(&rich.document));
    assert_eq!(session.journal_len(), 0);
}

#[test]
fn unknown_member_name_throws_before_any_mutation() {
    let source = r#"
Widget.GetObjectByName("no-such-member").textChars = "x";
"#;
    let rich = rich_document(Some(source));
    let mut session = Session::new(rich.document.clone());
    let mut script = engine();
    let report = rederive_symbol(&mut session, &mut script, rich.instance, 1);
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(
        report
            .message
            .expect("message")
            .contains("no object named no-such-member")
    );
    assert_eq!(bytes(session.document()), bytes(&rich.document));
    assert_eq!(session.journal_len(), 0);
}

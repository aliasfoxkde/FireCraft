//! The mutating-host battery: every implemented mutator runs three
//! ways — through a script, through a direct registry invocation, and
//! through journal replay — and the three documents must be
//! byte-equal (the host-model mutation contract). Also pinned: the
//! failing-script guarantee (untouched document), setUpRevert/revert,
//! and the selection-carrying mutations.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use doc::{
    Canvas, Cell, Contour, Document, Layer, LayerList, ObjectData, Page, PageInfo, PathObject,
    PathPoint,
};
use engine::Session;
use geom::Point;
use scripting::{Limits, Outcome, ScriptEngine, run_in_session};

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

fn square(document: &mut Document, x: f64, y: f64, name: &str) -> doc::ObjectId {
    let point = |px: f64, py: f64| PathPoint {
        anchor: Point::new(px, py),
        primary: None,
        secondary: None,
        is_curve: false,
    };
    let mut path = PathObject::neutral(vec![Contour {
        closed: true,
        point_count: 4,
        points: vec![
            point(x, y),
            point(x + 8.0, y),
            point(x + 8.0, y + 8.0),
            point(x, y + 8.0),
        ],
    }]);
    path.name = Some(name.to_owned());
    document.registry.insert(ObjectData::Path(path))
}

fn layer(name: &str, elements: Vec<doc::ObjectId>) -> Layer {
    Layer {
        name: name.to_owned(),
        display: true,
        cells: vec![Cell {
            locked: false,
            visible: true,
            elements,
        }],
        ..Layer::default()
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
            layers: vec![layer("bottom", Vec::new()), layer("top", Vec::new())],
        },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    });
    document
}

/// Two layers; `bottom` carries two 8x8 squares.
fn two_square_document() -> Document {
    let mut document = base_document();
    let a = square(&mut document, 10.0, 10.0, "a");
    let b = square(&mut document, 30.0, 10.0, "b");
    let page = &mut document.pages[0];
    page.layers.layers[0].cells[0].elements = vec![a, b];
    document
}

fn bytes(document: &Document) -> Vec<u8> {
    serde_json::to_vec(document).expect("document serializes")
}

fn engine() -> ScriptEngine {
    ScriptEngine::new(Limits::default()).expect("engine")
}

/// Runs `source` on a fresh session over `base`, returning the session.
fn scripted(base: Document, source: &str) -> Session {
    let mut session = Session::new(base);
    let mut script = engine();
    let report = run_in_session(&mut session, &mut script, "mutate.jsf", source);
    assert!(report.completed(), "script failed: {:?}", report.message);
    session
}

// --- the contract: script ≡ direct ≡ replay ------------------------------

#[test]
fn add_new_text_is_direct_and_replay_equivalent() {
    let source = "fw.getDocumentDOM().addNewText(10, 20, 100, 50, 'hello', 'greeting');";
    let scripted_session = scripted(base_document(), source);

    let mut direct = Session::new(base_document());
    direct
        .apply(
            "text.createText",
            serde_json::json!({
                "page": 0, "layer": 1, "cell": 0,
                "x": 10.0, "y": 20.0, "width": 100.0, "height": 50.0,
                "text": "hello", "name": "greeting",
            }),
        )
        .expect("direct apply");

    let replay = Session::reopen(base_document(), &scripted_session.journal_lines())
        .expect("journal reopens");

    assert_eq!(bytes(scripted_session.document()), bytes(direct.document()));
    assert_eq!(bytes(scripted_session.document()), bytes(replay.document()));
}

#[test]
fn add_new_rectangle_is_direct_and_replay_equivalent() {
    let scripted_session = scripted(
        base_document(),
        "fw.getDocumentDOM().addNewRectangle(4, 6, 32, 16, 'plate');",
    );

    let mut direct = Session::new(base_document());
    direct
        .apply(
            "object.createPath",
            serde_json::json!({
                "kind": "rect",
                "x": 4.0, "y": 6.0, "width": 32.0, "height": 16.0,
                "page": 0, "layer": 1, "cell": 0, "name": "plate",
            }),
        )
        .expect("direct apply");

    assert_eq!(bytes(scripted_session.document()), bytes(direct.document()));
    let replay = Session::reopen(base_document(), &scripted_session.journal_lines())
        .expect("journal reopens");
    assert_eq!(bytes(scripted_session.document()), bytes(replay.document()));
}

#[test]
fn add_and_delete_layer_are_direct_and_replay_equivalent() {
    // Corpus shape: addNewLayer(null, false).
    let scripted_session = scripted(
        base_document(),
        "var dom = fw.getDocumentDOM(); dom.addNewLayer(null, false); dom.deleteLayer(0);",
    );

    let mut direct = Session::new(base_document());
    direct
        .apply(
            "layer.add",
            serde_json::json!({ "page": 0, "name": null, "index": null }),
        )
        .expect("layer.add");
    direct
        .apply("layer.delete", serde_json::json!({ "page": 0, "layer": 0 }))
        .expect("layer.delete");

    assert_eq!(bytes(scripted_session.document()), bytes(direct.document()));
    let replay = Session::reopen(base_document(), &scripted_session.journal_lines())
        .expect("journal reopens");
    assert_eq!(bytes(scripted_session.document()), bytes(replay.document()));
    // The scripted walk confirms the surviving layer list.
    assert_eq!(scripted_session.document().pages[0].layers.layers.len(), 2);
    assert_eq!(
        scripted_session.document().pages[0].layers.layers[0].name,
        "top"
    );
}

#[test]
fn move_selection_to_layer_is_direct_and_replay_equivalent() {
    let scripted_session = scripted(
        two_square_document(),
        "var dom = fw.getDocumentDOM(); dom.selectAllOnLayer(0); dom.moveSelectionToLayer(1, false, 'none', -1);",
    );

    // Direct: the same selection command the script queues, then the
    // two squares move bottom → top by id.
    let mut direct = Session::new(two_square_document());
    direct
        .apply(
            "selection.selectAllOnLayer",
            serde_json::json!({
                "page": 0, "layer": 0, "frame": 0,
                "remember_selection": false, "toggle_selection": false,
            }),
        )
        .expect("selectAllOnLayer");
    for id in direct.document().pages[0].layers.layers[0].cells[0]
        .elements
        .clone()
    {
        direct
            .apply(
                "object.moveToCell",
                serde_json::json!({ "id": id, "page": 0, "layer": 1, "cell": 0, "index": null }),
            )
            .expect("moveToCell");
    }

    assert_eq!(bytes(scripted_session.document()), bytes(direct.document()));
    let replay = Session::reopen(two_square_document(), &scripted_session.journal_lines())
        .expect("journal reopens");
    assert_eq!(bytes(scripted_session.document()), bytes(replay.document()));
    assert!(
        direct.document().pages[0].layers.layers[0].cells[0]
            .elements
            .is_empty()
    );
    assert_eq!(
        direct.document().pages[0].layers.layers[1].cells[0]
            .elements
            .len(),
        2
    );
}

#[test]
fn path_union_folds_the_selection() {
    let scripted_session = scripted(
        two_square_document(),
        "var dom = fw.getDocumentDOM(); dom.selectAllOnLayer(0); dom.pathUnion();",
    );

    // Direct: the same selection command the script queues, then the
    // same fold through the registry.
    let mut direct = Session::new(two_square_document());
    direct
        .apply(
            "selection.selectAllOnLayer",
            serde_json::json!({
                "page": 0, "layer": 0, "frame": 0,
                "remember_selection": false, "toggle_selection": false,
            }),
        )
        .expect("selectAllOnLayer");
    let ids = direct.document().pages[0].layers.layers[0].cells[0]
        .elements
        .clone();
    direct
        .apply(
            "path.boolean",
            serde_json::json!({ "a": ids[0], "b": ids[1], "op": "union" }),
        )
        .expect("boolean");

    assert_eq!(bytes(scripted_session.document()), bytes(direct.document()));
    let replay = Session::reopen(two_square_document(), &scripted_session.journal_lines())
        .expect("journal reopens");
    assert_eq!(bytes(scripted_session.document()), bytes(replay.document()));
    assert_eq!(
        direct.document().pages[0].layers.layers[0].cells[0]
            .elements
            .len(),
        1
    );
}

#[test]
fn set_text_auto_expand_maps_to_text_set_attrs() {
    // One text object on `bottom`, created through the registry.
    let mut base = base_document();
    let mut seed = Session::new(base.clone());
    seed.apply(
        "text.createText",
        serde_json::json!({
            "page": 0, "layer": 0, "cell": 0,
            "x": 1.0, "y": 2.0, "width": 40.0, "height": 12.0,
            "text": "label", "name": null,
        }),
    )
    .expect("seed text");
    base = seed_document(seed);

    let scripted_session = scripted(
        base.clone(),
        "var dom = fw.getDocumentDOM(); dom.selectAllOnLayer(0); dom.setTextAutoExpand(true);",
    );

    let mut direct = Session::new(base);
    direct
        .apply(
            "selection.selectAllOnLayer",
            serde_json::json!({
                "page": 0, "layer": 0, "frame": 0,
                "remember_selection": false, "toggle_selection": false,
            }),
        )
        .expect("selectAllOnLayer");
    let id = direct.document().pages[0].layers.layers[0].cells[0].elements[0];
    direct
        .apply(
            "text.setAttrs",
            serde_json::json!({ "id": id, "auto_enlarge": true }),
        )
        .expect("setAttrs");

    assert_eq!(bytes(scripted_session.document()), bytes(direct.document()));
}

/// Extracts the seed session's mutated document.
fn seed_document(seed: Session) -> Document {
    seed.document().clone()
}

// --- the guarantees -------------------------------------------------------

#[test]
fn failing_script_leaves_document_untouched() {
    let base = base_document();
    let mut session = Session::new(base.clone());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "boom.jsf",
        "fw.getDocumentDOM().addNewText(0, 0, 10, 10, 'ghost'); throw new Error('aborted');",
    );
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(report.message.expect("message").contains("aborted"));
    // Nothing applied: same bytes, empty journal.
    assert_eq!(bytes(session.document()), bytes(&base));
    assert_eq!(session.journal_len(), 0);
}

#[test]
fn budget_exhaustion_also_leaves_document_untouched() {
    let base = base_document();
    let mut session = Session::new(base.clone());
    let mut script = ScriptEngine::new(Limits {
        host_calls: 8,
        ..Limits::default()
    })
    .expect("engine");
    let report = run_in_session(
        &mut session,
        &mut script,
        "flood.jsf",
        "var dom = fw.getDocumentDOM(); while (true) { dom.addNewRectangle(1, 2, 3, 4); }",
    );
    assert_eq!(report.outcome, Outcome::LimitExceeded);
    assert_eq!(bytes(session.document()), bytes(&base));
    assert_eq!(session.journal_len(), 0);
}

#[test]
fn revert_discards_queued_mutations() {
    // setUpRevert / addNewText / revert: net zero, empty journal.
    let base = base_document();
    let mut session = Session::new(base.clone());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "revert.jsf",
        "fw.setUpRevert(); fw.getDocumentDOM().addNewText(0, 0, 5, 5, 'temp'); fw.revert();",
    );
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(bytes(session.document()), bytes(&base));
    assert_eq!(session.journal_len(), 0);

    // setUpRevert / A / revert / B: only B lands.
    let mut session = Session::new(base.clone());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "partial.jsf",
        "var dom = fw.getDocumentDOM(); fw.setUpRevert(); dom.addNewText(0, 0, 5, 5, 'discarded'); fw.revert(); dom.addNewText(1, 1, 5, 5, 'kept');",
    );
    assert!(report.completed(), "{:?}", report.message);
    let elements = &session.document().pages[0].layers.layers[1].cells[0].elements;
    assert_eq!(elements.len(), 1);
    let text = session.document().registry.get(elements[0]).expect("live");
    match text {
        ObjectData::Text(text) => {
            assert_eq!(text.runs[0].text, "kept");
        }
        other => panic!("expected text, got {other:?}"),
    }
}

#[test]
fn change_current_page_is_validated_and_persisted() {
    let mut base = base_document();
    let mut second = base.pages[0].clone();
    second.name = Some("Page 2".to_owned());
    base.pages.push(second);

    let mut session = Session::new(base.clone());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "page.jsf",
        "var dom = fw.getDocumentDOM(); dom.changeCurrentPage(1); fw.__log('page ' + dom.currentPageNum);",
    );
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["page 1"]);
    // The change is view state: no journal rows, but it persists
    // across attaches (the next run projects page 1).
    assert_eq!(session.journal_len(), 0);
    let report = run_in_session(
        &mut session,
        &mut script,
        "again.jsf",
        "fw.__log('again ' + fw.getDocumentDOM().currentPageNum);",
    );
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["again 1"]);

    // Out of range: typed throw, nothing applied.
    let mut session = Session::new(base);
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "bad.jsf",
        "fw.getDocumentDOM().changeCurrentPage(9);",
    );
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(
        report
            .message
            .expect("message")
            .contains("not on this document")
    );
}

#[test]
fn view_flags_are_session_state_and_persist() {
    let mut session = Session::new(base_document());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "flags.jsf",
        "var dom = fw.getDocumentDOM(); dom.setShowGrid(true); dom.setSnapToGuides(false);
         fw.__log('readback ' + dom.showGrid + ' ' + dom.snapToGuides);",
    );
    assert!(report.completed(), "{:?}", report.message);
    // The projection updates in place, so the same run reads back true.
    assert_eq!(report.notes, vec!["readback true false"]);
    // View state: no journal rows (no document delta was queued).
    assert_eq!(session.journal_len(), 0);

    // Persisted across attaches: the next run projects the same flags.
    let report = run_in_session(
        &mut session,
        &mut script,
        "again.jsf",
        "var dom = fw.getDocumentDOM();
         fw.__log('again ' + dom.showGrid + ' ' + dom.showGuides + ' ' + dom.snapToGrid + ' ' + dom.snapToGuides);",
    );
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["again true false false false"]);
}

#[test]
fn view_flags_default_off_and_throw_on_non_boolean() {
    let mut session = Session::new(base_document());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "defaults.jsf",
        "var dom = fw.getDocumentDOM();
         fw.__log('defaults ' + dom.showGrid + ' ' + dom.showGuides + ' ' + dom.snapToGrid + ' ' + dom.snapToGuides);",
    );
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["defaults false false false false"]);

    // Non-boolean: typed throw, nothing queued (document untouched).
    let before = serde_json::to_vec(session.document()).unwrap();
    let report = run_in_session(
        &mut session,
        &mut script,
        "bad.jsf",
        "fw.getDocumentDOM().setShowGrid(1);",
    );
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(
        report
            .message
            .expect("message")
            .contains("expected a boolean")
    );
    assert_eq!(serde_json::to_vec(session.document()).unwrap(), before);
    assert_eq!(session.journal_len(), 0);
}

#[test]
fn created_element_becomes_the_selection() {
    let mut session = Session::new(base_document());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "select.jsf",
        "fw.getDocumentDOM().addNewText(0, 0, 10, 10, 'x');",
    );
    assert!(report.completed(), "{:?}", report.message);

    // The NEXT run on the same engine sees the created text selected
    // (addNew* semantics; selection persists across attaches).
    let report = run_in_session(
        &mut session,
        &mut script,
        "read.jsf",
        "fw.__log('sel ' + fw.selection.length + ' ' + (fw.selection[0] && fw.selection[0].width));",
    );
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["sel 1 10"]);
}

#[test]
fn current_layer_num_routes_creations() {
    let mut session = Session::new(base_document());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "route.jsf",
        "var dom = fw.getDocumentDOM(); dom.currentLayerNum = 0; dom.addNewText(0, 0, 5, 5, 'bottom');",
    );
    assert!(report.completed(), "{:?}", report.message);
    let bottom = &session.document().pages[0].layers.layers[0].cells[0].elements;
    assert_eq!(bottom.len(), 1, "creation routed to the assigned layer");
}

#[test]
fn corpus_layer_shape_reads_frames_elements() {
    // Shipped scripts read layer.frames[frameNum].elements — pinned.
    let mut session = Session::new(two_square_document());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "walk.jsf",
        "var dom = fw.getDocumentDOM(); var l = dom.layers[0]; fw.__log(l.frames[0].elements.length + '/' + l.elements.length + ' locked=' + l.frames[0].locked);",
    );
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["2/2 locked=false"]);
}

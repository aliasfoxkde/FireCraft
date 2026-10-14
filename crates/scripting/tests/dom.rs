//! The read-side DOM projection battery: a fixture document projects
//! into `fw.getDocumentDOM()` / `fw.documents` / `fw.selection`, and
//! the corpus-shaped walk (width/height/resolution, pagesCount,
//! currentPageNum, frames, layers[].elements[] bounds) reads back the
//! pinned values.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use doc::{
    Canvas, Cell, Contour, Document, Layer, LayerList, ObjectData, Page, PageInfo, PathObject,
    PathPoint, SliceObject, TextObject,
};
use geom::{Point, Rect};
use scripting::{Limits, ScriptEngine};

/// A fixture: 320x240 @ 96dpi, one page, two layers; layer 1 carries
/// a path (10,20 → 110,120), a text frame, and a slice; layer 2 is
/// empty.
fn fixture() -> Document {
    let mut document = Document::default();
    document.info.canvas = Canvas {
        x: 0.0,
        y: 0.0,
        width: 320.0,
        height: 240.0,
        resolution: 96.0,
        resolution_unit: 0,
        background_raw: 0,
    };
    document.info.name = Some("fixture".to_owned());
    document.info.frame_count = 1;

    let mut badge = PathObject::neutral(vec![Contour {
        closed: true,
        point_count: 4,
        points: vec![
            PathPoint {
                anchor: Point::new(10.0, 20.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
            PathPoint {
                anchor: Point::new(110.0, 20.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
            PathPoint {
                anchor: Point::new(110.0, 120.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
            PathPoint {
                anchor: Point::new(10.0, 120.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
        ],
    }]);
    badge.name = Some("badge".to_owned());
    let path = document.registry.insert(ObjectData::Path(badge));
    let text = document.registry.insert(ObjectData::Text(TextObject {
        frame: Rect::new(5.0, 6.0, 105.0, 26.0),
        style: None,
        runs: Vec::new(),
        flow: None,
        on_path: None,
        anti_alias: None,
        anti_alias_sharpness: None,
        anti_alias_strength: None,
        anti_alias_oversample: None,
        auto_enlarge: None,
        auto_kern: None,
        name: None,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        transform: None,
        effects: Vec::new(),
    }));
    let slice = document.registry.insert(ObjectData::Slice(SliceObject {
        internal_name: None,
        frame: Rect::new(0.0, 0.0, 64.0, 32.0),
        shape: 0,
        fill_raw: 0,
        name: Some("nav".to_owned()),
        slice_type: None,
        link: None,
        alt: None,
        label: None,
        td_text: None,
        symbol_number: None,
        contours: Vec::new(),
        background: None,
        behaviors: Vec::new(),
        custom_data: std::collections::BTreeMap::new(),
        export: None,
    }));

    document.pages.push(Page {
        info: PageInfo {
            frame_count: 3,
            frame_count_duplicate: 3,
            canvas: document.info.canvas.clone(),
            ..page_info_default()
        },
        name: Some("Page 1".to_owned()),
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList {
            layers: vec![
                Layer {
                    name: "empty".to_owned(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: Vec::new(),
                    }],
                    ..Layer::default()
                },
                Layer {
                    name: "content".to_owned(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: vec![path, text, slice],
                    }],
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

/// A zeroed `PageInfo` (no `Default` — its scalars are required).
fn page_info_default() -> PageInfo {
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

fn logged(source: &str) -> Vec<String> {
    let mut engine = ScriptEngine::new(Limits::default()).expect("engine");
    let document = fixture();
    engine.attach_document(&document).expect("attach");
    let report = engine.run("dom.jsf", source);
    assert!(report.completed(), "{:?}", report.message);
    report.notes
}

#[test]
fn document_scalars_read_back() {
    let notes = logged(
        r#"
var dom = fw.getDocumentDOM();
fw.__log(dom.width + "x" + dom.height + " @" + dom.resolution);
fw.__log("origin " + dom.left + "," + dom.top);
fw.__log("units " + dom.resolutionUnits);
fw.__log("pages " + dom.pagesCount + " cur " + dom.currentPageNum);
"#,
    );
    assert_eq!(
        notes,
        vec!["320x240 @96", "origin 0,0", "units 0", "pages 1 cur 0",]
    );
}

#[test]
fn frames_layers_and_element_walk() {
    let notes = logged(
        r#"
var dom = fw.getDocumentDOM();
fw.__log("frames " + dom.frames.length + " cur " + dom.currentFrameNum);
fw.__log("layers " + dom.layers.length);
fw.__log("l0 " + dom.layers[0].name + " elems " + dom.layers[0].elements.length);
var l1 = dom.layers[1];
fw.__log("l1 " + l1.name + " elems " + l1.elements.length);
var p = l1.elements[0];
fw.__log(p.name + " " + Math.round(p.left) + "," + Math.round(p.top) + " " + Math.round(p.width) + "x" + Math.round(p.height));
var t = l1.elements[1];
fw.__log("text " + Math.round(t.left) + "," + Math.round(t.top) + " " + Math.round(t.width) + "x" + Math.round(t.height));
fw.__log("slice " + l1.elements[2].name + " w" + Math.round(l1.elements[2].width));
"#,
    );
    assert_eq!(
        notes,
        vec![
            "frames 3 cur 0",
            "layers 2",
            "l0 empty elems 0",
            "l1 content elems 3",
            "badge 10,20 100x100",
            "text 5,6 100x20",
            "slice nav w64",
        ]
    );
}

#[test]
fn documents_array_and_identity() {
    let notes = logged(
        r#"
fw.__log("docs " + fw.documents.length);
fw.__log("same " + (fw.documents[0] === fw.getDocumentDOM()));
fw.__log("sel " + fw.selection.length);
"#,
    );
    assert_eq!(notes, vec!["docs 1", "same true", "sel 0"]);
}

#[test]
fn no_attachment_throws_typed() {
    let mut engine = ScriptEngine::new(Limits::default()).expect("engine");
    let report = engine.run("bare.jsf", "fw.getDocumentDOM();");
    assert_eq!(report.outcome, scripting::Outcome::Threw);
    // The prelude stub refuses before any attach happened.
    let message = report.message.expect("message");
    assert!(message.contains("not implemented"), "{message}");
}

#[test]
fn reattach_updates_the_projection() {
    let mut engine = ScriptEngine::new(Limits::default()).expect("engine");
    let mut document = fixture();
    document.info.canvas.width = 640.0;
    document.info.canvas.height = 480.0;
    engine.attach_document(&fixture()).expect("attach a");
    engine.attach_document(&document).expect("attach b");
    let report = engine.run(
        "again.jsf",
        "var dom = fw.getDocumentDOM(); fw.__log(dom.width + 'x' + dom.height); fw.__log(fw.documents.length);",
    );
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["640x480", "1"]);
}

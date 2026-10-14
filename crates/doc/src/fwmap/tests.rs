//! Round-trip and preservation proofs for the node-tree ⇄ model
//! mapping (M3 tasks 3–5). The full-struct-literal test is the
//! compile-fail completeness gate: adding a model field without a
//! mapping breaks this build.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::MapError;
use super::reader::{read, read_style_library};
use super::unknown::UnknownBag;
use super::writer::{mapped_indices, splice, write, write_style_library};
use crate::model::{
    Canvas, Cell, Contour, Document, DocumentInfo, Effect, EffectProperty, EffectValue,
    ExportOptions, Grid, GroupObject, Guides, ImageObject, Layer, LayerList, NineSliceGuides, Page,
    PageInfo, PaintStyle, PathObject, PathPoint, PrimitiveSource, SharedLayerDef, SharedLayerLink,
    SliceBackground, SliceBehavior, SliceObject, StyleEntry, StyleText, SymbolAnimation,
    SymbolDefinition, SymbolInstanceObject, SymbolType, TextObject, TextRun, TextStyle, TileRef,
};
use crate::registry::{ObjectData, ObjectId, Registry};
use fwpng::nodetree::{Node, NodeValue, parse, serialize};
use geom::{Affine, Point, Rect};

// --- node builders ------------------------------------------------------

fn n_i(fourcc: &[u8; 4], value: i64) -> Node {
    Node {
        fourcc: *fourcc,
        value: NodeValue::Int(value),
    }
}

fn n_f(fourcc: &[u8; 4], value: f64) -> Node {
    Node {
        fourcc: *fourcc,
        value: NodeValue::Float(value),
    }
}

fn n_b(fourcc: &[u8; 4], value: bool) -> Node {
    Node {
        fourcc: *fourcc,
        value: NodeValue::Bool(value),
    }
}

fn n_s(fourcc: &[u8; 4], value: &str) -> Node {
    Node {
        fourcc: *fourcc,
        value: NodeValue::Str(value.to_owned()),
    }
}

fn n_k(fourcc: &[u8; 4], children: Vec<Node>) -> Node {
    Node {
        fourcc: *fourcc,
        value: NodeValue::Children(children),
    }
}

// --- sample document (insertion order matches the reader's) -------------

/// A fully-populated, tree-shaped document. Object insertion order is
/// the order a reader discovers them in, so a round trip reproduces the
/// same registry ids and `Document` equality holds.
fn sample_document() -> Document {
    let canvas = Canvas {
        x: 0.0,
        y: 0.0,
        width: 480.0,
        height: 320.0,
        resolution: 72.0,
        resolution_unit: 0,
        background_raw: 0xFF_FF_FF,
    };
    let mut doc = Document {
        info: DocumentInfo {
            name: Some("Cover".into()),
            version: Some(1),
            revision: Some(0),
            frame_count: 2,
            request_id: 7,
            canvas: canvas.clone(),
            onion_before: 0,
            onion_after: 1,
        },
        grid: Grid {
            offset: Point::ZERO,
            size: Point::new(24.0, 24.0),
            color_raw: 0xC0_C0_C0,
        },
        guides: Guides {
            locked: false,
            color_raw: 0x00_FF_00,
            horizontal: vec![16.0, 32.0],
            vertical: vec![10.0],
        },
        ..Document::default()
    };

    // Slot order mirrors reader discovery: `MKBv` (and its `MSLv`) is
    // read before any `PDCv`, so the symbol-internal path lands first;
    // a group's children register before the group itself.
    let symbol_path_id = doc.registry.insert(ObjectData::Path(PathObject {
        contours: Vec::new(),
        primitive: None,
        even_odd: false,
        paint: None,
        fill_start: None,
        fill_end: None,
        fill_third: None,
        brush_placement: None,
        fill_on_top: None,
        locked: false,
        visible: true,
        opacity: 1000,
        blend: 0,
        name: None,
        effects: Vec::new(),
    }));
    let path_id = doc.registry.insert(ObjectData::Path(PathObject {
        contours: vec![Contour {
            closed: true,
            point_count: 2,
            points: vec![
                PathPoint {
                    anchor: Point::new(0.0, 0.0),
                    primary: Some(Point::new(-2.0, 0.0)),
                    secondary: Some(Point::new(2.0, 0.0)),
                    is_curve: true,
                },
                PathPoint {
                    anchor: Point::new(10.0, 20.0),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                },
            ],
        }],
        primitive: Some(PrimitiveSource {
            rect: Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 10.0,
                y1: 20.0,
            },
            radius: 2.5,
            transform: Affine::new([1.0, 0.25, 0.0, 1.0, 4.0, 6.0]),
            maintain_radius: true,
        }),
        even_odd: true,
        paint: Some(PaintStyle {
            stroke_raw: 0x20_40_80_FF,
            fill_raw: 0xC0_80_40_FF,
            fill: None,
        }),
        fill_start: Some(Point::new(0.0, 0.0)),
        fill_end: Some(Point::new(10.0, 20.0)),
        fill_third: None,
        brush_placement: Some(1),
        fill_on_top: Some(false),
        locked: false,
        visible: true,
        opacity: 850,
        blend: 2,
        name: Some("panel".into()),
        effects: vec![Effect {
            category: "Shadow and Glow".into(),
            internal_name: "dropShadow".into(),
            properties: vec![
                EffectProperty {
                    key: "EffectMoaID".into(),
                    value: EffectValue::Text("9CS6CoreShadowAndGlowDropShadow".into()),
                },
                EffectProperty {
                    key: "gaussian_blur_radius".into(),
                    value: EffectValue::Text("5".into()),
                },
                EffectProperty {
                    key: "rgb_points".into(),
                    value: EffectValue::Raw(curve_points()),
                },
            ],
        }],
    }));
    let image_id = doc.registry.insert(ObjectData::Image(ImageObject {
        location: Point::new(8.0, 8.0),
        offset: Point::new(0.5, -0.5),
        pixels: (64, 32),
        tile_size: 256,
        tiles: vec![
            TileRef::Tile(0),
            TileRef::Tile(1),
            TileRef::Matte(0xAA_BB_CC_DD),
        ],
        name: Some("img".into()),
        opacity: 750,
        blend: 3,
        visible: true,
        locked: false,
        effects: vec![Effect {
            category: "Adjust Color".into(),
            internal_name: "gaussianBlur".into(),
            properties: vec![EffectProperty {
                key: "gaussian_blur_radius".into(),
                value: EffectValue::Text("2.5".into()),
            }],
        }],
    }));
    let text_id = doc.registry.insert(ObjectData::Text(TextObject {
        frame: Rect {
            x0: 1.0,
            y0: 2.0,
            x1: 91.0,
            y1: 22.0,
        },
        style: Some(TextStyle {
            font: Some("Liberation Sans".into()),
            size: Some(12.0),
            bold: Some(false),
            italic: Some(false),
            underline: Some(false),
            horizontal_scale: Some(100.0),
            kerning: Some(0.0),
            baseline_shift: Some(0.0),
            leading: Some(14.0),
            leading_unit: Some(0),
            justification: Some(0),
            fill_raw: Some(0xFF_00_00_00),
            paragraph_indent: Some(0.0),
            space_before: Some(0.0),
            space_after: Some(0.0),
        }),
        runs: vec![TextRun {
            text: "Fireworks".into(),
        }],
        flow: Some(0),
        on_path: None,
        anti_alias: Some(2),
        auto_enlarge: None,
        auto_kern: Some(true),
        name: Some("caption".into()),
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        transform: Some(Affine::new([2.0, 0.0, 0.0, 2.0, 0.0, 0.0])),
        effects: vec![Effect {
            category: String::new(),
            internal_name: "brightnessContrast".into(),
            properties: vec![
                EffectProperty {
                    key: "EffectMoaID".into(),
                    value: EffectValue::Text("9CS6AdjustColorBrightnessContrast".into()),
                },
                EffectProperty {
                    key: "brightness".into(),
                    value: EffectValue::Text("12".into()),
                },
                EffectProperty {
                    key: "gdt_payload".into(),
                    value: EffectValue::Raw(n_k(
                        b"GDTv",
                        vec![n_i(b"GDIi", 7), n_s(b"GDSs", "nested")],
                    )),
                },
            ],
        }],
    }));
    let slice_id = doc.registry.insert(ObjectData::Slice(SliceObject {
        internal_name: Some("slice_1".into()),
        frame: Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 64.0,
            y1: 64.0,
        },
        shape: 0,
        fill_raw: 0x33_66_99,
        name: Some("Slice".into()),
        slice_type: Some(1),
        // The full URLv string/behavior/data family: the compile-fail
        // gate keeps every mapped node honest.
        link: Some("https://firecraft.example/".into()),
        alt: Some("a slice".into()),
        label: Some("hero".into()),
        td_text: Some("cell text".into()),
        symbol_number: Some(5),
        contours: vec![Contour {
            closed: true,
            point_count: 3,
            points: vec![
                PathPoint {
                    anchor: Point::new(1.0, 2.0),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                },
                PathPoint {
                    anchor: Point::new(9.0, 2.0),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                },
                PathPoint {
                    anchor: Point::new(5.0, 8.0),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                },
            ],
        }],
        background: Some(SliceBackground {
            is_background: Some(false),
            repeat: Some(2),
            attachment: Some(1),
            h_position: Some(3),
            h_value: Some(12),
            v_position: Some(0),
            v_value: None,
        }),
        behaviors: vec![SliceBehavior { kind: 9 }],
        custom_data: std::collections::BTreeMap::from([(
            "swapTarget".to_owned(),
            "hero_f2.png".to_owned(),
        )]),
        export: Some(ExportOptions {
            format: Some(1),
            jpeg_quality: Some(82),
            ..ExportOptions::default()
        }),
    }));
    let group_id = doc.registry.insert(ObjectData::Group(GroupObject {
        group_type: 1,
        name: Some("grp".into()),
        children: vec![text_id, slice_id],
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        mask_linked: None,
        effects: vec![Effect {
            category: "Adjust Color".into(),
            internal_name: "autoLevels".into(),
            properties: Vec::new(),
        }],
    }));
    let instance_id = doc
        .registry
        .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
            symbol_number: 5,
            transform: Affine::new([1.0, 0.0, 0.0, 1.0, 12.0, 0.0]),
            locked: false,
            name: None,
            // The sample's instance animates across 12 frames so the
            // full-spine round-trip covers the `ANNi` family.
            animation: Some(SymbolAnimation {
                frames: 12,
                x_shift: 90.0,
                y_shift: -20.0,
                scaling: 140.0,
                rotation: Some(-45),
                start_opacity: Some(200),
                end_opacity: Some(900),
            }),
            custom_data: std::collections::BTreeMap::new(),
        }));
    let layer = Layer {
        name: "Layer 1".into(),
        is_background: false,
        has_url: false,
        is_special: false,
        frame_changing: true,
        display: true,
        nesting_level: Some(0),
        cells: vec![
            Cell {
                locked: false,
                visible: true,
                elements: vec![path_id, image_id],
            },
            Cell {
                locked: false,
                visible: true,
                elements: vec![group_id, instance_id],
            },
        ],
        visibility_per_frame: Some(vec![true, false]),
        shared_link: None,
    };
    doc.pages.push(Page {
        info: PageInfo {
            version: Some(1),
            revision: Some(0),
            frame_count: 2,
            frame_count_duplicate: 2,
            request_id: 8,
            canvas,
            onion_before: 0,
            onion_after: 0,
            frame_durations: Vec::new(),
        },
        name: Some("Page 1".into()),
        is_master: false,
        is_linked: false,
        page_id: Some("ABC".into()),
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList {
            layers: vec![layer],
        },
        grid: None,
        guides: Guides {
            locked: true,
            color_raw: 1,
            horizontal: vec![],
            vertical: vec![5.0],
        },
        export: None,
    });
    doc.symbols.push(SymbolDefinition {
        name: "Sym 5".into(),
        symbol_number: 5,
        origin: Point::new(1.0, 2.0),
        frame_count: 1,
        layers: LayerList {
            layers: vec![Layer {
                name: "sym layer".into(),
                is_background: false,
                has_url: false,
                is_special: true,
                frame_changing: false,
                display: true,
                nesting_level: None,
                cells: vec![Cell {
                    locked: false,
                    visible: true,
                    elements: vec![symbol_path_id],
                }],
                visibility_per_frame: None,
                shared_link: None,
            }],
        },
        background_raw: 0xFF_FF_FF,
        symbol_type: SymbolType::Button,
        auto_slice: Some(true),
        include_down_state: Some(true),
        show_down_on_load: Some(false),
        include_over_while_down: Some(true),
        nine_slice: Some(NineSliceGuides {
            locked: true,
            color_raw: 0x00_80_FF,
            controls: [3.0, 5.0, 2.0, 7.0],
            shape_bounds: Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 40.0,
                y1: 16.0,
            },
            limits: Rect {
                x0: -2.0,
                y0: -2.0,
                x1: 44.0,
                y1: 20.0,
            },
        }),
        op_code: None,
    });
    doc
}

fn round_trip_bytes(doc: &Document, bag: &UnknownBag) -> (Vec<u8>, Document, UnknownBag) {
    let (tree, stats) = write(doc, bag).expect("write");
    assert_eq!(stats.unconsumed_unknown, 0);
    let bytes = serialize(&tree);
    let parsed = parse(&bytes).expect("reparse");
    let (doc2, bag2) = read(&parsed).expect("read");
    (bytes, doc2, bag2)
}

/// An `rgb_points` grab-point curve: `GPLv` of two `GPTv` `XLCf`/`YLCf`
/// pairs (dictionary shape for the colour-transform curves).
fn curve_points() -> Node {
    n_k(
        b"GPLv",
        vec![
            n_k(b"GPTv", vec![n_f(b"XLCf", 0.0), n_f(b"YLCf", 0.0)]),
            n_k(b"GPTv", vec![n_f(b"XLCf", 1.0), n_f(b"YLCf", 1.0)]),
        ],
    )
}

// --- tests --------------------------------------------------------------

#[test]
fn full_spine_round_trips_losslessly() {
    let doc = sample_document();
    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &UnknownBag::new());
    assert!(bag2.is_empty(), "nothing in the mapped spine may be bagged");
    assert_eq!(doc, doc2, "model equality through bytes");
    // Second generation is byte-stable (canonical emission).
    let (bytes2, _doc3, bag3) = round_trip_bytes(&doc2, &bag2);
    assert_eq!(bytes, bytes2);
    assert!(bag3.is_empty());
}

#[test]
fn lnkb_writes_only_when_unlinked_and_round_trips() {
    // The dictionary: `LNKb` appears only as `false` (unlinked); the
    // linked form is the node's absence.
    let mut doc = sample_document();
    let group_id = doc
        .registry
        .iter()
        .find(|(_, o)| matches!(o, ObjectData::Group(_)))
        .map(|(id, _)| id)
        .expect("sample carries a group");

    // Unlinked: the node is written and survives bytes.
    if let Some(ObjectData::Group(slot)) = doc.registry.get_mut(group_id) {
        slot.mask_linked = Some(false);
    }
    let (_bytes, doc2, _bag2) = round_trip_bytes(&doc, &UnknownBag::new());
    let read_back = match doc2.registry.get(group_id) {
        Some(ObjectData::Group(g)) => g.mask_linked,
        other => panic!("group vanished through bytes: {other:?}"),
    };
    assert_eq!(read_back, Some(false), "unlinked survives bytes");
    let (tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    let mut hits = Vec::new();
    find_nodes(&tree, b"LNKb", &mut hits);
    assert_eq!(hits.len(), 1, "the unlinked group carries one LNKb");

    // Linked: the node's absence, and no `LNKb` in the bytes.
    if let Some(ObjectData::Group(slot)) = doc.registry.get_mut(group_id) {
        slot.mask_linked = None;
    }
    let (bytes, doc3, _bag3) = round_trip_bytes(&doc, &UnknownBag::new());
    assert_eq!(
        match doc3.registry.get(group_id) {
            Some(ObjectData::Group(g)) => g.mask_linked,
            _ => panic!("group vanished through bytes"),
        },
        None,
        "linked survives bytes"
    );
    let (tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    let mut hits = Vec::new();
    find_nodes(&tree, b"LNKb", &mut hits);
    assert!(hits.is_empty(), "linked groups write no LNKb");
    assert!(!bytes.is_empty());
}

// --- M10: symbol wire contract (MSYi / NGSv / ANNi family) --------------

/// Collect references to every node carrying `fourcc`, at any depth.
fn find_nodes<'a>(nodes: &'a [Node], fourcc: &[u8; 4], out: &mut Vec<&'a Node>) {
    for node in nodes {
        if &node.fourcc == fourcc {
            out.push(node);
        }
        if let NodeValue::Children(kids) = &node.value {
            find_nodes(kids, fourcc, out);
        }
    }
}

fn kids_of(node: &Node) -> &[Node] {
    match &node.value {
        NodeValue::Children(kids) => kids,
        _ => panic!("expected a container node"),
    }
}

#[test]
fn symbol_definition_wire_shape_matches_dictionary() {
    let doc = sample_document();
    let (tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    let mut mstvs = Vec::new();
    find_nodes(&tree, b"MSTv", &mut mstvs);
    assert_eq!(mstvs.len(), 1, "the sample carries one definition");
    let kids = kids_of(mstvs[0]);

    // `MSYi` and `NGSv` sit at their dictionary slots: after `BGCi`,
    // `MSYi` before `NGSv`.
    let bgci = kids
        .iter()
        .position(|k| &k.fourcc == b"BGCi")
        .expect("BGCi");
    let msyi = kids
        .iter()
        .position(|k| &k.fourcc == b"MSYi")
        .expect("MSYi");
    let ngsv = kids
        .iter()
        .position(|k| &k.fourcc == b"NGSv")
        .expect("NGSv");
    assert!(
        bgci < msyi && msyi < ngsv,
        "dictionary order BGCi < MSYi < NGSv"
    );
    assert_eq!(
        kids[msyi].as_int(),
        Some(1),
        "the sample symbol is a button"
    );

    // `NGSv` children in dictionary order: NGLb NGKi NGCf×4 NGMv NGVv.
    let ngsv_kids = kids_of(&kids[ngsv]);
    let codes: Vec<[u8; 4]> = ngsv_kids.iter().map(|k| k.fourcc).collect();
    assert_eq!(
        codes,
        vec![
            *b"NGLb", *b"NGKi", *b"NGCf", *b"NGCf", *b"NGCf", *b"NGCf", *b"NGMv", *b"NGVv",
        ]
    );
    let controls: Vec<f64> = ngsv_kids[2..6]
        .iter()
        .map(|k| k.as_float().expect("NGCf float"))
        .collect();
    assert_eq!(controls, vec![3.0, 5.0, 2.0, 7.0]);
    assert_eq!(ngsv_kids[0].as_bool(), Some(true), "NGLb");
    assert_eq!(ngsv_kids[1].as_int(), Some(0x00_80_FF), "NGKi");

    // `NGMv`/`NGVv` bounds: LFTf TOPf RITf BOTf.
    let bounds = |container: &Node| -> Rect {
        let kids = kids_of(container);
        Rect {
            x0: kids[0].as_float().expect("LFTf"),
            y0: kids[1].as_float().expect("TOPf"),
            x1: kids[2].as_float().expect("RITf"),
            y1: kids[3].as_float().expect("BOTf"),
        }
    };
    assert_eq!(
        bounds(&ngsv_kids[6]),
        Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 40.0,
            y1: 16.0
        }
    );
    assert_eq!(
        bounds(&ngsv_kids[7]),
        Rect {
            x0: -2.0,
            y0: -2.0,
            x1: 44.0,
            y1: 20.0
        }
    );
}

#[test]
fn graphic_symbol_omits_type_and_guides() {
    let mut doc = sample_document();
    doc.symbols[0].symbol_type = SymbolType::Graphic;
    doc.symbols[0].nine_slice = None;
    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &UnknownBag::new());
    assert!(
        bag2.is_empty(),
        "MSYi/NGSv omission must not spawn bag entries"
    );
    assert_eq!(doc, doc2);
    let parsed = parse(&bytes).expect("reparse");
    let mut mstvs = Vec::new();
    find_nodes(&parsed, b"MSTv", &mut mstvs);
    assert_eq!(mstvs.len(), 1);
    assert!(
        kids_of(mstvs[0])
            .iter()
            .all(|k| &k.fourcc != b"MSYi" && &k.fourcc != b"NGSv"),
        "graphic definitions omit MSYi/NGSv rather than storing zeros"
    );
}

#[test]
fn button_booleans_and_slice_link_round_trip() {
    // The sample definition carries the button-behavior booleans
    // (ATSb true, IDSb true, DVLb false, IODb true — decision 0036)
    // and its slice carries the MSNi link. No corpus oracle carries
    // either yet: the synthetic byte-equal round-trip is the gate.
    let doc = sample_document();
    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &UnknownBag::new());
    assert!(
        bag2.is_empty(),
        "mapped booleans must not spawn bag entries"
    );
    assert_eq!(doc, doc2);

    let parsed = parse(&bytes).expect("reparse");
    let mut mstvs = Vec::new();
    find_nodes(&parsed, b"MSTv", &mut mstvs);
    let kids = kids_of(mstvs[0]);
    let slot = |fourcc: &[u8; 4]| {
        kids.iter()
            .position(|k| &k.fourcc == fourcc)
            .unwrap_or_else(|| panic!("{} on the wire", String::from_utf8_lossy(fourcc)))
    };
    let (bgci, atsb, idsb, dvlb, iodb, msyi) = (
        slot(b"BGCi"),
        slot(b"ATSb"),
        slot(b"IDSb"),
        slot(b"DVLb"),
        slot(b"IODb"),
        slot(b"MSYi"),
    );
    assert!(
        bgci < atsb && atsb < idsb && idsb < dvlb && dvlb < iodb && iodb < msyi,
        "dictionary order BGCi < ATSb < IDSb < DVLb < IODb < MSYi"
    );
    assert_eq!(kids[atsb].as_bool(), Some(true), "ATSb");
    assert_eq!(kids[idsb].as_bool(), Some(true), "IDSb");
    assert_eq!(kids[dvlb].as_bool(), Some(false), "DVLb");
    assert_eq!(kids[iodb].as_bool(), Some(true), "IODb");

    // The button's slice points back at the definition (`MSNi` under
    // `URLv`).
    let mut urlvs = Vec::new();
    find_nodes(&parsed, b"URLv", &mut urlvs);
    assert_eq!(urlvs.len(), 1, "the sample carries one slice");
    let msni = kids_of(urlvs[0])
        .iter()
        .find(|k| &k.fourcc == b"MSNi")
        .expect("slice MSNi");
    assert_eq!(msni.as_int(), Some(5), "slice → definition 5");
}

#[test]
fn instance_animation_family_wire_shape() {
    let doc = sample_document();
    let (tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    let mut alsvs = Vec::new();
    find_nodes(&tree, b"ALSv", &mut alsvs);
    assert_eq!(alsvs.len(), 1, "the sample carries one instance");
    let kids = kids_of(alsvs[0]);
    let pos = |fourcc: &[u8; 4]| {
        kids.iter()
            .position(|k| &k.fourcc == fourcc)
            .unwrap_or_else(|| panic!("missing {}", std::str::from_utf8(fourcc).unwrap_or("")))
    };
    let (anni, anxf, anyf, ascf) = (pos(b"ANNi"), pos(b"ANXf"), pos(b"ANYf"), pos(b"ASCf"));
    let (anri, asoi, aeoi) = (pos(b"ANRi"), pos(b"ASOi"), pos(b"AEOi"));
    assert!(
        anni < anxf && anxf < anyf && anyf < ascf && ascf < anri && anri < asoi && asoi < aeoi,
        "dictionary order ANNi ANXf ANYf ASCf ANRi ASOi AEOi"
    );
    assert_eq!(kids[anni].as_int(), Some(12));
    assert_eq!(kids[anxf].as_float(), Some(90.0));
    assert_eq!(kids[anyf].as_float(), Some(-20.0));
    assert_eq!(kids[ascf].as_float(), Some(140.0));
    assert_eq!(kids[anri].as_int(), Some(-45));
    assert_eq!(kids[asoi].as_int(), Some(200));
    assert_eq!(kids[aeoi].as_int(), Some(900));

    // A static instance drops the whole family from the wire.
    let mut doc = sample_document();
    for (_id, data) in doc.registry.iter_mut() {
        if let ObjectData::SymbolInstance(instance) = data {
            instance.animation = None;
        }
    }
    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &UnknownBag::new());
    assert!(bag2.is_empty());
    assert_eq!(doc, doc2);
    let parsed = parse(&bytes).expect("reparse");
    let mut alsvs = Vec::new();
    find_nodes(&parsed, b"ALSv", &mut alsvs);
    assert_eq!(alsvs.len(), 1);
    assert!(
        kids_of(alsvs[0]).iter().all(|k| {
            !matches!(
                &k.fourcc,
                b"ANNi" | b"ANXf" | b"ANYf" | b"ASCf" | b"ANRi" | b"ASOi" | b"AEOi"
            )
        }),
        "static instances emit no animation nodes"
    );
}

#[test]
fn unknown_symbol_type_preserves_raw() {
    let mut doc = sample_document();
    doc.symbols[0].symbol_type = SymbolType::Other(7);
    let (_, doc2, bag2) = round_trip_bytes(&doc, &UnknownBag::new());
    assert!(bag2.is_empty());
    assert_eq!(doc2.symbols[0].symbol_type, SymbolType::Other(7));
}

#[test]
fn splice_restores_bagged_positions() {
    let mapped = vec![n_i(b"AAAi", 1), n_i(b"BBBi", 2)];
    let bagged = vec![(1, n_i(b"MIDi", 9))];
    let out = splice(mapped, bagged);
    let fourccs: Vec<[u8; 4]> = out.iter().map(|node| node.fourcc).collect();
    assert_eq!(fourccs, vec![*b"AAAi", *b"MIDi", *b"BBBi"]);
}

#[test]
fn splice_appends_unplaceable_entries() {
    let mapped = vec![n_i(b"AAAi", 1)];
    let bagged = vec![(5, n_i(b"MIDi", 9))];
    let out = splice(mapped, bagged);
    assert_eq!(out.len(), 2, "content stays lossless even off-position");
}

#[test]
fn mapped_indices_skip_bagged_slots() {
    let bagged = vec![(0, n_i(b"MIDi", 9)), (3, n_i(b"MIDi", 9))];
    assert_eq!(mapped_indices(&bagged, 2), vec![1, 2]);
    let empty: Vec<(usize, Node)> = Vec::new();
    assert_eq!(mapped_indices(&empty, 3), vec![0, 1, 2]);
}

#[test]
fn unknown_nodes_survive_modeled_edits() {
    // Write the sample, then inject unknown nodes into the first path
    // object: unknown children inside its modeled `PATv`, and an
    // unknown scalar (`TOXf`) between the object's own children.
    let doc = sample_document();
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    let brus = n_s(b"BRUs", "brush-payload");
    let sedi = n_i(b"SEDi", 4242);
    let toxf = n_f(b"TOXf", 1.5);
    {
        let pthv = first_path_object_mut(&mut tree);
        let kids = children_mut(pthv);
        assert_eq!(kids[0].fourcc, *b"PATv", "sample path carries a pattern");
        let patv = &mut kids[0];
        children_mut(patv).insert(0, brus.clone());
        children_mut(patv).insert(1, sedi.clone());
        kids.insert(1, toxf.clone());
    }

    let (mut doc2, bag) = {
        let bytes = serialize(&tree);
        let parsed = parse(&bytes).expect("parse");
        read(&parsed).expect("read")
    };
    assert_eq!(
        bag.len(),
        3,
        "BRUs + SEDi inside PATv, TOXf at the object level"
    );
    assert!(
        bag.iter()
            .all(|(path, _, _)| path.ends_with("PATv") || path.ends_with("ELMv[0]"))
    );

    // Modeled edit: turn even-odd off on the path object.
    for (_, object) in doc2.registry.iter_mut() {
        if let ObjectData::Path(path) = object {
            path.even_odd = false;
        }
    }
    let (mut tree2, stats2) = write(&doc2, &bag).expect("write");
    assert_eq!(stats2.unconsumed_unknown, 0);

    {
        let pthv2 = first_path_object_mut(&mut tree2);
        let kids = children(pthv2);
        assert_eq!(kids[0].fourcc, *b"PATv", "pattern still first");
        assert_eq!(kids[1].fourcc, *b"TOXf", "unknown scalar at recorded index");
        let patv_kids = children(&kids[0]);
        assert_eq!(
            serialize(&patv_kids[0..2]),
            serialize(&[brus, sedi]),
            "unknown PATv children byte-identical across the edit"
        );
        assert_eq!(
            &patv_kids[2..4],
            &[n_i(b"BCLi", 0x20_40_80_FF), n_i(b"FCLi", 0xC0_80_40_FF),],
            "mapped colour slots re-emit at canonical depth after the unknowns"
        );
    }
    let edited = serialize(&tree2);

    // And the edit is visible in the model after a second read, with the
    // bagged nodes still in place.
    let reparsed = parse(&edited).expect("reparse");
    let (doc3, bag3) = read(&reparsed).expect("read");
    assert_eq!(bag.iter().count(), bag3.iter().count());
    for (_, object) in doc3.registry.iter() {
        if let ObjectData::Path(path) = object {
            assert!(!path.even_odd, "modeled edit survived the round trip");
        }
    }
}

#[test]
fn effect_lists_round_trip_through_bytes() {
    // The sample carries effects on all four container kinds (path
    // text group image) — the lossless harness proves them end to end.
    let doc = sample_document();
    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &UnknownBag::new());
    assert!(bag2.is_empty(), "mapped effects leave no bag residue");
    assert_eq!(doc, doc2, "effect payloads survive real PNG bytes");
    let (bytes2, _doc3, bag3) = round_trip_bytes(&doc2, &bag2);
    assert_eq!(bytes, bytes2, "canonical emission is byte-stable");
    assert!(bag3.is_empty());
}

#[test]
fn effect_node_shape_preserves_raw_values() {
    let doc = sample_document();
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    let pthv = first_path_object_mut(&mut tree);
    let eflv = children_mut(pthv)
        .iter_mut()
        .find(|node| node.fourcc == *b"EFLv")
        .expect("sample path carries an effect");
    let kids = children(eflv).clone();
    let fourccs: Vec<[u8; 4]> = kids.iter().map(|node| node.fourcc).collect();
    assert_eq!(
        fourccs,
        vec![*b"CATs", *b"INMs", *b"EPSv"],
        "dictionary slot order inside EFLv"
    );
    let epsv = &kids[2];
    let entries = children(epsv).clone();
    assert_eq!(entries.len(), 3);
    for entry in &entries {
        assert_eq!(entry.fourcc, *b"DCEv");
        let pair = children(entry);
        assert_eq!(pair.len(), 2, "DCKs + exactly one value node");
        assert_eq!(pair[0].fourcc, *b"DCKs");
    }
    // The curve property is the raw node preserved verbatim —
    // byte-equal to its construction, grab points included.
    let rgb = entries
        .iter()
        .find(|entry| children(entry)[0] == n_s(b"DCKs", "rgb_points"))
        .expect("rgb_points entry");
    let expected = n_k(b"DCEv", vec![n_s(b"DCKs", "rgb_points"), curve_points()]);
    assert_eq!(
        serialize(children(rgb)),
        serialize(children(&expected)),
        "GPLv grab points survive byte-identically"
    );
}

#[test]
fn malformed_effect_entries_bag_and_reemit() {
    // A DCEv with two value nodes and a second EPSv sibling: both are
    // structural surprises — bagged whole, never half-mapped.
    let doc = sample_document();
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    let malformed = n_k(
        b"DCEv",
        vec![
            n_s(b"DCKs", "mystery_param"),
            n_s(b"DCVs", "value"),
            n_i(b"DCVi", 17),
        ],
    );
    let extra = n_k(
        b"EPSv",
        vec![n_k(
            b"DCEv",
            vec![n_s(b"DCKs", "EffectMoaID"), n_s(b"DCVs", "SecondList")],
        )],
    );
    {
        let pthv = first_path_object_mut(&mut tree);
        let eflv = children_mut(pthv)
            .iter_mut()
            .find(|node| node.fourcc == *b"EFLv")
            .expect("EFLv");
        let epsv = children_mut(eflv)
            .iter_mut()
            .find(|node| node.fourcc == *b"EPSv")
            .expect("EPSv");
        children_mut(epsv).push(malformed.clone());
        children_mut(eflv).push(extra.clone());
    }

    let (doc2, bag) = {
        let bytes = serialize(&tree);
        let parsed = parse(&bytes).expect("parse");
        read(&parsed).expect("read")
    };
    assert_eq!(bag.len(), 2, "malformed DCEv + extra EPSv");
    let mut checked = 0;
    for (_, object) in doc2.registry.iter() {
        if let ObjectData::Path(path) = object {
            if path.name.as_deref() == Some("panel") {
                let effect = &path.effects[0];
                assert_eq!(
                    effect.properties.len(),
                    3,
                    "only the well-formed entries map"
                );
                assert!(
                    effect
                        .properties
                        .iter()
                        .all(|prop| prop.key != "mystery_param")
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 1, "the panel path was inspected");

    // Writing back re-emits both surprises byte-exact at their slots.
    let (mut tree2, stats2) = write(&doc2, &bag).expect("write");
    assert_eq!(stats2.unconsumed_unknown, 0);
    {
        let pthv = first_path_object_mut(&mut tree2);
        let eflv = children_mut(pthv)
            .iter_mut()
            .find(|node| node.fourcc == *b"EFLv")
            .expect("EFLv");
        let kids = children(eflv);
        assert_eq!(
            kids.last(),
            Some(&extra),
            "extra EPSv back at its recorded child index"
        );
        let epsv = &kids[2];
        assert_eq!(
            children(epsv).last(),
            Some(&malformed),
            "malformed DCEv back at its recorded entry index"
        );
    }
}

#[test]
fn projective_primitive_preserves_whole_path_node() {
    // PRIv whose MTXv has a non-identity projective column: the whole
    // PTHv is bagged, not half-mapped.
    let pthv = n_k(
        b"PTHv",
        vec![
            n_k(
                b"PBLv",
                vec![n_k(
                    b"PBPv",
                    vec![
                        n_b(b"ISCb", true),
                        n_i(b"PPCi", 1),
                        n_k(b"PBTv", vec![n_f(b"XLCf", 0.0), n_f(b"YLCf", 0.0)]),
                    ],
                )],
            ),
            n_k(
                b"PRIv",
                vec![
                    n_f(b"PSXf", 0.0),
                    n_k(
                        b"MTXv",
                        vec![
                            n_f(b"M00f", 1.0),
                            n_f(b"M01f", 0.0),
                            n_f(b"M02f", 0.5),
                            n_f(b"M10f", 0.0),
                            n_f(b"M11f", 1.0),
                            n_f(b"M12f", 0.0),
                            n_f(b"M20f", 0.0),
                            n_f(b"M21f", 0.0),
                            n_f(b"M22f", 1.0),
                        ],
                    ),
                    n_k(
                        b"RCTv",
                        vec![
                            n_f(b"RDSf", 0.0),
                            n_f(b"LFTf", 0.0),
                            n_f(b"TOPf", 0.0),
                            n_f(b"RITf", 10.0),
                            n_f(b"BOTf", 10.0),
                        ],
                    ),
                ],
            ),
        ],
    );
    let tree = vec![n_k(
        b"MKBv",
        vec![
            n_i(b"FRCi", 1),
            n_k(
                b"LYLv",
                vec![n_k(
                    b"LAYv",
                    vec![
                        n_b(b"BKGb", false),
                        n_b(b"URLb", false),
                        n_s(b"LNMs", "L"),
                        n_b(b"DISb", true),
                        n_k(
                            b"CLLv",
                            vec![n_k(
                                b"CELv",
                                vec![
                                    n_b(b"LCKb", false),
                                    n_k(b"ELMv", vec![pthv.clone()]),
                                    n_b(b"VISb", true),
                                ],
                            )],
                        ),
                    ],
                )],
            ),
            n_k(b"GRDv", vec![n_i(b"GCLi", 0)]),
            n_k(b"GDSv", vec![n_k(b"GDHv", vec![]), n_k(b"GDVv", vec![])]),
            n_i(b"OSBi", 0),
            n_i(b"OSAi", 0),
        ],
    )];
    let (doc, bag) = read(&tree).expect("read");
    assert!(
        doc.registry.is_empty(),
        "unmodelable path is not half-mapped"
    );
    assert_eq!(bag.len(), 1);
    let (path, index, node) = bag.iter().next().expect("one entry");
    assert!(path.ends_with("ELMv"));
    assert_eq!(index, 0);
    assert_eq!(*node, pthv, "preserved verbatim");
}

#[test]
fn duplicate_scalars_bag_and_reemit() {
    let tree = vec![n_k(
        b"MKBv",
        vec![n_i(b"VRSi", 1), n_i(b"VRSi", 2), n_i(b"FRCi", 3)],
    )];
    let (doc, mut bag) = read(&tree).expect("read");
    assert_eq!(doc.info.version, Some(1));
    assert_eq!(doc.info.frame_count, 3);
    let taken = bag.take_under("/MKBv");
    assert_eq!(taken.len(), 1);
    assert_eq!(taken[0].0, 1, "second VRSi bagged at its index");
    assert_eq!(taken[0].1.as_int(), Some(2));
}

#[test]
fn write_reports_dangling_ids() {
    let mut doc = Document::default();
    doc.pages.push(Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: 1,
            frame_count_duplicate: 1,
            request_id: 0,
            canvas: Canvas::default(),
            onion_before: 0,
            onion_after: 0,
            frame_durations: Vec::new(),
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList {
            layers: vec![Layer {
                cells: vec![Cell {
                    elements: vec![ObjectId::dangling(42, 9)],
                    ..Cell::default()
                }],
                ..Layer::default()
            }],
        },
        grid: None,
        guides: Guides::default(),
        export: None,
    });
    let result = write(&doc, &UnknownBag::new());
    assert_eq!(
        result.err(),
        Some(MapError::DanglingId {
            index: 42,
            generation: 9
        })
    );
}

#[test]
fn bad_shape_is_reported_for_known_nodes() {
    let tree = vec![n_k(b"MKBv", vec![n_s(b"FRCi", "not-an-int")])];
    let err = read(&tree).expect_err("string FRCi is a shape violation");
    assert!(matches!(
        err,
        MapError::BadShape {
            expected: "int",
            ..
        }
    ));
}

// --- helpers ------------------------------------------------------------

fn first_path_object_mut(tree: &mut [Node]) -> &mut Node {
    let pdcv = &mut tree[1];
    assert_eq!(pdcv.fourcc, *b"PDCv");
    let lylv = children_mut(pdcv)
        .iter_mut()
        .find(|node| node.fourcc == *b"LYLv")
        .expect("LYLv");
    let layv = children_mut(lylv)
        .iter_mut()
        .find(|node| node.fourcc == *b"LAYv")
        .expect("LAYv");
    let cllv = children_mut(layv)
        .iter_mut()
        .find(|node| node.fourcc == *b"CLLv")
        .expect("CLLv");
    let celv = children_mut(cllv)
        .iter_mut()
        .find(|node| node.fourcc == *b"CELv")
        .expect("CELv");
    let elmv = children_mut(celv)
        .iter_mut()
        .find(|node| node.fourcc == *b"ELMv")
        .expect("ELMv");
    let first = &mut children_mut(elmv)[0];
    assert_eq!(first.fourcc, *b"PTHv");
    first
}

fn children(node: &Node) -> &Vec<Node> {
    match &node.value {
        NodeValue::Children(children) => children,
        _ => std::panic!("expected a container node"),
    }
}

fn children_mut(node: &mut Node) -> &mut Vec<Node> {
    match &mut node.value {
        NodeValue::Children(children) => children,
        _ => std::panic!("expected a container node"),
    }
}

// --- compile-fail completeness (M3 task 5) ------------------------------

/// Every model struct constructed with a full struct literal — no
/// `..Default::default()`, no `..rest`. Adding a field without mapping
/// it in reader/writer breaks this test's build.
#[test]
fn model_structs_are_fully_constructed() {
    let canvas = Canvas {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
        resolution: 72.0,
        resolution_unit: 0,
        background_raw: 0,
    };
    let _document = Document {
        info: DocumentInfo {
            name: None,
            version: None,
            revision: None,
            frame_count: 1,
            request_id: 0,
            canvas: canvas.clone(),
            onion_before: 0,
            onion_after: 0,
        },
        shared_layers: LayerList { layers: Vec::new() },
        grid: Grid {
            offset: Point::ZERO,
            size: Point::new(24.0, 24.0),
            color_raw: 0,
        },
        guides: Guides {
            locked: false,
            color_raw: 0,
            horizontal: Vec::new(),
            vertical: Vec::new(),
        },
        pages: Vec::new(),
        symbols: Vec::new(),
        registry: Registry::new(),
        ..Document::default()
    };
    let _page = Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: 1,
            frame_count_duplicate: 1,
            request_id: 0,
            canvas: canvas.clone(),
            onion_before: 0,
            onion_after: 0,
            frame_durations: Vec::new(),
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList { layers: Vec::new() },
        grid: None,
        guides: Guides {
            locked: false,
            color_raw: 0,
            horizontal: Vec::new(),
            vertical: Vec::new(),
        },
        export: None,
    };
    let _layer = Layer {
        name: String::new(),
        is_background: false,
        has_url: false,
        is_special: false,
        frame_changing: false,
        display: true,
        nesting_level: None,
        cells: Vec::new(),
        visibility_per_frame: None,
        shared_link: None,
    };
    let _cell = Cell {
        locked: false,
        visible: true,
        elements: Vec::new(),
    };
    let _symbol = SymbolDefinition {
        name: String::new(),
        symbol_number: 0,
        origin: Point::ZERO,
        frame_count: 1,
        layers: LayerList { layers: Vec::new() },
        background_raw: 0,
        symbol_type: SymbolType::Graphic,
        nine_slice: None,
        auto_slice: None,
        include_down_state: None,
        show_down_on_load: None,
        include_over_while_down: None,
        op_code: None,
    };
    let _path = PathObject {
        contours: vec![Contour {
            closed: false,
            point_count: 0,
            points: vec![PathPoint {
                anchor: Point::ZERO,
                primary: None,
                secondary: None,
                is_curve: false,
            }],
        }],
        primitive: Some(PrimitiveSource {
            rect: Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 0.0,
                y1: 0.0,
            },
            radius: 0.0,
            transform: Affine::IDENTITY,
            maintain_radius: false,
        }),
        even_odd: false,
        paint: Some(PaintStyle {
            stroke_raw: 0,
            fill_raw: 0,
            fill: None,
        }),
        fill_start: None,
        fill_end: None,
        fill_third: None,
        brush_placement: None,
        fill_on_top: None,
        locked: false,
        visible: true,
        opacity: 1000,
        blend: 0,
        name: None,
        effects: Vec::new(),
    };
    let _text = TextObject {
        frame: Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
        },
        style: None,
        runs: Vec::new(),
        flow: None,
        on_path: None,
        anti_alias: None,
        auto_enlarge: None,
        auto_kern: None,
        name: None,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        transform: None,
        effects: Vec::new(),
    };
    let _group = GroupObject {
        group_type: 0,
        name: None,
        children: Vec::new(),
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        mask_linked: None,
        effects: Vec::new(),
    };
    let _image = ImageObject {
        location: Point::ZERO,
        offset: Point::ZERO,
        pixels: (0, 0),
        tile_size: 256,
        tiles: vec![TileRef::Tile(0), TileRef::Matte(0)],
        name: None,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        effects: Vec::new(),
    };
    let _slice = SliceObject {
        internal_name: None,
        frame: Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
        },
        shape: 0,
        fill_raw: 0,
        name: None,
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
    };
    let _instance = SymbolInstanceObject {
        symbol_number: 0,
        transform: Affine::IDENTITY,
        locked: false,
        name: None,
        animation: None,
        custom_data: std::collections::BTreeMap::new(),
    };
}

// Keep `children` referenced (used by future tests); silence unused warn
// without dead code in the hot path.
#[test]
fn children_helper_reads_containers() {
    let node = n_k(b"TSTv", vec![n_i(b"AAAi", 1)]);
    assert_eq!(children(&node).len(), 1);
}

/// A `TAPv` guide path maps into `TextObject.on_path` and round-trips:
/// the `PTHv`'s unknown styling children splice back, the contours
/// survive byte-stable regeneration, and a second generation is
/// canonical.
#[test]
fn text_on_path_round_trips_through_bytes() {
    let txtv = n_k(
        b"TXTv",
        vec![
            n_f(b"LFTf", 10.0),
            n_f(b"TOPf", 20.0),
            n_f(b"RITf", 210.0),
            n_f(b"BOTf", 40.0),
            n_i(b"TFTi", 2),
            n_k(
                b"TAPv",
                vec![
                    n_i(b"TAMi", 2),
                    n_f(b"CHOf", 7.5),
                    n_k(
                        b"PTHv",
                        vec![
                            // Unknown styling inside the guide node.
                            n_s(b"OBNs", "guide"),
                            n_k(
                                b"PBLv",
                                vec![n_k(
                                    b"PBPv",
                                    vec![
                                        n_b(b"ISCb", false),
                                        n_i(b"PPCi", 3),
                                        n_k(b"PBTv", vec![n_f(b"XLCf", 0.0), n_f(b"YLCf", 50.0)]),
                                        n_k(
                                            b"PBTv",
                                            vec![
                                                n_f(b"XLCf", 40.0),
                                                n_f(b"YLCf", 0.0),
                                                n_f(b"XPCf", 22.0),
                                                n_f(b"YPCf", 0.0),
                                                n_b(b"CRVb", true),
                                            ],
                                        ),
                                        n_k(
                                            b"PBTv",
                                            vec![
                                                n_f(b"XLCf", 100.0),
                                                n_f(b"YLCf", 0.0),
                                                n_f(b"XSCf", 66.0),
                                                n_f(b"YSCf", 0.0),
                                                n_b(b"CRVb", true),
                                            ],
                                        ),
                                    ],
                                )],
                            ),
                        ],
                    ),
                ],
            ),
            n_s(b"OBNs", "arc tag"),
            n_b(b"VISb", true),
            n_i(b"OPAi", 1000),
            n_i(b"BLDi", 0),
        ],
    );
    let tree = vec![n_k(
        b"MKBv",
        vec![
            n_i(b"FRCi", 1),
            n_k(
                b"LYLv",
                vec![n_k(
                    b"LAYv",
                    vec![
                        n_b(b"BKGb", false),
                        n_b(b"URLb", false),
                        n_s(b"LNMs", "L"),
                        n_b(b"DISb", true),
                        n_k(
                            b"CLLv",
                            vec![n_k(
                                b"CELv",
                                vec![
                                    n_b(b"LCKb", false),
                                    n_k(b"ELMv", vec![txtv]),
                                    n_b(b"VISb", true),
                                ],
                            )],
                        ),
                    ],
                )],
            ),
            n_k(b"GRDv", vec![n_i(b"GCLi", 0)]),
            n_k(b"GDSv", vec![n_k(b"GDHv", vec![]), n_k(b"GDVv", vec![])]),
            n_i(b"OSBi", 0),
            n_i(b"OSAi", 0),
        ],
    )];
    let (doc, bag) = read(&tree).expect("read");
    let (_, ObjectData::Text(text)) = doc.registry.iter().next().expect("text object") else {
        panic!("first object must be the text");
    };
    assert_eq!(text.flow, Some(2));
    let on_path = text.on_path.as_ref().expect("TAPv mapped");
    assert_eq!(on_path.alignment, Some(2));
    assert_eq!(on_path.offset, 7.5);
    assert_eq!(on_path.path.len(), 1);
    let contour = &on_path.path[0];
    assert!(!contour.closed);
    assert_eq!(contour.points.len(), 3);
    assert_eq!(
        contour.points[1].primary.as_ref().map(|p| (p.x, p.y)),
        Some((22.0, 0.0))
    );
    assert_eq!(
        contour.points[2].secondary.as_ref().map(|p| (p.x, p.y)),
        Some((66.0, 0.0))
    );

    // Through bytes: model equal, bagged styling preserved, and the
    // second generation byte-stable.
    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &bag);
    let (_, ObjectData::Text(text2)) = doc2.registry.iter().next().expect("text object") else {
        panic!("text object must survive the round trip");
    };
    assert_eq!(text.on_path, text2.on_path);
    assert!(bag2.iter().any(|(path, _, _)| path.ends_with("TAPv/PTHv")));
    let (bytes2, _doc3, _bag3) = round_trip_bytes(&doc2, &bag2);
    assert_eq!(bytes, bytes2);
}

// --- EXPv export options (M8 task 8) -------------------------------------

/// A zeroed [`PageInfo`] (the model type carries no `Default` — its
/// scalars are semantically required).
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

/// A doc-level `EXPv` carrying every mapped scalar plus two unknowns
/// (`PALv` palette payload, `MCTi` bitmap-format discriminator) at
/// recorded positions.
fn export_tree() -> Vec<Node> {
    vec![n_k(
        b"MKBv",
        vec![
            n_i(b"FRCi", 1),
            n_k(
                b"EXPv",
                vec![
                    n_i(b"FMTi", 1),
                    n_i(b"JPQi", 82),
                    n_b(b"JPPb", true),
                    n_s(b"OBNs", "palette"), // unknown
                    n_i(b"FCLi", 0x00_20_40_80),
                    n_i(b"MCTi", 2), // unknown
                    n_i(b"FLPi", 3),
                ],
            ),
        ],
    )]
}

#[test]
fn expv_document_level_maps_and_round_trips() {
    let tree = export_tree();
    let (doc, bag) = read(&tree).expect("read");
    let export = doc.export.as_ref().expect("EXPv mapped");
    assert_eq!(export.format, Some(1));
    assert_eq!(export.jpeg_quality, Some(82));
    assert_eq!(export.jpeg_progressive, Some(true));
    assert_eq!(export.matte_raw, Some(0x00_20_40_80));
    assert_eq!(export.frame_loop, Some(3));
    // The two unmapped children ride in the bag at exact paths.
    let unknowns: Vec<&str> = bag.iter().map(|(path, _, _)| path).collect();
    assert_eq!(
        unknowns,
        vec!["/MKBv/EXPv", "/MKBv/EXPv"],
        "both unknowns recorded under the EXPv container"
    );

    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &bag);
    assert_eq!(doc.export, doc2.export, "export block survives bytes");
    assert_eq!(bag.iter().count(), bag2.iter().count());
    let (bytes2, _doc3, _bag3) = round_trip_bytes(&doc2, &bag2);
    assert_eq!(bytes, bytes2, "canonical emission is byte-stable");
}

#[test]
fn expv_unknown_children_splice_back_at_recorded_indices() {
    let (tree, _stats) = {
        let (doc, bag) = read(&export_tree()).expect("read");
        write(&doc, &bag).expect("write")
    };
    let mkbv = &tree[0];
    let kids = children(mkbv);
    let expv = kids
        .iter()
        .find(|node| node.fourcc == *b"EXPv")
        .expect("EXPv re-emitted");
    let fourccs: Vec<[u8; 4]> = children(expv).iter().map(|node| node.fourcc).collect();
    assert_eq!(
        fourccs,
        vec![
            *b"FMTi", *b"FCLi", *b"JPQi", *b"OBNs", *b"JPPb", *b"MCTi", *b"FLPi",
        ],
        "mapped slots in canonical order around unknowns at recorded indices"
    );
}

#[test]
fn expv_page_level_with_fnlv_maps_durations() {
    let tree = vec![n_k(
        b"PDCv",
        vec![
            n_i(b"FRCi", 2),
            n_k(
                b"EXPv",
                vec![
                    n_i(b"FMTi", 4),
                    n_k(
                        b"FNLv",
                        vec![
                            n_k(b"FNMv", vec![n_i(b"FNDi", 20), n_s(b"OBNs", "a")]),
                            n_k(
                                b"FNMv",
                                vec![n_i(b"FNDi", 7), n_b(b"VISb", true), n_i(b"FNPi", 0)],
                            ),
                        ],
                    ),
                ],
            ),
            n_k(
                b"LYLv",
                vec![n_k(
                    b"LAYv",
                    vec![
                        n_b(b"BKGb", false),
                        n_b(b"URLb", false),
                        n_s(b"LNMs", "L"),
                        n_b(b"DISb", true),
                        n_k(b"CLLv", vec![]),
                    ],
                )],
            ),
        ],
    )];
    let (doc, bag) = read(&tree).expect("read");
    let page = &doc.pages[0];
    assert_eq!(
        page.info.frame_durations,
        vec![20, 7],
        "positional FNMv mapping"
    );
    let export = page.export.as_ref().expect("page EXPv mapped");
    assert_eq!(export.format, Some(4));

    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &bag);
    assert_eq!(
        doc.pages[0].info.frame_durations,
        doc2.pages[0].info.frame_durations
    );
    assert_eq!(doc.pages[0].export, doc2.pages[0].export);
    // The OBNs/VISb/FNPi configuration scalars are bagged at their
    // FNMv paths and splice back inside the regenerated FNLv.
    assert!(
        bag.iter()
            .any(|(path, _, _)| path == "/PDCv[0]/EXPv/FNLv/FNMv[0]")
    );
    assert!(
        bag.iter()
            .any(|(path, _, _)| path == "/PDCv[0]/EXPv/FNLv/FNMv[1]")
    );
    let (bytes2, _doc3, _bag3) = round_trip_bytes(&doc2, &bag2);
    assert_eq!(bytes, bytes2);
}

#[test]
fn incomplete_fnlv_bags_whole_instead_of_shifting() {
    // Second FNMv lacks FNDi: a positional mapping would make frame 2
    // eat configuration 3's slot, so the entire FNLv is preserved.
    let tree = vec![n_k(
        b"PDCv",
        vec![
            n_i(b"FRCi", 3),
            n_k(
                b"EXPv",
                vec![n_k(
                    b"FNLv",
                    vec![
                        n_k(b"FNMv", vec![n_i(b"FNDi", 20)]),
                        n_k(b"FNMv", vec![n_s(b"OBNs", "no duration")]),
                    ],
                )],
            ),
            n_k(
                b"LYLv",
                vec![n_k(
                    b"LAYv",
                    vec![
                        n_b(b"BKGb", false),
                        n_b(b"URLb", false),
                        n_s(b"LNMs", "L"),
                        n_b(b"DISb", true),
                        n_k(b"CLLv", vec![]),
                    ],
                )],
            ),
        ],
    )];
    let (doc, bag) = read(&tree).expect("read");
    assert!(
        doc.pages[0].info.frame_durations.is_empty(),
        "all-or-nothing: no partial durations"
    );
    assert!(
        bag.iter()
            .any(|(path, _, node)| path == "/PDCv[0]/EXPv" && node.fourcc == *b"FNLv"),
        "the incomplete FNLv is preserved verbatim"
    );

    // Write and re-read: the verbatim FNLv rides along untouched.
    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &bag);
    assert!(doc2.pages[0].info.frame_durations.is_empty());
    assert!(
        bag2.iter()
            .any(|(path, _, node)| path == "/PDCv[0]/EXPv" && node.fourcc == *b"FNLv"),
        "still preserved through the round trip"
    );
    let _ = bytes;
}

#[test]
fn fresh_write_mints_expv_for_durations_only() {
    let mut doc = Document::default();
    doc.pages.push(Page {
        info: PageInfo {
            frame_count: 2,
            frame_durations: vec![10, 50],
            ..page_info_default()
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList::default(),
        grid: None,
        guides: Guides::default(),
        export: None,
    });
    let (tree, stats) = write(&doc, &UnknownBag::new()).expect("write");
    assert_eq!(stats.unconsumed_unknown, 0);
    let pdcv = tree
        .iter()
        .find(|node| node.fourcc == *b"PDCv")
        .expect("page written");
    let expv = children(pdcv)
        .iter()
        .find(|node| node.fourcc == *b"EXPv")
        .expect("durations force an EXPv");
    let fnlv = children(expv)
        .iter()
        .find(|node| node.fourcc == *b"FNLv")
        .expect("FNLv written");
    let fnmvs = children(fnlv);
    assert_eq!(fnmvs.len(), 2);
    assert_eq!(
        children(&fnmvs[0])[0],
        n_i(b"FNDi", 10),
        "canonical FNMv shape: FNDi first"
    );

    // And it reads back into the same durations with no unknowns.
    let bytes = serialize(&tree);
    let parsed = parse(&bytes).expect("parse");
    let (doc2, bag2) = read(&parsed).expect("read");
    assert_eq!(doc2.pages[0].info.frame_durations, vec![10, 50]);
    assert!(bag2.is_empty(), "self-produced tree maps cleanly");
}

#[test]
fn no_expv_emitted_when_unset() {
    let mut doc = Document::default();
    doc.pages.push(Page {
        info: page_info_default(),
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList::default(),
        grid: None,
        guides: Guides::default(),
        export: None,
    });
    let (tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    assert!(
        !children(&tree[0])
            .iter()
            .any(|node| node.fourcc == *b"EXPv"),
        "doc-level EXPv only when export is set"
    );
    assert!(
        !children(&tree[0])
            .iter()
            .any(|node| node.fourcc == *b"PDCv"),
        "default doc has no pages to write"
    );
}

#[test]
fn duplicate_expv_first_wins_second_bags() {
    let tree = vec![n_k(
        b"MKBv",
        vec![
            n_i(b"FRCi", 1),
            n_k(b"EXPv", vec![n_i(b"JPQi", 10)]),
            n_k(b"EXPv", vec![n_i(b"JPQi", 99)]),
        ],
    )];
    let (doc, bag) = read(&tree).expect("read");
    assert_eq!(doc.export.as_ref().and_then(|e| e.jpeg_quality), Some(10));
    assert!(
        bag.iter()
            .any(|(path, index, node)| path == "/MKBv" && index == 2 && node.fourcc == *b"EXPv"),
        "the losing EXPv is preserved verbatim"
    );
}

#[test]
fn expv_bad_payload_shape_is_an_error() {
    let tree = vec![n_k(
        b"MKBv",
        vec![n_k(b"EXPv", vec![n_i(b"JPQi", 10), n_i(b"JPPb", 5)])],
    )];
    // JPPb is a bool slot holding an int: BadShape, not silent coercion.
    assert!(matches!(read(&tree), Err(MapError::BadShape { .. })));
}

#[test]
fn slice_expv_maps_selective_settings() {
    let slice = n_k(
        b"URLv",
        vec![
            n_s(b"INMs", "slice_0"),
            n_i(b"TSLi", 0),
            n_i(b"URSi", 0),
            n_f(b"LFTf", 0.0),
            n_f(b"TOPf", 0.0),
            n_f(b"RITf", 100.0),
            n_f(b"BOTf", 40.0),
            n_k(b"EXPv", vec![n_i(b"FMTi", 1), n_i(b"JPQi", 55)]),
        ],
    );
    let tree = vec![n_k(
        b"MKBv",
        vec![
            n_i(b"FRCi", 1),
            n_k(
                b"LYLv",
                vec![n_k(
                    b"LAYv",
                    vec![
                        n_b(b"BKGb", false),
                        n_b(b"URLb", false),
                        n_s(b"LNMs", "L"),
                        n_b(b"DISb", true),
                        n_k(
                            b"CLLv",
                            vec![n_k(
                                b"CELv",
                                vec![
                                    n_b(b"LCKb", false),
                                    n_k(b"ELMv", vec![slice]),
                                    n_b(b"VISb", true),
                                ],
                            )],
                        ),
                    ],
                )],
            ),
        ],
    )];
    let (doc, bag) = read(&tree).expect("read");
    let mut slices = Vec::new();
    for (_, object) in doc.registry.iter() {
        if let ObjectData::Slice(slice) = object {
            slices.push(slice.clone());
        }
    }
    assert_eq!(slices.len(), 1);
    let export = slices[0].export.as_ref().expect("slice EXPv mapped");
    assert_eq!(export.format, Some(1));
    assert_eq!(
        export.jpeg_quality,
        Some(55),
        "the selective JPEG quality slot"
    );
    assert!(bag.iter().all(|(path, _, _)| !path.contains("EXPv")));
}

#[test]
fn bevv_behavior_payloads_survive_round_trips() {
    // One readable behavior (`BETi` 9 popup-menu plus an opaque
    // payload sibling) and one behavior with no readable `BETi`. The
    // model carries the kinds; everything else must re-emerge verbatim
    // at its recorded position.
    let menu_payload = n_s(b"BMXs", "menu-item-payload");
    let opaque = n_k(
        b"BEHv",
        vec![n_s(b"BZZs", "unreadable-behavior"), n_i(b"BZXi", 7)],
    );
    let slice = n_k(
        b"URLv",
        vec![
            n_s(b"INMs", "slice_0"),
            n_i(b"FCLi", 0),
            n_i(b"URSi", 0),
            n_f(b"LFTf", 0.0),
            n_f(b"TOPf", 0.0),
            n_f(b"RITf", 100.0),
            n_f(b"BOTf", 40.0),
            n_k(
                b"BEVv",
                vec![
                    n_k(b"BEHv", vec![n_i(b"BETi", 9), menu_payload.clone()]),
                    opaque.clone(),
                ],
            ),
            n_i(b"TSLi", 0),
        ],
    );
    let tree = vec![n_k(
        b"MKBv",
        vec![
            n_i(b"FRCi", 1),
            n_k(
                b"LYLv",
                vec![n_k(
                    b"LAYv",
                    vec![
                        n_b(b"BKGb", false),
                        n_b(b"URLb", false),
                        n_s(b"LNMs", "L"),
                        n_b(b"DISb", true),
                        n_k(
                            b"CLLv",
                            vec![n_k(
                                b"CELv",
                                vec![
                                    n_b(b"LCKb", false),
                                    n_k(b"ELMv", vec![slice]),
                                    n_b(b"VISb", true),
                                ],
                            )],
                        ),
                    ],
                )],
            ),
        ],
    )];

    let (doc, bag) = read(&tree).expect("read");
    let mut slices = Vec::new();
    for (_, object) in doc.registry.iter() {
        if let ObjectData::Slice(slice) = object {
            slices.push(slice.clone());
        }
    }
    let slice = slices.first().expect("the slice was modeled");
    assert_eq!(
        slice.behaviors,
        vec![SliceBehavior { kind: 9 }],
        "only the readable behavior is modeled"
    );
    assert_eq!(
        bag.iter()
            .filter(|(path, _, _)| path.ends_with("/BEVv/BEHv[0]"))
            .count(),
        1,
        "the popup-menu payload rides the bag inside its BEHv"
    );
    assert_eq!(
        bag.iter()
            .filter(|(path, _, node)| path.ends_with("/BEVv") && node.fourcc == opaque.fourcc)
            .count(),
        1,
        "the unreadable behavior is whole-bagged at the BEVv level"
    );

    let (tree2, stats) = write(&doc, &bag).expect("write");
    assert_eq!(stats.unconsumed_unknown, 0);
    let mkbv = children(&tree2[0]);
    let layv = &children(
        mkbv.iter()
            .find(|node| node.fourcc == *b"LYLv")
            .expect("LYLv"),
    )[0];
    let celv = &children(
        &children(
            children(layv)
                .iter()
                .find(|node| node.fourcc == *b"CLLv")
                .expect("CLLv"),
        )[0],
    );
    let elm = celv
        .iter()
        .find(|node| node.fourcc == *b"ELMv")
        .expect("ELMv");
    let urlv = children(elm)
        .iter()
        .find(|node| node.fourcc == *b"URLv")
        .expect("URLv");
    let bevv = children(urlv)
        .iter()
        .find(|node| node.fourcc == *b"BEVv")
        .expect("BEVv re-emitted");
    let behvs = children(bevv);
    assert_eq!(behvs.len(), 2);
    assert_eq!(
        serialize(children(&behvs[0])),
        serialize(&[n_i(b"BETi", 9), menu_payload]),
        "the modeled kind re-emits with its payload at the recorded slot"
    );
    assert_eq!(
        serialize(std::slice::from_ref(&behvs[1])),
        serialize(std::slice::from_ref(&opaque)),
        "the unreadable behavior re-emits verbatim"
    );

    // Byte stability: re-read and re-write produce the same tree.
    let (doc3, bag3) = {
        let bytes = serialize(&tree2);
        read(&parse(&bytes).expect("parse")).expect("re-read")
    };
    assert_eq!(doc3, doc);
    let (tree3, _) = write(&doc3, &bag3).expect("rewrite");
    assert_eq!(
        serialize(&tree3),
        serialize(&tree2),
        "second write is stable"
    );
}

// --- M15: FPLv fill subtree ----------------------------------------------

use crate::model::{Dither, Fill, FillShape, Gradient, GradientStop};

/// An `FPLv` in dictionary order with modelable slots plus the nodes
/// the model does not interpret: `RDOb`, the identity names, the
/// observed-always `FETi`/`FRRi` flags, and a `FPTv` pattern tile
/// wrapping an unknown image payload.
fn fplv_children() -> Vec<Node> {
    vec![
        n_s(b"CATs", "Linear Gradient"),
        n_s(b"INMs", "fw.grad.linear"),
        n_s(b"UNMs", "Linear Gradient"),
        n_b(b"RDOb", true),
        n_i(b"FTBi", 0),
        n_i(b"FEFi", 0),
        n_i(b"FSMi", 1),
        n_i(b"FETi", 1),
        n_i(b"FRRi", 0),
        n_i(b"FSXi", 2),
        n_k(
            b"FGLv",
            vec![
                n_s(b"CATs", "Custom"),
                n_s(b"INMs", "fw.grad.custom"),
                n_k(
                    b"FGVv",
                    vec![
                        n_i(b"FNCi", 2),
                        n_k(
                            b"FGIv",
                            vec![n_f(b"FGPf", 0.0), n_i(b"FGCi", 0x00_00_00_FF)],
                        ),
                        n_k(
                            b"FGIv",
                            vec![n_f(b"FGPf", 0.5), n_i(b"FGCi", 0xFF_FF_FF_FF)],
                        ),
                    ],
                ),
                n_k(
                    b"FGYv",
                    vec![
                        n_k(
                            b"FG0v",
                            vec![
                                n_i(b"FNCi", 3),
                                n_k(
                                    b"FGIv",
                                    vec![n_f(b"FGPf", 0.0), n_i(b"FGCi", 0xFF_00_00_FF)],
                                ),
                                n_k(
                                    b"FGIv",
                                    vec![n_f(b"FGPf", 0.5), n_i(b"FGCi", 0x00_FF_00_FF)],
                                ),
                                n_k(
                                    b"FGIv",
                                    vec![n_f(b"FGPf", 1.0), n_i(b"FGCi", 0x00_00_FF_FF)],
                                ),
                            ],
                        ),
                        n_k(
                            b"FG1v",
                            vec![
                                n_i(b"FNCi", 2),
                                n_k(
                                    b"FGIv",
                                    vec![n_f(b"FGPf", 0.0), n_i(b"FGCi", 0xFF_00_00_00)],
                                ),
                                n_k(
                                    b"FGIv",
                                    vec![n_f(b"FGPf", 1.0), n_i(b"FGCi", 0x00_00_00_00)],
                                ),
                            ],
                        ),
                    ],
                ),
            ],
        ),
        n_i(b"FD1i", 0xFF_FF_FF_FF),
        n_i(b"FD2i", 0x00_00_00_FF),
        n_b(b"FDTb", true),
        n_k(
            b"FPTv",
            vec![
                n_s(b"INMs", "tile"),
                n_k(b"IMGv", vec![n_s(b"RAFs", "raw")]),
            ],
        ),
    ]
}

/// The `Fill` the reader must produce for [`fplv_children`].
fn expected_fill() -> Fill {
    Fill {
        shape: FillShape::Linear,
        shape_extended: true,
        texture_blend: 0,
        feather: 0,
        stamping: Some(1),
        hard_edge: false,
        gradient: Some(Gradient {
            vertices: vec![
                GradientStop {
                    position: 0.0,
                    color_raw: 0x00_00_00_FF,
                },
                GradientStop {
                    position: 0.5,
                    color_raw: 0xFF_FF_FF_FF,
                },
            ],
            rgb: vec![
                GradientStop {
                    position: 0.0,
                    color_raw: 0xFF_00_00_FF,
                },
                GradientStop {
                    position: 0.5,
                    color_raw: 0x00_FF_00_FF,
                },
                GradientStop {
                    position: 1.0,
                    color_raw: 0x00_00_FF_FF,
                },
            ],
            alpha: vec![
                GradientStop {
                    position: 0.0,
                    color_raw: 0xFF_00_00_00,
                },
                GradientStop {
                    position: 1.0,
                    color_raw: 0x00_00_00_00,
                },
            ],
        }),
        dither: Some(Dither {
            colors: [Some(0xFF_FF_FF_FF), Some(0x00_00_00_FF), None, None],
            transparent: Some(true),
        }),
        // The fixture's `FPTv` wraps a raw payload the model cannot
        // carry, so the whole tile node rides the bag.
        pattern: None,
    }
}

#[test]
fn the_fplv_subtree_decodes_and_round_trips_byte_exactly() {
    // The sample's paint carries no fill: the injected subtree below
    // is the model's only input.
    let doc = sample_document();

    // Inject the raw subtree into the first path's PATv after the
    // colour slots, the way a real file lays it out.
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    {
        let pthv = first_path_object_mut(&mut tree);
        let patv = &mut children_mut(pthv)[0];
        children_mut(patv).insert(2, n_k(b"FPLv", fplv_children()));
    }

    let (doc2, bag) = {
        let bytes = serialize(&tree);
        read(&parse(&bytes).expect("parse")).expect("read")
    };
    assert_eq!(
        bag.iter().count(),
        9,
        "names, RDOb/FETi/FRRi, FPTv and the two FGLv names ride the bag"
    );
    let mut decoded = 0;
    for (_, object) in doc2.registry.iter() {
        if let ObjectData::Path(path) = object {
            if let Some(paint) = &path.paint {
                if paint.fill.is_some() {
                    decoded += 1;
                    assert_eq!(paint.fill.as_ref(), Some(&expected_fill()));
                }
            }
        }
    }
    assert_eq!(decoded, 1, "exactly the injected path decodes a fill");

    // The byte round trip: write back and compare against the injected
    // bytes — the fill re-emits at the recorded slot in dictionary
    // order with nothing lost or reordered.
    let (tree2, stats2) = write(&doc2, &bag).expect("write");
    assert_eq!(stats2.unconsumed_unknown, 0);
    assert_eq!(serialize(&tree2), serialize(&tree), "byte-exact round trip");

    // Second generation is stable.
    let (doc3, bag3) = {
        let bytes = serialize(&tree2);
        read(&parse(&bytes).expect("parse")).expect("re-read")
    };
    assert_eq!(doc3, doc2);
    let (tree3, _) = write(&doc3, &bag3).expect("rewrite");
    assert_eq!(serialize(&tree3), serialize(&tree2));
}

/// A modelable `FPTv` tile: name, extent, tile side, `TILv` mixing
/// a bank reference and a matte, plus an `UNMs` and an `IMGv` extra
/// the model does not interpret (both must ride the bag).
fn modelable_fptv() -> Node {
    n_k(
        b"FPTv",
        vec![
            n_s(b"INMs", "tile"),
            n_s(b"UNMs", "Checker"),
            n_k(
                b"IMGv",
                vec![
                    n_i(b"WPXi", 256),
                    n_i(b"HPXi", 128),
                    n_i(b"TSZi", 128),
                    n_k(b"TILv", vec![n_i(b"TIDi", 7), n_i(b"TMCi", 0xFF_00_FF_00)]),
                    n_s(b"RAFs", "raw"),
                ],
            ),
        ],
    )
}

#[test]
fn a_modelable_fptv_maps_and_round_trips_byte_exactly() {
    let doc = sample_document();
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    {
        let pthv = first_path_object_mut(&mut tree);
        let patv = &mut children_mut(pthv)[0];
        children_mut(patv).insert(2, n_k(b"FPLv", fplv_children()));
        let position = children_mut(patv)
            .iter()
            .position(|node| node.fourcc == *b"FPLv")
            .expect("FPLv present");
        let fplv = &mut children_mut(patv)[position];
        let tile_position = children_mut(fplv)
            .iter()
            .position(|node| node.fourcc == *b"FPTv")
            .expect("FPTv present");
        children_mut(fplv)[tile_position] = modelable_fptv();
    }

    let bytes = serialize(&tree);
    let (doc2, bag) = read(&parse(&bytes).expect("parse")).expect("read");
    // The whole-tile entry goes away (the tile now models); `UNMs`
    // and the `IMGv` extra take its place: 9 - 1 + 2.
    assert_eq!(bag.iter().count(), 10, "UNMs + RAFs replace the tile");
    let mut seen = 0;
    for (_, object) in doc2.registry.iter() {
        if let ObjectData::Path(path) = object {
            if let Some(paint) = &path.paint {
                if let Some(fill) = &paint.fill {
                    let pattern = fill.pattern.as_ref().expect("pattern maps");
                    assert_eq!(pattern.name.as_deref(), Some("tile"));
                    assert_eq!(pattern.pixels, (256, 128));
                    assert_eq!(pattern.tile_size, 128);
                    assert_eq!(
                        pattern.tiles,
                        vec![TileRef::Tile(7), TileRef::Matte(0xFF_00_FF_00)]
                    );
                    seen += 1;
                }
            }
        }
    }
    assert_eq!(seen, 1);

    let (tree2, stats2) = write(&doc2, &bag).expect("write");
    assert_eq!(stats2.unconsumed_unknown, 0);
    assert_eq!(serialize(&tree2), bytes, "byte-exact round trip");
}

#[test]
fn an_fptv_without_a_usable_imgv_bags_whole() {
    let doc = sample_document();
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    {
        let pthv = first_path_object_mut(&mut tree);
        let patv = &mut children_mut(pthv)[0];
        children_mut(patv).insert(2, n_k(b"FPLv", fplv_children()));
        let position = children_mut(patv)
            .iter()
            .position(|node| node.fourcc == *b"FPLv")
            .expect("FPLv present");
        let fplv = &mut children_mut(patv)[position];
        // Strip the tile's `IMGv` down to a bare name: no extent, no
        // tile list — the whole subtree rides the bag verbatim.
        let tile_position = children_mut(fplv)
            .iter()
            .position(|node| node.fourcc == *b"FPTv")
            .expect("FPTv present");
        children_mut(fplv)[tile_position] = n_k(b"FPTv", vec![n_s(b"INMs", "tile")]);
    }

    let bytes = serialize(&tree);
    let (doc2, bag) = read(&parse(&bytes).expect("parse")).expect("read");
    let mut decoded = false;
    for (_, object) in doc2.registry.iter() {
        if let ObjectData::Path(path) = object {
            if let Some(paint) = &path.paint {
                if let Some(fill) = &paint.fill {
                    assert!(fill.pattern.is_none(), "nothing decoded");
                    decoded = true;
                }
            }
        }
    }
    assert!(decoded, "the fill itself still decoded");
    let (tree2, stats2) = write(&doc2, &bag).expect("write");
    assert_eq!(stats2.unconsumed_unknown, 0);
    assert_eq!(serialize(&tree2), bytes, "byte-exact round trip");
}

#[test]
fn compatibility_fshi_decodes_when_fsxi_is_absent() {
    // An old file carries only `FSHi`; the writer must re-emit `FSHi`
    // (no invented `FSXi`) for byte-exactness.
    let doc = sample_document();
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    {
        let pthv = first_path_object_mut(&mut tree);
        let patv = &mut children_mut(pthv)[0];
        let mut kids = fplv_children();
        // Swap the extended slot for the compatibility one, keeping
        // dictionary order (FSHi sits where FSXi sat).
        let position = kids
            .iter()
            .position(|node| node.fourcc == *b"FSXi")
            .expect("FSXi present");
        kids[position] = n_i(b"FSHi", 3);
        children_mut(patv).insert(2, n_k(b"FPLv", kids));
    }

    let bytes = serialize(&tree);
    let (doc2, bag) = read(&parse(&bytes).expect("parse")).expect("read");
    assert_eq!(bag.iter().count(), 9);
    let mut decoded = 0;
    for (_, object) in doc2.registry.iter() {
        if let ObjectData::Path(path) = object {
            if let Some(fill) = path.paint.as_ref().and_then(|p| p.fill.as_ref()) {
                decoded += 1;
                assert_eq!(fill.shape, FillShape::Radial);
                assert!(!fill.shape_extended, "FSHi carried the kind");
            }
        }
    }
    assert_eq!(decoded, 1, "exactly the injected path decodes a fill");
    let (tree2, _) = write(&doc2, &bag).expect("write");
    assert_eq!(
        serialize(&tree2),
        serialize(&tree),
        "FSHi re-emits, no FSXi invented"
    );
}

#[test]
fn fsxi_wins_and_the_compatibility_slot_splices_back() {
    // A file carrying BOTH slots: the extended kind decodes, the
    // compatibility node rides the bag and re-emits at its index.
    let doc = sample_document();
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    let mut raw_children = fplv_children();
    // Dictionary order puts FSHi immediately before FSXi.
    let position = raw_children
        .iter()
        .position(|node| node.fourcc == *b"FSXi")
        .expect("FSXi present");
    raw_children.insert(position, n_i(b"FSHi", 3));
    {
        let pthv = first_path_object_mut(&mut tree);
        let patv = &mut children_mut(pthv)[0];
        children_mut(patv).insert(2, n_k(b"FPLv", raw_children.clone()));
    }

    let bytes = serialize(&tree);
    let (doc2, bag) = read(&parse(&bytes).expect("parse")).expect("read");
    assert_eq!(
        bag.iter().count(),
        10,
        "9 unmodeled slots + the bagged FSHi"
    );
    let mut decoded = 0;
    for (_, object) in doc2.registry.iter() {
        if let ObjectData::Path(path) = object {
            if let Some(fill) = path.paint.as_ref().and_then(|p| p.fill.as_ref()) {
                decoded += 1;
                assert_eq!(fill.shape, FillShape::Linear, "FSXi carries the kind");
            }
        }
    }
    assert_eq!(decoded, 1, "exactly the injected path decodes a fill");
    let (tree2, stats2) = write(&doc2, &bag).expect("write");
    assert_eq!(stats2.unconsumed_unknown, 0);
    assert_eq!(serialize(&tree2), serialize(&tree), "both slots survive");
}

#[test]
fn an_unknown_shape_bags_the_whole_fill() {
    // FSXi 11 is in neither enumeration: the subtree cannot be
    // modeled, so it stays verbatim in the bag and `fill` stays None.
    let doc = sample_document();
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    {
        let pthv = first_path_object_mut(&mut tree);
        let patv = &mut children_mut(pthv)[0];
        let mut kids = fplv_children();
        let position = kids
            .iter()
            .position(|node| node.fourcc == *b"FSXi")
            .expect("FSXi present");
        kids[position] = n_i(b"FSXi", 11);
        children_mut(patv).insert(2, n_k(b"FPLv", kids));
    }

    let bytes = serialize(&tree);
    let (doc2, bag) = read(&parse(&bytes).expect("parse")).expect("read");
    assert_eq!(bag.iter().count(), 1);
    assert!(bag.iter().any(|(path, _, _)| path.ends_with("/PATv")));
    for (_, object) in doc2.registry.iter() {
        if let ObjectData::Path(path) = object {
            assert!(path.paint.as_ref().and_then(|p| p.fill.as_ref()).is_none());
        }
    }
    // Verbatim preservation through a modeled edit.
    let (tree2, _) = write(&doc2, &bag).expect("write");
    assert_eq!(serialize(&tree2), serialize(&tree));
}

#[test]
fn a_malformed_ramp_bags_the_whole_fill() {
    // FNCi promises three points, the ramp carries two: unmodelable.
    let doc = sample_document();
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    {
        let pthv = first_path_object_mut(&mut tree);
        let patv = &mut children_mut(pthv)[0];
        let mut kids = fplv_children();
        let position = kids
            .iter()
            .position(|node| node.fourcc == *b"FGLv")
            .expect("FGLv present");
        if let NodeValue::Children(fglv) = &mut kids[position].value {
            let fgyv = fglv
                .iter_mut()
                .find(|node| node.fourcc == *b"FGYv")
                .expect("FGYv present");
            if let NodeValue::Children(fgyv_kids) = &mut fgyv.value {
                let fg0 = fgyv_kids
                    .iter_mut()
                    .find(|node| node.fourcc == *b"FG0v")
                    .expect("FG0v present");
                if let NodeValue::Children(ramp) = &mut fg0.value {
                    ramp.truncate(ramp.len() - 1);
                }
            }
        }
        children_mut(patv).insert(2, n_k(b"FPLv", kids));
    }

    let bytes = serialize(&tree);
    let (doc2, bag) = read(&parse(&bytes).expect("parse")).expect("read");
    assert_eq!(bag.iter().count(), 1, "the broken subtree stays whole");
    for (_, object) in doc2.registry.iter() {
        if let ObjectData::Path(path) = object {
            assert!(path.paint.as_ref().and_then(|p| p.fill.as_ref()).is_none());
        }
    }
    let (tree2, _) = write(&doc2, &bag).expect("write");
    assert_eq!(serialize(&tree2), serialize(&tree));
}

// --- M15 task 6: the style model (ZSLv/ZSTv) ----------------------------

/// The raw `ZSTv` of the CS6 library corpus's fixed child spine: names,
/// flags, kind, the `PATv` paint (solid pair, modeled `FPLv` gradient,
/// and the texture subtrees that ride the bag), the fill-handle trio,
/// brush slots, one live effect, the text block (with `CFTs`), and the
/// two `IMGv` texture slots last.
fn zstv_node() -> Node {
    n_k(
        b"ZSTv",
        vec![
            n_s(b"ZSNs", "Pastels 001"),
            n_i(b"ZSFi", 0x1ff),
            n_i(b"ZSOi", 8),
            n_k(
                b"PATv",
                vec![
                    n_i(b"BCLi", 0xff_ff_ff_ff),
                    n_i(b"FCLi", 0xff_f1_1c_06),
                    n_k(b"FPLv", fplv_children()),
                    // Texture subtrees: unmodeled everywhere, bagged
                    // under the `PATv` path and spliced back whole.
                    n_k(
                        b"TXBv",
                        vec![
                            n_s(b"INMs", "DNA"),
                            n_k(
                                b"MSKv",
                                vec![
                                    n_i(b"WPXi", 96),
                                    n_i(b"HPXi", 96),
                                    n_i(b"TSZi", 256),
                                    n_k(b"TILv", vec![n_i(b"TIDi", 0x7fa231)]),
                                ],
                            ),
                        ],
                    ),
                    n_k(
                        b"TXFv",
                        vec![
                            n_s(b"INMs", "4"),
                            n_k(
                                b"MSKv",
                                vec![
                                    n_i(b"WPXi", 60),
                                    n_i(b"HPXi", 60),
                                    n_i(b"TSZi", 256),
                                    n_k(b"TILv", vec![n_i(b"TIDi", 0x7fa232)]),
                                ],
                            ),
                        ],
                    ),
                ],
            ),
            n_f(b"PSXf", 74.0),
            n_f(b"PSYf", 1792.0),
            n_f(b"PEXf", 74.0),
            n_f(b"PEYf", 1817.0),
            n_f(b"PFXf", 124.0),
            n_f(b"PFYf", 1792.0),
            n_i(b"BRPi", 1),
            n_b(b"FOTb", false),
            n_i(b"FALi", 18),
            n_i(b"FATi", 700),
            n_i(b"FARi", 0x7c),
            n_i(b"FABi", 719),
            n_k(
                b"EFLv",
                vec![
                    n_s(b"CATs", "Shadow and Glow"),
                    n_s(b"INMs", "dropShadow"),
                    n_k(
                        b"EPSv",
                        vec![n_k(
                            b"DCEv",
                            vec![n_s(b"DCKs", "gaussian_blur_radius"), n_s(b"DCVs", "5")],
                        )],
                    ),
                ],
            ),
            n_s(b"FONs", "Arial"),
            n_s(b"CFTs", "Arial-Regular"),
            n_f(b"PTSf", 12.0),
            n_b(b"BOLb", false),
            n_b(b"ITLb", false),
            n_b(b"UNDb", false),
            n_i(b"JSTi", 0),
            n_f(b"HSCf", 1.0),
            n_f(b"RKNf", 0.0),
            n_f(b"LEDf", 1.0),
            n_i(b"LDMi", 0),
            n_b(b"ATKb", true),
            n_b(b"TIAb", true),
            n_i(b"TAAi", 0),
            n_i(b"TA1i", 0xc0),
            n_i(b"TA2i", 0x40),
            n_i(b"TOSi", 8),
            // The two texture-image slots that end every record.
            n_k(
                b"IMGv",
                vec![
                    n_b(b"LCKb", false),
                    n_f(b"XOFf", 0.0),
                    n_f(b"YOFf", 0.0),
                    n_f(b"XLCf", 0.0),
                    n_f(b"YLCf", 0.0),
                    n_i(b"WPXi", 45),
                    n_i(b"HPXi", 45),
                    n_i(b"TSZi", 80),
                    n_k(b"TILv", vec![n_i(b"TIDi", 0x7fa241)]),
                    n_k(b"DTAv", Vec::new()),
                ],
            ),
            n_k(
                b"IMGv",
                vec![
                    n_b(b"LCKb", false),
                    n_f(b"XOFf", 0.0),
                    n_f(b"YOFf", 0.0),
                    n_f(b"XLCf", 0.0),
                    n_f(b"YLCf", 0.0),
                    n_i(b"WPXi", 21),
                    n_i(b"HPXi", 21),
                    n_i(b"TSZi", 80),
                    n_k(b"TILv", vec![n_i(b"TIDi", 0x7fa242)]),
                    n_k(b"DTAv", Vec::new()),
                ],
            ),
        ],
    )
}

/// What [`zstv_node`] must decode into — the model twin of the raw
/// record (the gradient is [`expected_fill`]'s).
fn expected_style() -> StyleEntry {
    StyleEntry {
        name: "Pastels 001".to_owned(),
        flags: 0x1ff,
        kind: 8,
        paint: PaintStyle {
            stroke_raw: 0xff_ff_ff_ff,
            fill_raw: 0xff_f1_1c_06,
            fill: Some(Fill {
                shape: FillShape::Linear,
                ..expected_fill()
            }),
        },
        fill_start: Point::new(74.0, 1792.0),
        fill_end: Point::new(74.0, 1817.0),
        fill_third: Point::new(124.0, 1792.0),
        brush_placement: 1,
        fill_on_top: false,
        effects: vec![Effect {
            category: "Shadow and Glow".to_owned(),
            internal_name: "dropShadow".to_owned(),
            properties: vec![EffectProperty {
                key: "gaussian_blur_radius".to_owned(),
                value: EffectValue::Text("5".to_owned()),
            }],
        }],
        text: StyleText {
            margin_left: 18,
            margin_top: 700,
            margin_right: 0x7c,
            margin_bottom: 719,
            font: "Arial".to_owned(),
            font_category: Some("Arial-Regular".to_owned()),
            size: 12.0,
            bold: false,
            italic: false,
            underline: false,
            justification: 0,
            horizontal_scale: 1.0,
            kerning: 0.0,
            leading: 1.0,
            leading_unit: 0,
            auto_kern: true,
            anti_alias_enabled: true,
            anti_alias: 0,
            attr_1: 0xc0,
            attr_2: 0x40,
            tab_size: 8,
        },
    }
}

#[test]
fn style_library_round_trips_byte_exact() {
    let root = vec![n_k(b"ZSLv", vec![zstv_node()])];
    let (styles, bag) = read_style_library(&root).expect("read");
    assert_eq!(styles, vec![expected_style()], "model equality");
    let (tree, stats) = write_style_library(&styles, &bag).expect("write");
    assert_eq!(stats.unconsumed_unknown, 0, "bag fully consumed");
    assert_eq!(
        serialize(&tree),
        serialize(&root),
        "Adobe wire bytes reproduce exactly"
    );
    // Second generation is byte-stable.
    let (styles2, bag2) = read_style_library(&tree).expect("re-read");
    assert_eq!(styles2, styles);
    let (tree2, stats2) = write_style_library(&styles2, &bag2).expect("re-write");
    assert_eq!(serialize(&tree2), serialize(&root));
    assert_eq!(stats2.unconsumed_unknown, 0);
}

#[test]
fn style_record_keeps_texture_slots_verbatim() {
    let root = vec![n_k(b"ZSLv", vec![zstv_node()])];
    let (styles, bag) = read_style_library(&root).expect("read");
    // The two `IMGv` slots and the `PATv` texture subtrees have no
    // model home; they must ride the bag, not vanish.
    let paths: Vec<&str> = bag.iter().map(|(path, _, _)| path).collect();
    assert!(
        paths.iter().any(|p| p.ends_with("/PATv")),
        "PATv texture subtrees bag under the paint path"
    );
    assert!(
        paths.iter().filter(|p| **p == "/ZSLv/ZSTv[0]").count() == 2,
        "the two IMGv slots bag at the record level"
    );
    let (tree, stats) = write_style_library(&styles, &bag).expect("write");
    assert_eq!(
        stats.unconsumed_unknown, 0,
        "every bagged node splices back"
    );
    let mut imgvs = Vec::new();
    find_nodes(&tree, b"IMGv", &mut imgvs);
    assert_eq!(
        imgvs.len(),
        3,
        "both record texture slots splice back (plus the FPLv tile's)"
    );
    // TXBv survives inside the paint too.
    let mut txbvs = Vec::new();
    find_nodes(&tree, b"TXBv", &mut txbvs);
    assert_eq!(txbvs.len(), 1);
}

#[test]
fn style_record_wire_shape_matches_corpus_spine() {
    let doc = Document {
        styles: vec![expected_style()],
        ..Document::default()
    };
    let (tree, _) = write(&doc, &UnknownBag::new()).expect("write");
    let mut zstvs = Vec::new();
    find_nodes(&tree, b"ZSTv", &mut zstvs);
    assert_eq!(zstvs.len(), 1);
    let fourccs: Vec<[u8; 4]> = kids_of(zstvs[0]).iter().map(|n| n.fourcc).collect();
    assert_eq!(
        fourccs,
        vec![
            *b"ZSNs", *b"ZSFi", *b"ZSOi", *b"PATv", *b"PSXf", *b"PSYf", *b"PEXf", *b"PEYf",
            *b"PFXf", *b"PFYf", *b"BRPi", *b"FOTb", *b"FALi", *b"FATi", *b"FARi", *b"FABi",
            *b"EFLv", *b"FONs", *b"CFTs", *b"PTSf", *b"BOLb", *b"ITLb", *b"UNDb", *b"JSTi",
            *b"HSCf", *b"RKNf", *b"LEDf", *b"LDMi", *b"ATKb", *b"TIAb", *b"TAAi", *b"TA1i",
            *b"TA2i", *b"TOSi",
        ],
        "the CS6 corpus's fixed record spine"
    );
}

#[test]
fn document_embedded_style_list_round_trips() {
    let mut doc = sample_document();
    doc.styles = vec![expected_style()];
    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &UnknownBag::new());
    assert!(bag2.is_empty(), "nothing in the mapped spine may be bagged");
    assert_eq!(doc, doc2, "styles survive the document round trip");
    let (bytes2, _doc3, bag3) = round_trip_bytes(&doc2, &bag2);
    assert_eq!(bytes, bytes2, "canonical emission is byte-stable");
    assert!(bag3.is_empty());
    // A document without styles emits no `ZSLv` at all.
    let plain = sample_document();
    let (tree, _) = write(&plain, &UnknownBag::new()).expect("write");
    let mut zslvs = Vec::new();
    find_nodes(&tree, b"ZSLv", &mut zslvs);
    assert!(zslvs.is_empty());
}

#[test]
fn style_library_refuses_non_style_payload() {
    // A document's `mkTS` fed to the library reader is a type error,
    // not an empty library.
    let root = vec![n_k(b"MKBv", Vec::new())];
    assert!(read_style_library(&root).is_err());
    assert!(read_style_library(&[]).is_err());
}

#[test]
fn shared_layers_round_trip_losslessly() {
    let mut doc = sample_document();
    // One `LSEv` definition and one page slot linked to it (`LSAv`).
    doc.shared_layer_defs.push(SharedLayerDef {
        uid: "fc00dead-beef-4000-8000-000000000001".into(),
        is_primary: true,
        layer: Layer {
            name: "Shared header".into(),
            display: true,
            cells: vec![Cell {
                locked: false,
                visible: true,
                elements: Vec::new(),
            }],
            ..Layer::default()
        },
    });
    doc.pages[0].layers.layers.push(Layer {
        shared_link: Some(SharedLayerLink {
            uid: "fc00dead-beef-4000-8000-000000000001".into(),
            is_primary: false,
        }),
        ..Layer::default()
    });

    let (bytes, doc2, bag2) = round_trip_bytes(&doc, &UnknownBag::new());
    assert!(bag2.is_empty(), "typed shared-layer nodes may not bag");
    assert_eq!(doc, doc2, "model equality through bytes");
    let (bytes2, _doc3, bag3) = round_trip_bytes(&doc2, &bag2);
    assert_eq!(bytes, bytes2, "canonical emission is byte-stable");
    assert!(bag3.is_empty());

    // The wire carries the nodes the dictionary names, in place: an
    // `LSMv` under `MKBv` and an `LSAv` inside the page's `LYLv`.
    let (tree, _) = write(&doc, &UnknownBag::new()).expect("write");
    let mut lsmvs = Vec::new();
    find_nodes(&tree, b"LSMv", &mut lsmvs);
    assert_eq!(lsmvs.len(), 1);
    let mut lsavs = Vec::new();
    find_nodes(&tree, b"LSAv", &mut lsavs);
    assert_eq!(lsavs.len(), 1);

    // A document that shares nothing emits no `LSMv` at all.
    let plain = sample_document();
    let (tree, _) = write(&plain, &UnknownBag::new()).expect("write");
    let mut lsmvs = Vec::new();
    find_nodes(&tree, b"LSMv", &mut lsmvs);
    assert!(lsmvs.is_empty());
}

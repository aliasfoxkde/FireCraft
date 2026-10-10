//! Round-trip and preservation proofs for the node-tree ⇄ model
//! mapping (M3 tasks 3–5). The full-struct-literal test is the
//! compile-fail completeness gate: adding a model field without a
//! mapping breaks this build.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::MapError;
use super::reader::read;
use super::unknown::UnknownBag;
use super::writer::{mapped_indices, splice, write};
use crate::model::{
    Canvas, Cell, Contour, Document, DocumentInfo, Grid, GroupObject, Guides, ImageObject, Layer,
    LayerList, Page, PageInfo, PathObject, PathPoint, PrimitiveSource, SliceObject,
    SymbolDefinition, SymbolInstanceObject, TextObject, TileRef,
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
    }));
    let text_id = doc.registry.insert(ObjectData::Text(TextObject {
        frame: Rect {
            x0: 1.0,
            y0: 2.0,
            x1: 91.0,
            y1: 22.0,
        },
        transform: Some(Affine::new([2.0, 0.0, 0.0, 2.0, 0.0, 0.0])),
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
    }));
    let instance_id = doc
        .registry
        .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
            symbol_number: 5,
            transform: Affine::new([1.0, 0.0, 0.0, 1.0, 12.0, 0.0]),
            locked: false,
            name: None,
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
            }],
        },
        background_raw: 0xFF_FF_FF,
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
    // object's children: PATv before PBLv, TOXf after it.
    let doc = sample_document();
    let (mut tree, _stats) = write(&doc, &UnknownBag::new()).expect("write");
    let patv_node = n_k(
        b"PATv",
        vec![n_s(b"BRUs", "brush-payload"), n_i(b"SEDi", 4242)],
    );
    {
        let pthv = first_path_object_mut(&mut tree);
        children_mut(pthv).insert(0, n_f(b"TOXf", 1.5));
        children_mut(pthv).insert(0, patv_node.clone());
    }

    let (mut doc2, bag) = {
        let bytes = serialize(&tree);
        let parsed = parse(&bytes).expect("parse");
        read(&parsed).expect("read")
    };
    assert_eq!(bag.len(), 2, "PATv + TOXf preserved");
    assert!(bag.iter().all(|(path, _, _)| path.ends_with("ELMv[0]")));

    // Modeled edit: turn even-odd off on the path object.
    for (_, object) in doc2.registry.iter_mut() {
        if let ObjectData::Path(path) = object {
            path.even_odd = false;
        }
    }
    let (mut tree2, stats2) = write(&doc2, &bag).expect("write");
    assert_eq!(stats2.unconsumed_unknown, 0);

    let patv_bytes = {
        let pthv2 = first_path_object_mut(&mut tree2);
        let kids = children(pthv2);
        assert_eq!(kids[0].fourcc, *b"PATv", "unknown container still first");
        assert_eq!(kids[1].fourcc, *b"TOXf", "unknown scalar at recorded index");
        serialize(std::slice::from_ref(&kids[0]))
    };
    assert_eq!(
        patv_bytes,
        serialize(&[patv_node]),
        "PATv subtree byte-identical across the edit"
    );
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
    };
    let _text = TextObject {
        frame: Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
        },
        transform: None,
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
    };
    let _instance = SymbolInstanceObject {
        symbol_number: 0,
        transform: Affine::IDENTITY,
        locked: false,
        name: None,
    };
}

// Keep `children` referenced (used by future tests); silence unused warn
// without dead code in the hot path.
#[test]
fn children_helper_reads_containers() {
    let node = n_k(b"TSTv", vec![n_i(b"AAAi", 1)]);
    assert_eq!(children(&node).len(), 1);
}

//! Deterministic randomization for panic hunting.
//!
//! One seeded PRNG plus two builders: a random document and a random
//! catalog call (payload sometimes valid, sometimes corrupt — errors
//! are expected outcomes, panics are not). Shared by three drivers so a
//! crash found by any one reproduces in the others:
//!
//! - `cargo test -p engine` property tests (`tests/catalog_fuzz.rs`),
//! - the stable `panic_hunt` driver (`fuzz/src/bin/panic_hunt.rs`),
//! - the libfuzzer target `engine_commands` (nightly campaigns).
//!
//! The call table here must cover [`crate::catalog::ids`]; a coverage
//! unit test pins that.

use doc::{
    Cell, Contour, Document, DocumentInfo, Effect, EffectProperty, EffectValue, GroupObject,
    ImageObject, Layer, LayerList, ObjectData, Page, PageInfo, PathObject, PathPoint, SliceObject,
    SymbolAnimation, SymbolDefinition, SymbolInstanceObject, SymbolType, TextObject, TileRef,
};

/// A seeded xorshift64* PRNG — small, fast, reproducible across
/// platforms, and independent of any external crate.
#[derive(Clone, Debug)]
pub struct FuzzRng(u64);

impl FuzzRng {
    /// Seeds the generator (any value; never degenerates to zero).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }

    /// The next raw 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A value in `0..bound` (`0` when `bound` is zero).
    pub fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        (self.next_u64() % bound as u64) as usize
    }

    /// `true` with `percent` percent probability.
    pub fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent as usize
    }

    /// Picks an element, or `None` when the slice is empty.
    pub fn pick<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        slice.get(self.below(slice.len()))
    }
}

/// Builds a small random document: 1-3 pages, 1-3 layers each,
/// document frame count 1-3, a mix of object kinds, maybe a group over
/// some of them. Every object is referenced exactly once (leaf in a
/// cell, or group in a cell with its children referenced only by the
/// group).
#[must_use]
pub fn random_document(rng: &mut FuzzRng) -> Document {
    let frame_count = 1 + rng.below(3) as i64;
    let mut document = Document {
        info: DocumentInfo {
            frame_count,
            ..DocumentInfo::default()
        },
        ..Document::default()
    };
    document.info.canvas.width = 32.0 + rng.below(64) as f64;
    document.info.canvas.height = 32.0 + rng.below(64) as f64;

    // One library definition so generated instances resolve during
    // renders — compose refuses a dangling `MSNi` as a document
    // defect. The content is a plain path (no instance inside), so no
    // cycle can arise from the seed itself.
    //
    // The definition content is inserted FIRST, before any page
    // object: the wire carries it inline in the document `MSLv`, which
    // the reader parses ahead of every page layer, so read order must
    // match this insertion order or reloaded documents renumber every
    // object id — silently corrupting journal replay.
    let definition_content = document.registry.insert(random_path(rng, 9_999));
    document.symbols.push(SymbolDefinition {
        name: "fuzz symbol".to_owned(),
        symbol_number: 0,
        origin: geom::Point::ZERO,
        frame_count: 1,
        layers: LayerList {
            layers: vec![Layer {
                name: "fuzz symbol layer".to_owned(),
                cells: vec![Cell {
                    elements: vec![definition_content],
                    ..Cell::default()
                }],
                ..Layer::default()
            }],
        },
        background_raw: 0,
        symbol_type: SymbolType::Graphic,
        nine_slice: None,
        op_code: None,
    });

    // A small style library so the style verbs mostly draw live
    // indices; `fuzz style {index}` names let `style.redefineByName`
    // usually resolve.
    let style_count = rng.below(4);
    for index in 0..style_count {
        document.styles.push(random_style(rng, index));
    }

    let page_count = 1 + rng.below(3);
    for page_index in 0..page_count {
        let mut page = Page {
            name: Some(format!("page-{page_index}")),
            ..page_template(frame_count, &document.info)
        };
        let layer_count = 1 + rng.below(3);
        for layer_index in 0..layer_count {
            let cells = (0..frame_count)
                .map(|_| Cell {
                    locked: rng.chance(10),
                    visible: true,
                    elements: Vec::new(),
                })
                .collect();
            page.layers.layers.push(Layer {
                name: format!("layer-{layer_index}"),
                display: rng.chance(80),
                cells,
                ..Layer::default()
            });
        }
        document.pages.push(page);
    }

    // Objects: a mix of kinds; most are paths. Leaf values are drawn
    // at insertion, in wire order: the fwmap writer emits a page's
    // cell elements cell-major (page, then layer, then cell) with a
    // group's children inline inside its `GRPv`, the reader assigns
    // ids in that encounter order, and a journal replayed over the
    // re-read document renumbers every id when insertion order
    // differs — the equivalence gate catches exactly that, so the
    // registry order here must follow the wire, not the draw order.
    let object_count = 1 + rng.below(6);
    fn draw_leaf(rng: &mut FuzzRng, index: usize) -> ObjectData {
        match rng.below(10) {
            0..=4 => random_path(rng, index),
            5 => random_image(rng),
            6 => random_text(rng),
            7 => random_slice(rng),
            _ => random_symbol_instance(rng),
        }
    }
    #[derive(Clone)]
    enum Top {
        /// One leaf object; kind drawn when the wire reaches it.
        Leaf,
        /// A group over leaf slots; the claimed leaves hang off the
        /// group and are drawn and inserted as the wire meets it.
        Group {
            children: Vec<usize>,
            effects: Vec<Effect>,
        },
    }
    let mut tops: Vec<Top> = vec![Top::Leaf; object_count];
    if object_count >= 2 && rng.chance(50) {
        let mut children = Vec::new();
        for leaf in 0..object_count {
            if rng.chance(50) {
                children.push(leaf);
            }
        }
        if children.len() >= 2 {
            let effects = random_effects(rng);
            let mut kept = Vec::new();
            for (slot, top) in tops.into_iter().enumerate() {
                if !children.contains(&slot) {
                    kept.push(top);
                }
            }
            tops = kept;
            tops.push(Top::Group { children, effects });
        }
    }

    // Distribute the top-level objects over the cells, each exactly
    // once, bottom-first round-robin — then insert in wire order, so
    // registry ids match the reader's encounter order.
    let total_cells: usize = document
        .pages
        .iter()
        .map(|page| {
            page.layers
                .layers
                .iter()
                .map(|layer| layer.cells.len())
                .sum::<usize>()
        })
        .sum();
    if total_cells == 0 {
        return document;
    }
    let mut slots: Vec<Vec<Top>> = vec![Vec::new(); total_cells];
    for (slot, top) in tops.into_iter().enumerate() {
        slots[slot % total_cells].push(top);
    }
    let cells = document
        .pages
        .iter_mut()
        .flat_map(|page| page.layers.layers.iter_mut())
        .flat_map(|layer| layer.cells.iter_mut());
    let mut leaf_counter = 0usize;
    for (cell, cell_tops) in cells.zip(slots) {
        for top in cell_tops {
            let id = match top {
                Top::Leaf => {
                    let id = document.registry.insert(draw_leaf(rng, leaf_counter));
                    leaf_counter += 1;
                    id
                }
                Top::Group { children, effects } => {
                    let child_ids = children
                        .iter()
                        .map(|child| document.registry.insert(draw_leaf(rng, *child)))
                        .collect();
                    document.registry.insert(ObjectData::Group(GroupObject {
                        group_type: 0,
                        name: Some("group".to_owned()),
                        children: child_ids,
                        opacity: 1000,
                        blend: 0,
                        visible: true,
                        locked: false,
                        mask_hidden: false,
                        mask_linked: None,
                        effects,
                    }))
                }
            };
            cell.elements.push(id);
        }
    }

    document
}

fn page_template(frame_count: i64, info: &DocumentInfo) -> Page {
    Page {
        info: PageInfo {
            version: info.version,
            revision: info.revision,
            frame_count,
            frame_count_duplicate: frame_count,
            request_id: 0,
            canvas: info.canvas.clone(),
            onion_before: info.onion_before,
            onion_after: info.onion_after,
            frame_durations: Vec::new(),
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList::default(),
        grid: None,
        guides: Default::default(),
        export: None,
    }
}

fn random_point(rng: &mut FuzzRng) -> geom::Point {
    geom::Point::new(rng.below(64) as f64, rng.below(64) as f64)
}

/// 0..=2 seeded effects with 0..=2 string parameters — the fuzz
/// documents carry live filter stacks for the `filter.*` commands
/// (text-valued parameters only; raw node values arrive via reads).
fn random_effects(rng: &mut FuzzRng) -> Vec<Effect> {
    if !rng.chance(35) {
        return Vec::new();
    }
    (0..1 + rng.below(2))
        .map(|_| Effect {
            category: if rng.chance(50) {
                "Fuzz".to_owned()
            } else {
                String::new()
            },
            internal_name: format!("effect-{}", rng.below(12)),
            properties: (0..rng.below(3))
                .map(|_| EffectProperty {
                    key: format!("p-{}", rng.below(6)),
                    value: EffectValue::Text(format!("{}", rng.below(100))),
                })
                .collect(),
        })
        .collect()
}

fn random_path(rng: &mut FuzzRng, index: usize) -> ObjectData {
    let point_count = 2 + rng.below(3);
    let points: Vec<PathPoint> = (0..point_count)
        .map(|_| PathPoint {
            anchor: random_point(rng),
            primary: None,
            secondary: None,
            is_curve: rng.chance(20),
        })
        .collect();
    ObjectData::Path(PathObject {
        contours: vec![Contour {
            closed: rng.chance(70),
            point_count: points.len() as i64,
            points,
        }],
        primitive: None,
        even_odd: rng.chance(20),
        paint: if rng.chance(50) {
            Some(doc::PaintStyle {
                stroke_raw: rng.next_u64() as i64,
                fill_raw: rng.next_u64() as i64,
                fill: None,
            })
        } else {
            None
        },
        fill_start: None,
        fill_end: None,
        fill_third: None,
        brush_placement: if rng.chance(50) {
            Some(rng.below(3) as i64)
        } else {
            None
        },
        fill_on_top: if rng.chance(50) {
            Some(rng.chance(50))
        } else {
            None
        },
        locked: rng.chance(10),
        visible: true,
        opacity: rng.below(1001) as i64,
        blend: rng.below(27) as i64,
        name: if rng.chance(50) {
            Some(format!("path-{index}"))
        } else {
            None
        },
        effects: random_effects(rng),
    })
}

fn random_image(rng: &mut FuzzRng) -> ObjectData {
    ObjectData::Image(ImageObject {
        location: random_point(rng),
        offset: geom::Point::ZERO,
        pixels: (1 + rng.below(8) as i64, 1 + rng.below(8) as i64),
        tile_size: 128,
        tiles: vec![TileRef::Matte(0xFF_00_00_00)],
        name: None,
        opacity: rng.below(1001) as i64,
        blend: rng.below(27) as i64,
        visible: true,
        locked: rng.chance(10),
        effects: random_effects(rng),
    })
}

fn random_text(rng: &mut FuzzRng) -> ObjectData {
    let a = random_point(rng);
    ObjectData::Text(TextObject {
        frame: geom::Rect::new(a.x, a.y, a.x + 10.0, a.y + 4.0),
        style: Some(doc::TextStyle {
            font: rng.chance(2).then(|| "Liberation Sans".to_owned()),
            size: rng.chance(2).then(|| (rng.below(96) as f64) + 6.0),
            bold: rng.chance(2).then(|| rng.chance(2)),
            italic: rng.chance(2).then(|| rng.chance(2)),
            underline: rng.chance(2).then(|| rng.chance(2)),
            ..doc::TextStyle::default()
        }),
        runs: if rng.chance(4) {
            vec![doc::TextRun {
                text: "FireCraft".to_owned(),
            }]
        } else {
            Vec::new()
        },
        flow: rng.chance(2).then(|| rng.below(4) as i64),
        on_path: None,
        anti_alias: rng.chance(2).then(|| rng.below(5) as i64),
        auto_enlarge: rng.chance(2).then(|| rng.chance(2)),
        auto_kern: rng.chance(2).then(|| rng.chance(2)),
        name: None,
        opacity: rng.below(1001) as i64,
        blend: rng.below(27) as i64,
        visible: true,
        locked: rng.chance(10),
        transform: None,
        effects: random_effects(rng),
    })
}

fn random_slice(rng: &mut FuzzRng) -> ObjectData {
    let a = random_point(rng);
    ObjectData::Slice(SliceObject {
        internal_name: None,
        frame: geom::Rect::new(a.x, a.y, a.x + 8.0, a.y + 8.0),
        shape: 0,
        fill_raw: 0,
        name: None,
        slice_type: None,
        link: None,
        alt: None,
        label: None,
        td_text: None,
        contours: Vec::new(),
        background: None,
        behaviors: Vec::new(),
        custom_data: std::collections::BTreeMap::new(),
        export: None,
    })
}

fn random_symbol_instance(rng: &mut FuzzRng) -> ObjectData {
    // Half of generated instances carry the animation family so the
    // contract invariants cover the `ANNi` block too.
    let animation = if rng.chance(50) {
        Some(SymbolAnimation {
            frames: rng.below(64) as i64 + 1,
            x_shift: rng.below(200) as f64 - 100.0,
            y_shift: rng.below(200) as f64 - 100.0,
            scaling: rng.below(150) as f64 + 100.0,
            rotation: if rng.chance(50) {
                Some(rng.below(720) as i64 - 360)
            } else {
                None
            },
            start_opacity: if rng.chance(50) {
                Some(rng.below(1001) as i64)
            } else {
                None
            },
            end_opacity: if rng.chance(50) {
                Some(rng.below(1001) as i64)
            } else {
                None
            },
        })
    } else {
        None
    };
    ObjectData::SymbolInstance(SymbolInstanceObject {
        symbol_number: 0,
        transform: geom::Affine::IDENTITY,
        locked: rng.chance(10),
        name: None,
        animation,
        custom_data: std::collections::BTreeMap::new(),
    })
}

/// One seed entry for the fuzz document's style library: names are
/// `fuzz style {index}` so the `style.redefineByName` builder usually
/// resolves, and every third entry carries the text kind so `style.apply`
/// draws both `ZSOi` values.
/// Base64 for fuzz payloads (the tile wire format).
fn b64(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn random_style(rng: &mut FuzzRng, index: usize) -> doc::StyleEntry {
    let effects = if rng.chance(40) {
        vec![doc::Effect {
            category: String::new(),
            internal_name: format!("fx-{}", rng.below(6)),
            properties: vec![doc::EffectProperty {
                key: "p".to_owned(),
                value: doc::EffectValue::Text(format!("{}", rng.below(20))),
            }],
        }]
    } else {
        Vec::new()
    };
    doc::StyleEntry {
        name: format!("fuzz style {index}"),
        flags: 0x1ff,
        kind: if index % 3 == 2 { 0x1f } else { 8 },
        paint: doc::PaintStyle {
            stroke_raw: rng.next_u64() as i64,
            fill_raw: rng.next_u64() as i64,
            fill: Some(doc::Fill {
                shape: doc::FillShape::Solid,
                shape_extended: false,
                texture_blend: 0,
                feather: 0,
                stamping: None,
                hard_edge: false,
                gradient: None,
                dither: None,
                pattern: None,
            }),
        },
        fill_start: geom::Point::ZERO,
        fill_end: geom::Point::new(rng.below(64) as f64, rng.below(64) as f64),
        fill_third: geom::Point::ZERO,
        brush_placement: rng.below(3) as i64,
        fill_on_top: rng.chance(50),
        effects,
        text: doc::StyleText::default(),
    }
}

/// Every catalog id the call builder knows. Kept in lockstep with
/// [`crate::catalog::install`] by a unit test.
pub const CALLS: &[&str] = &[
    "object.setOpacity",
    "object.setBlend",
    "object.setVisible",
    "object.setLocked",
    "object.setFill",
    "object.setStroke",
    "object.rename",
    "object.delete",
    "object.duplicate",
    "object.translate",
    "object.reorder",
    "object.moveToCell",
    "object.group",
    "object.ungroup",
    "object.transform",
    "object.scale",
    "object.rotate",
    "object.skew",
    "object.reflect",
    "object.resize",
    "object.setBounds",
    "object.createPath",
    "object.align",
    "object.distribute",
    "autoshape.rect",
    "autoshape.star",
    "autoshape.polygon",
    "autoshape.spiral",
    "autoshape.pie",
    "autoshape.doughnut",
    "autoshape.lshape",
    "autoshape.arrow",
    "autoshape.arrowLine",
    "autoshape.measure",
    "autoshape.connectorLine",
    "path.boolean",
    "path.simplify",
    "path.expandStroke",
    "path.inset",
    "path.movePoints",
    "path.warp",
    "path.reshapeArea",
    "path.redrawPath",
    "path.setFill",
    "path.setFillColor",
    "path.setFillVector",
    "filter.add",
    "filter.remove",
    "filter.setParam",
    "cell.setVisible",
    "cell.setLocked",
    "layer.add",
    "layer.delete",
    "layer.rename",
    "layer.setDisplay",
    "page.add",
    "page.delete",
    "page.setCanvasSize",
    "page.addGuide",
    "page.changeGuide",
    "page.removeGuide",
    "page.removeAllGuides",
    "page.setGridOrigin",
    "page.setGridSize",
    "page.setOnionSkinning",
    "frame.add",
    "frame.delete",
    "frame.duplicate",
    "frame.reorder",
    "frame.distributeLayer",
    "bitmap.selectRect",
    "bitmap.selectEllipse",
    "bitmap.selectLasso",
    "bitmap.selectPolygon",
    "bitmap.selectWand",
    "bitmap.feather",
    "bitmap.selectNone",
    "bitmap.fill",
    "bitmap.erase",
    "bitmap.brush",
    "bitmap.pencil",
    "bitmap.movePixels",
    "retouch.blur",
    "retouch.smudge",
    "retouch.clone",
    "retouch.eraser",
    "retouch.redEye",
    "text.createText",
    "text.setText",
    "text.setAttrs",
    "text.setFont",
    "text.attachPath",
    "text.detachPath",
    "symbol.create",
    "symbol.createInstance",
    "symbol.breakApart",
    "symbol.setType",
    "symbol.set9Slice",
    "symbol.setScript",
    "symbol.setParam",
    "symbol.setCustomData",
    "symbol.editAttach",
    "symbol.editDetach",
    "symbol.editReorder",
    "library.add",
    "library.importSymbol",
    "library.remove",
    "library.rename",
    "slice.create",
    "hotspot.create",
    "slice.setLink",
    "slice.setType",
    "slice.setBackground",
    "slice.setExport",
    "slice.addBehavior",
    "slice.removeBehavior",
    "slice.setData",
    "style.apply",
    "style.redefineByIndex",
    "style.redefineByName",
    "style.redefineFromSelection",
    "style.importLibrary",
    "style.importLibraryTiles",
    "style.scaleStyles",
    "mask.add",
    "mask.enable",
    "mask.link",
    "mask.moveBy",
    "mask.moveContentsBy",
    "mask.remove",
    "mask.setMode",
    "mask.setShowAttrs",
    "mask.pathToMarquee",
    "mask.marqueeToPath",
    "mask.saveJPEG",
    "mask.restoreJPEG",
    "mask.clearJPEG",
    "bitmap.movePixelMaskBy",
    "bitmap.setPixelMask",
    "page.setName",
    "page.reorder",
    "page.setMaster",
    "page.unsetMaster",
    "page.addMasterLayer",
    "layer.reorder",
    "layer.setLocked",
];

/// Builds a random call against `document`: a random catalog id and a
/// payload whose targets are usually live and whose values are usually
/// in range — with deliberate invalid variation, because refused calls
/// must not mutate either.
#[must_use]
pub fn random_call(rng: &mut FuzzRng, document: &Document) -> (String, serde_json::Value) {
    let Some(name) = rng.pick(CALLS) else {
        return ("object.setOpacity".to_owned(), serde_json::json!({}));
    };
    let name = *name;

    let live_ids: Vec<doc::ObjectId> = document.registry.iter().map(|(id, _)| id).collect();
    let any_id = |rng: &mut FuzzRng| -> serde_json::Value {
        if !live_ids.is_empty() && rng.chance(85) {
            serde_json::json!(rng.pick(&live_ids).copied())
        } else {
            // Stale or out-of-range slot on purpose.
            let index = rng.below(live_ids.len() + 2);
            serde_json::json!(doc::ObjectId::from_parts(
                index as u32,
                rng.next_u64() as u32
            ))
        }
    };
    let any_index = |rng: &mut FuzzRng, bound: usize| rng.below(bound + 2);
    let any_opt_index = |rng: &mut FuzzRng, bound: usize| -> serde_json::Value {
        if rng.chance(70) {
            serde_json::json!(rng.below(bound + 2))
        } else {
            serde_json::Value::Null
        }
    };
    // Guide coordinates: mostly on-canvas, sometimes off-canvas
    // (negative is legal — guides need not sit inside the canvas).
    let guide_position = |rng: &mut FuzzRng| -> f64 {
        if rng.chance(80) {
            rng.below(600) as f64
        } else {
            -(rng.below(100) as f64)
        }
    };

    let pages = document.pages.len();
    let styles = document.styles.len();
    let frames = 1 + rng.below(3);
    let payload = match name {
        "object.setOpacity" => serde_json::json!({
            "id": any_id(rng),
            "opacity": rng.below(1100),
        }),
        "object.setBlend" => serde_json::json!({
            "id": any_id(rng),
            "blend": rng.below(30),
        }),
        "object.setVisible" => serde_json::json!({
            "id": any_id(rng),
            "visible": rng.chance(50),
        }),
        "object.setLocked" => serde_json::json!({
            "id": any_id(rng),
            "locked": rng.chance(50),
        }),
        "object.setFill" | "object.setStroke" => serde_json::json!({
            "id": any_id(rng),
            "color_raw": rng.next_u64() as i64,
        }),
        "object.rename" => serde_json::json!({
            "id": any_id(rng),
            "name": if rng.chance(80) {
                serde_json::Value::String(format!("n-{}", rng.below(1000)))
            } else {
                serde_json::Value::Null
            },
        }),
        "object.delete" => serde_json::json!({ "id": any_id(rng) }),
        "filter.add" => {
            let mut payload = serde_json::json!({
                "id": any_id(rng),
                "name": if rng.chance(5) {
                    String::new()
                } else {
                    format!("effect-{}", rng.below(12))
                },
            });
            if rng.chance(60) {
                payload["category"] = serde_json::Value::String("Fuzz".to_owned());
            }
            if rng.chance(70) {
                payload["params"] = serde_json::json!([
                    {
                        "key": format!("p-{}", rng.below(6)),
                        "value": format!("{}", rng.below(100)),
                    },
                ]);
            }
            if rng.chance(50) {
                payload["index"] = any_opt_index(rng, 4);
            }
            payload
        }
        "filter.remove" => serde_json::json!({
            "id": any_id(rng),
            "index": rng.below(5),
        }),
        "filter.setParam" => serde_json::json!({
            "id": any_id(rng),
            "index": rng.below(5),
            "key": format!("p-{}", rng.below(6)),
            "value": format!("{}", rng.below(200)),
        }),
        "object.duplicate" => serde_json::json!({ "id": any_id(rng) }),
        "object.translate" => {
            // Either one id or a small spread of live ids: the group +
            // descendant overlap is exactly the case the dedup guards.
            let ids: Vec<serde_json::Value> = if live_ids.is_empty() || rng.chance(20) {
                vec![any_id(rng)]
            } else {
                let count = 1 + rng.below(live_ids.len().min(4));
                (0..count)
                    .map(|_| serde_json::json!(rng.pick(&live_ids).copied()))
                    .collect()
            };
            serde_json::json!({
                "ids": ids,
                "dx": rng.below(201) as f64 - 100.0,
                "dy": rng.below(201) as f64 - 100.0,
            })
        }
        "object.reorder" => serde_json::json!({
            "id": any_id(rng),
            "to": rng.below(8),
        }),
        "object.moveToCell" => serde_json::json!({
            "id": any_id(rng),
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "index": any_opt_index(rng, 6),
        }),
        "object.createPath" => serde_json::json!({
            "kind": rng.pick(&["rect", "ellipse", "line"]).copied(),
            "x": -(rng.below(400) as f64),
            "y": -(rng.below(400) as f64),
            "width": rng.below(400) as f64,
            "height": rng.below(400) as f64,
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": if rng.chance(50) {
                serde_json::Value::Null
            } else {
                serde_json::json!("fuzz path")
            },
        }),
        "autoshape.rect" => serde_json::json!({
            "x": -(rng.below(400) as f64),
            "y": -(rng.below(400) as f64),
            "width": rng.below(400) as f64,
            "height": rng.below(400) as f64,
            "corner": rng.below(100) as f64,
            "kind": rng.pick(&["bevel", "round", "chamfer"]).copied(),
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": if rng.chance(50) {
                serde_json::Value::Null
            } else {
                serde_json::json!("fuzz shape")
            },
        }),
        "autoshape.star" => serde_json::json!({
            "centerX": -(rng.below(400) as f64),
            "centerY": -(rng.below(400) as f64),
            "outer": rng.below(300) as f64,
            "inner": rng.below(150) as f64,
            "outerRound": rng.below(300) as f64,
            "innerRound": rng.below(150) as f64,
            "points": 3 + rng.below(20),
            "startAngle": (rng.below(628) as f64) / 100.0 - std::f64::consts::PI,
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": serde_json::Value::Null,
        }),
        "autoshape.polygon" => serde_json::json!({
            "centerX": -(rng.below(400) as f64),
            "centerY": -(rng.below(400) as f64),
            "outer": rng.below(300) as f64,
            "inner": rng.below(150) as f64,
            "points": 3 + rng.below(20),
            "sections": 1 + rng.below(20),
            "startAngle": (rng.below(628) as f64) / 100.0 - std::f64::consts::PI,
            "split": rng.chance(2),
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": serde_json::Value::Null,
        }),
        "autoshape.spiral" => serde_json::json!({
            "centerX": -(rng.below(400) as f64),
            "centerY": -(rng.below(400) as f64),
            "radius": rng.below(300) as f64,
            "tightnessDeg": 50.0 + (rng.below(39) as f64),
            "clockwise": rng.chance(2),
            "closed": rng.chance(5),
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": serde_json::Value::Null,
        }),
        "autoshape.pie" | "autoshape.doughnut" => serde_json::json!({
            "centerX": -(rng.below(400) as f64),
            "centerY": -(rng.below(400) as f64),
            "outer": rng.below(300) as f64,
            "inner": rng.below(150) as f64,
            "start": (rng.below(628) as f64) / 100.0 - std::f64::consts::PI,
            "end": (rng.below(628) as f64) / 100.0 - std::f64::consts::PI,
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": serde_json::Value::Null,
        }),
        "autoshape.lshape" => serde_json::json!({
            "x": -(rng.below(400) as f64),
            "y": -(rng.below(400) as f64),
            "width": rng.below(400) as f64,
            "height": rng.below(400) as f64,
            "thickness": rng.below(200) as f64,
            "roundness": rng.below(200) as f64,
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": serde_json::Value::Null,
        }),
        "autoshape.arrow" => serde_json::json!({
            "x": -(rng.below(400) as f64),
            "y": -(rng.below(400) as f64),
            "width": rng.below(400) as f64,
            "height": rng.below(400) as f64,
            "thickness": rng.below(200) as f64,
            "roundness": rng.below(200) as f64,
            "arrowWidth": rng.below(300) as f64,
            "arrowLength": rng.below(100) as f64,
            "arrowTip": rng.below(200) as f64,
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": serde_json::Value::Null,
        }),
        "autoshape.arrowLine" => serde_json::json!({
            "startX": -(rng.below(400) as f64),
            "startY": -(rng.below(400) as f64),
            "endX": rng.below(400) as f64,
            "endY": rng.below(400) as f64,
            "startHead": rng.pick(&["none", "open", "solid", "tail", "box"]).copied(),
            "endHead": rng.pick(&["none", "open", "solid", "tail", "box"]).copied(),
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": serde_json::Value::Null,
        }),
        "autoshape.measure" => serde_json::json!({
            "startX": -(rng.below(400) as f64),
            "startY": -(rng.below(400) as f64),
            "endX": rng.below(400) as f64,
            "endY": rng.below(400) as f64,
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": serde_json::Value::Null,
        }),
        "autoshape.connectorLine" => serde_json::json!({
            "startX": -(rng.below(400) as f64),
            "startY": -(rng.below(400) as f64),
            "endX": rng.below(400) as f64,
            "endY": rng.below(400) as f64,
            "elbowX": -(rng.below(200) as f64),
            "elbowY": -(rng.below(200) as f64),
            "corner1": rng.below(80) as f64,
            "corner2": rng.below(80) as f64,
            "vertical": rng.chance(2),
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "name": serde_json::Value::Null,
        }),
        "object.align" => {
            let count = 2 + rng.below(3);
            let ids: Vec<serde_json::Value> = (0..count).map(|_| any_id(rng)).collect();
            serde_json::json!({
                "ids": ids,
                "mode": rng
                    .pick(&["left", "centerX", "right", "top", "centerY", "bottom"])
                    .copied(),
            })
        }
        "object.distribute" => {
            let count = 3 + rng.below(3);
            let ids: Vec<serde_json::Value> = (0..count).map(|_| any_id(rng)).collect();
            serde_json::json!({
                "ids": ids,
                "axis": rng.pick(&["horizontal", "vertical"]).copied(),
            })
        }
        "object.group" => {
            let count = 2 + rng.below(3);
            let ids: Vec<serde_json::Value> = (0..count).map(|_| any_id(rng)).collect();
            serde_json::json!({ "ids": ids, "name": serde_json::Value::Null })
        }
        "object.ungroup" => serde_json::json!({ "id": any_id(rng) }),
        // The transform family: ids follow the translate idiom (one id
        // or a small spread; the dedup guard covers group+descendant).
        // Singular and degenerate matrices are in-band values — they
        // must still round-trip through undo byte-exactly.
        "object.transform" => {
            let ids: Vec<serde_json::Value> = if live_ids.is_empty() || rng.chance(20) {
                vec![any_id(rng)]
            } else {
                let count = 1 + rng.below(live_ids.len().min(4));
                (0..count)
                    .map(|_| serde_json::json!(rng.pick(&live_ids).copied()))
                    .collect()
            };
            serde_json::json!({
                "ids": ids,
                "matrix": [
                    rng.below(401) as f64 / 100.0 - 2.0,
                    rng.below(401) as f64 / 100.0 - 2.0,
                    rng.below(401) as f64 / 100.0 - 2.0,
                    rng.below(401) as f64 / 100.0 - 2.0,
                    rng.below(201) as f64 - 100.0,
                    rng.below(201) as f64 - 100.0,
                ],
            })
        }
        "object.scale" => {
            let ids: Vec<serde_json::Value> = if live_ids.is_empty() || rng.chance(20) {
                vec![any_id(rng)]
            } else {
                let count = 1 + rng.below(live_ids.len().min(4));
                (0..count)
                    .map(|_| serde_json::json!(rng.pick(&live_ids).copied()))
                    .collect()
            };
            let pivot = if rng.chance(50) {
                serde_json::json!([rng.below(600) as f64, rng.below(600) as f64])
            } else {
                serde_json::Value::Null
            };
            serde_json::json!({
                "ids": ids,
                "sx": rng.below(301) as f64 / 100.0,
                "sy": rng.below(301) as f64 / 100.0,
                "pivot": pivot,
            })
        }
        "object.rotate" => {
            let ids: Vec<serde_json::Value> = if live_ids.is_empty() || rng.chance(20) {
                vec![any_id(rng)]
            } else {
                let count = 1 + rng.below(live_ids.len().min(4));
                (0..count)
                    .map(|_| serde_json::json!(rng.pick(&live_ids).copied()))
                    .collect()
            };
            let pivot = if rng.chance(50) {
                serde_json::json!([rng.below(600) as f64, rng.below(600) as f64])
            } else {
                serde_json::Value::Null
            };
            serde_json::json!({
                "ids": ids,
                "degrees": rng.below(3600) as f64 / 10.0 - 180.0,
                "pivot": pivot,
            })
        }
        "object.skew" => {
            let ids: Vec<serde_json::Value> = if live_ids.is_empty() || rng.chance(20) {
                vec![any_id(rng)]
            } else {
                let count = 1 + rng.below(live_ids.len().min(4));
                (0..count)
                    .map(|_| serde_json::json!(rng.pick(&live_ids).copied()))
                    .collect()
            };
            let pivot = if rng.chance(50) {
                serde_json::json!([rng.below(600) as f64, rng.below(600) as f64])
            } else {
                serde_json::Value::Null
            };
            serde_json::json!({
                "ids": ids,
                "skew_x": rng.below(1601) as f64 / 10.0 - 80.0,
                "skew_y": rng.below(1601) as f64 / 10.0 - 80.0,
                "pivot": pivot,
            })
        }
        "object.reflect" => {
            let ids: Vec<serde_json::Value> = if live_ids.is_empty() || rng.chance(20) {
                vec![any_id(rng)]
            } else {
                let count = 1 + rng.below(live_ids.len().min(4));
                (0..count)
                    .map(|_| serde_json::json!(rng.pick(&live_ids).copied()))
                    .collect()
            };
            let pivot = if rng.chance(50) {
                serde_json::json!([rng.below(600) as f64, rng.below(600) as f64])
            } else {
                serde_json::Value::Null
            };
            serde_json::json!({
                "ids": ids,
                "horizontal": rng.chance(50),
                "vertical": rng.chance(50),
                "pivot": pivot,
            })
        }
        "object.resize" => serde_json::json!({
            "ids": [any_id(rng)],
            "width": 1.0 + rng.below(600) as f64,
            "height": 1.0 + rng.below(600) as f64,
        }),
        "object.setBounds" => serde_json::json!({
            "ids": [any_id(rng)],
            "left": -(rng.below(400) as f64),
            "top": -(rng.below(400) as f64),
            "width": 1.0 + rng.below(600) as f64,
            "height": 1.0 + rng.below(600) as f64,
        }),
        "path.movePoints" => {
            let edits: Vec<serde_json::Value> = (0..1 + rng.below(4))
                .map(|_| {
                    serde_json::json!({
                        "contour": rng.below(4),
                        "point": rng.below(12),
                        "dx": -(rng.below(100) as f64) / 10.0,
                        "dy": -(rng.below(100) as f64) / 10.0,
                    })
                })
                .collect();
            serde_json::json!({ "id": any_id(rng), "edits": edits })
        }
        "path.warp" => serde_json::json!({
            "id": any_id(rng),
            "x": -(rng.below(400) as f64),
            "y": -(rng.below(400) as f64),
            "radius": 5.0 + rng.below(200) as f64,
            "dx": -(rng.below(100) as f64) / 5.0,
            "dy": -(rng.below(100) as f64) / 5.0,
        }),
        "path.setFill" => {
            let mut payload = serde_json::json!({
                "id": any_id(rng),
                "shape": rng
                    .pick(&[
                        "solid",
                        "linear",
                        "radial",
                        "conical",
                        "elliptical",
                        "contour",
                        "unknownFamily",
                    ])
                    .copied()
                    .unwrap_or("linear"),
                "stops": [
                    { "position": 0.0, "color_raw": rng.next_u64() as i64 },
                    { "position": 1.0, "color_raw": rng.next_u64() as i64 },
                ],
            });
            if rng.chance(50) {
                payload["alpha"] = serde_json::json!([
                    { "position": 0.0, "color_raw": 0 },
                    { "position": 1.0, "color_raw": 0xFF00_0000u64 },
                ]);
            }
            payload
        }
        "path.setFillColor" => serde_json::json!({
            "id": any_id(rng),
            "color_raw": rng.next_u64() as i64,
        }),
        "path.setFillVector" => {
            let mut payload = serde_json::json!({
                "id": any_id(rng),
                "start": {
                    "x": -(rng.below(200) as f64),
                    "y": -(rng.below(200) as f64),
                },
                "end": {
                    "x": rng.below(300) as f64,
                    "y": rng.below(300) as f64,
                },
            });
            if rng.chance(30) {
                payload["third"] = serde_json::json!({
                    "x": rng.below(300) as f64,
                    "y": rng.below(300) as f64,
                });
            }
            payload
        }
        "path.reshapeArea" => serde_json::json!({
            "id": any_id(rng),
            "x0": -(rng.below(200) as f64),
            "y0": -(rng.below(200) as f64),
            "x1": rng.below(300) as f64,
            "y1": rng.below(300) as f64,
            "falloff": rng.below(60) as f64,
            "dx": -(rng.below(100) as f64) / 5.0,
            "dy": -(rng.below(100) as f64) / 5.0,
        }),
        "path.redrawPath" => {
            let points: Vec<serde_json::Value> = (0..1 + rng.below(4))
                .map(|_| {
                    serde_json::json!({
                        "x": -(rng.below(200) as f64),
                        "y": -(rng.below(200) as f64),
                        "isCurve": rng.below(2) == 0,
                    })
                })
                .collect();
            serde_json::json!({
                "id": any_id(rng),
                "contour": rng.below(4),
                "from": rng.below(12),
                "to": rng.below(12),
                "points": points,
            })
        }
        "path.boolean" => serde_json::json!({
            "a": any_id(rng),
            "b": any_id(rng),
            "op": rng.pick(&["union", "intersect", "punch", "crop"]).copied(),
        }),
        "path.simplify" => serde_json::json!({
            "id": any_id(rng),
            "tolerance": if rng.chance(85) {
                0.1 + rng.below(400) as f64 / 100.0
            } else {
                // Non-positive on purpose (refused).
                -(rng.below(10) as f64)
            },
        }),
        "path.expandStroke" => serde_json::json!({
            "id": any_id(rng),
            "width": if rng.chance(85) { 1.0 + rng.below(40) as f64 } else { 0.0 },
            "cap": rng.pick(&["butt", "square", "round"]).copied(),
            "join": rng.pick(&["miter", "bevel", "round"]).copied(),
            "miter_limit": 1.0 + rng.below(80) as f64 / 10.0,
        }),
        "path.inset" => {
            let distance = |rng: &mut FuzzRng| -> f64 {
                if rng.chance(85) {
                    let magnitude = rng.below(30) as f64 / 10.0;
                    if rng.chance(50) {
                        magnitude
                    } else {
                        -magnitude
                    }
                } else {
                    // Non-finite on purpose (refused).
                    f64::NAN
                }
            };
            serde_json::json!({
                "id": any_id(rng),
                "distance": distance(rng),
            })
        }
        "cell.setVisible" => serde_json::json!({
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "visible": rng.chance(50),
        }),
        "cell.setLocked" => serde_json::json!({
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "locked": rng.chance(50),
        }),
        "layer.add" => serde_json::json!({
            "page": any_index(rng, pages),
            "name": if rng.chance(50) {
                serde_json::Value::Null
            } else {
                serde_json::json!(format!("L-{}", rng.below(100)))
            },
            "index": any_opt_index(rng, 4),
        }),
        "layer.delete" => serde_json::json!({
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
        }),
        "layer.rename" => serde_json::json!({
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "name": format!("L-{}", rng.below(100)),
        }),
        "layer.setDisplay" => serde_json::json!({
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "display": rng.chance(50),
        }),
        "page.add" => serde_json::json!({ "index": any_opt_index(rng, pages) }),
        "page.delete" => serde_json::json!({ "page": any_index(rng, pages) }),
        "page.addGuide" => serde_json::json!({
            "page": any_index(rng, pages),
            "position": guide_position(rng),
            "kind": rng.pick(&["horizontal", "vertical"]).copied(),
        }),
        "page.changeGuide" => serde_json::json!({
            "page": any_index(rng, pages),
            "current": guide_position(rng),
            "new": guide_position(rng),
            "kind": rng.pick(&["horizontal", "vertical"]).copied(),
        }),
        "page.removeGuide" => serde_json::json!({
            "page": any_index(rng, pages),
            "position": guide_position(rng),
            "kind": rng.pick(&["horizontal", "vertical"]).copied(),
        }),
        "page.removeAllGuides" => serde_json::json!({
            "page": any_index(rng, pages),
            "kind": if rng.chance(60) {
                serde_json::json!(rng.pick(&["horizontal", "vertical"]).copied())
            } else {
                serde_json::Value::Null
            },
        }),
        "page.setGridOrigin" => serde_json::json!({
            "page": any_index(rng, pages),
            "x": guide_position(rng),
            "y": guide_position(rng),
        }),
        "page.setGridSize" => {
            let size = |rng: &mut FuzzRng| -> f64 {
                if rng.chance(85) {
                    1.0 + rng.below(200) as f64
                } else {
                    // Degenerate on purpose (zero, negative, non-finite
                    // via a huge value that stays finite in JSON).
                    -(rng.below(10) as f64)
                }
            };
            serde_json::json!({
                "page": any_index(rng, pages),
                "x": size(rng),
                "y": size(rng),
            })
        }
        "page.setOnionSkinning" => {
            let count = |rng: &mut FuzzRng| -> i64 {
                if rng.chance(85) {
                    rng.below(5) as i64
                } else {
                    // Negative on purpose.
                    -(rng.below(3) as i64) - 1
                }
            };
            serde_json::json!({
                "page": any_index(rng, pages),
                "before": count(rng),
                "after": count(rng),
            })
        }
        "frame.add" => {
            let location = if rng.chance(70) {
                serde_json::json!(
                    rng.pick(&["beginning", "before current", "after current", "end"])
                        .copied()
                )
            } else {
                // Missing `current` for a current-relative location is a
                // refusal the grammar should reach.
                serde_json::Value::Null
            };
            serde_json::json!({
                "page": any_index(rng, pages),
                "how_many": 1 + rng.below(3),
                "location": location,
                "current": any_opt_index(rng, 3),
            })
        }
        "frame.delete" => serde_json::json!({
            "page": any_index(rng, pages),
            "index": any_index(rng, frames + 1),
            "how_many": 1 + rng.below(2),
        }),
        "frame.duplicate" => serde_json::json!({
            "page": any_index(rng, pages),
            "index": any_index(rng, frames + 1),
            "how_many": 1 + rng.below(2),
            "location": rng.pick(&["beginning", "before current", "after current", "end"]).copied(),
            "current": any_opt_index(rng, 3),
            "selection_only": rng.chance(90),
        }),
        "frame.reorder" => serde_json::json!({
            "page": any_index(rng, pages),
            "from": any_index(rng, frames + 1),
            "before": any_index(rng, frames + 2),
            "make_copy": rng.chance(50),
        }),
        "frame.distributeLayer" => serde_json::json!({
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
        }),
        "page.setCanvasSize" => {
            let size = |rng: &mut FuzzRng| -> f64 {
                if rng.chance(90) {
                    1.0 + rng.below(4096) as f64
                } else {
                    // Out of range on purpose (0, negative, huge).
                    -(rng.below(10) as f64)
                }
            };
            serde_json::json!({
                "page": any_index(rng, pages),
                "width": size(rng),
                "height": size(rng),
            })
        }
        "bitmap.selectRect" | "bitmap.selectEllipse" => serde_json::json!({
            "id": any_id(rng),
            "x": -(rng.below(300) as f64),
            "y": -(rng.below(300) as f64),
            "width": rng.below(600) as f64,
            "height": rng.below(600) as f64,
            "mode": rng.pick(&["replace", "add", "subtract", "intersect"]).copied(),
        }),
        "bitmap.selectLasso" => serde_json::json!({
            "id": any_id(rng),
            "points": (0..2 + rng.below(5))
                .map(|_| [-(rng.below(300) as f64), -(rng.below(300) as f64)])
                .collect::<Vec<_>>(),
            "mode": rng.pick(&["replace", "add", "subtract", "intersect"]).copied(),
        }),
        "bitmap.selectPolygon" => serde_json::json!({
            "id": any_id(rng),
            "points": (0..3 + rng.below(5))
                .map(|_| [-(rng.below(300) as f64), -(rng.below(300) as f64)])
                .collect::<Vec<_>>(),
            "mode": rng.pick(&["replace", "add", "subtract", "intersect"]).copied(),
        }),
        "bitmap.selectWand" => serde_json::json!({
            "id": any_id(rng),
            "x": rng.below(300),
            "y": rng.below(300),
            "tolerance": rng.below(256),
            "contiguous": rng.chance(2),
            "mode": rng.pick(&["replace", "add", "subtract", "intersect"]).copied(),
        }),
        "bitmap.feather" => serde_json::json!({
            "radius": rng.below(70),
        }),
        "bitmap.selectNone" => serde_json::json!({}),
        "bitmap.fill" => serde_json::json!({
            "id": any_id(rng),
            "color_raw": rng.next_u64() as i64,
        }),
        "bitmap.erase" => serde_json::json!({
            "id": any_id(rng),
        }),
        "bitmap.brush" => serde_json::json!({
            "id": any_id(rng),
            "x": -(rng.below(300) as f64),
            "y": -(rng.below(300) as f64),
            "size": rng.below(1030),
            "color_raw": rng.next_u64() as i64,
            "strength": rng.below(105),
            "brush": {
                "edge": rng.below(110),
                "shape": if rng.below(2) == 0 { "round" } else { "square" },
                "aspect": rng.below(110),
                "angle": rng.below(360) as i16 - 180,
                "pressure": rng.below(110),
            },
        }),
        "bitmap.pencil" => serde_json::json!({
            "id": any_id(rng),
            "x": -(rng.below(300) as f64),
            "y": -(rng.below(300) as f64),
            "color_raw": rng.next_u64() as i64,
            "strength": rng.below(105),
            "pressure": rng.below(110),
        }),
        "bitmap.movePixels" => serde_json::json!({
            "id": any_id(rng),
            "dx": rng.below(600) as i64 - 300,
            "dy": rng.below(600) as i64 - 300,
        }),
        "retouch.blur" | "retouch.eraser" => serde_json::json!({
            "id": any_id(rng),
            "x": -(rng.below(300) as f64),
            "y": -(rng.below(300) as f64),
            "radius": rng.below(520),
            "strength": rng.below(105),
        }),
        "retouch.smudge" => serde_json::json!({
            "id": any_id(rng),
            "x": -(rng.below(300) as f64),
            "y": -(rng.below(300) as f64),
            "radius": rng.below(520),
            "strength": rng.below(105),
            "dx": rng.below(20) as i32 - 10,
            "dy": rng.below(20) as i32 - 10,
        }),
        "retouch.clone" => serde_json::json!({
            "id": any_id(rng),
            "source_x": -(rng.below(300) as f64),
            "source_y": -(rng.below(300) as f64),
            "target_x": -(rng.below(300) as f64),
            "target_y": -(rng.below(300) as f64),
            "radius": rng.below(520),
        }),
        "retouch.redEye" => serde_json::json!({
            "id": any_id(rng),
            "x": -(rng.below(300) as f64),
            "y": -(rng.below(300) as f64),
            "radius": rng.below(520),
        }),
        "text.createText" => serde_json::json!({
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "x": -(rng.below(400) as f64),
            "y": -(rng.below(400) as f64),
            "width": rng.below(400) as f64,
            "height": rng.below(400) as f64,
            "text": if rng.chance(4) {
                String::new()
            } else {
                "FireCraft".to_owned()
            },
            "name": if rng.chance(50) {
                serde_json::Value::Null
            } else {
                serde_json::json!("fuzz text")
            },
        }),
        "text.setText" => serde_json::json!({
            "id": any_id(rng),
            "text": if rng.chance(4) {
                String::new()
            } else {
                format!("text {}", rng.below(100))
            },
        }),
        "text.setAttrs" => {
            let mut payload = serde_json::json!({ "id": any_id(rng) });
            if rng.chance(60) {
                payload["size"] = serde_json::json!(rng.below(200) as f64 + 0.5);
            }
            if rng.chance(50) {
                payload["bold"] = serde_json::json!(rng.chance(50));
            }
            if rng.chance(50) {
                payload["italic"] = serde_json::json!(rng.chance(50));
            }
            if rng.chance(50) {
                payload["underline"] = serde_json::json!(rng.chance(50));
            }
            if rng.chance(40) {
                payload["horizontal_scale"] = serde_json::json!(rng.below(200) as f64 + 0.5);
            }
            if rng.chance(40) {
                payload["kerning"] = serde_json::json!((rng.below(200) as f64) - 100.0);
            }
            if rng.chance(40) {
                payload["baseline_shift"] = serde_json::json!((rng.below(100) as f64) - 50.0);
            }
            if rng.chance(40) {
                payload["leading"] = serde_json::json!(rng.below(100) as f64 + 0.5);
            }
            if rng.chance(30) {
                payload["leading_unit"] = serde_json::json!(rng.below(3));
            }
            if rng.chance(30) {
                payload["justification"] = serde_json::json!(rng.below(5));
            }
            if rng.chance(30) {
                payload["fill_raw"] = serde_json::json!(rng.next_u64() as i64);
            }
            if rng.chance(20) {
                payload["anti_alias"] = serde_json::json!(rng.below(10));
            }
            if rng.chance(20) {
                payload["auto_enlarge"] = serde_json::json!(rng.chance(50));
            }
            if rng.chance(20) {
                payload["auto_kern"] = serde_json::json!(rng.chance(50));
            }
            payload
        }
        "text.setFont" => serde_json::json!({
            "id": any_id(rng),
            "font": rng
                .pick(&["Liberation Sans", "Arial", "Unknown Face"])
                .copied(),
        }),
        "text.attachPath" => {
            let point = |rng: &mut FuzzRng| -> serde_json::Value {
                let mut point = serde_json::json!({
                    "x": (rng.below(200) as f64) - 20.0,
                    "y": (rng.below(200) as f64) - 20.0,
                });
                if rng.chance(40) {
                    point["secondary"] = serde_json::json!({
                        "x": (rng.below(200) as f64) - 20.0,
                        "y": (rng.below(200) as f64) - 20.0,
                    });
                }
                if rng.chance(40) {
                    point["primary"] = serde_json::json!({
                        "x": (rng.below(200) as f64) - 20.0,
                        "y": (rng.below(200) as f64) - 20.0,
                    });
                }
                point
            };
            let contour = |rng: &mut FuzzRng| -> serde_json::Value {
                let points: Vec<serde_json::Value> =
                    (0..2 + rng.below(3)).map(|_| point(rng)).collect();
                serde_json::json!({ "points": points })
            };
            let contours: Vec<serde_json::Value> = if rng.chance(15) {
                // A degenerate empty path on purpose — must refuse.
                Vec::new()
            } else {
                (0..1 + rng.below(2)).map(|_| contour(rng)).collect()
            };
            let mut payload = serde_json::json!({ "id": any_id(rng), "path": contours });
            if rng.chance(70) {
                payload["alignment"] = serde_json::json!(rng.below(5));
            }
            if rng.chance(70) {
                payload["offset"] = serde_json::json!((rng.below(200) as f64) - 100.0);
            }
            payload
        }
        "text.detachPath" => serde_json::json!({ "id": any_id(rng) }),
        "symbol.create" => {
            let count = 1 + rng.below(3);
            let mut ids = Vec::with_capacity(count);
            for _ in 0..count {
                ids.push(any_id(rng));
            }
            serde_json::json!({
                "ids": ids,
                "name": "fuzz lib",
                "symbol_type": rng
                    .pick(&["graphic", "button", "animation"])
                    .copied()
                    .unwrap_or("graphic"),
            })
        }
        "symbol.createInstance" => serde_json::json!({
            "symbol": rng.below(3),
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "cell": any_index(rng, frames),
            "index": any_opt_index(rng, 6),
            "x": (rng.below(400) as f64) - 200.0,
            "y": (rng.below(400) as f64) - 200.0,
            "name": if rng.chance(50) {
                serde_json::Value::Null
            } else {
                serde_json::json!("fuzz instance")
            },
        }),
        "symbol.breakApart" => serde_json::json!({ "id": any_id(rng) }),
        "symbol.setType" => serde_json::json!({
            "symbol": rng.below(3),
            "symbol_type": rng
                .pick(&["graphic", "button", "animation"])
                .copied()
                .unwrap_or("graphic"),
        }),
        "symbol.set9Slice" => {
            let symbol = rng.below(3);
            if rng.chance(50) {
                serde_json::json!({ "symbol": symbol, "guides": serde_json::Value::Null })
            } else {
                serde_json::json!({
                    "symbol": symbol,
                    "guides": {
                        "locked": rng.chance(50),
                        "color_raw": rng.below(0x1_000_000),
                        "controls": [
                            rng.below(40) as f64,
                            rng.below(40) as f64,
                            rng.below(40) as f64,
                            rng.below(40) as f64,
                        ],
                        "shape_bounds": {
                            "x0": 0.0,
                            "y0": 0.0,
                            "x1": rng.below(200) as f64,
                            "y1": rng.below(200) as f64,
                        },
                        "limits": {
                            "x0": 0.0,
                            "y0": 0.0,
                            "x1": rng.below(200) as f64,
                            "y1": rng.below(200) as f64,
                        },
                    },
                })
            }
        }
        "symbol.setScript" => {
            let symbol = rng.below(3);
            if rng.chance(25) {
                serde_json::json!({ "symbol": symbol, "source": serde_json::Value::Null })
            } else {
                serde_json::json!({
                    "symbol": symbol,
                    "source": format!(
                        "Widget.elem.customData['fuzz{}'] = 'x';",
                        rng.below(10)
                    ),
                })
            }
        }
        "symbol.setParam" => serde_json::json!({
            "id": any_id(rng),
            "name": format!("p-{}", rng.below(6)),
            "value": format!("v-{}", rng.below(20)),
            "kind": rng.pick(&["color", "font", "text"]).copied().unwrap_or("text"),
        }),
        "symbol.setCustomData" => {
            if rng.chance(15) {
                serde_json::json!({
                    "id": any_id(rng),
                    "key": format!("k-{}", rng.below(6)),
                    "value": serde_json::Value::Null,
                })
            } else {
                serde_json::json!({
                    "id": any_id(rng),
                    "key": format!("k-{}", rng.below(6)),
                    "value": format!("v-{}", rng.below(20)),
                })
            }
        }
        "symbol.editAttach" => serde_json::json!({
            "symbol": rng.below(3),
            "id": any_id(rng),
            "layer": any_index(rng, 2),
            "cell": any_index(rng, frames),
            "index": any_opt_index(rng, 6),
        }),
        "symbol.editDetach" => serde_json::json!({
            "symbol": rng.below(3),
            "id": any_id(rng),
        }),
        "symbol.editReorder" => serde_json::json!({
            "symbol": rng.below(3),
            "id": any_id(rng),
            "to": rng.below(6),
        }),
        "library.add" => serde_json::json!({
            "name": format!("L-{}", rng.below(100)),
            "symbol_type": rng
                .pick(&["graphic", "button", "animation"])
                .copied()
                .unwrap_or("graphic"),
        }),
        "library.remove" => serde_json::json!({ "symbol": rng.below(3) }),
        "library.rename" => serde_json::json!({
            "symbol": rng.below(3),
            "name": format!("L-{}", rng.below(100)),
        }),
        "slice.create" | "hotspot.create" => {
            let shape = rng.below(3);
            let shape_name = match shape {
                0 => "rect",
                1 => "ellipse",
                _ => "polygon",
            };
            let points = (shape == 2).then(|| {
                (0..3 + rng.below(3))
                    .map(|_| {
                        serde_json::json!({
                            "x": rng.below(64) as f64,
                            "y": rng.below(64) as f64,
                        })
                    })
                    .collect::<Vec<_>>()
            });
            serde_json::json!({
                "shape": shape_name,
                "x": rng.below(40) as f64,
                "y": rng.below(40) as f64,
                "width": 1.0 + rng.below(24) as f64,
                "height": 1.0 + rng.below(24) as f64,
                "points": points,
                "page": any_index(rng, 2),
                "name": rng.chance(50).then(|| format!("S-{}", rng.below(100))),
                "link": rng.chance(30).then(|| format!("https://example.invalid/{}", rng.below(9))),
                "alt": rng.chance(30).then(|| "alt".to_owned()),
            })
        }
        "slice.setLink" => serde_json::json!({
            "id": any_id(rng),
            "link": rng.chance(70).then(|| format!("https://example.invalid/{}", rng.below(9))),
            "alt": rng.chance(50).then(|| "alt".to_owned()),
        }),
        "slice.setType" => serde_json::json!({
            "id": any_id(rng),
            "slice_type": match rng.below(3) {
                0 => serde_json::json!(0),
                1 => serde_json::json!(2),
                _ => serde_json::Value::Null,
            },
        }),
        "slice.setBackground" => serde_json::json!({
            "id": any_id(rng),
            "is_background": rng.chance(70).then(|| rng.chance(50)),
            "repeat": rng.chance(50).then(|| rng.below(5) as i64 - 1),
            "attachment": rng.chance(50).then(|| rng.below(3) as i64 - 1),
            "h_position": rng.chance(50).then(|| rng.below(5) as i64 - 1),
            "v_position": rng.chance(50).then(|| rng.below(5) as i64 - 1),
        }),
        "slice.setExport" => serde_json::json!({
            "id": any_id(rng),
            "format": rng.chance(60).then(|| rng.below(5) as i64),
            "jpeg_quality": rng.chance(40).then(|| rng.below(110) as i64),
            "clear": rng.chance(15),
        }),
        "slice.addBehavior" => serde_json::json!({
            "id": any_id(rng),
            "kind": rng.below(12) as i64,
        }),
        "slice.removeBehavior" => serde_json::json!({
            "id": any_id(rng),
            "index": any_index(rng, 3),
        }),
        "slice.setData" => serde_json::json!({
            "id": any_id(rng),
            "key": format!("k-{}", rng.below(4)),
            "value": rng.chance(75).then(|| format!("v-{}", rng.below(9))),
        }),
        "library.importSymbol" => {
            // A tiny synthetic closure with fabricated source ids:
            // closure completeness is checked inside the payload only,
            // so this imports cleanly whatever the target holds.
            let mut library = Document::default();
            let leaf = library.registry.insert(random_path(rng, 0));
            let group = library.registry.insert(ObjectData::Group(GroupObject {
                group_type: 0,
                name: None,
                children: vec![leaf],
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                mask_hidden: false,
                mask_linked: None,
                effects: Vec::new(),
            }));
            library.symbols.push(SymbolDefinition {
                name: format!("L-{}", rng.below(100)),
                symbol_number: 0,
                origin: geom::Point::ZERO,
                frame_count: 1,
                layers: LayerList {
                    layers: vec![Layer {
                        name: String::new(),
                        display: true,
                        cells: vec![Cell {
                            locked: false,
                            visible: true,
                            elements: vec![group],
                        }],
                        ..Layer::default()
                    }],
                },
                background_raw: 0,
                symbol_type: SymbolType::Graphic,
                nine_slice: None,
                op_code: None,
            });
            let payload = crate::catalog::symbol::LibraryImportPayload {
                definition: library.symbols.remove(0),
                members: library
                    .registry
                    .iter()
                    .map(|(id, object)| crate::catalog::symbol::ImportedMember {
                        id,
                        object: object.clone(),
                    })
                    .collect(),
            };
            serde_json::to_value(&payload).unwrap_or_default()
        }
        "style.apply" => {
            let ids: Vec<serde_json::Value> = if live_ids.is_empty() || rng.chance(20) {
                vec![any_id(rng)]
            } else {
                let count = 1 + rng.below(live_ids.len().min(3));
                (0..count)
                    .map(|_| serde_json::json!(rng.pick(&live_ids).copied()))
                    .collect()
            };
            serde_json::json!({
                "index": any_index(rng, styles),
                "ids": ids,
            })
        }
        "style.redefineByIndex" => serde_json::json!({
            "index": any_index(rng, styles),
            "source": any_id(rng),
        }),
        "style.redefineByName" => serde_json::json!({
            "name": if styles > 0 && rng.chance(70) {
                format!("fuzz style {}", rng.below(styles))
            } else {
                format!("s-{}", rng.below(30))
            },
            "source": any_id(rng),
        }),
        "style.redefineFromSelection" => serde_json::json!({
            "name": format!("s-{}", rng.below(30)),
            "source": any_id(rng),
        }),
        "style.importLibrary" => {
            // A small library of fresh entries: names collide with the
            // seed library sometimes — redefinition must not, since
            // import appends unconditionally.
            let entries: Vec<doc::StyleEntry> = (0..1 + rng.below(3))
                .map(|index| {
                    let mut entry = random_style(rng, 100 + index);
                    entry.name = format!("imported {index}");
                    entry
                })
                .collect();
            serde_json::json!({ "entries": entries })
        }
        "style.importLibraryTiles" => {
            // One fresh bank slot past the live bank: valid payloads
            // join the bank, colliding ones (deliberately, via
            // `document.tiles.len()` twice) exercise the
            // equal-content-replay / different-content-refuse guard.
            let base = document.tiles.len();
            let id = if rng.chance(85) {
                base
            } else {
                base + 1 + rng.below(3)
            };
            let rgba: Vec<u8> = (0..raster::TILE_BYTES)
                .map(|_| rng.next_u64() as u8)
                .collect();
            serde_json::json!({ "tiles": [{ "id": id, "rgba": b64(&rgba) }] })
        }
        "style.scaleStyles" => serde_json::json!({
            "scale": (1 + rng.below(300)) as f64 / 100.0,
        }),
        // The element-mask family (decision 0020): structure minting
        // (add/remove/setMode/converters) rides the same generic loop
        // as every other minting command — refusals must not mutate,
        // and every delta must undo byte-exactly, mint order included.
        "mask.add" => serde_json::json!({
            "id": any_id(rng),
            "mode": rng.pick(&["reveal all", "hide all", "reveal selection", "hide selection"]).copied(),
            "enter_edit_mode": if rng.chance(50) { serde_json::json!(rng.chance(2)) } else { serde_json::Value::Null },
        }),
        "mask.enable" => serde_json::json!({
            "id": any_id(rng),
            "enable": rng.chance(2),
        }),
        "mask.link" => serde_json::json!({
            "id": any_id(rng),
            "linked": rng.chance(2),
        }),
        "mask.moveBy" => {
            let ids: Vec<serde_json::Value> = (0..1 + rng.below(3)).map(|_| any_id(rng)).collect();
            serde_json::json!({
                "ids": ids,
                "dx": rng.below(64) as f64 - 32.0,
                "dy": rng.below(64) as f64 - 32.0,
            })
        }
        "mask.moveContentsBy" => {
            let ids: Vec<serde_json::Value> = (0..1 + rng.below(3)).map(|_| any_id(rng)).collect();
            serde_json::json!({
                "ids": ids,
                "dx": rng.below(64) as f64 - 32.0,
                "dy": rng.below(64) as f64 - 32.0,
            })
        }
        "mask.remove" => serde_json::json!({
            "id": any_id(rng),
            "what_if_image": if rng.chance(60) {
                serde_json::json!(rng.pick(&["apply", "discard"]).copied())
            } else {
                serde_json::Value::Null
            },
        }),
        "mask.setMode" => serde_json::json!({
            "id": any_id(rng),
            "mode": rng.pick(&["mask to image", "mask to path"]).copied(),
        }),
        "mask.setShowAttrs" => serde_json::json!({
            "id": any_id(rng),
            "show": rng.chance(2),
        }),
        "mask.pathToMarquee" => serde_json::json!({
            "id": any_id(rng),
            "image": any_id(rng),
            "mode": rng.pick(&["hard edge", "antialias", "feather"]).copied(),
            "feather_amount": if rng.chance(50) { serde_json::json!(1.0 + rng.below(8) as f64) } else { serde_json::Value::Null },
        }),
        "mask.marqueeToPath" => serde_json::json!({
            "image": any_id(rng),
        }),
        "mask.saveJPEG" | "mask.restoreJPEG" | "mask.clearJPEG" => serde_json::json!({}),
        "bitmap.movePixelMaskBy" => serde_json::json!({
            "id": any_id(rng),
            "dx": rng.below(64) as i64 - 32,
            "dy": rng.below(64) as i64 - 32,
        }),
        "bitmap.setPixelMask" => {
            // Mostly a null mask (clear); otherwise a small plane the
            // dims gate will usually refuse — refusals are grammar.
            let mask = if rng.chance(60) {
                serde_json::Value::Null
            } else {
                let width = 1 + rng.below(8);
                let height = 1 + rng.below(8);
                let alpha: Vec<serde_json::Value> = (0..width * height)
                    .map(|_| serde_json::json!(rng.below(256) as u8))
                    .collect();
                serde_json::json!({ "width": width, "height": height, "alpha": alpha })
            };
            serde_json::json!({
                "id": any_id(rng),
                "mask": mask,
                "mode": if rng.chance(70) {
                    serde_json::json!(rng.pick(&["replace", "add", "subtract", "intersect"]).copied())
                } else {
                    serde_json::Value::Null
                },
            })
        }
        // The layer/page structure family (decision 0021): order and
        // flag flips over existing model bits, list-capturing undo.
        "page.setName" => serde_json::json!({
            "page": any_index(rng, pages),
            "name": rng.pick(&["Cover", "Page A", ""]).copied(),
        }),
        "page.reorder" => serde_json::json!({
            "from": any_index(rng, pages),
            "before": any_index(rng, pages + 1),
        }),
        "page.setMaster" => serde_json::json!({
            "page": any_index(rng, pages),
        }),
        "page.unsetMaster" => serde_json::json!({}),
        "page.addMasterLayer" => serde_json::json!({
            "index": any_opt_index(rng, 3),
        }),
        "layer.reorder" => serde_json::json!({
            "page": any_index(rng, pages),
            "from": any_index(rng, 3),
            "before": any_index(rng, 4),
            "make_copy": rng.chance(50),
        }),
        "layer.setLocked" => serde_json::json!({
            "page": any_index(rng, pages),
            "layer": any_index(rng, 3),
            "frame": any_index(rng, frames),
            "locked": rng.chance(2),
            "all_layers": rng.chance(30),
        }),
        other => {
            let _ = other;
            // A catalog command without a builder: an empty payload is
            // still a refused call — and the coverage unit test keeps
            // this arm unreachable.
            serde_json::json!({})
        }
    };

    (name.to_owned(), corrupt(rng, payload))
}

/// With ~10% probability damages a payload: a key goes missing, turns
/// null, changes type, or an unknown key appears. Every variant must
/// come back as [`crate::EngineError::BadPayload`] or a target error —
/// never a panic.
fn corrupt(rng: &mut FuzzRng, mut payload: serde_json::Value) -> serde_json::Value {
    if !rng.chance(10) {
        return payload;
    }
    let Some(map) = payload.as_object_mut() else {
        return payload;
    };
    let Some(key) = rng
        .pick(&[
            "id",
            "page",
            "layer",
            "cell",
            "visible",
            "locked",
            "opacity",
            "blend",
            "color_raw",
            "radius",
            "strength",
            "name",
            "to",
            "index",
            "ids",
            "source",
            "scale",
            "width",
            "height",
        ])
        .copied()
    else {
        return payload;
    };
    match rng.below(4) {
        0 => {
            map.insert("bogus".to_owned(), serde_json::Value::Bool(true));
        }
        1 => {
            map.insert(key.to_owned(), serde_json::Value::Null);
        }
        2 => {
            map.insert(
                key.to_owned(),
                serde_json::Value::String("corrupt".to_owned()),
            );
        }
        _ => {
            map.remove(key);
        }
    }
    payload
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::catalog;

    #[test]
    fn the_call_table_covers_the_catalog() {
        let mut catalog_ids = catalog::ids();
        catalog_ids.sort_unstable();
        let mut table: Vec<&str> = CALLS.to_vec();
        table.sort_unstable();
        assert_eq!(table, catalog_ids, "fuzzdrive must cover every command");
    }

    #[test]
    fn documents_are_deterministic_per_seed() {
        let a = {
            let mut rng = FuzzRng::new(7);
            random_document(&mut rng)
        };
        let b = {
            let mut rng = FuzzRng::new(7);
            random_document(&mut rng)
        };
        assert_eq!(a, b);
    }

    #[test]
    fn rng_is_bounded_and_total() {
        let mut rng = FuzzRng::new(99);
        for _ in 0..100 {
            assert!(rng.below(5) < 5);
            assert_eq!(rng.below(0), 0);
        }
    }
}

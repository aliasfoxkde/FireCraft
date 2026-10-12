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
    SymbolInstanceObject, TextObject, TileRef,
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

    // Objects: a mix of kinds; most are paths.
    let object_count = 1 + rng.below(6);
    let mut leaves: Vec<doc::ObjectId> = Vec::new();
    for index in 0..object_count {
        let object = match rng.below(10) {
            0..=4 => random_path(rng, index),
            5 => random_image(rng),
            6 => random_text(rng),
            7 => random_slice(rng),
            _ => random_symbol_instance(rng),
        };
        leaves.push(document.registry.insert(object));
    }

    // Maybe one group claiming a random subset of the leaves; the
    // group becomes a top-level object, its claimed leaves do not.
    let mut tops = leaves.clone();
    if leaves.len() >= 2 && rng.chance(50) {
        let mut children = Vec::new();
        for leaf in &leaves {
            if rng.chance(50) {
                children.push(*leaf);
            }
        }
        if children.len() >= 2 {
            let group_id = document.registry.insert(ObjectData::Group(GroupObject {
                group_type: 0,
                name: Some("group".to_owned()),
                children: children.clone(),
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                mask_hidden: false,
                effects: random_effects(rng),
            }));
            tops.retain(|top| !children.contains(top));
            tops.push(group_id);
        }
    }

    // Distribute the top-level objects over the cells, each exactly
    // once, bottom-first round-robin.
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
    for (slot, id) in tops.into_iter().enumerate() {
        if total_cells == 0 {
            break;
        }
        let mut remaining = slot % total_cells;
        'place: for page in &mut document.pages {
            for layer in &mut page.layers.layers {
                for cell in &mut layer.cells {
                    if remaining == 0 {
                        cell.elements.push(id);
                        break 'place;
                    }
                    remaining -= 1;
                }
            }
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
        export: None,
    })
}

fn random_symbol_instance(rng: &mut FuzzRng) -> ObjectData {
    ObjectData::SymbolInstance(SymbolInstanceObject {
        symbol_number: 0,
        transform: geom::Affine::IDENTITY,
        locked: rng.chance(10),
        name: None,
    })
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
    "object.reorder",
    "object.moveToCell",
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
    "filter.add",
    "filter.remove",
    "filter.setParam",
    "cell.setVisible",
    "cell.setLocked",
    "layer.rename",
    "layer.setDisplay",
    "page.add",
    "page.delete",
    "page.setCanvasSize",
    "bitmap.selectRect",
    "bitmap.selectEllipse",
    "bitmap.selectLasso",
    "bitmap.selectWand",
    "bitmap.feather",
    "bitmap.selectNone",
    "bitmap.fill",
    "bitmap.erase",
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

    let pages = document.pages.len();
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
            "mode": rng.pick(&["replace", "add", "subtract"]).copied(),
        }),
        "bitmap.selectLasso" => serde_json::json!({
            "id": any_id(rng),
            "points": (0..2 + rng.below(5))
                .map(|_| [-(rng.below(300) as f64), -(rng.below(300) as f64)])
                .collect::<Vec<_>>(),
            "mode": rng.pick(&["replace", "add", "subtract"]).copied(),
        }),
        "bitmap.selectWand" => serde_json::json!({
            "id": any_id(rng),
            "x": rng.below(300),
            "y": rng.below(300),
            "tolerance": rng.below(256),
            "contiguous": rng.chance(2),
            "mode": rng.pick(&["replace", "add", "subtract"]).copied(),
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

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
    Cell, Contour, Document, DocumentInfo, GroupObject, ImageObject, Layer, LayerList, ObjectData,
    Page, PageInfo, PathObject, PathPoint, SliceObject, SymbolInstanceObject, TextObject, TileRef,
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
    }
}

fn random_point(rng: &mut FuzzRng) -> geom::Point {
    geom::Point::new(rng.below(64) as f64, rng.below(64) as f64)
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
    })
}

fn random_text(rng: &mut FuzzRng) -> ObjectData {
    let a = random_point(rng);
    ObjectData::Text(TextObject {
        frame: geom::Rect::new(a.x, a.y, a.x + 10.0, a.y + 4.0),
        transform: None,
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
    "path.boolean",
    "path.simplify",
    "path.expandStroke",
    "path.inset",
    "cell.setVisible",
    "cell.setLocked",
    "layer.rename",
    "layer.setDisplay",
    "page.add",
    "page.delete",
    "page.setCanvasSize",
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

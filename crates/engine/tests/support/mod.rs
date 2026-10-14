//! Shared test support: a structural sample document and byte-level
//! document equality.

// Each integration-test binary compiles its own copy of this module;
// not every binary uses every helper.
#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]

use doc::{
    Cell, Contour, Document, Layer, LayerList, ObjectData, Page, PageInfo, PathObject, PathPoint,
};
use engine::{Call, CommandRegistry, History, Journal};

pub fn point(x: f64, y: f64) -> PathPoint {
    PathPoint {
        anchor: geom::Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    }
}

pub fn path_object(seed: u64) -> ObjectData {
    ObjectData::Path(PathObject {
        contours: vec![Contour {
            closed: true,
            point_count: 4,
            points: vec![
                point(0.0, 0.0),
                point(seed as f64 + 8.0, 0.0),
                point(seed as f64 + 8.0, 8.0),
                point(0.0, 8.0),
            ],
        }],
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
    })
}

/// A graphic definition numbered `number`, carrying the CS6
/// button-behavior defaults (decision 0036) and no content.
pub fn definition(number: i64) -> doc::SymbolDefinition {
    doc::SymbolDefinition {
        name: format!("definition {number}"),
        symbol_number: number,
        origin: geom::Point::ZERO,
        frame_count: 1,
        layers: doc::LayerList::default(),
        background_raw: 0,
        symbol_type: doc::SymbolType::Graphic,
        nine_slice: None,
        auto_slice: Some(true),
        include_down_state: Some(false),
        show_down_on_load: Some(false),
        include_over_while_down: Some(false),
        op_code: None,
    }
}

/// Two pages, two layers per page (two cells each — two states), two
/// path objects per cell: page/layer/cell indices are all live.
pub fn sample_document() -> (Document, Vec<doc::ObjectId>) {
    let mut document = Document::default();
    let mut ids = Vec::new();
    for page_index in 0..2 {
        let mut layers = LayerList::default();
        for layer_index in 0..2 {
            let mut layer = Layer {
                name: format!("Layer {layer_index}"),
                display: true,
                ..Layer::default()
            };
            for _ in 0..2 {
                let mut cell = Cell {
                    locked: false,
                    visible: true,
                    elements: Vec::new(),
                };
                for _ in 0..2 {
                    let id = document.registry.insert(path_object(ids.len() as u64 + 1));
                    cell.elements.push(id);
                    ids.push(id);
                }
                layer.cells.push(cell);
            }
            layers.layers.push(layer);
        }
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: 2,
                frame_count_duplicate: 2,
                request_id: 0,
                canvas: Default::default(),
                onion_before: 0,
                onion_after: 0,
                frame_durations: Vec::new(),
            },
            name: Some(format!("Page {page_index}")),
            is_master: false,
            is_linked: false,
            page_id: None,
            last_export_dir: None,
            last_export_file: None,
            layers,
            grid: None,
            guides: Default::default(),
            export: None,
        });
    }
    (document, ids)
}

/// One page/layer/cell holding a 128x128 bitmap whose single tile is an
/// opaque red/blue checkerboard: the base for bitmap pixel-edit tests.
/// Returns the document and the image object's id.
pub fn sample_bitmap_document() -> (Document, doc::ObjectId) {
    let mut document = Document::default();
    let mut tile = raster::Tile::new();
    for (index, pixel) in tile.pixels_mut().chunks_exact_mut(4).enumerate() {
        let column = (index % raster::TILE_SIDE) as u8;
        pixel.copy_from_slice(if column % 2 == 0 {
            &[255, 0, 0, 255]
        } else {
            &[0, 0, 255, 255]
        });
    }
    document.tiles.insert(0, tile);
    let image = document
        .registry
        .insert(ObjectData::Image(doc::ImageObject {
            location: geom::Point::ZERO,
            offset: geom::Point::ZERO,
            pixels: (128, 128),
            tile_size: raster::TILE_SIDE as i64,
            tiles: vec![doc::TileRef::Tile(0)],
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }));
    let mut layer = Layer {
        name: "Layer 0".to_owned(),
        display: true,
        ..Layer::default()
    };
    layer.cells.push(Cell {
        locked: false,
        visible: true,
        elements: vec![image],
    });
    let mut layers = LayerList::default();
    layers.layers.push(layer);
    document.pages.push(Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: 1,
            frame_count_duplicate: 1,
            request_id: 0,
            canvas: Default::default(),
            onion_before: 0,
            onion_after: 0,
            frame_durations: Vec::new(),
        },
        name: Some("Page 0".to_owned()),
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers,
        grid: None,
        guides: Default::default(),
        export: None,
    });
    (document, image)
}

/// A scripted "session": registry + document + history + journal, with
/// apply/undo/redo — the shape the platform Session wrapper takes,
/// without platform services. `restart` replays the journal over a
/// fresh clone of the base document, so bitmap sessions (whose base
/// carries tile payloads) replay from the same pixels.
pub struct Scratch {
    pub registry: CommandRegistry,
    pub document: Document,
    pub history: History,
    pub journal: Journal,
    pub open_txn: Option<String>,
    base: Document,
}

impl Scratch {
    pub fn open(base: Document) -> Self {
        let mut registry = CommandRegistry::new();
        engine::catalog::install(&mut registry).expect("builtins");
        Self {
            registry,
            base: base.clone(),
            document: base,
            history: History::new(),
            journal: Journal::new(),
            open_txn: None,
        }
    }

    pub fn apply(&mut self, name: &str, payload: serde_json::Value, txn: Option<&str>) {
        match txn {
            Some(label) => {
                if self.open_txn.as_deref() != Some(label) {
                    if let Some(open) = &self.open_txn {
                        self.history
                            .commit_transaction()
                            .unwrap_or_else(|error| panic!("commit {open:?}: {error}"));
                    }
                    self.history
                        .begin_transaction(label)
                        .expect("open transaction");
                    self.open_txn = Some(label.to_owned());
                }
            }
            None => self.commit_open_txn(),
        }
        let delta = self
            .registry
            .execute(&mut self.document, name, payload.clone())
            .unwrap_or_else(|error| panic!("apply {name}: {error}"));
        self.journal.record(name, &payload, txn);
        self.history.record(
            Call {
                name: name.to_owned(),
                payload,
            },
            delta,
        );
    }

    pub fn commit_open_txn(&mut self) {
        if self.open_txn.take().is_some() {
            self.history.commit_transaction().expect("commit");
        }
    }

    pub fn undo(&mut self) -> String {
        self.commit_open_txn();
        let step = self.history.pop_undo().expect("undo");
        step.delta
            .apply_inverse(&mut self.document)
            .expect("undo delta");
        self.history.file_redo(step.clone());
        step.label
    }

    pub fn redo(&mut self) -> String {
        self.commit_open_txn();
        let step = self.history.pop_redo().expect("redo");
        for call in &step.calls {
            self.registry
                .execute(&mut self.document, &call.name, call.payload.clone())
                .unwrap_or_else(|error| panic!("redo {}: {error}", call.name));
        }
        self.history.file_undo(step.clone());
        step.label
    }

    pub fn bytes(&self) -> DocumentBytes {
        DocumentBytes::of(&self.document)
    }

    pub fn restart(&self) -> Scratch {
        let text = self.journal.to_lines();
        let journal = Journal::from_lines(&text).expect("journal parses");
        let mut document = self.base.clone();
        let mut registry = CommandRegistry::new();
        engine::catalog::install(&mut registry).expect("builtins");
        let history =
            engine::journal::replay(&mut document, &registry, journal.entries()).expect("replay");
        Scratch {
            registry,
            base: self.base.clone(),
            document,
            history,
            journal,
            open_txn: None,
        }
    }
}

/// Byte-level document equality (`serde_json`, field order is
/// declaration order — deterministic).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentBytes(Vec<u8>);

impl DocumentBytes {
    pub fn of(document: &Document) -> Self {
        Self(serde_json::to_vec(document).expect("document serializes"))
    }
}

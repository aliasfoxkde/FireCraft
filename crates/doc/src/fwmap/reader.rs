//! Node tree → [`Document`] + [`UnknownBag`].
//!
//! Policy (recorded in `docs/planning/firecraft/M3-doc-model.md`):
//! anything outside the mapped spine is bagged verbatim; a *known* node
//! with a payload the model cannot use is a [`MapError::BadShape`]; a
//! node whose required sub-structure cannot be modeled at all (e.g. a
//! primitive carrying a projective `MTXv`) is bagged whole — preservation
//! never waits on semantics.

use crate::fwmap::MapError;
use crate::fwmap::unknown::UnknownBag;
use crate::model::{
    Canvas, Cell, Contour, Dither, Document, DocumentInfo, Effect, EffectProperty, EffectValue,
    ExportOptions, Fill, FillShape, Gradient, GradientStop, Grid, GroupObject, Guides, ImageObject,
    Layer, LayerList, NineSliceGuides, Page, PageInfo, PaintStyle, PathObject, PathPoint,
    PatternTile, PrimitiveSource, SharedLayerDef, SharedLayerLink, SliceBackground, SliceBehavior,
    SliceObject, StyleEntry, StyleText, SymbolAnimation, SymbolDefinition, SymbolInstanceObject,
    SymbolType, TextObject, TextOnPath, TextRun, TextStyle, TileRef,
};
use crate::registry::{ObjectData, ObjectId, Registry};
use fwpng::nodetree::{Node, NodeValue};
use geom::{Affine, Point, Rect};

/// Reads a parsed node tree (the `mkTS`/`mkBS` payload) into a typed
/// document plus the bag of everything the model does not interpret.
pub fn read(nodes: &[Node]) -> Result<(Document, UnknownBag), MapError> {
    let mut reader = Reader {
        bag: UnknownBag::new(),
        registry: Registry::new(),
    };
    let mut document = Document::default();
    let mut mkbv_seen = false;
    let mut page_number = 0usize;
    for (i, node) in nodes.iter().enumerate() {
        match &node.fourcc {
            b"MKBv" => {
                if mkbv_seen {
                    reader.bag.insert("", i, node.clone());
                    continue;
                }
                mkbv_seen = true;
                let children = children_of(node, "")?;
                reader.read_document_body(children, "/MKBv", &mut document)?;
            }
            b"PDCv" => {
                let path = format!("/PDCv[{page_number}]");
                page_number += 1;
                let children = children_of(node, &path)?;
                let page = reader.read_page(children, &path)?;
                document.pages.push(page);
            }
            _ => reader.bag.insert("", i, node.clone()),
        }
    }
    document.registry = reader.registry;
    Ok((document, reader.bag))
}

/// Reads a style library: the `ZSLv` payload of a `.stl` container's
/// `mkTS` chunk into named [`StyleEntry`] records plus the bag of
/// everything the model does not interpret (the per-record `IMGv`
/// texture slots, mostly). The library list is the whole `mkTS`
/// payload in a style file — no `MKBv` wraps it — so this is the entry
/// the style-library loader calls; the document path reaches the same
/// records through [`read`]'s `ZSLv` branch. A payload with no style
/// content at all is a type error, not an empty library: a document's
/// `mkTS` fed here must refuse, not silently produce `[]`.
///
/// [`read_style_library`]: crate::fwmap::read_style_library
pub fn read_style_library(nodes: &[Node]) -> Result<(Vec<StyleEntry>, UnknownBag), MapError> {
    let mut styles = Vec::new();
    let mut reader = Reader {
        bag: UnknownBag::new(),
        registry: Registry::new(),
    };
    let mut saw_library = false;
    for (i, node) in nodes.iter().enumerate() {
        match &node.fourcc {
            b"ZSLv" => {
                saw_library = true;
                let children = children_of(node, "/ZSLv")?;
                reader.read_style_list(children, "", &mut styles)?;
            }
            // A bare `ZSTv` list is accepted too; the corpus has only
            // ever carried the `ZSLv` wrapper.
            b"ZSTv" => {
                saw_library = true;
                let path = format!("/ZSTv[{i}]");
                let children = children_of(node, &path)?;
                styles.push(reader.read_style_entry(children, &path)?);
            }
            // Anything else preserves in the root-level bag; refusal
            // is reserved for payloads with no style content at all.
            _ => reader.bag.insert("", i, node.clone()),
        }
    }
    if !saw_library {
        let first = nodes.first().ok_or_else(|| MapError::BadShape {
            path: "/ZSLv".to_owned(),
            node: *b"\0\0\0\0",
            expected: "ZSLv",
            found: "nothing",
        })?;
        return Err(bad_shape(first, "/ZSLv", "ZSLv"));
    }
    Ok((styles, reader.bag))
}

fn value_kind(node: &Node) -> &'static str {
    match node.value {
        NodeValue::Int(_) => "int",
        NodeValue::Float(_) => "float",
        NodeValue::Bool(_) => "bool",
        NodeValue::Str(_) => "string",
        NodeValue::Children(_) => "children",
    }
}

fn bad_shape(node: &Node, path: &str, expected: &'static str) -> MapError {
    MapError::BadShape {
        path: path.to_owned(),
        node: node.fourcc,
        expected,
        found: value_kind(node),
    }
}

fn children_of<'a>(node: &'a Node, path: &str) -> Result<&'a [Node], MapError> {
    node.as_children()
        .ok_or_else(|| bad_shape(node, path, "children"))
}

/// Maps the fill's shape slot through its enumeration: `FSXi` when
/// `extended`, else the narrower `FSHi` (same values minus contour;
/// its 3 means "radial or contour" and reads as [`FillShape::Radial`]
/// — recorded ambiguity, the extended file's `17` distinguishes).
/// `None` for an absent or unknown value: the caller bags the whole
/// `FPLv` so the subtree stays verbatim.
fn fill_shape(raw: Option<i64>, extended: bool) -> Option<FillShape> {
    let shape = match raw? {
        0 => FillShape::Solid,
        2 => FillShape::Linear,
        3 => FillShape::Radial,
        4 => FillShape::Conical,
        5 => FillShape::Satin,
        7 => FillShape::Pinch,
        8 => FillShape::Folds,
        9 => FillShape::Elliptical,
        10 => FillShape::Rectangular,
        12 => FillShape::Bars,
        13 => FillShape::Ripple,
        14 => FillShape::Waves,
        15 => FillShape::Pattern,
        16 => FillShape::WebDither,
        17 if extended => FillShape::Contour,
        _ => return None,
    };
    Some(shape)
}

struct Reader {
    bag: UnknownBag,
    registry: Registry,
}

impl Reader {
    fn bag_node(&mut self, path: &str, index: usize, node: &Node) {
        self.bag.insert(path, index, node.clone());
    }

    /// First occurrence wins; duplicates of an already-mapped node are
    /// bagged so writes still re-emit them.
    fn slot_i64(
        &mut self,
        node: &Node,
        path: &str,
        index: usize,
        slot: &mut Option<i64>,
    ) -> Result<(), MapError> {
        if slot.is_some() {
            self.bag_node(path, index, node);
            return Ok(());
        }
        match node.as_int() {
            Some(value) => {
                *slot = Some(value);
                Ok(())
            }
            None => Err(bad_shape(node, path, "int")),
        }
    }

    fn slot_f64(
        &mut self,
        node: &Node,
        path: &str,
        index: usize,
        slot: &mut Option<f64>,
    ) -> Result<(), MapError> {
        if slot.is_some() {
            self.bag_node(path, index, node);
            return Ok(());
        }
        match node.as_float() {
            Some(value) => {
                *slot = Some(value);
                Ok(())
            }
            None => Err(bad_shape(node, path, "float")),
        }
    }

    fn slot_bool(
        &mut self,
        node: &Node,
        path: &str,
        index: usize,
        slot: &mut Option<bool>,
    ) -> Result<(), MapError> {
        if slot.is_some() {
            self.bag_node(path, index, node);
            return Ok(());
        }
        match node.as_bool() {
            Some(value) => {
                *slot = Some(value);
                Ok(())
            }
            None => Err(bad_shape(node, path, "bool")),
        }
    }

    fn slot_str(
        &mut self,
        node: &Node,
        path: &str,
        index: usize,
        slot: &mut Option<String>,
    ) -> Result<(), MapError> {
        if slot.is_some() {
            self.bag_node(path, index, node);
            return Ok(());
        }
        match node.as_str() {
            Some(value) => {
                *slot = Some(value.to_owned());
                Ok(())
            }
            None => Err(bad_shape(node, path, "string")),
        }
    }

    /// Document body (`MKBv` children). `IMPb`/`PILb`/`PIDs`/`LXDs`/
    /// `LXFs`/`LSMv` and the definition lists have no document-level
    /// model slot and are bagged.
    fn read_document_body(
        &mut self,
        children: &[Node],
        path: &str,
        document: &mut Document,
    ) -> Result<(), MapError> {
        let (mut version, mut revision, mut frame_count, mut request_id) = (None, None, None, None);
        let (mut x, mut y, mut width, mut height, mut resolution) = (None, None, None, None, None);
        let (mut resolution_unit, mut background_raw) = (None, None);
        let (mut onion_before, mut onion_after) = (None, None);
        let (mut name, mut grid, mut guides) = (None, None, None);
        let mut export: Option<ExportOptions> = None;

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"VRSi" => self.slot_i64(child, path, i, &mut version)?,
                b"VRMi" => self.slot_i64(child, path, i, &mut revision)?,
                b"FRCi" => self.slot_i64(child, path, i, &mut frame_count)?,
                b"RQNi" => self.slot_i64(child, path, i, &mut request_id)?,
                b"XLCf" => self.slot_f64(child, path, i, &mut x)?,
                b"YLCf" => self.slot_f64(child, path, i, &mut y)?,
                b"WIDf" => self.slot_f64(child, path, i, &mut width)?,
                b"HITf" => self.slot_f64(child, path, i, &mut height)?,
                b"RESf" => self.slot_f64(child, path, i, &mut resolution)?,
                b"RSUi" => self.slot_i64(child, path, i, &mut resolution_unit)?,
                b"BGCi" => self.slot_i64(child, path, i, &mut background_raw)?,
                b"PGNs" => self.slot_str(child, path, i, &mut name)?,
                b"OSBi" => self.slot_i64(child, path, i, &mut onion_before)?,
                b"OSAi" => self.slot_i64(child, path, i, &mut onion_after)?,
                b"MSLv" => {
                    if document.symbols.is_empty() {
                        let kids = children_of(child, path)?;
                        let mslv_path = format!("{path}/MSLv");
                        self.read_symbol_list(kids, &mslv_path, &mut document.symbols)?;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"LYLv" => {
                    if document.shared_layers.layers.is_empty() {
                        let kids = children_of(child, path)?;
                        let lylv_path = format!("{path}/LYLv");
                        document.shared_layers = self.read_layer_list(kids, &lylv_path)?;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"LSMv" => {
                    // Shared-layer definitions (decision 0022). The
                    // node is also legal (and empty) under `PDCv`;
                    // only the document-level one is typed.
                    if document.shared_layer_defs.is_empty() {
                        let kids = children_of(child, path)?;
                        let lsmv_path = format!("{path}/LSMv");
                        document.shared_layer_defs = self.read_shared_defs(kids, &lsmv_path)?;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"GRDv" => {
                    if grid.is_some() {
                        self.bag_node(path, i, child);
                    } else {
                        let kids = children_of(child, path)?;
                        let grd_path = format!("{path}/GRDv");
                        grid = Some(self.read_grid(kids, &grd_path)?);
                    }
                }
                b"GDSv" => {
                    if guides.is_some() {
                        self.bag_node(path, i, child);
                    } else {
                        let kids = children_of(child, path)?;
                        let gds_path = format!("{path}/GDSv");
                        guides = Some(self.read_guides(kids, &gds_path)?);
                    }
                }
                b"EXPv" => {
                    // The document node carries the export *defaults*;
                    // its `FNLv` has no model home (frame timing is
                    // per-page) and stays preserved verbatim.
                    if export.is_some() {
                        self.bag_node(path, i, child);
                    } else {
                        let kids = children_of(child, path)?;
                        let expv_path = format!("{path}/EXPv");
                        let (options, _) = self.read_export(kids, &expv_path, false)?;
                        export = Some(options);
                    }
                }
                b"ZSLv" => {
                    if document.styles.is_empty() {
                        let kids = children_of(child, path)?;
                        let zslv_path = format!("{path}/ZSLv");
                        self.read_style_list(kids, &zslv_path, &mut document.styles)?;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }

        let canvas = Canvas {
            x: x.unwrap_or(0.0),
            y: y.unwrap_or(0.0),
            width: width.unwrap_or(0.0),
            height: height.unwrap_or(0.0),
            resolution: resolution.unwrap_or(72.0),
            resolution_unit: resolution_unit.unwrap_or(0),
            background_raw: background_raw.unwrap_or(0xFF_FF_FF),
        };
        document.info = DocumentInfo {
            name,
            version,
            revision,
            frame_count: frame_count.unwrap_or(1),
            request_id: request_id.unwrap_or(0),
            canvas,
            onion_before: onion_before.unwrap_or(0),
            onion_after: onion_after.unwrap_or(0),
        };
        if let Some(grid) = grid {
            document.grid = grid;
        }
        if let Some(guides) = guides {
            document.guides = guides;
        }
        document.export = export;
        Ok(())
    }

    /// Page body (`PDCv` children). Frame count appears twice; both are
    /// kept. `GRDv` is optional on pages and comes last in the
    /// dictionary order.
    fn read_page(&mut self, children: &[Node], path: &str) -> Result<Page, MapError> {
        let (mut version, mut revision, mut request_id) = (None, None, None);
        let mut frame_counts: Vec<i64> = Vec::new();
        let (mut x, mut y, mut width, mut height, mut resolution) = (None, None, None, None, None);
        let (mut resolution_unit, mut background_raw) = (None, None);
        let (mut onion_before, mut onion_after) = (None, None);
        let (mut name, mut guides, mut grid) = (None, None, None);
        let (mut is_master, mut is_linked) = (None, None);
        let (mut page_id, mut last_export_dir, mut last_export_file) = (None, None, None);
        let mut layers = LayerList::default();
        let (mut export, mut frame_durations) = (None, Vec::new());

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"VRSi" => self.slot_i64(child, path, i, &mut version)?,
                b"VRMi" => self.slot_i64(child, path, i, &mut revision)?,
                b"FRCi" => {
                    if frame_counts.len() < 2 {
                        if let Some(value) = child.as_int() {
                            frame_counts.push(value);
                            continue;
                        }
                        return Err(bad_shape(child, path, "int"));
                    }
                    self.bag_node(path, i, child);
                }
                b"RQNi" => self.slot_i64(child, path, i, &mut request_id)?,
                b"XLCf" => self.slot_f64(child, path, i, &mut x)?,
                b"YLCf" => self.slot_f64(child, path, i, &mut y)?,
                b"WIDf" => self.slot_f64(child, path, i, &mut width)?,
                b"HITf" => self.slot_f64(child, path, i, &mut height)?,
                b"RESf" => self.slot_f64(child, path, i, &mut resolution)?,
                b"RSUi" => self.slot_i64(child, path, i, &mut resolution_unit)?,
                b"BGCi" => self.slot_i64(child, path, i, &mut background_raw)?,
                b"PGNs" => self.slot_str(child, path, i, &mut name)?,
                b"OSBi" => self.slot_i64(child, path, i, &mut onion_before)?,
                b"OSAi" => self.slot_i64(child, path, i, &mut onion_after)?,
                b"IMPb" => self.slot_bool(child, path, i, &mut is_master)?,
                b"PILb" => self.slot_bool(child, path, i, &mut is_linked)?,
                b"PIDs" => self.slot_str(child, path, i, &mut page_id)?,
                b"LXDs" => self.slot_str(child, path, i, &mut last_export_dir)?,
                b"LXFs" => self.slot_str(child, path, i, &mut last_export_file)?,
                b"LYLv" => {
                    if layers.layers.is_empty() {
                        let kids = children_of(child, path)?;
                        let lylv_path = format!("{path}/LYLv");
                        layers = self.read_layer_list(kids, &lylv_path)?;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"GDSv" => {
                    if guides.is_some() {
                        self.bag_node(path, i, child);
                    } else {
                        let kids = children_of(child, path)?;
                        let gds_path = format!("{path}/GDSv");
                        guides = Some(self.read_guides(kids, &gds_path)?);
                    }
                }
                b"GRDv" => {
                    if grid.is_some() {
                        self.bag_node(path, i, child);
                    } else {
                        let kids = children_of(child, path)?;
                        let grd_path = format!("{path}/GRDv");
                        grid = Some(self.read_grid(kids, &grd_path)?);
                    }
                }
                b"EXPv" => {
                    // The page's `EXPv` is where `FNLv` frame
                    // configurations live — the on-disk home of
                    // `frame_durations`.
                    if export.is_some() {
                        self.bag_node(path, i, child);
                    } else {
                        let kids = children_of(child, path)?;
                        let expv_path = format!("{path}/EXPv");
                        let (options, frames) = self.read_export(kids, &expv_path, true)?;
                        frame_durations = frames.unwrap_or_default();
                        export = Some(options);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }

        let frame_count = frame_counts.first().copied().unwrap_or(1);
        let frame_count_duplicate = frame_counts.get(1).copied().unwrap_or(frame_count);
        let canvas = Canvas {
            x: x.unwrap_or(0.0),
            y: y.unwrap_or(0.0),
            width: width.unwrap_or(0.0),
            height: height.unwrap_or(0.0),
            resolution: resolution.unwrap_or(72.0),
            resolution_unit: resolution_unit.unwrap_or(0),
            background_raw: background_raw.unwrap_or(0xFF_FF_FF),
        };
        Ok(Page {
            info: PageInfo {
                version,
                revision,
                frame_count,
                frame_count_duplicate,
                request_id: request_id.unwrap_or(0),
                canvas,
                onion_before: onion_before.unwrap_or(0),
                onion_after: onion_after.unwrap_or(0),
                frame_durations,
            },
            name,
            is_master: is_master.unwrap_or(false),
            is_linked: is_linked.unwrap_or(false),
            page_id,
            last_export_dir,
            last_export_file,
            layers,
            grid,
            guides: guides.unwrap_or_default(),
            export,
        })
    }

    fn read_symbol_list(
        &mut self,
        children: &[Node],
        path: &str,
        out: &mut Vec<SymbolDefinition>,
    ) -> Result<(), MapError> {
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"MSTv" => {
                    let kids = children_of(child, path)?;
                    let mstv_path = format!("{path}/MSTv[{i}]");
                    out.push(self.read_symbol_definition(kids, &mstv_path)?);
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(())
    }

    fn read_symbol_definition(
        &mut self,
        children: &[Node],
        path: &str,
    ) -> Result<SymbolDefinition, MapError> {
        let (mut name, mut number, mut frame_count, mut background_raw) = (None, None, None, None);
        let (mut ox, mut oy) = (None, None);
        let (mut symbol_type, mut nine_slice) = (None, None);
        let (mut op_code, mut layers) = (None, LayerList::default());

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"UNMs" => self.slot_str(child, path, i, &mut name)?,
                b"MSNi" => self.slot_i64(child, path, i, &mut number)?,
                b"MOXf" => self.slot_f64(child, path, i, &mut ox)?,
                b"MOYf" => self.slot_f64(child, path, i, &mut oy)?,
                b"FRCi" => self.slot_i64(child, path, i, &mut frame_count)?,
                b"BGCi" => self.slot_i64(child, path, i, &mut background_raw)?,
                b"MSYi" => {
                    let mut raw = None;
                    self.slot_i64(child, path, i, &mut raw)?;
                    symbol_type = Some(SymbolType::from_raw(raw));
                }
                b"JSVv" => {
                    if op_code.is_none() {
                        let kids = children_of(child, path)?;
                        match Self::read_script_source(kids) {
                            Some(source) => op_code = Some(source),
                            None => self.bag_node(path, i, child),
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"NGSv" => {
                    if nine_slice.is_none() {
                        let kids = children_of(child, path)?;
                        let ngsv_path = format!("{path}/NGSv");
                        nine_slice = Some(self.read_nine_slice(kids, &ngsv_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"LYLv" => {
                    if layers.layers.is_empty() {
                        let kids = children_of(child, path)?;
                        let lylv_path = format!("{path}/LYLv");
                        layers = self.read_layer_list(kids, &lylv_path)?;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }

        Ok(SymbolDefinition {
            name: name.unwrap_or_default(),
            symbol_number: number.unwrap_or(0),
            origin: Point::new(ox.unwrap_or(0.0), oy.unwrap_or(0.0)),
            frame_count: frame_count.unwrap_or(1),
            layers,
            background_raw: background_raw.unwrap_or(0xFF_FF_FF),
            symbol_type: symbol_type.unwrap_or(SymbolType::Graphic),
            nine_slice,
            op_code,
        })
    }

    /// Style library list (`ZSLv` children): `ZSTv` records in file
    /// order; anything else bags in place (the corpus carries none).
    /// `parent_path` is the container holding the list — the document
    /// body (`/MKBv`) or the empty root of a bare library.
    fn read_style_list(
        &mut self,
        children: &[Node],
        parent_path: &str,
        out: &mut Vec<StyleEntry>,
    ) -> Result<(), MapError> {
        let path = format!("{parent_path}/ZSLv");
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"ZSTv" => {
                    let kids = children_of(child, &path)?;
                    let zstv_path = format!("{path}/ZSTv[{i}]");
                    out.push(self.read_style_entry(kids, &zstv_path)?);
                }
                _ => self.bag_node(&path, i, child),
            }
        }
        Ok(())
    }

    /// One `ZSTv` — the named-style record. The mapped spine is the
    /// corpus's fixed child order; the two `IMGv` texture slots and
    /// any unmodeled extra splice back verbatim (pattern fills are
    /// unmodeled everywhere, so the slots have no model home yet).
    fn read_style_entry(&mut self, children: &[Node], path: &str) -> Result<StyleEntry, MapError> {
        let (mut name, mut flags, mut kind) = (None, None, None);
        let mut paint = None;
        let mut effects = Vec::new();
        let (mut start_x, mut start_y, mut end_x, mut end_y) = (None, None, None, None);
        let (mut third_x, mut third_y) = (None, None);
        let (mut brush_placement, mut fill_on_top) = (None, None);
        let mut text = StyleTextSlots::default();

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"ZSNs" => self.slot_str(child, path, i, &mut name)?,
                b"ZSFi" => self.slot_i64(child, path, i, &mut flags)?,
                b"ZSOi" => self.slot_i64(child, path, i, &mut kind)?,
                b"PATv" => {
                    if paint.is_none() {
                        let kids = children_of(child, path)?;
                        let patv_path = format!("{path}/PATv");
                        paint = Some(self.read_paint(kids, &patv_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"PSXf" => self.slot_f64(child, path, i, &mut start_x)?,
                b"PSYf" => self.slot_f64(child, path, i, &mut start_y)?,
                b"PEXf" => self.slot_f64(child, path, i, &mut end_x)?,
                b"PEYf" => self.slot_f64(child, path, i, &mut end_y)?,
                b"PFXf" => self.slot_f64(child, path, i, &mut third_x)?,
                b"PFYf" => self.slot_f64(child, path, i, &mut third_y)?,
                b"BRPi" => self.slot_i64(child, path, i, &mut brush_placement)?,
                b"FOTb" => self.slot_bool(child, path, i, &mut fill_on_top)?,
                b"EFLv" => {
                    let eflv_path = format!("{path}/EFLv[{}]", effects.len());
                    effects.push(self.read_effect(child, &eflv_path)?);
                }
                b"FALi" => self.slot_i64(child, path, i, &mut text.margin_left)?,
                b"FATi" => self.slot_i64(child, path, i, &mut text.margin_top)?,
                b"FARi" => self.slot_i64(child, path, i, &mut text.margin_right)?,
                b"FABi" => self.slot_i64(child, path, i, &mut text.margin_bottom)?,
                b"FONs" => self.slot_str(child, path, i, &mut text.font)?,
                b"CFTs" => self.slot_str(child, path, i, &mut text.font_category)?,
                b"PTSf" => self.slot_f64(child, path, i, &mut text.size)?,
                b"BOLb" => self.slot_bool(child, path, i, &mut text.bold)?,
                b"ITLb" => self.slot_bool(child, path, i, &mut text.italic)?,
                b"UNDb" => self.slot_bool(child, path, i, &mut text.underline)?,
                b"JSTi" => self.slot_i64(child, path, i, &mut text.justification)?,
                b"HSCf" => self.slot_f64(child, path, i, &mut text.horizontal_scale)?,
                b"RKNf" => self.slot_f64(child, path, i, &mut text.kerning)?,
                b"LEDf" => self.slot_f64(child, path, i, &mut text.leading)?,
                b"LDMi" => self.slot_i64(child, path, i, &mut text.leading_unit)?,
                b"ATKb" => self.slot_bool(child, path, i, &mut text.auto_kern)?,
                b"TIAb" => self.slot_bool(child, path, i, &mut text.anti_alias_enabled)?,
                b"TAAi" => self.slot_i64(child, path, i, &mut text.anti_alias)?,
                b"TA1i" => self.slot_i64(child, path, i, &mut text.attr_1)?,
                b"TA2i" => self.slot_i64(child, path, i, &mut text.attr_2)?,
                b"TOSi" => self.slot_i64(child, path, i, &mut text.tab_size)?,
                _ => self.bag_node(path, i, child),
            }
        }

        Ok(StyleEntry {
            name: name.unwrap_or_default(),
            flags: flags.unwrap_or(0x1ff),
            kind: kind.unwrap_or(8),
            paint: paint.unwrap_or(PaintStyle {
                stroke_raw: 0,
                fill_raw: 0,
                fill: None,
            }),
            fill_start: Point::new(start_x.unwrap_or(0.0), start_y.unwrap_or(0.0)),
            fill_end: Point::new(end_x.unwrap_or(0.0), end_y.unwrap_or(0.0)),
            fill_third: Point::new(third_x.unwrap_or(0.0), third_y.unwrap_or(0.0)),
            brush_placement: brush_placement.unwrap_or(0),
            fill_on_top: fill_on_top.unwrap_or(false),
            effects,
            text: StyleText {
                margin_left: text.margin_left.unwrap_or(0),
                margin_top: text.margin_top.unwrap_or(0),
                margin_right: text.margin_right.unwrap_or(0),
                margin_bottom: text.margin_bottom.unwrap_or(0),
                font: text.font.unwrap_or_default(),
                font_category: text.font_category,
                size: text.size.unwrap_or(12.0),
                bold: text.bold.unwrap_or(false),
                italic: text.italic.unwrap_or(false),
                underline: text.underline.unwrap_or(false),
                justification: text.justification.unwrap_or(0),
                horizontal_scale: text.horizontal_scale.unwrap_or(100.0),
                kerning: text.kerning.unwrap_or(0.0),
                leading: text.leading.unwrap_or(1.0),
                leading_unit: text.leading_unit.unwrap_or(0),
                auto_kern: text.auto_kern.unwrap_or(true),
                anti_alias_enabled: text.anti_alias_enabled.unwrap_or(true),
                anti_alias: text.anti_alias.unwrap_or(0),
                attr_1: text.attr_1.unwrap_or(0xc0),
                attr_2: text.attr_2.unwrap_or(0x40),
                tab_size: text.tab_size.unwrap_or(8),
            },
        })
    }

    /// `JSVv` children → script source: the concatenated `JSSs`
    fn read_script_source(children: &[Node]) -> Option<String> {
        let mut source = String::new();
        for child in children {
            source.push_str(child.as_str()?);
        }
        Some(source)
    }

    /// `DTAv` children → custom data. Claimed only when every child
    /// is a `DCEv` carrying `DCKs` + exactly one `DCVs` string — the
    /// shape this model writes — and no key repeats. `GPLv`/`GDTv`
    /// values or any other shape stay preserved whole in the bag, so
    /// foreign nested data round-trips untouched.
    fn read_custom_data(children: &[Node]) -> Option<std::collections::BTreeMap<String, String>> {
        let mut entries = std::collections::BTreeMap::new();
        for child in children {
            if child.fourcc != *b"DCEv" {
                return None;
            }
            let kids = child.as_children()?;
            if kids.len() != 2 || kids[0].fourcc != *b"DCKs" || kids[1].fourcc != *b"DCVs" {
                return None;
            }
            let key = kids[0].as_str()?;
            let value = kids[1].as_str()?;
            if entries.insert(key.to_owned(), value.to_owned()).is_some() {
                // A repeated key would lose data on rewrite — refuse
                // the claim instead.
                return None;
            }
        }
        Some(entries)
    }

    /// `NGSv` children → [`NineSliceGuides`]. Dictionary order:
    /// `NGLb`, `NGKi`, `NGCf`×4 (top, bottom, left, right), `NGMv`,
    /// `NGVv`; anything else stays preserved.
    fn read_nine_slice(
        &mut self,
        children: &[Node],
        path: &str,
    ) -> Result<NineSliceGuides, MapError> {
        let (mut locked, mut color_raw) = (None, None);
        let mut controls: Vec<f64> = Vec::new();
        let (mut shape, mut limits) = (None, None);

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"NGLb" => self.slot_bool(child, path, i, &mut locked)?,
                b"NGKi" => self.slot_i64(child, path, i, &mut color_raw)?,
                b"NGCf" => {
                    let mut value = None;
                    self.slot_f64(child, path, i, &mut value)?;
                    controls.push(value.unwrap_or(0.0));
                }
                b"NGMv" => {
                    if shape.is_none() {
                        let kids = children_of(child, path)?;
                        let ngmv_path = format!("{path}/NGMv");
                        shape = Some(self.read_bounds(kids, &ngmv_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"NGVv" => {
                    if limits.is_none() {
                        let kids = children_of(child, path)?;
                        let ngvv_path = format!("{path}/NGVv");
                        limits = Some(self.read_bounds(kids, &ngvv_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }

        let mut four = [0.0_f64; 4];
        for (slot, value) in four.iter_mut().zip(controls.iter()) {
            *slot = *value;
        }
        Ok(NineSliceGuides {
            locked: locked.unwrap_or(false),
            color_raw: color_raw.unwrap_or(0),
            controls: four,
            shape_bounds: shape.unwrap_or_default(),
            limits: limits.unwrap_or_default(),
        })
    }

    /// `LFTf`/`TOPf`/`RITf`/`BOTf` children → a bounds [`Rect`].
    fn read_bounds(&mut self, children: &[Node], path: &str) -> Result<Rect, MapError> {
        let (mut left, mut top, mut right, mut bottom) = (None, None, None, None);
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"LFTf" => self.slot_f64(child, path, i, &mut left)?,
                b"TOPf" => self.slot_f64(child, path, i, &mut top)?,
                b"RITf" => self.slot_f64(child, path, i, &mut right)?,
                b"BOTf" => self.slot_f64(child, path, i, &mut bottom)?,
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(Rect {
            x0: left.unwrap_or(0.0),
            y0: top.unwrap_or(0.0),
            x1: right.unwrap_or(0.0),
            y1: bottom.unwrap_or(0.0),
        })
    }

    fn read_grid(&mut self, children: &[Node], path: &str) -> Result<Grid, MapError> {
        let (mut gox, mut goy, mut gsx, mut gsy, mut gcl) = (None, None, None, None, None);
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"GOXf" => self.slot_f64(child, path, i, &mut gox)?,
                b"GOYf" => self.slot_f64(child, path, i, &mut goy)?,
                b"GSXf" => self.slot_f64(child, path, i, &mut gsx)?,
                b"GSYf" => self.slot_f64(child, path, i, &mut gsy)?,
                b"GCLi" => self.slot_i64(child, path, i, &mut gcl)?,
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(Grid {
            offset: Point::new(gox.unwrap_or(0.0), goy.unwrap_or(0.0)),
            size: Point::new(gsx.unwrap_or(24.0), gsy.unwrap_or(24.0)),
            color_raw: gcl.unwrap_or(0),
        })
    }

    fn read_guides(&mut self, children: &[Node], path: &str) -> Result<Guides, MapError> {
        let (mut locked, mut color) = (None, None);
        let (mut horizontal, mut vertical) = (None, None);
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"GDLb" => self.slot_bool(child, path, i, &mut locked)?,
                b"GDKi" => self.slot_i64(child, path, i, &mut color)?,
                b"GDHv" => {
                    if horizontal.is_none() {
                        let kids = children_of(child, path)?;
                        let axis_path = format!("{path}/GDHv");
                        horizontal = Some(self.read_guide_axis(kids, &axis_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"GDVv" => {
                    if vertical.is_none() {
                        let kids = children_of(child, path)?;
                        let axis_path = format!("{path}/GDVv");
                        vertical = Some(self.read_guide_axis(kids, &axis_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(Guides {
            locked: locked.unwrap_or(false),
            color_raw: color.unwrap_or(0),
            horizontal: horizontal.unwrap_or_default(),
            vertical: vertical.unwrap_or_default(),
        })
    }

    fn read_guide_axis(&mut self, children: &[Node], path: &str) -> Result<Vec<f64>, MapError> {
        let mut positions = Vec::new();
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"GDCf" => match child.as_float() {
                    Some(value) => positions.push(value),
                    None => return Err(bad_shape(child, path, "float")),
                },
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(positions)
    }

    fn read_layer_list(&mut self, children: &[Node], path: &str) -> Result<LayerList, MapError> {
        let mut list = LayerList::default();
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"LAYv" => {
                    let kids = children_of(child, path)?;
                    let layv_path = format!("{path}/LAYv[{i}]");
                    list.layers.push(self.read_layer(kids, &layv_path)?);
                }
                b"LSAv" => {
                    // A shared-layer link (`UIDs` + `ISPb`, decision
                    // 0022): a slot with no layer body.
                    let kids = children_of(child, path)?;
                    let lsav_path = format!("{path}/LSAv[{i}]");
                    let (mut uid, mut is_primary) = (None, None);
                    for (j, grandchild) in kids.iter().enumerate() {
                        match &grandchild.fourcc {
                            b"UIDs" => self.slot_str(grandchild, &lsav_path, j, &mut uid)?,
                            b"ISPb" => {
                                self.slot_bool(grandchild, &lsav_path, j, &mut is_primary)?
                            }
                            _ => self.bag_node(&lsav_path, j, grandchild),
                        }
                    }
                    list.layers.push(Layer {
                        shared_link: Some(SharedLayerLink {
                            uid: uid.unwrap_or_default(),
                            is_primary: is_primary.unwrap_or(false),
                        }),
                        ..Layer::default()
                    });
                }
                // MPLv / anything else: preserved verbatim.
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(list)
    }

    fn read_layer(&mut self, children: &[Node], path: &str) -> Result<Layer, MapError> {
        let (mut is_background, mut has_url) = (None, None);
        let (mut is_special, mut frame_changing, mut display) = (None, None, None);
        let (mut name, mut nesting, mut visibility) = (None, None, None);
        let mut cells = Vec::new();

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"BKGb" => self.slot_bool(child, path, i, &mut is_background)?,
                b"URLb" => self.slot_bool(child, path, i, &mut has_url)?,
                b"LNMs" => self.slot_str(child, path, i, &mut name)?,
                b"LLLi" => self.slot_i64(child, path, i, &mut nesting)?,
                b"MSLb" => self.slot_bool(child, path, i, &mut is_special)?,
                b"MCLb" => self.slot_bool(child, path, i, &mut frame_changing)?,
                b"DISb" => self.slot_bool(child, path, i, &mut display)?,
                b"CLLv" => {
                    if cells.is_empty() {
                        let kids = children_of(child, path)?;
                        let cllv_path = format!("{path}/CLLv");
                        cells = self.read_cell_list(kids, &cllv_path)?;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"VIFv" => {
                    if visibility.is_none() {
                        let kids = children_of(child, path)?;
                        let vifv_path = format!("{path}/VIFv");
                        visibility = Some(self.read_visibility(kids, &vifv_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }

        Ok(Layer {
            name: name.unwrap_or_default(),
            is_background: is_background.unwrap_or(false),
            has_url: has_url.unwrap_or(false),
            is_special: is_special.unwrap_or(false),
            frame_changing: frame_changing.unwrap_or(false),
            display: display.unwrap_or(true),
            nesting_level: nesting,
            cells,
            visibility_per_frame: visibility,
            shared_link: None,
        })
    }

    /// One shared-layer definition (`LSEv` = `UIDs` + `ISPb` + `LAYv`,
    /// decision 0022). Unexpected inner shapes stay bagged.
    fn read_shared_def(
        &mut self,
        children: &[Node],
        path: &str,
    ) -> Result<SharedLayerDef, MapError> {
        let (mut uid, mut is_primary, mut layer) = (None, None, None);
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"UIDs" => self.slot_str(child, path, i, &mut uid)?,
                b"ISPb" => self.slot_bool(child, path, i, &mut is_primary)?,
                b"LAYv" => {
                    if layer.is_some() {
                        self.bag_node(path, i, child);
                    } else {
                        let kids = children_of(child, path)?;
                        let layv_path = format!("{path}/LAYv[{i}]");
                        layer = Some(self.read_layer(kids, &layv_path)?);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(SharedLayerDef {
            uid: uid.unwrap_or_default(),
            is_primary: is_primary.unwrap_or(false),
            layer: layer.unwrap_or_default(),
        })
    }

    /// The shared-definitions node (`LSMv`, decision 0022): `LSEv`*
    /// children, in order.
    fn read_shared_defs(
        &mut self,
        children: &[Node],
        path: &str,
    ) -> Result<Vec<SharedLayerDef>, MapError> {
        let mut defs = Vec::new();
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"LSEv" => {
                    let kids = children_of(child, path)?;
                    let lsev_path = format!("{path}/LSEv[{i}]");
                    defs.push(self.read_shared_def(kids, &lsev_path)?);
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(defs)
    }

    fn read_visibility(&mut self, children: &[Node], path: &str) -> Result<Vec<bool>, MapError> {
        let mut flags = Vec::new();
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"VISb" => match child.as_bool() {
                    Some(value) => flags.push(value),
                    None => return Err(bad_shape(child, path, "bool")),
                },
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(flags)
    }

    fn read_cell_list(&mut self, children: &[Node], path: &str) -> Result<Vec<Cell>, MapError> {
        let mut cells = Vec::new();
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"CELv" => {
                    let kids = children_of(child, path)?;
                    let celv_path = format!("{path}/CELv[{i}]");
                    cells.push(self.read_cell(kids, &celv_path)?);
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(cells)
    }

    fn read_cell(&mut self, children: &[Node], path: &str) -> Result<Cell, MapError> {
        let (mut locked, mut visible) = (None, None);
        let mut elements = Vec::new();
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"LCKb" => self.slot_bool(child, path, i, &mut locked)?,
                b"VISb" => self.slot_bool(child, path, i, &mut visible)?,
                b"ELMv" => {
                    if elements.is_empty() {
                        let kids = children_of(child, path)?;
                        let elmv_path = format!("{path}/ELMv");
                        elements = self.read_elements(kids, &elmv_path)?;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(Cell {
            locked: locked.unwrap_or(false),
            visible: visible.unwrap_or(true),
            elements,
        })
    }

    fn read_elements(&mut self, children: &[Node], path: &str) -> Result<Vec<ObjectId>, MapError> {
        let mut ids = Vec::new();
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"PTHv" | b"TXTv" | b"GRPv" | b"URLv" | b"IMGv" | b"ALSv" => {
                    let object_path = format!("{path}[{i}]");
                    if let Some(object) = self.read_object(child, &object_path)? {
                        ids.push(self.registry.insert(object));
                    } else {
                        // Unmodelable (e.g. a projective `MTXv`): the whole
                        // object goes to the bag verbatim; any partial bags
                        // made while probing it are subsumed.
                        self.bag.purge_under(&object_path);
                        self.bag_node(path, i, child);
                    }
                }
                // LSLv sub-layers and anything else stay verbatim.
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(ids)
    }

    fn read_object(&mut self, node: &Node, path: &str) -> Result<Option<ObjectData>, MapError> {
        let children = children_of(node, path)?;
        match &node.fourcc {
            b"PTHv" => self.read_path(children, path),
            b"TXTv" => self.read_text(children, path).map(Some),
            b"GRPv" => self.read_group(children, path).map(Some),
            b"URLv" => self.read_slice(children, path).map(Some),
            b"IMGv" => self.read_image(children, path).map(Some),
            b"ALSv" => self.read_symbol_instance(children, path),
            _ => Ok(None),
        }
    }

    /// Returns `None` when the whole `PTHv` must be preserved (its
    /// primitive carries a matrix the model cannot represent).
    fn read_path(&mut self, children: &[Node], path: &str) -> Result<Option<ObjectData>, MapError> {
        let mut contours = Vec::new();
        let mut primitive: Option<PrimitiveSource> = None;
        let mut primitive_lost = false;
        let mut even_odd = None;
        let (mut paint, mut brush_placement, mut fill_on_top) = (None, None, None);
        let (mut psx, mut psy, mut pex, mut pey, mut pfx, mut pfy) =
            (None, None, None, None, None, None);
        let (mut locked, mut visible, mut opacity, mut blend, mut name) =
            (None, None, None, None, None);
        let mut effects = Vec::new();

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"LCKb" => self.slot_bool(child, path, i, &mut locked)?,
                b"PATv" => {
                    if paint.is_none() {
                        let kids = children_of(child, path)?;
                        let patv_path = format!("{path}/PATv");
                        paint = Some(self.read_paint(kids, &patv_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"BRPi" => self.slot_i64(child, path, i, &mut brush_placement)?,
                b"FOTb" => self.slot_bool(child, path, i, &mut fill_on_top)?,
                b"PSXf" => self.slot_f64(child, path, i, &mut psx)?,
                b"PSYf" => self.slot_f64(child, path, i, &mut psy)?,
                b"PEXf" => self.slot_f64(child, path, i, &mut pex)?,
                b"PEYf" => self.slot_f64(child, path, i, &mut pey)?,
                b"PFXf" => self.slot_f64(child, path, i, &mut pfx)?,
                b"PFYf" => self.slot_f64(child, path, i, &mut pfy)?,
                b"PBLv" => {
                    if contours.is_empty() {
                        let kids = children_of(child, path)?;
                        let pblv_path = format!("{path}/PBLv");
                        contours = self.read_contours(kids, &pblv_path)?;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"EOFb" => self.slot_bool(child, path, i, &mut even_odd)?,
                b"PRIv" => {
                    if primitive.is_none() && !primitive_lost {
                        match self.read_primitive(child, path)? {
                            Some(source) => primitive = Some(source),
                            // Unmodelable (e.g. projective MTXv): the whole
                            // PTHv goes to the bag, so bag nothing here.
                            None => primitive_lost = true,
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"OBNs" => self.slot_str(child, path, i, &mut name)?,
                b"BLDi" => self.slot_i64(child, path, i, &mut blend)?,
                b"EFLv" => {
                    let eflv_path = format!("{path}/EFLv[{}]", effects.len());
                    effects.push(self.read_effect(child, &eflv_path)?);
                }
                b"VISb" => self.slot_bool(child, path, i, &mut visible)?,
                b"OPAi" => self.slot_i64(child, path, i, &mut opacity)?,
                _ => self.bag_node(path, i, child),
            }
        }

        // Fill points are recorded as x/y pairs; half a pair is a shape
        // we cannot represent faithfully (same discipline as PBTv
        // control points).
        if psx.is_some() != psy.is_some()
            || pex.is_some() != pey.is_some()
            || pfx.is_some() != pfy.is_some()
        {
            return Err(MapError::BadShape {
                path: path.to_owned(),
                node: *b"PTHv",
                expected: "paired x/y fill points",
                found: "unpaired fill scalar",
            });
        }

        if primitive_lost {
            return Ok(None);
        }
        Ok(Some(ObjectData::Path(PathObject {
            contours,
            primitive,
            even_odd: even_odd.unwrap_or(false),
            paint,
            fill_start: psx.zip(psy).map(|(x, y)| Point::new(x, y)),
            fill_end: pex.zip(pey).map(|(x, y)| Point::new(x, y)),
            fill_third: pfx.zip(pfy).map(|(x, y)| Point::new(x, y)),
            brush_placement,
            fill_on_top,
            locked: locked.unwrap_or(false),
            visible: visible.unwrap_or(true),
            opacity: opacity.unwrap_or(1000),
            blend: blend.unwrap_or(0),
            name,
            effects,
        })))
    }

    /// Reads a `PATv` paint: the two solid colours and the `FPLv`
    /// fill subtree are modeled; the texture subtrees (`TXBv` `TXFv`
    /// `BPLv`) and unmodeled fill nodes stay in the bag under the
    /// `PATv` path.
    fn read_paint(&mut self, children: &[Node], path: &str) -> Result<PaintStyle, MapError> {
        let (mut brush, mut fill) = (None, None);
        let mut fill_style = None;
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"BCLi" => self.slot_i64(child, path, i, &mut brush)?,
                b"FCLi" => self.slot_i64(child, path, i, &mut fill)?,
                b"FPLv" => {
                    if fill_style.is_none() {
                        let fplv_path = format!("{path}/FPLv");
                        match self.read_fill(child, &fplv_path)? {
                            Some(parsed) => fill_style = Some(parsed),
                            // Unmodelable fill: preserve the whole
                            // subtree in the bag, subsuming the
                            // partial entries the walk left behind.
                            None => {
                                self.bag.purge_under(&fplv_path);
                                self.bag_node(path, i, child);
                            }
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(PaintStyle {
            stroke_raw: brush.unwrap_or(0),
            fill_raw: fill.unwrap_or(0),
            fill: fill_style,
        })
    }

    /// One `FPLv` fill definition, or `None` when the subtree cannot
    /// be modeled faithfully (a shape value outside either
    /// enumeration, a malformed gradient list — the caller bags the
    /// whole node so bytes stay verbatim).
    ///
    /// Mapped children: `FTBi` `FEFi` `FSMi` `FRDb`, the shape slot
    /// (`FSXi` preferred, `FSHi` fallback), `FGLv`, the dither colours
    /// (`FD1i`..`FD4i`, `FDTb`), the `FPTv` pattern tile. Everything
    /// else — names, `RDOb`, `FETi`/`FRRi` flags, a bagged-back `FSHi`
    /// when `FSXi` carried the kind — splices from the bag.
    fn read_fill(&mut self, node: &Node, path: &str) -> Result<Option<Fill>, MapError> {
        let children = children_of(node, path)?;
        let (mut texture_blend, mut feather, mut stamping, mut hard_edge) =
            (None, None, None, None);
        let (mut shape, mut shape_extended) = (None, false);
        let (mut gradient, mut dither, mut pattern) = (None, None, None);
        let mut dither_slots: [Option<i64>; 4] = [None; 4];
        // The compatibility slot's node, held until the walk ends: an
        // `FSHi` seen before `FSXi` must not claim the kind.
        let mut compat: Option<(usize, Node)> = None;

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"FTBi" => self.slot_i64(child, path, i, &mut texture_blend)?,
                b"FEFi" => self.slot_i64(child, path, i, &mut feather)?,
                b"FSMi" => self.slot_i64(child, path, i, &mut stamping)?,
                b"FRDb" => self.slot_bool(child, path, i, &mut hard_edge)?,
                b"FSXi" => {
                    if shape_extended {
                        self.bag_node(path, i, child);
                    } else {
                        match fill_shape(child.as_int(), true) {
                            Some(parsed) => {
                                shape = Some(parsed);
                                shape_extended = true;
                                // A compatibility node seen earlier
                                // loses the slot and splices back.
                                if let Some((j, held)) = compat.take() {
                                    self.bag_node(path, j, &held);
                                }
                            }
                            // Unknown kind: the whole subtree is
                            // unmodelable.
                            None => return Ok(None),
                        }
                    }
                }
                b"FSHi" => {
                    if shape_extended {
                        // The extended kind won; the compatibility
                        // node splices back verbatim.
                        self.bag_node(path, i, child);
                    } else if shape.is_some() || compat.is_some() {
                        // Duplicate compatibility slot.
                        self.bag_node(path, i, child);
                    } else {
                        match fill_shape(child.as_int(), false) {
                            Some(parsed) => {
                                shape = Some(parsed);
                                compat = Some((i, child.clone()));
                            }
                            // A compatibility value outside its own
                            // enumeration is unmodelable too.
                            None => return Ok(None),
                        }
                    }
                }
                b"FGLv" => {
                    if gradient.is_none() {
                        match self.read_gradient(child, &format!("{path}/FGLv"))? {
                            Some(parsed) => gradient = Some(parsed),
                            None => return Ok(None),
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"FD1i" | b"FD2i" | b"FD3i" | b"FD4i" => {
                    let slot = (child.fourcc[2] - b'1') as usize;
                    match (dither_slots[slot], child.as_int()) {
                        (None, Some(color)) => dither_slots[slot] = Some(color),
                        // Duplicate slot or a non-integer payload
                        // splices back verbatim.
                        _ => self.bag_node(path, i, child),
                    }
                }
                b"FDTb" => {
                    let mut transparent = None;
                    self.slot_bool(child, path, i, &mut transparent)?;
                    if dither.is_none() {
                        dither = Some(Dither {
                            colors: [None; 4],
                            transparent,
                        });
                    } else {
                        // A second flag splices back verbatim.
                        self.bag_node(path, i, child);
                    }
                }
                b"FPTv" => {
                    if pattern.is_none() {
                        let fptv_path = format!("{path}/FPTv");
                        match self.read_pattern_tile(child, &fptv_path)? {
                            Some(parsed) => pattern = Some(parsed),
                            // Unmodelable tile: preserve the whole
                            // subtree in the bag, subsuming the
                            // partial entries the walk left behind.
                            None => {
                                self.bag.purge_under(&fptv_path);
                                self.bag_node(path, i, child);
                            }
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }

        // A held compatibility node needs no bag entry: when `FSXi`
        // won, it was bagged at its index the moment it lost; when it
        // itself carries the kind (`shape_extended == false`), the
        // writer re-emits it as the mapped `FSHi` slot.
        drop(compat);

        // A fill without a shape slot has no kind to render from.
        let shape = match shape {
            Some(parsed) => parsed,
            None => return Ok(None),
        };
        let dither = if dither_slots.iter().any(Option::is_some) {
            Some(match dither.as_mut() {
                Some(existing) => {
                    existing.colors = dither_slots;
                    *existing
                }
                None => Dither {
                    colors: dither_slots,
                    transparent: None,
                },
            })
        } else {
            dither
        };
        Ok(Some(Fill {
            shape,
            shape_extended,
            texture_blend: texture_blend.unwrap_or(0),
            feather: feather.unwrap_or(0),
            stamping,
            hard_edge: hard_edge.unwrap_or(false),
            gradient,
            dither,
            pattern,
        }))
    }

    /// One `FPTv` pattern tile: `INMs` plus the `IMGv` raster bits
    /// (`WPXi`/`HPXi` extent, `TSZi` tile side, `TILv` references).
    /// `None` when the extent or the tile list cannot be modeled —
    /// the caller bags the node so bytes stay verbatim.
    fn read_pattern_tile(
        &mut self,
        node: &Node,
        path: &str,
    ) -> Result<Option<PatternTile>, MapError> {
        let children = children_of(node, path)?;
        let (mut name, mut pixels, mut tile_size, mut tiles) = (None, None, None, None);
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"INMs" => self.slot_str(child, path, i, &mut name)?,
                b"IMGv" => {
                    if tiles.is_none() {
                        let img_path = format!("{path}/IMGv");
                        let kids = children_of(child, &img_path)?;
                        let (mut wpx, mut hpx, mut tsz) = (None, None, None);
                        let mut refs: Option<Vec<TileRef>> = None;
                        for (j, grandchild) in kids.iter().enumerate() {
                            match &grandchild.fourcc {
                                b"WPXi" => self.slot_i64(grandchild, &img_path, j, &mut wpx)?,
                                b"HPXi" => self.slot_i64(grandchild, &img_path, j, &mut hpx)?,
                                b"TSZi" => self.slot_i64(grandchild, &img_path, j, &mut tsz)?,
                                b"TILv" => {
                                    if refs.is_none() {
                                        let tilv_path = format!("{img_path}/TILv");
                                        let tile_kids = children_of(grandchild, &tilv_path)?;
                                        refs = Some(self.read_tiles(tile_kids, &tilv_path)?);
                                    } else {
                                        self.bag_node(&img_path, j, grandchild);
                                    }
                                }
                                // DTAv and anything else: preserved.
                                _ => self.bag_node(&img_path, j, grandchild),
                            }
                        }
                        match (wpx, hpx, refs) {
                            (Some(w), Some(h), Some(refs)) => {
                                pixels = Some((w, h));
                                tiles = Some(refs);
                                tile_size = Some(tsz.unwrap_or(256));
                            }
                            // No extent or tile list: unmodelable —
                            // the caller bags the whole `FPTv` and
                            // purges whatever this walk bagged.
                            _ => {
                                return Ok(None);
                            }
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                // UNMs and anything else: preserved.
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(match (pixels, tiles) {
            (Some(pixels), Some(tiles)) => Some(PatternTile {
                name,
                pixels,
                tile_size: tile_size.unwrap_or(256),
                tiles,
            }),
            _ => None,
        })
    }

    /// One `FGLv` gradient: the legacy `FGVv` vertex list plus the
    /// `FGYv` ramps (`FG0v` RGB, `FG1v` alpha). Names ride the bag.
    /// `None` when any list is malformed (count mismatch, unpaired
    /// position/colour).
    fn read_gradient(&mut self, node: &Node, path: &str) -> Result<Option<Gradient>, MapError> {
        let children = children_of(node, path)?;
        let mut gradient = Gradient::default();

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"FGVv" => match self.read_ramp(child, &format!("{path}/FGVv"))? {
                    Some(ramp) => gradient.vertices = ramp,
                    None => return Ok(None),
                },
                b"FGYv" => {
                    let fgyv_path = format!("{path}/FGYv");
                    for (j, kid) in children_of(child, &fgyv_path)?.iter().enumerate() {
                        match &kid.fourcc {
                            b"FG0v" => match self.read_ramp(kid, &format!("{fgyv_path}/FG0v"))? {
                                Some(ramp) => gradient.rgb = ramp,
                                None => return Ok(None),
                            },
                            b"FG1v" => match self.read_ramp(kid, &format!("{fgyv_path}/FG1v"))? {
                                Some(ramp) => gradient.alpha = ramp,
                                None => return Ok(None),
                            },
                            _ => self.bag_node(&fgyv_path, j, kid),
                        }
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(Some(gradient))
    }

    /// One ramp: `FNCi` count followed by `FNCi` many `FGIv`
    /// (position + colour) pairs. Malformed lists return `None`.
    fn read_ramp(
        &mut self,
        node: &Node,
        path: &str,
    ) -> Result<Option<Vec<GradientStop>>, MapError> {
        let children = children_of(node, path)?;
        let mut count: Option<i64> = None;
        let mut stops: Vec<GradientStop> = Vec::new();

        for child in children {
            match &child.fourcc {
                b"FNCi" => {
                    if count.is_some() {
                        return Ok(None);
                    }
                    count = child.as_int();
                }
                b"FGIv" => {
                    let fgiv_path = format!("{path}/FGIv[{}]", stops.len());
                    let (mut position, mut color) = (None, None);
                    for (k, kid) in children_of(child, &fgiv_path)?.iter().enumerate() {
                        match &kid.fourcc {
                            b"FGPf" => position = kid.as_float(),
                            b"FGCi" => color = kid.as_int(),
                            _ => self.bag_node(&fgiv_path, k, kid),
                        }
                    }
                    match (position, color) {
                        (Some(position), Some(color)) => stops.push(GradientStop {
                            position,
                            color_raw: color,
                        }),
                        // Unpaired point: unmodelable.
                        _ => return Ok(None),
                    }
                }
                _ => return Ok(None),
            }
        }
        let expected = usize::try_from(count.unwrap_or(0)).unwrap_or(usize::MAX);
        // An empty ramp (`FNCi` 0) cannot be distinguished on write
        // from a missing ramp — treat it as unmodelable (the
        // dictionary's files carry x2+ points anyway).
        if stops.len() != expected || stops.is_empty() {
            return Ok(None);
        }
        Ok(Some(stops))
    }

    fn read_contours(&mut self, children: &[Node], path: &str) -> Result<Vec<Contour>, MapError> {
        let mut contours = Vec::new();
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"PBPv" => {
                    let kids = children_of(child, path)?;
                    let pbpv_path = format!("{path}/PBPv[{i}]");
                    contours.push(self.read_contour(kids, &pbpv_path)?);
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(contours)
    }

    fn read_contour(&mut self, children: &[Node], path: &str) -> Result<Contour, MapError> {
        let (mut closed, mut point_count) = (None, None);
        let mut points = Vec::new();
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"ISCb" => self.slot_bool(child, path, i, &mut closed)?,
                b"PPCi" => self.slot_i64(child, path, i, &mut point_count)?,
                b"PBTv" => {
                    let kids = children_of(child, path)?;
                    let pbtv_path = format!("{path}/PBTv[{i}]");
                    points.push(self.read_path_point(kids, &pbtv_path)?);
                }
                // BSLf and anything else: preserved.
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(Contour {
            closed: closed.unwrap_or(true),
            point_count: point_count.unwrap_or(points.len() as i64),
            points,
        })
    }

    fn read_path_point(&mut self, children: &[Node], path: &str) -> Result<PathPoint, MapError> {
        let (mut xl, mut yl, mut curve) = (None, None, None);
        let (mut xp, mut yp, mut xs, mut ys) = (None, None, None, None);
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"XLCf" => self.slot_f64(child, path, i, &mut xl)?,
                b"YLCf" => self.slot_f64(child, path, i, &mut yl)?,
                b"XPCf" => self.slot_f64(child, path, i, &mut xp)?,
                b"YPCf" => self.slot_f64(child, path, i, &mut yp)?,
                b"XSCf" => self.slot_f64(child, path, i, &mut xs)?,
                b"YSCf" => self.slot_f64(child, path, i, &mut ys)?,
                b"CRVb" => self.slot_bool(child, path, i, &mut curve)?,
                // RNDi / NSDf / BZLv / PPLv and anything else: preserved.
                _ => self.bag_node(path, i, child),
            }
        }
        if xp.is_some() != yp.is_some() || xs.is_some() != ys.is_some() {
            return Err(MapError::BadShape {
                path: path.to_owned(),
                node: *b"PBTv",
                expected: "paired x/y control points",
                found: "unpaired control scalar",
            });
        }
        Ok(PathPoint {
            anchor: Point::new(xl.unwrap_or(0.0), yl.unwrap_or(0.0)),
            primary: xp.zip(yp).map(|(x, y)| Point::new(x, y)),
            secondary: xs.zip(ys).map(|(x, y)| Point::new(x, y)),
            is_curve: curve.unwrap_or(false),
        })
    }

    /// Reads a primitive. Returns `Ok(None)` when the primitive cannot be
    /// modeled (no `RCTv`, or a matrix with a non-identity projective
    /// column) — the caller then preserves the containing node whole.
    fn read_primitive(
        &mut self,
        node: &Node,
        parent_path: &str,
    ) -> Result<Option<PrimitiveSource>, MapError> {
        let path = format!("{parent_path}/PRIv");
        let children = children_of(node, &path)?;
        let (mut matrix, mut rect_radius, mut maintain) = (None, None, None);

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"MTXv" => {
                    if matrix.is_none() {
                        let mtxv_path = format!("{path}/MTXv");
                        matrix = self.read_matrix(child, &mtxv_path)?;
                    } else {
                        self.bag_node(&path, i, child);
                    }
                }
                b"RCTv" => {
                    if rect_radius.is_none() {
                        let kids = children_of(child, &path)?;
                        let rctv_path = format!("{path}/RCTv");
                        rect_radius = Some(self.read_rect_with_radius(kids, &rctv_path)?);
                    } else {
                        self.bag_node(&path, i, child);
                    }
                }
                b"RDMb" => self.slot_bool(child, &path, i, &mut maintain)?,
                // Fill points (PCFb/PSXf/…), EFLv and anything else:
                // preserved.
                _ => self.bag_node(&path, i, child),
            }
        }

        let (rect, radius) = match rect_radius {
            Some((rect, radius)) => (rect, radius),
            None => return Ok(None),
        };
        match matrix {
            Some(affine) => Ok(Some(PrimitiveSource {
                rect,
                radius,
                transform: affine,
                maintain_radius: maintain.unwrap_or(false),
            })),
            // No matrix or a projective one: not representable.
            None => Ok(None),
        }
    }

    /// `Ok(None)` = projective (or absent) matrix; the caller preserves
    /// the containing node instead of half-mapping it. `path` is the
    /// `MTXv` node's own path.
    fn read_matrix(&mut self, node: &Node, path: &str) -> Result<Option<Affine>, MapError> {
        let children = children_of(node, path)?;
        let mut m = [None::<f64>; 9];
        for (i, child) in children.iter().enumerate() {
            let named = matches!(
                &child.fourcc,
                b"M00f"
                    | b"M01f"
                    | b"M02f"
                    | b"M10f"
                    | b"M11f"
                    | b"M12f"
                    | b"M20f"
                    | b"M21f"
                    | b"M22f"
            );
            if !named {
                self.bag_node(path, i, child);
                continue;
            }
            match child.as_float() {
                Some(value) => {
                    // M<row><col>f: the row digit follows the M.
                    let row = (child.fourcc[1] - b'0') as usize;
                    let col = (child.fourcc[2] - b'0') as usize;
                    m[row * 3 + col] = Some(value);
                }
                None => return Err(bad_shape(child, path, "float")),
            }
        }
        let value = |slot: usize| m[slot].unwrap_or(0.0);
        // Projective column must be the identity, else the matrix stays
        // bagged (see M3 addendum): M02 == M12 == 0 and M22 == 1.
        if value(2) != 0.0 || value(5) != 0.0 || m[8] != Some(1.0) {
            return Ok(None);
        }
        // Row-vector convention: kurbo stores [a b c d e f] for the
        // matrix [[a b 0] [c d 0] [e f 1]], matching MTXv's layout with
        // the projective column in M02/M12/M22.
        Ok(Some(Affine::new([
            value(0),
            value(1),
            value(3),
            value(4),
            value(6),
            value(7),
        ])))
    }

    fn read_rect_with_radius(
        &mut self,
        children: &[Node],
        path: &str,
    ) -> Result<(Rect, f64), MapError> {
        let (mut radius, mut left, mut top, mut right, mut bottom) = (None, None, None, None, None);
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"RDSf" => self.slot_f64(child, path, i, &mut radius)?,
                b"LFTf" => self.slot_f64(child, path, i, &mut left)?,
                b"TOPf" => self.slot_f64(child, path, i, &mut top)?,
                b"RITf" => self.slot_f64(child, path, i, &mut right)?,
                b"BOTf" => self.slot_f64(child, path, i, &mut bottom)?,
                _ => self.bag_node(path, i, child),
            }
        }
        // Rect fields are set directly: kurbo's constructor normalizes,
        // and an inverted rectangle from a corrupt file must survive the
        // read (not panic, not silently flip).
        Ok((
            Rect {
                x0: left.unwrap_or(0.0),
                y0: top.unwrap_or(0.0),
                x1: right.unwrap_or(0.0),
                y1: bottom.unwrap_or(0.0),
            },
            radius.unwrap_or(0.0),
        ))
    }

    fn read_text(&mut self, children: &[Node], path: &str) -> Result<ObjectData, MapError> {
        let (mut left, mut top, mut right, mut bottom) = (None, None, None, None);
        let (mut flow, mut anti_alias, mut auto_enlarge, mut auto_kern) = (None, None, None, None);
        let (mut locked, mut visible, mut opacity, mut blend, mut name) =
            (None, None, None, None, None);
        let mut style: Option<TextStyle> = None;
        let mut runs: Vec<TextRun> = Vec::new();
        let mut on_path: Option<TextOnPath> = None;
        let mut transform: Option<Affine> = None;
        let mut transform_lost = false;
        let mut effects = Vec::new();

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"LFTf" => self.slot_f64(child, path, i, &mut left)?,
                b"TOPf" => self.slot_f64(child, path, i, &mut top)?,
                b"RITf" => self.slot_f64(child, path, i, &mut right)?,
                b"BOTf" => self.slot_f64(child, path, i, &mut bottom)?,
                b"TFTi" => self.slot_i64(child, path, i, &mut flow)?,
                b"TAPv" => {
                    // One TAPv is the model slot; a second splices back
                    // verbatim.
                    if on_path.is_none() {
                        let kids = children_of(child, path)?;
                        let tapv_path = format!("{path}/TAPv");
                        on_path = Some(self.read_text_on_path(kids, &tapv_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"TAAi" => self.slot_i64(child, path, i, &mut anti_alias)?,
                b"ATEb" => self.slot_bool(child, path, i, &mut auto_enlarge)?,
                b"ATKb" => self.slot_bool(child, path, i, &mut auto_kern)?,
                b"LCKb" => self.slot_bool(child, path, i, &mut locked)?,
                b"OBNs" => self.slot_str(child, path, i, &mut name)?,
                b"OPAi" => self.slot_i64(child, path, i, &mut opacity)?,
                b"VISb" => self.slot_bool(child, path, i, &mut visible)?,
                b"BLDi" => self.slot_i64(child, path, i, &mut blend)?,
                b"TFSv" => {
                    // One TFSv is the model slot; style copies live at
                    // TXTv level and stay bagged. A second TFSv splices
                    // back verbatim.
                    if style.is_none() {
                        let kids = children_of(child, path)?;
                        let tfsv_path = format!("{path}/TFSv");
                        let (parsed_style, parsed_runs) =
                            self.read_formatted_string(kids, &tfsv_path)?;
                        style = Some(parsed_style);
                        runs = parsed_runs;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"EFLv" => {
                    let eflv_path = format!("{path}/EFLv[{}]", effects.len());
                    effects.push(self.read_effect(child, &eflv_path)?);
                }
                b"MTXv" => {
                    if transform.is_none() && !transform_lost {
                        let mtxv_path = format!("{path}/MTXv");
                        match self.read_matrix(child, &mtxv_path)? {
                            Some(affine) => transform = Some(affine),
                            // Projective matrix: keep the MTXv itself.
                            None => {
                                transform_lost = true;
                                self.bag_node(path, i, child);
                            }
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                // Anchors, TXTv-level style mirrors (`FONs` `PTSf` …),
                // `PATv`, fill points, data: preserved verbatim.
                _ => self.bag_node(path, i, child),
            }
        }

        Ok(ObjectData::Text(TextObject {
            frame: Rect {
                x0: left.unwrap_or(0.0),
                y0: top.unwrap_or(0.0),
                x1: right.unwrap_or(0.0),
                y1: bottom.unwrap_or(0.0),
            },
            style,
            runs,
            flow,
            on_path,
            anti_alias,
            auto_enlarge,
            auto_kern,
            name,
            opacity: opacity.unwrap_or(1000),
            blend: blend.unwrap_or(0),
            visible: visible.unwrap_or(true),
            locked: locked.unwrap_or(false),
            transform,
            effects,
        }))
    }

    /// Reads one `TAPv`: `TAMi` orientation, `CHOf` start offset, and
    /// the embedded `PTHv` guide path (its `PBLv` contours map; the
    /// node's styling children bag in place — the guide path renders as
    /// nothing itself).
    fn read_text_on_path(&mut self, children: &[Node], path: &str) -> Result<TextOnPath, MapError> {
        let (mut alignment, mut offset) = (None, None);
        let mut path_contours: Option<Vec<Contour>> = None;
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"TAMi" => self.slot_i64(child, path, i, &mut alignment)?,
                b"CHOf" => self.slot_f64(child, path, i, &mut offset)?,
                b"PTHv" => {
                    // One guide path maps; extras splice back verbatim.
                    if path_contours.is_none() {
                        let kids = children_of(child, path)?;
                        let pthv_path = format!("{path}/PTHv");
                        let mut contours = Vec::new();
                        for (j, sub) in kids.iter().enumerate() {
                            match &sub.fourcc {
                                b"PBLv" => {
                                    // The writer re-emits the guide as
                                    // the PTHv's first mapped child, so
                                    // descendants bag under the
                                    // canonical [0] regardless of where
                                    // the node sat in the file.
                                    if contours.is_empty() {
                                        let pblv_kids = children_of(sub, path)?;
                                        let pblv_path = format!("{pthv_path}[0]/PBLv");
                                        contours = self.read_contours(pblv_kids, &pblv_path)?;
                                    } else {
                                        self.bag_node(&pthv_path, j, sub);
                                    }
                                }
                                _ => self.bag_node(&pthv_path, j, sub),
                            }
                        }
                        path_contours = Some(contours);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(TextOnPath {
            alignment,
            offset: offset.unwrap_or(0.0),
            path: path_contours.unwrap_or_default(),
        })
    }

    /// Reads one `TFSv`: the mapped style scalars plus `TRNs`+ runs.
    /// Unknown styling (gradient fills and friends) bags in place, so
    /// the container still round-trips whole.
    fn read_formatted_string(
        &mut self,
        children: &[Node],
        path: &str,
    ) -> Result<(TextStyle, Vec<TextRun>), MapError> {
        let mut style = TextStyle::default();
        let mut runs = Vec::new();

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"FONs" => self.slot_str(child, path, i, &mut style.font)?,
                b"PTSf" => self.slot_f64(child, path, i, &mut style.size)?,
                b"BOLb" => self.slot_bool(child, path, i, &mut style.bold)?,
                b"ITLb" => self.slot_bool(child, path, i, &mut style.italic)?,
                b"UNDb" => self.slot_bool(child, path, i, &mut style.underline)?,
                b"HSCf" => self.slot_f64(child, path, i, &mut style.horizontal_scale)?,
                b"RKNf" => self.slot_f64(child, path, i, &mut style.kerning)?,
                b"BLSf" => self.slot_f64(child, path, i, &mut style.baseline_shift)?,
                b"LEDf" => self.slot_f64(child, path, i, &mut style.leading)?,
                b"LDMi" => self.slot_i64(child, path, i, &mut style.leading_unit)?,
                b"JSTi" => self.slot_i64(child, path, i, &mut style.justification)?,
                b"FCLi" => self.slot_i64(child, path, i, &mut style.fill_raw)?,
                b"PINf" => self.slot_f64(child, path, i, &mut style.paragraph_indent)?,
                b"PSBf" => self.slot_f64(child, path, i, &mut style.space_before)?,
                b"PSAf" => self.slot_f64(child, path, i, &mut style.space_after)?,
                b"TRNs" => match child.as_str() {
                    // Decision 0007: the run characters as a string node.
                    Some(text) => runs.push(TextRun {
                        text: text.to_owned(),
                    }),
                    // Undocumented payload shape: preserve it verbatim.
                    None => self.bag_node(path, i, child),
                },
                _ => self.bag_node(path, i, child),
            }
        }
        Ok((style, runs))
    }

    /// Reads one `EFLv` — one live effect: `CATs`/`INMs` names plus the
    /// first `EPSv` property list. Unknown children and malformed
    /// `DCEv` entries bag in place rather than fail: an effect we only
    /// half-understand must still round-trip whole.
    fn read_effect(&mut self, node: &Node, path: &str) -> Result<Effect, MapError> {
        let children = children_of(node, path)?;
        let (mut category, mut internal_name) = (None, None);
        let mut properties = Vec::new();
        let mut epsv_read = false;

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"CATs" => match child.as_str() {
                    Some(text) => category = Some(text.to_owned()),
                    None => self.bag_node(path, i, child),
                },
                b"INMs" => match child.as_str() {
                    Some(text) => internal_name = Some(text.to_owned()),
                    None => self.bag_node(path, i, child),
                },
                b"EPSv" => {
                    if epsv_read {
                        // First property list wins; extras splice back.
                        self.bag_node(path, i, child);
                    } else {
                        let epsv_path = format!("{path}/EPSv");
                        let kids = children_of(child, &epsv_path)?;
                        for (j, entry) in kids.iter().enumerate() {
                            match self.read_effect_entry(entry, &epsv_path, j)? {
                                Some(property) => properties.push(property),
                                // Malformed entry: preserve it whole.
                                None => self.bag_node(&epsv_path, j, entry),
                            }
                        }
                        epsv_read = true;
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }

        Ok(Effect {
            category: category.unwrap_or_default(),
            internal_name: internal_name.unwrap_or_default(),
            properties,
        })
    }

    /// One `DCEv` dictionary entry: a `DCKs` key plus exactly one value
    /// node. String values (`DCVs`) read as [`EffectValue::Text`];
    /// everything else (`GPLv` grab-point lists, `GDTv` nested
    /// dictionaries, anything unrecognized) is preserved verbatim.
    /// Returns `None` for entries without exactly one key and one
    /// value — the caller then keeps the whole entry in the bag.
    fn read_effect_entry(
        &mut self,
        node: &Node,
        parent_path: &str,
        index: usize,
    ) -> Result<Option<EffectProperty>, MapError> {
        let path = format!("{parent_path}[{index}]");
        let children = children_of(node, &path)?;
        let (mut key, mut value) = (None, None);

        for child in children {
            match &child.fourcc {
                b"DCKs" => {
                    if key.is_some() {
                        return Ok(None);
                    }
                    match child.as_str() {
                        Some(text) => key = Some(text.to_owned()),
                        None => return Ok(None),
                    }
                }
                _ => {
                    if value.is_some() {
                        return Ok(None);
                    }
                    value = Some(match (child.fourcc == *b"DCVs", child.as_str()) {
                        (true, Some(text)) => EffectValue::Text(text.to_owned()),
                        _ => EffectValue::Raw(child.clone()),
                    });
                }
            }
        }

        Ok(match (key, value) {
            (Some(key), Some(value)) => Some(EffectProperty { key, value }),
            _ => None,
        })
    }

    fn read_group(&mut self, children: &[Node], path: &str) -> Result<ObjectData, MapError> {
        let (mut group_type, mut opacity, mut blend) = (None, None, None);
        let (mut visible, mut locked, mut mask_hidden) = (None, None, None);
        let (mut name, mut element_ids, mut mask_linked) = (None, None, None);
        let mut effects = Vec::new();

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"GRTi" => self.slot_i64(child, path, i, &mut group_type)?,
                b"OBNs" => self.slot_str(child, path, i, &mut name)?,
                b"OPAi" => self.slot_i64(child, path, i, &mut opacity)?,
                b"BLDi" => self.slot_i64(child, path, i, &mut blend)?,
                b"VISb" => self.slot_bool(child, path, i, &mut visible)?,
                b"LCKb" => self.slot_bool(child, path, i, &mut locked)?,
                b"MNAb" => self.slot_bool(child, path, i, &mut mask_hidden)?,
                // The dictionary never writes LNKb `true`; the node is
                // present only as `false` (unlinked).
                b"LNKb" => self.slot_bool(child, path, i, &mut mask_linked)?,
                b"ELMv" => {
                    if element_ids.is_none() {
                        let kids = children_of(child, path)?;
                        let elmv_path = format!("{path}/ELMv");
                        element_ids = Some(self.read_elements(kids, &elmv_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                // DTAv / IMGv / MRMv / ISSv and anything else:
                // preserved (EFLv maps below).
                b"EFLv" => {
                    let eflv_path = format!("{path}/EFLv[{}]", effects.len());
                    effects.push(self.read_effect(child, &eflv_path)?);
                }
                _ => self.bag_node(path, i, child),
            }
        }

        Ok(ObjectData::Group(GroupObject {
            group_type: group_type.unwrap_or(0),
            name,
            children: element_ids.unwrap_or_default(),
            opacity: opacity.unwrap_or(1000),
            blend: blend.unwrap_or(0),
            visible: visible.unwrap_or(true),
            locked: locked.unwrap_or(false),
            mask_hidden: mask_hidden.unwrap_or(false),
            // `LNKb` is only ever written as `false`; a hostile `true`
            // canonicalizes to the absent (= linked) form so the model
            // never carries a value the writer cannot emit.
            mask_linked: (mask_linked == Some(false)).then_some(false),
            effects,
        }))
    }

    fn read_image(&mut self, children: &[Node], path: &str) -> Result<ObjectData, MapError> {
        let (mut xl, mut yl, mut xo, mut yo) = (None, None, None, None);
        let (mut wpx, mut hpx, mut tsz) = (None, None, None);
        let (mut opacity, mut blend, mut visible, mut locked, mut name) =
            (None, None, None, None, None);
        let mut tiles = Vec::new();
        let mut effects = Vec::new();

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"XLCf" => self.slot_f64(child, path, i, &mut xl)?,
                b"YLCf" => self.slot_f64(child, path, i, &mut yl)?,
                b"XOFf" => self.slot_f64(child, path, i, &mut xo)?,
                b"YOFf" => self.slot_f64(child, path, i, &mut yo)?,
                b"WPXi" => self.slot_i64(child, path, i, &mut wpx)?,
                b"HPXi" => self.slot_i64(child, path, i, &mut hpx)?,
                b"TSZi" => self.slot_i64(child, path, i, &mut tsz)?,
                b"OBNs" => self.slot_str(child, path, i, &mut name)?,
                b"OPAi" => self.slot_i64(child, path, i, &mut opacity)?,
                b"BLDi" => self.slot_i64(child, path, i, &mut blend)?,
                b"VISb" => self.slot_bool(child, path, i, &mut visible)?,
                b"LCKb" => self.slot_bool(child, path, i, &mut locked)?,
                b"TILv" => {
                    if tiles.is_empty() {
                        let kids = children_of(child, path)?;
                        let tilv_path = format!("{path}/TILv");
                        tiles = self.read_tiles(kids, &tilv_path)?;
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"EFLv" => {
                    let eflv_path = format!("{path}/EFLv[{}]", effects.len());
                    effects.push(self.read_effect(child, &eflv_path)?);
                }
                // DTAv / nested IMGv and anything else: preserved.
                _ => self.bag_node(path, i, child),
            }
        }

        Ok(ObjectData::Image(ImageObject {
            location: Point::new(xl.unwrap_or(0.0), yl.unwrap_or(0.0)),
            offset: Point::new(xo.unwrap_or(0.0), yo.unwrap_or(0.0)),
            pixels: (wpx.unwrap_or(0), hpx.unwrap_or(0)),
            tile_size: tsz.unwrap_or(256),
            tiles,
            name,
            opacity: opacity.unwrap_or(1000),
            blend: blend.unwrap_or(0),
            visible: visible.unwrap_or(true),
            locked: locked.unwrap_or(false),
            effects,
        }))
    }

    fn read_tiles(&mut self, children: &[Node], path: &str) -> Result<Vec<TileRef>, MapError> {
        let mut tiles = Vec::new();
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"TIDi" => match child.as_int() {
                    Some(value) if (0..=u32::MAX as i64).contains(&value) => {
                        tiles.push(TileRef::Tile(value as u32));
                    }
                    _ => return Err(bad_shape(child, path, "tile id in 0..=u32::MAX")),
                },
                b"TMCi" => match child.as_int() {
                    Some(value) => tiles.push(TileRef::Matte(value)),
                    None => return Err(bad_shape(child, path, "int")),
                },
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(tiles)
    }

    fn read_slice(&mut self, children: &[Node], path: &str) -> Result<ObjectData, MapError> {
        let (mut internal_name, mut name, mut slice_type) = (None, None, None);
        let (mut shape, mut fill_raw) = (None, None);
        let (mut left, mut top, mut right, mut bottom) = (None, None, None, None);
        let (mut link, mut alt, mut label, mut td_text) = (None, None, None, None);
        let mut contours: Vec<Contour> = Vec::new();
        let (mut cbt, mut cbr, mut cba, mut cbh, mut chv, mut cbv, mut cvv) =
            (None, None, None, None, None, None, None);
        let mut behaviors: Vec<SliceBehavior> = Vec::new();
        let mut custom_data = std::collections::BTreeMap::new();
        let mut export: Option<ExportOptions> = None;

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"INMs" => self.slot_str(child, path, i, &mut internal_name)?,
                b"OBNs" => self.slot_str(child, path, i, &mut name)?,
                b"TSLi" => self.slot_i64(child, path, i, &mut slice_type)?,
                b"URSi" => self.slot_i64(child, path, i, &mut shape)?,
                b"FCLi" => self.slot_i64(child, path, i, &mut fill_raw)?,
                b"LFTf" => self.slot_f64(child, path, i, &mut left)?,
                b"TOPf" => self.slot_f64(child, path, i, &mut top)?,
                b"RITf" => self.slot_f64(child, path, i, &mut right)?,
                b"BOTf" => self.slot_f64(child, path, i, &mut bottom)?,
                b"A2Ts" => self.slot_str(child, path, i, &mut link)?,
                b"ALTs" => self.slot_str(child, path, i, &mut alt)?,
                b"FILs" => self.slot_str(child, path, i, &mut label)?,
                b"TDTs" => self.slot_str(child, path, i, &mut td_text)?,
                b"PBPv" => {
                    // Polygon slice outlines — the path contour
                    // layout (`ISCb`/`PPCi`/`PBTv`).
                    let kids = children_of(child, path)?;
                    let pbpv_path = format!("{path}/PBPv[{i}]");
                    contours.push(self.read_contour(kids, &pbpv_path)?);
                }
                b"BEVv" => {
                    // Each `BEHv` contributes its `BETi`; the rest of
                    // the behavior subtree rides the bag under this
                    // `BEHv`'s own path so the writer re-emits it.
                    if behaviors.is_empty() {
                        let kids = children_of(child, path)?;
                        let bevv_path = format!("{path}/BEVv");
                        for (j, behv) in kids.iter().enumerate() {
                            let behv_path = format!("{bevv_path}/BEHv[{j}]");
                            if behv.fourcc != *b"BEHv" {
                                self.bag_node(&bevv_path, j, behv);
                                continue;
                            }
                            let behv_kids = children_of(behv, &behv_path)?;
                            match Self::read_behavior_kind(behv_kids, &behv_path) {
                                Some(kind) => {
                                    behaviors.push(SliceBehavior { kind });
                                    for (k, entry) in behv_kids.iter().enumerate() {
                                        if entry.fourcc != *b"BETi" {
                                            self.bag_node(&behv_path, k, entry);
                                        }
                                    }
                                }
                                // No readable `BETi`: preserve the
                                // whole behavior untouched.
                                None => self.bag_node(&bevv_path, j, behv),
                            }
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"DTAv" => {
                    if custom_data.is_empty() {
                        let kids = children_of(child, path)?;
                        match Self::read_custom_data(kids) {
                            Some(entries) => custom_data = entries,
                            None => self.bag_node(path, i, child),
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"CBTb" => self.slot_bool(child, path, i, &mut cbt)?,
                b"CBRi" => self.slot_i64(child, path, i, &mut cbr)?,
                b"CBAi" => self.slot_i64(child, path, i, &mut cba)?,
                b"CBHi" => self.slot_i64(child, path, i, &mut cbh)?,
                b"CHVi" => self.slot_i64(child, path, i, &mut chv)?,
                b"CBVi" => self.slot_i64(child, path, i, &mut cbv)?,
                b"CVVi" => self.slot_i64(child, path, i, &mut cvv)?,
                b"EXPv" => {
                    // The per-slice export override — the selective
                    // JPEG quality table's home. A slice's `FNLv` has
                    // no model home and stays preserved.
                    if export.is_some() {
                        self.bag_node(path, i, child);
                    } else {
                        let kids = children_of(child, path)?;
                        let expv_path = format!("{path}/EXPv");
                        let (options, _) = self.read_export(kids, &expv_path, false)?;
                        export = Some(options);
                    }
                }
                // MSNi and anything else: preserved.
                _ => self.bag_node(path, i, child),
            }
        }

        // The cell-background family is minted only when some member
        // was recorded — an absent family means no nodes on the wire.
        let has_background = cbt.is_some()
            || cbr.is_some()
            || cba.is_some()
            || cbh.is_some()
            || chv.is_some()
            || cbv.is_some()
            || cvv.is_some();
        let background = has_background.then_some(SliceBackground {
            is_background: cbt,
            repeat: cbr,
            attachment: cba,
            h_position: cbh,
            h_value: chv,
            v_position: cbv,
            v_value: cvv,
        });

        Ok(ObjectData::Slice(SliceObject {
            internal_name,
            frame: Rect {
                x0: left.unwrap_or(0.0),
                y0: top.unwrap_or(0.0),
                x1: right.unwrap_or(0.0),
                y1: bottom.unwrap_or(0.0),
            },
            shape: shape.unwrap_or(0),
            fill_raw: fill_raw.unwrap_or(0),
            name,
            slice_type,
            link,
            alt,
            label,
            td_text,
            contours,
            background,
            behaviors,
            custom_data,
            export,
        }))
    }

    /// The `BETi` of one `BEHv`, or `None` when absent/mis-shaped
    /// (the whole behavior then stays bagged).
    fn read_behavior_kind(children: &[Node], path: &str) -> Option<i64> {
        for child in children {
            if child.fourcc == *b"BETi" {
                return child.as_int();
            }
        }
        let _ = path;
        None
    }

    /// Returns `Ok(None)` when the instance cannot be modeled (absent or
    /// projective `MTXv`); the caller preserves the whole `ALSv`.
    fn read_symbol_instance(
        &mut self,
        children: &[Node],
        path: &str,
    ) -> Result<Option<ObjectData>, MapError> {
        let (mut number, mut locked, mut name) = (None, None, None);
        let mut transform: Option<Affine> = None;
        let mut custom_data = std::collections::BTreeMap::new();
        // The animation family: presence keyed on `ANNi`; the optional
        // tail (`ANRi`/`ASOi`/`AEOi`) only models when the anchor is
        // present, so a stray tail node rides the bag.
        let mut frames = None;
        let (mut x_shift, mut y_shift, mut scaling) = (None, None, None);
        let (mut rotation, mut start_opacity, mut end_opacity) = (None, None, None);

        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"MSNi" => self.slot_i64(child, path, i, &mut number)?,
                b"LCKb" => self.slot_bool(child, path, i, &mut locked)?,
                b"OBNs" => self.slot_str(child, path, i, &mut name)?,
                b"DTAv" => {
                    if custom_data.is_empty() {
                        let kids = children_of(child, path)?;
                        match Self::read_custom_data(kids) {
                            Some(entries) => custom_data = entries,
                            None => self.bag_node(path, i, child),
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                b"ANNi" => self.slot_i64(child, path, i, &mut frames)?,
                b"ANXf" => self.slot_f64(child, path, i, &mut x_shift)?,
                b"ANYf" => self.slot_f64(child, path, i, &mut y_shift)?,
                b"ASCf" => self.slot_f64(child, path, i, &mut scaling)?,
                b"ANRi" => self.slot_i64(child, path, i, &mut rotation)?,
                b"ASOi" => self.slot_i64(child, path, i, &mut start_opacity)?,
                b"AEOi" => self.slot_i64(child, path, i, &mut end_opacity)?,
                b"MTXv" => {
                    if transform.is_none() {
                        let mtxv_path = format!("{path}/MTXv");
                        match self.read_matrix(child, &mtxv_path)? {
                            Some(affine) => transform = Some(affine),
                            // The instance transform is not optional in the
                            // model: preserve the whole ALSv.
                            None => return Ok(None),
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                // XFMi / DTAv / ATMv / ALNi / nested IMGv and anything
                // else: preserved.
                _ => self.bag_node(path, i, child),
            }
        }

        let transform = match transform {
            Some(affine) => affine,
            None => return Ok(None),
        };
        let animation = frames.map(|frames| SymbolAnimation {
            frames,
            x_shift: x_shift.unwrap_or(0.0),
            y_shift: y_shift.unwrap_or(0.0),
            scaling: scaling.unwrap_or(100.0),
            rotation,
            start_opacity,
            end_opacity,
        });
        Ok(Some(ObjectData::SymbolInstance(SymbolInstanceObject {
            symbol_number: number.unwrap_or(0),
            transform,
            locked: locked.unwrap_or(false),
            name,
            animation,
            custom_data,
        })))
    }

    /// `EXPv` children → ([`ExportOptions`], frame durations).
    /// Dictionary-documented children slot into the struct (first
    /// occurrence wins, duplicates bag); everything else — the HTML
    /// family, `PALv`, `MCTi`, export-area `LR*i`, scale, color mode —
    /// is preserved verbatim under `path`, the `EXPv` container's own
    /// path. Frame configurations (`FNLv`) map positionally only when
    /// `map_frames` is set (the page-level call, where `PageInfo::
    /// frame_durations` is the home); the returned vec is `None` when
    /// no mapped `FNLv` was found or its mapping had to stay preserved.
    ///
    /// # Errors
    /// [`MapError::BadShape`] for known children carrying the wrong
    /// payload kind, or an `FNLv` whose configurations are not
    /// children lists.
    fn read_export(
        &mut self,
        children: &[Node],
        path: &str,
        map_frames: bool,
    ) -> Result<(ExportOptions, Option<Vec<i64>>), MapError> {
        let mut options = ExportOptions::default();
        let mut frames: Option<Vec<i64>> = None;
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"FMTi" => self.slot_i64(child, path, i, &mut options.format)?,
                b"FCLi" => self.slot_i64(child, path, i, &mut options.matte_raw)?,
                b"JPQi" => self.slot_i64(child, path, i, &mut options.jpeg_quality)?,
                b"JPSi" => self.slot_i64(child, path, i, &mut options.jpeg_smoothing)?,
                b"JPOi" => self.slot_i64(child, path, i, &mut options.jpeg_colour)?,
                b"JPPb" => self.slot_bool(child, path, i, &mut options.jpeg_progressive)?,
                b"GFIb" => self.slot_bool(child, path, i, &mut options.interlaced)?,
                b"GLTi" => self.slot_i64(child, path, i, &mut options.gif_loss)?,
                b"PLTi" => self.slot_i64(child, path, i, &mut options.palette_type)?,
                b"PLDi" => self.slot_i64(child, path, i, &mut options.dither)?,
                b"PLOb" => self.slot_bool(child, path, i, &mut options.palette_pruning)?,
                b"PTNi" => self.slot_i64(child, path, i, &mut options.transparency_type)?,
                b"FLPi" => self.slot_i64(child, path, i, &mut options.frame_loop)?,
                b"AACb" => self.slot_bool(child, path, i, &mut options.anim_crop)?,
                b"AADb" => self.slot_bool(child, path, i, &mut options.anim_diff)?,
                b"FNLv" => {
                    // Frame configurations map only where the model has
                    // a home for them (the page), and only once.
                    if map_frames && frames.is_none() {
                        let kids = children_of(child, path)?;
                        let fnlv_path = format!("{path}/FNLv");
                        match self.read_frame_configs(kids, &fnlv_path)? {
                            Some(values) => frames = Some(values),
                            None => {
                                // A configuration without a duration
                                // would shift the positional mapping;
                                // the partial slots are subsumed by the
                                // preserved whole node.
                                self.bag.purge_under(&fnlv_path);
                                self.bag_node(path, i, child);
                            }
                        }
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                _ => self.bag_node(path, i, child),
            }
        }
        Ok((options, frames))
    }

    /// `FNLv` children (`FNMv`* frame configurations) → per-frame
    /// durations, positionally. `FNMv` children other than `FNDi`
    /// (`FNPi`, `VISb`, `OBNs`) are bagged under their configuration's
    /// path. Returns `None` when any configuration carries no `FNDi` —
    /// a hole would shift every later frame — and the caller preserves
    /// the whole `FNLv` instead.
    ///
    /// # Errors
    /// [`MapError::BadShape`] for a configuration that is not a
    /// children list.
    fn read_frame_configs(
        &mut self,
        children: &[Node],
        path: &str,
    ) -> Result<Option<Vec<i64>>, MapError> {
        let mut frames: Vec<Option<i64>> = Vec::new();
        for (i, child) in children.iter().enumerate() {
            if &child.fourcc != b"FNMv" {
                self.bag_node(path, i, child);
                continue;
            }
            let fnmv_path = format!("{path}/FNMv[{i}]");
            let kids = children_of(child, &fnmv_path)?;
            let mut duration = None;
            for (j, cfg) in kids.iter().enumerate() {
                if &cfg.fourcc == b"FNDi" {
                    self.slot_i64(cfg, &fnmv_path, j, &mut duration)?;
                } else {
                    self.bag_node(&fnmv_path, j, cfg);
                }
            }
            frames.push(duration);
        }
        let mut out = Vec::with_capacity(frames.len());
        for duration in frames {
            match duration {
                Some(value) => out.push(value),
                None => return Ok(None),
            }
        }
        Ok(Some(out))
    }
}

/// Option slots gathered while walking one `ZSTv`'s text block, before
/// they assemble into a [`StyleText`]. A plain struct of `Option`s —
/// the record spine fixes every slot's type, so defaults only cover
/// malformed records that omit slots.
#[derive(Default)]
struct StyleTextSlots {
    margin_left: Option<i64>,
    margin_top: Option<i64>,
    margin_right: Option<i64>,
    margin_bottom: Option<i64>,
    font: Option<String>,
    font_category: Option<String>,
    size: Option<f64>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    justification: Option<i64>,
    horizontal_scale: Option<f64>,
    kerning: Option<f64>,
    leading: Option<f64>,
    leading_unit: Option<i64>,
    auto_kern: Option<bool>,
    anti_alias_enabled: Option<bool>,
    anti_alias: Option<i64>,
    attr_1: Option<i64>,
    attr_2: Option<i64>,
    tab_size: Option<i64>,
}

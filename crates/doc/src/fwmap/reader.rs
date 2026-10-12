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
    Canvas, Cell, Contour, Document, DocumentInfo, Effect, EffectProperty, EffectValue,
    ExportOptions, Grid, GroupObject, Guides, ImageObject, Layer, LayerList, NineSliceGuides, Page,
    PageInfo, PaintStyle, PathObject, PathPoint, PrimitiveSource, SliceObject, SymbolAnimation,
    SymbolDefinition, SymbolInstanceObject, SymbolType, TextObject, TextOnPath, TextRun, TextStyle,
    TileRef,
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

    /// `JSVv` children → script source: the concatenated `JSSs`
    /// string pieces (the format's answer to the 65535-byte string
    /// limit). `None` leaves the whole node to the bag — a `JSVv`
    /// carrying anything but `JSSs` strings is not ours to interpret.
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
                // LSAv / MPLv / anything else: preserved verbatim.
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
        })
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

    /// Reads a `PATv` paint: the two solid colours are modeled; the
    /// gradient/pattern subtrees stay in the bag under the `PATv` path.
    fn read_paint(&mut self, children: &[Node], path: &str) -> Result<PaintStyle, MapError> {
        let (mut brush, mut fill) = (None, None);
        for (i, child) in children.iter().enumerate() {
            match &child.fourcc {
                b"BCLi" => self.slot_i64(child, path, i, &mut brush)?,
                b"FCLi" => self.slot_i64(child, path, i, &mut fill)?,
                _ => self.bag_node(path, i, child),
            }
        }
        Ok(PaintStyle {
            stroke_raw: brush.unwrap_or(0),
            fill_raw: fill.unwrap_or(0),
        })
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
        let (mut name, mut element_ids) = (None, None);
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
                b"ELMv" => {
                    if element_ids.is_none() {
                        let kids = children_of(child, path)?;
                        let elmv_path = format!("{path}/ELMv");
                        element_ids = Some(self.read_elements(kids, &elmv_path)?);
                    } else {
                        self.bag_node(path, i, child);
                    }
                }
                // DTAv / IMGv / MRMv / LNKb / ISSv and anything else:
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
                // A2Ts / ALTs / PBPv / DTAv / BEVv / MSNi and
                // anything else: preserved.
                _ => self.bag_node(path, i, child),
            }
        }

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
            export,
        }))
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

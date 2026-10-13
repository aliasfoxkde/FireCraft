//! [`Document`] + [`UnknownBag`] → node tree, canonical emission.
//!
//! Emission order per container follows the vendored node dictionary
//! (`docs/research/fireworks-png-nodes.md`); bagged nodes are spliced
//! back at their recorded child indices, so trees that were canonical on
//! read re-serialize byte-identically. Nodes the dictionary marks as
//! "never present as `false`" (`MNAb`) are emitted only when set.
//! Emission always writes non-`Option` fields: reading a file that
//! *omits* an optional node defaults it in the model, so its write-back
//! canonicalizes rather than reproduces the omission.

use crate::fwmap::unknown::UnknownBag;
use crate::fwmap::{FourCC, MapError};
use crate::model::{
    Cell, Contour, Document, Effect, EffectProperty, EffectValue, ExportOptions, Fill, FillShape,
    Gradient, GradientStop, Grid, GroupObject, Guides, ImageObject, Layer, LayerList,
    NineSliceGuides, PaintStyle, PathObject, PathPoint, PrimitiveSource, SliceObject,
    SymbolDefinition, SymbolInstanceObject, TextObject, TextOnPath, TextRun, TextStyle, TileRef,
};
use crate::registry::{ObjectData, ObjectId};
use fwpng::nodetree::{Node, NodeValue};
use geom::{Affine, Rect};

/// Side-channel of a write: how many bagged nodes had no container to be
/// spliced back into (non-zero only for inputs whose structure the
/// model-side write cannot visit — e.g. an `MSLv` whose children are all
/// non-`MSTv`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WriteStats {
    /// Preserved nodes dropped for lack of a host container.
    pub unconsumed_unknown: usize,
}

/// Writes a document plus its preserved-node bag back into a node tree.
pub fn write(
    document: &Document,
    unknown: &UnknownBag,
) -> Result<(Vec<Node>, WriteStats), MapError> {
    let mut writer = Writer {
        bag: unknown.clone(),
        doc: document,
    };
    let mut mapped = Vec::new();
    mapped.push(writer.write_document()?);
    for page_index in 0..document.pages.len() {
        mapped.push(writer.write_page(page_index)?);
    }
    let stats = WriteStats {
        unconsumed_unknown: writer.bag.len(),
    };
    let bagged = writer.bag.take_under("");
    Ok((splice(mapped, bagged), stats))
}

fn n_i(fourcc: FourCC, value: i64) -> Node {
    Node {
        fourcc,
        value: NodeValue::Int(value),
    }
}

fn n_f(fourcc: FourCC, value: f64) -> Node {
    Node {
        fourcc,
        value: NodeValue::Float(value),
    }
}

fn n_b(fourcc: FourCC, value: bool) -> Node {
    Node {
        fourcc,
        value: NodeValue::Bool(value),
    }
}

fn n_s(fourcc: FourCC, value: &str) -> Node {
    Node {
        fourcc,
        value: NodeValue::Str(value.to_owned()),
    }
}

fn n_kids(fourcc: FourCC, children: Vec<Node>) -> Node {
    Node {
        fourcc,
        value: NodeValue::Children(children),
    }
}

/// One `DCEv` property: `DCKs` key, then exactly one value node —
/// text properties become `DCVs`; raw values are the preserved node
/// verbatim (grab-point lists, nested dictionaries).
fn effect_property_node(prop: &EffectProperty) -> Node {
    let value = match &prop.value {
        EffectValue::Text(text) => n_s(*b"DCVs", text),
        EffectValue::Raw(node) => node.clone(),
    };
    n_kids(*b"DCEv", vec![n_s(*b"DCKs", &prop.key), value])
}

/// Merges mapped children with bagged ones: bagged nodes land at their
/// recorded indices, mapped children fill the remaining slots in
/// emission order. Bagged entries whose index cannot be honored (input
/// tree was not canonically ordered) are appended — content stays
/// lossless even when position does not.
/// Inverse of the reader's `fill_shape`: the file value for a model
/// shape. `Contour` writes `17` whichever slot the file uses (the
/// compatibility `FSHi` enumeration has no 17 — a file that decodes
/// to `Contour` always carried `FSXi`, and the writer honours that
/// via `Fill::shape_extended`).
fn shape_raw(shape: FillShape) -> i64 {
    match shape {
        FillShape::Solid => 0,
        FillShape::Linear => 2,
        FillShape::Radial => 3,
        FillShape::Conical => 4,
        FillShape::Satin => 5,
        FillShape::Pinch => 7,
        FillShape::Folds => 8,
        FillShape::Elliptical => 9,
        FillShape::Rectangular => 10,
        FillShape::Bars => 12,
        FillShape::Ripple => 13,
        FillShape::Waves => 14,
        FillShape::Pattern => 15,
        FillShape::WebDither => 16,
        FillShape::Contour => 17,
    }
}

pub(crate) fn splice(mapped: Vec<Node>, bagged: Vec<(usize, Node)>) -> Vec<Node> {
    let mut out = Vec::with_capacity(mapped.len() + bagged.len());
    let mut mapped = mapped.into_iter();
    let mut bag_at = 0usize;
    let total = out.capacity();
    for i in 0..total {
        if bag_at < bagged.len() && bagged[bag_at].0 == i {
            out.push(bagged[bag_at].1.clone());
            bag_at += 1;
        } else if let Some(node) = mapped.next() {
            out.push(node);
        } else {
            break;
        }
    }
    out.extend(mapped);
    while bag_at < bagged.len() {
        out.push(bagged[bag_at].1.clone());
        bag_at += 1;
    }
    out
}

/// Original child indices of `mapped_len` mapped children of a container
/// whose bagged entries are `bagged` — the complement of the bagged
/// indices in `0..(mapped_len + bagged.len())`.
pub(crate) fn mapped_indices(bagged: &[(usize, Node)], mapped_len: usize) -> Vec<usize> {
    let total = mapped_len + bagged.len();
    let mut out = Vec::with_capacity(mapped_len);
    let mut bag_at = 0usize;
    for i in 0..total {
        if out.len() == mapped_len {
            break;
        }
        if bag_at < bagged.len() && bagged[bag_at].0 == i {
            bag_at += 1;
        } else {
            out.push(i);
        }
    }
    out
}

struct Writer<'a> {
    bag: UnknownBag,
    doc: &'a Document,
}

impl Writer<'_> {
    fn splice_container(&mut self, mapped: Vec<Node>, path: &str) -> Vec<Node> {
        let bagged = self.bag.take_under(path);
        splice(mapped, bagged)
    }

    /// Document node (`MKBv`). Dictionary order: version scalars, frame
    /// count, canvas, `MSLv`, `PGNs`, `LYLv`, grid, guides, onion
    /// skinning, `EXPv`. Definition lists and the page scalars have no
    /// document-level model slot — they come back from the bag.
    fn write_document(&mut self) -> Result<Node, MapError> {
        let doc = self.doc;
        let path = "/MKBv";
        let info = &doc.info;
        let canvas = &info.canvas;
        let mut mapped = Vec::new();
        if let Some(version) = info.version {
            mapped.push(n_i(*b"VRSi", version));
        }
        if let Some(revision) = info.revision {
            mapped.push(n_i(*b"VRMi", revision));
        }
        mapped.push(n_i(*b"FRCi", info.frame_count));
        mapped.push(n_i(*b"RQNi", info.request_id));
        mapped.push(n_f(*b"XLCf", canvas.x));
        mapped.push(n_f(*b"YLCf", canvas.y));
        mapped.push(n_f(*b"WIDf", canvas.width));
        mapped.push(n_f(*b"HITf", canvas.height));
        mapped.push(n_f(*b"RESf", canvas.resolution));
        mapped.push(n_i(*b"RSUi", canvas.resolution_unit));
        mapped.push(n_i(*b"BGCi", canvas.background_raw));
        if !doc.symbols.is_empty() {
            mapped.push(self.write_symbol_list(path)?);
        }
        if let Some(name) = &info.name {
            mapped.push(n_s(*b"PGNs", name));
        }
        mapped.push(self.write_layer_list(&doc.shared_layers, path)?);
        mapped.push(self.write_grid(&doc.grid, path)?);
        mapped.push(self.write_guides(&doc.guides, path)?);
        mapped.push(n_i(*b"OSBi", info.onion_before));
        mapped.push(n_i(*b"OSAi", info.onion_after));
        if let Some(export) = &doc.export {
            // The document's `EXPv` carries export defaults only; its
            // `FNLv` has no model home here, so no frames are emitted
            // and any bagged ones splice back verbatim.
            let expv_path = format!("{path}/EXPv");
            mapped.push(self.write_export_options(export, &expv_path, &[]));
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"MKBv", children))
    }

    /// Page node (`PDCv`). Like the document body but with the frame
    /// count twice, page scalars, and `GRDv` last (dictionary order).
    fn write_page(&mut self, page_index: usize) -> Result<Node, MapError> {
        let doc = self.doc;
        let page = &doc.pages[page_index];
        let path = format!("/PDCv[{page_index}]");
        let info = &page.info;
        let canvas = &info.canvas;
        let mut mapped = Vec::new();
        if let Some(version) = info.version {
            mapped.push(n_i(*b"VRSi", version));
        }
        if let Some(revision) = info.revision {
            mapped.push(n_i(*b"VRMi", revision));
        }
        mapped.push(n_i(*b"FRCi", info.frame_count));
        mapped.push(n_i(*b"FRCi", info.frame_count_duplicate));
        mapped.push(n_i(*b"RQNi", info.request_id));
        mapped.push(n_f(*b"XLCf", canvas.x));
        mapped.push(n_f(*b"YLCf", canvas.y));
        mapped.push(n_f(*b"WIDf", canvas.width));
        mapped.push(n_f(*b"HITf", canvas.height));
        mapped.push(n_f(*b"RESf", canvas.resolution));
        mapped.push(n_i(*b"RSUi", canvas.resolution_unit));
        mapped.push(n_i(*b"BGCi", canvas.background_raw));
        if let Some(name) = &page.name {
            mapped.push(n_s(*b"PGNs", name));
        }
        mapped.push(n_b(*b"IMPb", page.is_master));
        mapped.push(n_b(*b"PILb", page.is_linked));
        if let Some(page_id) = &page.page_id {
            mapped.push(n_s(*b"PIDs", page_id));
        }
        mapped.push(self.write_layer_list(&page.layers, &path)?);
        mapped.push(self.write_guides(&page.guides, &path)?);
        mapped.push(n_i(*b"OSBi", info.onion_before));
        mapped.push(n_i(*b"OSAi", info.onion_after));
        if let Some(dir) = &page.last_export_dir {
            mapped.push(n_s(*b"LXDs", dir));
        }
        if let Some(file) = &page.last_export_file {
            mapped.push(n_s(*b"LXFs", file));
        }
        if let Some(grid) = &page.grid {
            mapped.push(self.write_grid(grid, &path)?);
        }
        // The page's `EXPv` carries the settings *and* the `FNLv` frame
        // configurations — the on-disk home of `frame_durations` — so
        // it is minted even around an empty settings record when the
        // document has timing to persist.
        if page.export.is_some() || !info.frame_durations.is_empty() {
            let default = ExportOptions::default();
            let export = page.export.as_ref().unwrap_or(&default);
            let expv_path = format!("{path}/EXPv");
            mapped.push(self.write_export_options(export, &expv_path, &info.frame_durations));
        }
        let children = self.splice_container(mapped, &path);
        Ok(n_kids(*b"PDCv", children))
    }

    /// `EXPv` node: mapped children in dictionary order, then the
    /// preserved unknowns at their recorded indices. `frames` non-empty
    /// appends an `FNLv` of one `FNMv` per entry (`FNDi` duration).
    fn write_export_options(&mut self, export: &ExportOptions, path: &str, frames: &[i64]) -> Node {
        let mut mapped: Vec<Node> = Vec::new();
        if let Some(value) = export.format {
            mapped.push(n_i(*b"FMTi", value));
        }
        if let Some(value) = export.matte_raw {
            mapped.push(n_i(*b"FCLi", value));
        }
        if let Some(value) = export.jpeg_quality {
            mapped.push(n_i(*b"JPQi", value));
        }
        if let Some(value) = export.jpeg_smoothing {
            mapped.push(n_i(*b"JPSi", value));
        }
        if let Some(value) = export.jpeg_colour {
            mapped.push(n_i(*b"JPOi", value));
        }
        if let Some(value) = export.jpeg_progressive {
            mapped.push(n_b(*b"JPPb", value));
        }
        if let Some(value) = export.interlaced {
            mapped.push(n_b(*b"GFIb", value));
        }
        if let Some(value) = export.gif_loss {
            mapped.push(n_i(*b"GLTi", value));
        }
        if let Some(value) = export.palette_type {
            mapped.push(n_i(*b"PLTi", value));
        }
        if let Some(value) = export.dither {
            mapped.push(n_i(*b"PLDi", value));
        }
        if let Some(value) = export.palette_pruning {
            mapped.push(n_b(*b"PLOb", value));
        }
        if let Some(value) = export.transparency_type {
            mapped.push(n_i(*b"PTNi", value));
        }
        if let Some(value) = export.anim_crop {
            mapped.push(n_b(*b"AACb", value));
        }
        if let Some(value) = export.anim_diff {
            mapped.push(n_b(*b"AADb", value));
        }
        if let Some(value) = export.frame_loop {
            mapped.push(n_i(*b"FLPi", value));
        }
        if !frames.is_empty() {
            let fnlv_path = format!("{path}/FNLv");
            mapped.push(self.write_frame_configs(frames, &fnlv_path));
        }
        let children = self.splice_container(mapped, path);
        n_kids(*b"EXPv", children)
    }

    /// `FNLv` node: one `FNMv` per recorded duration, each carrying its
    /// `FNDi` plus whatever configuration scalars the source file kept.
    /// Configurations past the recorded count have no host frame — the
    /// model wins on count.
    fn write_frame_configs(&mut self, frames: &[i64], fnlv_path: &str) -> Node {
        let fnmv_prefix = format!("{fnlv_path}/FNMv[");
        self.bag.purge_prefix(&fnmv_prefix);
        let mut mapped = Vec::with_capacity(frames.len());
        for (i, duration) in frames.iter().enumerate() {
            let fnmv_path = format!("{fnlv_path}/FNMv[{i}]");
            let kids = self.splice_container(vec![n_i(*b"FNDi", *duration)], &fnmv_path);
            mapped.push(n_kids(*b"FNMv", kids));
        }
        let children = self.splice_container(mapped, fnlv_path);
        n_kids(*b"FNLv", children)
    }

    fn write_symbol_list(&mut self, parent_path: &str) -> Result<Node, MapError> {
        let doc = self.doc;
        let path = format!("{parent_path}/MSLv");
        let bagged = self.bag.take_under(&path);
        let indices = mapped_indices(&bagged, doc.symbols.len());
        let mut mapped = Vec::with_capacity(doc.symbols.len());
        for (i, symbol) in doc.symbols.iter().enumerate() {
            let mstv_path = format!("{path}/MSTv[{}]", indices[i]);
            mapped.push(self.write_symbol_definition(symbol, &mstv_path)?);
        }
        Ok(n_kids(*b"MSLv", splice(mapped, bagged)))
    }

    fn write_symbol_definition(
        &mut self,
        symbol: &SymbolDefinition,
        path: &str,
    ) -> Result<Node, MapError> {
        // Dictionary order: UNMs MSNi MOXf MOYf … LYLv … FRCi … BGCi …
        // MSYi? NGSv? — graphic symbols omit `MSYi` rather than
        // storing a zero.
        let mut mapped = vec![
            n_s(*b"UNMs", &symbol.name),
            n_i(*b"MSNi", symbol.symbol_number),
            n_f(*b"MOXf", symbol.origin.x),
            n_f(*b"MOYf", symbol.origin.y),
        ];
        mapped.push(self.write_layer_list(&symbol.layers, path)?);
        mapped.push(n_i(*b"FRCi", symbol.frame_count));
        mapped.push(n_i(*b"BGCi", symbol.background_raw));
        if let Some(raw) = symbol.symbol_type.raw() {
            mapped.push(n_i(*b"MSYi", raw));
        }
        if let Some(guides) = &symbol.nine_slice {
            let ngsv_path = format!("{path}/NGSv");
            mapped.push(self.write_nine_slice(guides, &ngsv_path)?);
        }
        if let Some(source) = &symbol.op_code {
            mapped.push(Self::write_script(source));
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"MSTv", children))
    }

    /// `JSVv` — the rich-symbol script: `JSSs` string pieces, split
    /// at the format's 65535-byte string limit on char boundaries (a
    /// byte chunk could cleave a UTF-8 character).
    fn write_script(source: &str) -> Node {
        let mut pieces: Vec<&str> = Vec::new();
        let mut start = 0usize;
        for (i, ch) in source.char_indices() {
            if i - start + ch.len_utf8() > 65_535 {
                pieces.push(&source[start..i]);
                start = i;
            }
        }
        pieces.push(&source[start..]);
        if pieces.is_empty() {
            pieces.push("");
        }
        let kids = pieces
            .into_iter()
            .map(|piece| n_s(*b"JSSs", piece))
            .collect();
        n_kids(*b"JSVv", kids)
    }

    /// `NGSv` — dictionary order `NGLb NGKi NGCf`×4 `NGMv NGVv`.
    fn write_nine_slice(&mut self, guides: &NineSliceGuides, path: &str) -> Result<Node, MapError> {
        let mut mapped = vec![
            n_b(*b"NGLb", guides.locked),
            n_i(*b"NGKi", guides.color_raw),
        ];
        for value in guides.controls {
            mapped.push(n_f(*b"NGCf", value));
        }
        mapped.push(self.write_bounds(&guides.shape_bounds, format!("{path}/NGMv"), *b"NGMv"));
        mapped.push(self.write_bounds(&guides.limits, format!("{path}/NGVv"), *b"NGVv"));
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"NGSv", children))
    }

    /// `NGMv`/`NGVv` — `LFTf TOPf RITf BOTf` bounds.
    fn write_bounds(&mut self, rect: &Rect, path: String, fourcc: FourCC) -> Node {
        let mapped = vec![
            n_f(*b"LFTf", rect.x0),
            n_f(*b"TOPf", rect.y0),
            n_f(*b"RITf", rect.x1),
            n_f(*b"BOTf", rect.y1),
        ];
        let children = self.splice_container(mapped, &path);
        n_kids(fourcc, children)
    }

    fn write_layer_list(&mut self, list: &LayerList, parent_path: &str) -> Result<Node, MapError> {
        let path = format!("{parent_path}/LYLv");
        let bagged = self.bag.take_under(&path);
        let indices = mapped_indices(&bagged, list.layers.len());
        let mut mapped = Vec::with_capacity(list.layers.len());
        for (i, layer) in list.layers.iter().enumerate() {
            let layv_path = format!("{path}/LAYv[{}]", indices[i]);
            mapped.push(self.write_layer(layer, &layv_path)?);
        }
        Ok(n_kids(*b"LYLv", splice(mapped, bagged)))
    }

    fn write_layer(&mut self, layer: &Layer, path: &str) -> Result<Node, MapError> {
        // Dictionary order: BKGb URLb LNMs LLLi? MSLb MCLb DISb CLLv VIFv.
        let mut mapped = vec![
            n_b(*b"BKGb", layer.is_background),
            n_b(*b"URLb", layer.has_url),
            n_s(*b"LNMs", &layer.name),
        ];
        if let Some(level) = layer.nesting_level {
            mapped.push(n_i(*b"LLLi", level));
        }
        mapped.push(n_b(*b"MSLb", layer.is_special));
        mapped.push(n_b(*b"MCLb", layer.frame_changing));
        mapped.push(n_b(*b"DISb", layer.display));
        mapped.push(self.write_cell_list(&layer.cells, path)?);
        if let Some(flags) = &layer.visibility_per_frame {
            let kids = flags.iter().map(|flag| n_b(*b"VISb", *flag)).collect();
            mapped.push(n_kids(*b"VIFv", kids));
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"LAYv", children))
    }

    fn write_cell_list(&mut self, cells: &[Cell], parent_path: &str) -> Result<Node, MapError> {
        let path = format!("{parent_path}/CLLv");
        let bagged = self.bag.take_under(&path);
        let indices = mapped_indices(&bagged, cells.len());
        let mut mapped = Vec::with_capacity(cells.len());
        for (i, cell) in cells.iter().enumerate() {
            let celv_path = format!("{path}/CELv[{}]", indices[i]);
            mapped.push(self.write_cell(cell, &celv_path)?);
        }
        Ok(n_kids(*b"CLLv", splice(mapped, bagged)))
    }

    fn write_cell(&mut self, cell: &Cell, path: &str) -> Result<Node, MapError> {
        // Dictionary order: LCKb ELMv VISb.
        let elements = self.write_elements(&cell.elements, path)?;
        let mapped = vec![
            n_b(*b"LCKb", cell.locked),
            elements,
            n_b(*b"VISb", cell.visible),
        ];
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"CELv", children))
    }

    /// Object list. Objects keep their original child index in their bag
    /// path, since canonical `ELMv` children interleave with bagged
    /// sub-layers (`LSLv`).
    fn write_elements(&mut self, ids: &[ObjectId], parent_path: &str) -> Result<Node, MapError> {
        let path = format!("{parent_path}/ELMv");
        let bagged = self.bag.take_under(&path);
        let indices = mapped_indices(&bagged, ids.len());
        let mut mapped = Vec::with_capacity(ids.len());
        for (i, id) in ids.iter().enumerate() {
            let object_path = format!("{path}[{}]", indices[i]);
            mapped.push(self.write_object(*id, &object_path)?);
        }
        Ok(n_kids(*b"ELMv", splice(mapped, bagged)))
    }

    /// Exhaustive over [`ObjectData`]: adding a variant without a
    /// mapping fails to compile (M3 decision 4).
    fn write_object(&mut self, id: ObjectId, path: &str) -> Result<Node, MapError> {
        let doc = self.doc;
        let object = doc.registry.get(id).ok_or(MapError::DanglingId {
            index: id.index() as u32,
            generation: id.generation(),
        })?;
        match object {
            ObjectData::Path(object) => self.write_path_object(object, path),
            ObjectData::Text(object) => self.write_text_object(object, path),
            ObjectData::Group(object) => self.write_group_object(object, path),
            ObjectData::Image(object) => self.write_image_object(object, path),
            ObjectData::Slice(object) => self.write_slice_object(object, path),
            ObjectData::SymbolInstance(object) => self.write_symbol_instance_object(object, path),
        }
    }

    fn write_path_object(&mut self, object: &PathObject, path: &str) -> Result<Node, MapError> {
        // Dictionary order: LCKb? PATv BRPi PTTi? FOTb RNDi? TOXf/TOYf?
        // PCFb? PSXf PSYf PEXf PEYf PFXf PFYf PBLv PCLv? DTAv? EOFb?
        // AGMb? IMGv? OBNs? BLDi? PRIv? EFLv? VISb? OPAi?. PTTi, RNDi,
        // texture/seed/contour and effect nodes have no model slot and
        // splice back from the bag between the mapped children.
        let mut mapped = Vec::new();
        if object.locked {
            mapped.push(n_b(*b"LCKb", true));
        }
        if let Some(paint) = &object.paint {
            mapped.push(self.write_paint(paint, &format!("{path}/PATv"))?);
        }
        if let Some(placement) = object.brush_placement {
            mapped.push(n_i(*b"BRPi", placement));
        }
        if let Some(on_top) = object.fill_on_top {
            mapped.push(n_b(*b"FOTb", on_top));
        }
        if let Some(point) = &object.fill_start {
            mapped.push(n_f(*b"PSXf", point.x));
            mapped.push(n_f(*b"PSYf", point.y));
        }
        if let Some(point) = &object.fill_end {
            mapped.push(n_f(*b"PEXf", point.x));
            mapped.push(n_f(*b"PEYf", point.y));
        }
        if let Some(point) = &object.fill_third {
            mapped.push(n_f(*b"PFXf", point.x));
            mapped.push(n_f(*b"PFYf", point.y));
        }
        mapped.push(self.write_contours(&object.contours, path)?);
        if object.even_odd {
            mapped.push(n_b(*b"EOFb", true));
        }
        if let Some(name) = &object.name {
            mapped.push(n_s(*b"OBNs", name));
        }
        mapped.push(n_i(*b"BLDi", object.blend));
        if let Some(primitive) = &object.primitive {
            mapped.push(self.write_primitive(primitive, path)?);
        }
        mapped.extend(self.write_effects(&object.effects, path)?);
        mapped.push(n_b(*b"VISb", object.visible));
        mapped.push(n_i(*b"OPAi", object.opacity));
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"PTHv", children))
    }

    /// Effect lists (`EFLv`+), one node per model effect. Bagged
    /// siblings handled by each object writer's `splice_container`;
    /// the `[i]` suffix matches the reader's path convention (both
    /// count mapped effects in file order).
    fn write_effects(
        &mut self,
        effects: &[Effect],
        parent_path: &str,
    ) -> Result<Vec<Node>, MapError> {
        let mut out = Vec::with_capacity(effects.len());
        for (i, effect) in effects.iter().enumerate() {
            let path = format!("{parent_path}/EFLv[{i}]");
            out.push(self.write_effect(effect, &path)?);
        }
        Ok(out)
    }

    /// One `EFLv`: `CATs`/`INMs` when present, then the first `EPSv`.
    /// Empty names are skipped rather than emitted (a name that arrived
    /// bagged — e.g. a non-string `CATs` — splices back from the bag).
    fn write_effect(&mut self, effect: &Effect, path: &str) -> Result<Node, MapError> {
        let mut mapped = Vec::new();
        if !effect.category.is_empty() {
            mapped.push(n_s(*b"CATs", &effect.category));
        }
        if !effect.internal_name.is_empty() {
            mapped.push(n_s(*b"INMs", &effect.internal_name));
        }
        if !effect.properties.is_empty() {
            let epsv_path = format!("{path}/EPSv");
            let bagged = self.bag.take_under(&epsv_path);
            let props = effect
                .properties
                .iter()
                .map(effect_property_node)
                .collect::<Vec<_>>();
            mapped.push(n_kids(*b"EPSv", splice(props, bagged)));
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"EFLv", children))
    }

    /// `PATv` children in dictionary order: BCLi FCLi, then the
    /// modeled `FPLv` fill; the texture subtrees (`TXBv` `TXFv`
    /// `BPLv`) splice back from the bag.
    fn write_paint(&mut self, paint: &PaintStyle, path: &str) -> Result<Node, MapError> {
        let mut mapped = vec![
            n_i(*b"BCLi", paint.stroke_raw),
            n_i(*b"FCLi", paint.fill_raw),
        ];
        if let Some(fill) = &paint.fill {
            let fplv_path = format!("{path}/FPLv");
            mapped.push(self.write_fill(fill, &fplv_path)?);
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"PATv", children))
    }

    /// `FPLv` children in dictionary order (mapped subset): FTBi FEFi
    /// FSMi FRDb, the shape slot (`FSXi` when the file carried the
    /// extended kind, else `FSHi`), `FGLv`, the dither colours. Names,
    /// `RDOb`, `FETi`/`FRRi`, `FPTv` and a compatibility `FSHi` (when
    /// `FSXi` carried the kind) splice back from the bag.
    fn write_fill(&mut self, fill: &Fill, path: &str) -> Result<Node, MapError> {
        let mut mapped = Vec::new();
        mapped.push(n_i(*b"FTBi", fill.texture_blend));
        mapped.push(n_i(*b"FEFi", fill.feather));
        if let Some(stamping) = fill.stamping {
            mapped.push(n_i(*b"FSMi", stamping));
        }
        if fill.hard_edge {
            mapped.push(n_b(*b"FRDb", true));
        }
        if fill.shape_extended {
            mapped.push(n_i(*b"FSXi", shape_raw(fill.shape)));
        } else {
            mapped.push(n_i(*b"FSHi", shape_raw(fill.shape)));
        }
        if let Some(gradient) = &fill.gradient {
            let fglv_path = format!("{path}/FGLv");
            mapped.push(self.write_gradient(gradient, &fglv_path)?);
        }
        if let Some(dither) = &fill.dither {
            for (i, slot) in dither.colors.iter().enumerate() {
                if let Some(color) = slot {
                    let fourcc = [b'F', b'D', b'1' + i as u8, b'i'];
                    mapped.push(n_i(fourcc, *color));
                }
            }
            if let Some(transparent) = dither.transparent {
                mapped.push(n_b(*b"FDTb", transparent));
            }
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"FPLv", children))
    }

    /// `FGLv`: the legacy `FGVv` vertex list first, then `FGYv` with
    /// the `FG0v`/`FG1v` ramps; names splice back from the bag.
    fn write_gradient(&mut self, gradient: &Gradient, path: &str) -> Result<Node, MapError> {
        let mut mapped = Vec::new();
        if !gradient.vertices.is_empty() {
            let fgvv_path = format!("{path}/FGVv");
            mapped.push(self.write_ramp(&gradient.vertices, *b"FGVv", &fgvv_path));
        }
        if !gradient.rgb.is_empty() || !gradient.alpha.is_empty() {
            let fgyv_path = format!("{path}/FGYv");
            let mut ramps = Vec::new();
            if !gradient.rgb.is_empty() {
                ramps.push(self.write_ramp(&gradient.rgb, *b"FG0v", &format!("{fgyv_path}/FG0v")));
            }
            if !gradient.alpha.is_empty() {
                ramps.push(self.write_ramp(
                    &gradient.alpha,
                    *b"FG1v",
                    &format!("{fgyv_path}/FG1v"),
                ));
            }
            let bagged = self.bag.take_under(&fgyv_path);
            mapped.push(n_kids(*b"FGYv", splice(ramps, bagged)));
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"FGLv", children))
    }

    /// One ramp: `FNCi` count then one `FGIv` per stop (position +
    /// colour); unknown point children splice back inside their `FGIv`.
    fn write_ramp(&mut self, stops: &[GradientStop], fourcc: [u8; 4], path: &str) -> Node {
        let mut mapped = Vec::with_capacity(1 + stops.len());
        mapped.push(n_i(*b"FNCi", stops.len() as i64));
        for (i, stop) in stops.iter().enumerate() {
            let fgiv_path = format!("{path}/FGIv[{i}]");
            let bagged = self.bag.take_under(&fgiv_path);
            let point = vec![n_f(*b"FGPf", stop.position), n_i(*b"FGCi", stop.color_raw)];
            mapped.push(n_kids(*b"FGIv", splice(point, bagged)));
        }
        let bagged = self.bag.take_under(path);
        n_kids(fourcc, splice(mapped, bagged))
    }

    fn write_contours(
        &mut self,
        contours: &[Contour],
        parent_path: &str,
    ) -> Result<Node, MapError> {
        let path = format!("{parent_path}/PBLv");
        let bagged = self.bag.take_under(&path);
        let indices = mapped_indices(&bagged, contours.len());
        let mut mapped = Vec::with_capacity(contours.len());
        for (i, contour) in contours.iter().enumerate() {
            let pbpv_path = format!("{path}/PBPv[{}]", indices[i]);
            mapped.push(self.write_contour(contour, &pbpv_path)?);
        }
        Ok(n_kids(*b"PBLv", splice(mapped, bagged)))
    }

    fn write_contour(&mut self, contour: &Contour, path: &str) -> Result<Node, MapError> {
        let bagged = self.bag.take_under(path);
        let mapped_len = 2 + contour.points.len();
        let indices = mapped_indices(&bagged, mapped_len);
        let mut mapped = Vec::with_capacity(mapped_len);
        mapped.push(n_b(*b"ISCb", contour.closed));
        mapped.push(n_i(*b"PPCi", contour.point_count));
        for (i, point) in contour.points.iter().enumerate() {
            let pbtv_path = format!("{path}/PBTv[{}]", indices[2 + i]);
            mapped.push(self.write_path_point(point, &pbtv_path)?);
        }
        Ok(n_kids(*b"PBPv", splice(mapped, bagged)))
    }

    fn write_path_point(&mut self, point: &PathPoint, path: &str) -> Result<Node, MapError> {
        // Dictionary order: XLCf YLCf XPCf? YPCf? XSCf? YSCf? CRVb? …
        let mut mapped = vec![n_f(*b"XLCf", point.anchor.x), n_f(*b"YLCf", point.anchor.y)];
        if let Some(primary) = &point.primary {
            mapped.push(n_f(*b"XPCf", primary.x));
            mapped.push(n_f(*b"YPCf", primary.y));
        }
        if let Some(secondary) = &point.secondary {
            mapped.push(n_f(*b"XSCf", secondary.x));
            mapped.push(n_f(*b"YSCf", secondary.y));
        }
        if point.is_curve {
            mapped.push(n_b(*b"CRVb", true));
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"PBTv", children))
    }

    fn write_primitive(
        &mut self,
        primitive: &PrimitiveSource,
        parent_path: &str,
    ) -> Result<Node, MapError> {
        let path = format!("{parent_path}/PRIv");
        // Bagged fill points (PCFb/PSXf/…) precede the matrix and
        // rectangle in canonical files; the splice below puts them back
        // while the mapped children fill the remaining slots. The MTXv
        // and RCTv paths are the FourCC chain — both appear at most once
        // per primitive.
        let bagged = self.bag.take_under(&path);
        let mapped_len = 2 + usize::from(primitive.maintain_radius);
        let mut mapped = Vec::with_capacity(mapped_len);
        mapped.push(self.write_matrix(&primitive.transform, &format!("{path}/MTXv")));
        let mut rect_kids = vec![n_f(*b"RDSf", primitive.radius)];
        rect_kids.push(n_f(*b"LFTf", primitive.rect.x0));
        rect_kids.push(n_f(*b"TOPf", primitive.rect.y0));
        rect_kids.push(n_f(*b"RITf", primitive.rect.x1));
        rect_kids.push(n_f(*b"BOTf", primitive.rect.y1));
        let rctv_path = format!("{path}/RCTv");
        let rctv_bagged = self.bag.take_under(&rctv_path);
        mapped.push(n_kids(*b"RCTv", splice(rect_kids, rctv_bagged)));
        if primitive.maintain_radius {
            mapped.push(n_b(*b"RDMb", true));
        }
        Ok(n_kids(*b"PRIv", splice(mapped, bagged)))
    }

    /// `path` is this `MTXv` node's own path (unknown `M*` children are
    /// spliced back inside it).
    fn write_matrix(&mut self, transform: &Affine, path: &str) -> Node {
        let coefficients = transform.as_coeffs();
        let mapped = vec![
            n_f(*b"M00f", coefficients[0]),
            n_f(*b"M01f", coefficients[1]),
            n_f(*b"M02f", 0.0),
            n_f(*b"M10f", coefficients[2]),
            n_f(*b"M11f", coefficients[3]),
            n_f(*b"M12f", 0.0),
            n_f(*b"M20f", coefficients[4]),
            n_f(*b"M21f", coefficients[5]),
            n_f(*b"M22f", 1.0),
        ];
        let bagged = self.bag.take_under(path);
        n_kids(*b"MTXv", splice(mapped, bagged))
    }

    fn write_text_object(&mut self, object: &TextObject, path: &str) -> Result<Node, MapError> {
        // Dictionary order: LFTf TOPf RITf BOTf … TFTi TAPv … LCKb …
        // TAAi … ATEb ATKb … TFSv … OBNs OPAi EFLv MTXv VISb BLDi.
        // Everything unmapped (anchors, TXTv-level style mirrors,
        // `PATv`, fill points) splices back from the bag around the
        // mapped children. TAPv's position is Fireworks-unverified —
        // no corpus file carries one yet — and pinned by the synthetic
        // fixture (see `TextOnPath`).
        let mut mapped = vec![
            n_f(*b"LFTf", object.frame.x0),
            n_f(*b"TOPf", object.frame.y0),
            n_f(*b"RITf", object.frame.x1),
            n_f(*b"BOTf", object.frame.y1),
        ];
        if let Some(flow) = object.flow {
            mapped.push(n_i(*b"TFTi", flow));
        }
        if let Some(on_path) = &object.on_path {
            let tapv = self.write_text_on_path(on_path, &format!("{path}/TAPv"))?;
            mapped.push(tapv);
        }
        // LCKb is never present with the value `false` (dictionary).
        if object.locked {
            mapped.push(n_b(*b"LCKb", true));
        }
        if let Some(anti_alias) = object.anti_alias {
            mapped.push(n_i(*b"TAAi", anti_alias));
        }
        if let Some(auto_enlarge) = object.auto_enlarge {
            mapped.push(n_b(*b"ATEb", auto_enlarge));
        }
        if let Some(auto_kern) = object.auto_kern {
            mapped.push(n_b(*b"ATKb", auto_kern));
        }
        if let Some(style) = &object.style {
            let tfsv_path = format!("{path}/TFSv");
            mapped.push(self.write_formatted_string(style, &object.runs, &tfsv_path));
        }
        if let Some(name) = &object.name {
            mapped.push(n_s(*b"OBNs", name));
        }
        mapped.push(n_i(*b"OPAi", object.opacity));
        mapped.extend(self.write_effects(&object.effects, path)?);
        if let Some(transform) = &object.transform {
            mapped.push(self.write_matrix(transform, &format!("{path}/MTXv")));
        }
        mapped.push(n_b(*b"VISb", object.visible));
        mapped.push(n_i(*b"BLDi", object.blend));
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"TXTv", children))
    }

    /// One `TAPv`: the mapped scalars plus the embedded guide path's
    /// `PBLv`; anything unmapped inside the `PTHv` (its styling) splices
    /// back between them.
    fn write_text_on_path(&mut self, on_path: &TextOnPath, path: &str) -> Result<Node, MapError> {
        let mut mapped = Vec::new();
        if let Some(alignment) = on_path.alignment {
            mapped.push(n_i(*b"TAMi", alignment));
        }
        if on_path.offset != 0.0 {
            mapped.push(n_f(*b"CHOf", on_path.offset));
        }
        let bagged = self.bag.take_under(&format!("{path}/PTHv"));
        let pblv = self.write_contours(&on_path.path, &format!("{path}/PTHv[0]"))?;
        let pthv = n_kids(*b"PTHv", splice(vec![pblv], bagged));
        mapped.push(pthv);
        let rest = self.bag.take_under(path);
        Ok(n_kids(*b"TAPv", splice(mapped, rest)))
    }

    /// One `TFSv`: the mapped style scalars in dictionary order, then
    /// one string-node `TRNs` per run (decision 0007). Unmapped styling
    /// splices back between the mapped children.
    fn write_formatted_string(&mut self, style: &TextStyle, runs: &[TextRun], path: &str) -> Node {
        let mut mapped = Vec::new();
        if let Some(font) = &style.font {
            mapped.push(n_s(*b"FONs", font));
        }
        if let Some(size) = style.size {
            mapped.push(n_f(*b"PTSf", size));
        }
        if let Some(bold) = style.bold {
            mapped.push(n_b(*b"BOLb", bold));
        }
        if let Some(italic) = style.italic {
            mapped.push(n_b(*b"ITLb", italic));
        }
        if let Some(underline) = style.underline {
            mapped.push(n_b(*b"UNDb", underline));
        }
        if let Some(scale) = style.horizontal_scale {
            mapped.push(n_f(*b"HSCf", scale));
        }
        if let Some(kerning) = style.kerning {
            mapped.push(n_f(*b"RKNf", kerning));
        }
        if let Some(shift) = style.baseline_shift {
            mapped.push(n_f(*b"BLSf", shift));
        }
        if let Some(leading) = style.leading {
            mapped.push(n_f(*b"LEDf", leading));
        }
        if let Some(unit) = style.leading_unit {
            mapped.push(n_i(*b"LDMi", unit));
        }
        if let Some(justification) = style.justification {
            mapped.push(n_i(*b"JSTi", justification));
        }
        if let Some(fill_raw) = style.fill_raw {
            mapped.push(n_i(*b"FCLi", fill_raw));
        }
        if let Some(indent) = style.paragraph_indent {
            mapped.push(n_f(*b"PINf", indent));
        }
        if let Some(before) = style.space_before {
            mapped.push(n_f(*b"PSBf", before));
        }
        if let Some(after) = style.space_after {
            mapped.push(n_f(*b"PSAf", after));
        }
        for run in runs {
            mapped.push(n_s(*b"TRNs", &run.text));
        }
        let bagged = self.bag.take_under(path);
        n_kids(*b"TFSv", splice(mapped, bagged))
    }

    fn write_group_object(&mut self, object: &GroupObject, path: &str) -> Result<Node, MapError> {
        // Dictionary order: LCKb? GRTi ELMv OBNs? … MNAb? … VISb? OPAi? BLDi?
        // MNAb is never present with the value `false` (dictionary).
        let mut mapped = Vec::new();
        if object.locked {
            mapped.push(n_b(*b"LCKb", true));
        }
        mapped.push(n_i(*b"GRTi", object.group_type));
        mapped.push(self.write_elements(&object.children, path)?);
        if let Some(name) = &object.name {
            mapped.push(n_s(*b"OBNs", name));
        }
        mapped.extend(self.write_effects(&object.effects, path)?);
        if object.mask_hidden {
            mapped.push(n_b(*b"MNAb", true));
        }
        mapped.push(n_b(*b"VISb", object.visible));
        mapped.push(n_i(*b"OPAi", object.opacity));
        mapped.push(n_i(*b"BLDi", object.blend));
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"GRPv", children))
    }

    fn write_image_object(&mut self, object: &ImageObject, path: &str) -> Result<Node, MapError> {
        // Dictionary order: LCKb? XOFf YOFf XLCf YLCf WPXi HPXi TSZi TILv
        // … BLDi? … OPAi VISb? … OBNs?
        let mut mapped = Vec::new();
        if object.locked {
            mapped.push(n_b(*b"LCKb", true));
        }
        mapped.push(n_f(*b"XOFf", object.offset.x));
        mapped.push(n_f(*b"YOFf", object.offset.y));
        mapped.push(n_f(*b"XLCf", object.location.x));
        mapped.push(n_f(*b"YLCf", object.location.y));
        mapped.push(n_i(*b"WPXi", object.pixels.0));
        mapped.push(n_i(*b"HPXi", object.pixels.1));
        mapped.push(n_i(*b"TSZi", object.tile_size));
        mapped.push(self.write_tiles(&object.tiles, path)?);
        mapped.push(n_i(*b"BLDi", object.blend));
        mapped.extend(self.write_effects(&object.effects, path)?);
        mapped.push(n_i(*b"OPAi", object.opacity));
        mapped.push(n_b(*b"VISb", object.visible));
        if let Some(name) = &object.name {
            mapped.push(n_s(*b"OBNs", name));
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"IMGv", children))
    }

    fn write_tiles(&mut self, tiles: &[TileRef], parent_path: &str) -> Result<Node, MapError> {
        let path = format!("{parent_path}/TILv");
        let bagged = self.bag.take_under(&path);
        let mut mapped = Vec::with_capacity(tiles.len());
        for tile in tiles {
            mapped.push(match tile {
                TileRef::Tile(id) => n_i(*b"TIDi", i64::from(*id)),
                TileRef::Matte(argb) => n_i(*b"TMCi", *argb),
            });
        }
        Ok(n_kids(*b"TILv", splice(mapped, bagged)))
    }

    fn write_slice_object(&mut self, object: &SliceObject, path: &str) -> Result<Node, MapError> {
        // Dictionary order: INMs? A2Ts? ALTs? FCLi URSi LFTf TOPf
        // RITf BOTf PBPv* OBNs? DTAv? LCKb? BEVv? TSLi? MSNi? CBTb?
        // CBRi? CBAi? CBHi? CHVi? CBVi? CVVi? FILs? TDTs? EXPv?
        let mut mapped = Vec::new();
        if let Some(internal_name) = &object.internal_name {
            mapped.push(n_s(*b"INMs", internal_name));
        }
        if let Some(link) = &object.link {
            mapped.push(n_s(*b"A2Ts", link));
        }
        if let Some(alt) = &object.alt {
            mapped.push(n_s(*b"ALTs", alt));
        }
        mapped.push(n_i(*b"FCLi", object.fill_raw));
        mapped.push(n_i(*b"URSi", object.shape));
        mapped.push(n_f(*b"LFTf", object.frame.x0));
        mapped.push(n_f(*b"TOPf", object.frame.y0));
        mapped.push(n_f(*b"RITf", object.frame.x1));
        mapped.push(n_f(*b"BOTf", object.frame.y1));
        for (i, contour) in object.contours.iter().enumerate() {
            // Polygon outlines ride `URLv` directly (no `PBLv` wrap —
            // the dictionary lists `PBPv` as a `URLv` child).
            let pbpv_path = format!("{path}/PBPv[{i}]");
            mapped.push(self.write_contour(contour, &pbpv_path)?);
        }
        if let Some(name) = &object.name {
            mapped.push(n_s(*b"OBNs", name));
        }
        if !object.custom_data.is_empty() {
            // One `DCEv` (`DCKs` + `DCVs`) per entry, in the map's
            // canonical order — the same shape the instance writer
            // emits for `ALSv` custom data.
            let entries: Vec<Node> = object
                .custom_data
                .iter()
                .map(|(key, value)| {
                    n_kids(*b"DCEv", vec![n_s(*b"DCKs", key), n_s(*b"DCVs", value)])
                })
                .collect();
            mapped.push(n_kids(*b"DTAv", entries));
        }
        if !object.behaviors.is_empty() {
            let bevv_path = format!("{path}/BEVv");
            let bagged_bevv = self.bag.take_under(&bevv_path);
            let indices = mapped_indices(&bagged_bevv, object.behaviors.len());
            let mut behv_nodes = Vec::with_capacity(object.behaviors.len());
            for (i, behavior) in object.behaviors.iter().enumerate() {
                // The rest of each behavior subtree re-splices from the
                // bag under this `BEHv`'s original position.
                let behv_path = format!("{bevv_path}/BEHv[{}]", indices[i]);
                let beti = vec![n_i(*b"BETi", behavior.kind)];
                let kids = self.splice_container(beti, &behv_path);
                behv_nodes.push(n_kids(*b"BEHv", kids));
            }
            let bevv = splice(behv_nodes, bagged_bevv);
            mapped.push(n_kids(*b"BEVv", bevv));
        } else {
            // Unreadable behaviors (whole-bagged) still re-emit.
            let bevv_path = format!("{path}/BEVv");
            let bagged_bevv = self.bag.take_under(&bevv_path);
            if !bagged_bevv.is_empty() {
                mapped.push(n_kids(*b"BEVv", splice(Vec::new(), bagged_bevv)));
            }
        }
        if let Some(slice_type) = object.slice_type {
            mapped.push(n_i(*b"TSLi", slice_type));
        }
        if let Some(background) = &object.background {
            if let Some(is_background) = background.is_background {
                mapped.push(n_b(*b"CBTb", is_background));
            }
            if let Some(repeat) = background.repeat {
                mapped.push(n_i(*b"CBRi", repeat));
            }
            if let Some(attachment) = background.attachment {
                mapped.push(n_i(*b"CBAi", attachment));
            }
            if let Some(h_position) = background.h_position {
                mapped.push(n_i(*b"CBHi", h_position));
            }
            if let Some(h_value) = background.h_value {
                mapped.push(n_i(*b"CHVi", h_value));
            }
            if let Some(v_position) = background.v_position {
                mapped.push(n_i(*b"CBVi", v_position));
            }
            if let Some(v_value) = background.v_value {
                mapped.push(n_i(*b"CVVi", v_value));
            }
        }
        if let Some(label) = &object.label {
            mapped.push(n_s(*b"FILs", label));
        }
        if let Some(td_text) = &object.td_text {
            mapped.push(n_s(*b"TDTs", td_text));
        }
        if let Some(export) = &object.export {
            let expv_path = format!("{path}/EXPv");
            mapped.push(self.write_export_options(export, &expv_path, &[]));
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"URLv", children))
    }

    fn write_symbol_instance_object(
        &mut self,
        object: &SymbolInstanceObject,
        path: &str,
    ) -> Result<Node, MapError> {
        // Dictionary order: MSNi MTXv LCKb XFMi OBNs? DTAv ANNi ANXf
        // ANYf ASCf ATMv ALNi? ANRi? ASOi? AEOi? IMGv? — the animation
        // family rides after the bagged `XFMi`/`DTAv`/`ATMv` slots and
        // is emitted only when `ANNi` is present.
        let mut mapped = vec![n_i(*b"MSNi", object.symbol_number)];
        mapped.push(self.write_matrix(&object.transform, &format!("{path}/MTXv")));
        if object.locked {
            mapped.push(n_b(*b"LCKb", true));
        }
        if let Some(name) = &object.name {
            mapped.push(n_s(*b"OBNs", name));
        }
        if !object.custom_data.is_empty() {
            // Dictionary order puts `DTAv` between `OBNs` and the
            // animation family; one `DCEv` (`DCKs` + `DCVs`) per
            // entry, in the map's canonical order.
            let entries: Vec<Node> = object
                .custom_data
                .iter()
                .map(|(key, value)| {
                    n_kids(*b"DCEv", vec![n_s(*b"DCKs", key), n_s(*b"DCVs", value)])
                })
                .collect();
            mapped.push(n_kids(*b"DTAv", entries));
        }
        if let Some(anim) = &object.animation {
            mapped.push(n_i(*b"ANNi", anim.frames));
            mapped.push(n_f(*b"ANXf", anim.x_shift));
            mapped.push(n_f(*b"ANYf", anim.y_shift));
            mapped.push(n_f(*b"ASCf", anim.scaling));
            if let Some(rotation) = anim.rotation {
                mapped.push(n_i(*b"ANRi", rotation));
            }
            if let Some(opacity) = anim.start_opacity {
                mapped.push(n_i(*b"ASOi", opacity));
            }
            if let Some(opacity) = anim.end_opacity {
                mapped.push(n_i(*b"AEOi", opacity));
            }
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"ALSv", children))
    }

    fn write_grid(&mut self, grid: &Grid, parent_path: &str) -> Result<Node, MapError> {
        let path = format!("{parent_path}/GRDv");
        let mapped = vec![
            n_f(*b"GOXf", grid.offset.x),
            n_f(*b"GOYf", grid.offset.y),
            n_f(*b"GSXf", grid.size.x),
            n_f(*b"GSYf", grid.size.y),
            n_i(*b"GCLi", grid.color_raw),
        ];
        let children = self.splice_container(mapped, &path);
        Ok(n_kids(*b"GRDv", children))
    }

    fn write_guides(&mut self, guides: &Guides, parent_path: &str) -> Result<Node, MapError> {
        let path = format!("{parent_path}/GDSv");
        let mut mapped = Vec::new();
        mapped.push(n_b(*b"GDLb", guides.locked));
        mapped.push(n_i(*b"GDKi", guides.color_raw));
        let hd = format!("{path}/GDHv");
        mapped.push(self.write_guide_axis(&guides.horizontal, &hd, *b"GDHv")?);
        let vd = format!("{path}/GDVv");
        mapped.push(self.write_guide_axis(&guides.vertical, &vd, *b"GDVv")?);
        let children = self.splice_container(mapped, &path);
        Ok(n_kids(*b"GDSv", children))
    }

    fn write_guide_axis(
        &mut self,
        positions: &[f64],
        path: &str,
        fourcc: FourCC,
    ) -> Result<Node, MapError> {
        let bagged = self.bag.take_under(path);
        let mut mapped = Vec::with_capacity(positions.len());
        for position in positions {
            mapped.push(n_f(*b"GDCf", *position));
        }
        Ok(n_kids(fourcc, splice(mapped, bagged)))
    }
}

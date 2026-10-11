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
    Cell, Contour, Document, Effect, EffectProperty, EffectValue, Grid, GroupObject, Guides,
    ImageObject, Layer, LayerList, PaintStyle, PathObject, PathPoint, PrimitiveSource, SliceObject,
    SymbolDefinition, SymbolInstanceObject, TextObject, TextRun, TextStyle, TileRef,
};
use crate::registry::{ObjectData, ObjectId};
use fwpng::nodetree::{Node, NodeValue};
use geom::Affine;

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
    /// skinning. Definition lists, export options and the page scalars
    /// have no document-level model slot — they come back from the bag.
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
        let children = self.splice_container(mapped, &path);
        Ok(n_kids(*b"PDCv", children))
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
        let mut mapped = vec![
            n_s(*b"UNMs", &symbol.name),
            n_i(*b"MSNi", symbol.symbol_number),
            n_f(*b"MOXf", symbol.origin.x),
            n_f(*b"MOYf", symbol.origin.y),
        ];
        mapped.push(self.write_layer_list(&symbol.layers, path)?);
        mapped.push(n_i(*b"FRCi", symbol.frame_count));
        mapped.push(n_i(*b"BGCi", symbol.background_raw));
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"MSTv", children))
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
    /// gradient/texture subtrees (`FPLv` `TXBv` `TXFv` `BPLv`) splice
    /// back from the bag.
    fn write_paint(&mut self, paint: &PaintStyle, path: &str) -> Result<Node, MapError> {
        let mapped = vec![
            n_i(*b"BCLi", paint.stroke_raw),
            n_i(*b"FCLi", paint.fill_raw),
        ];
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"PATv", children))
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
        // Dictionary order: LFTf TOPf RITf BOTf … TFTi … LCKb … TAAi …
        // ATEb ATKb … TFSv … OBNs OPAi EFLv MTXv VISb BLDi. Everything
        // unmapped (anchors, TXTv-level style mirrors, `TAPv`, `PATv`,
        // fill points) splices back from the bag around the mapped
        // children.
        let mut mapped = vec![
            n_f(*b"LFTf", object.frame.x0),
            n_f(*b"TOPf", object.frame.y0),
            n_f(*b"RITf", object.frame.x1),
            n_f(*b"BOTf", object.frame.y1),
        ];
        if let Some(flow) = object.flow {
            mapped.push(n_i(*b"TFTi", flow));
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
        // Dictionary order: INMs? … FCLi URSi LFTf TOPf RITf BOTf …
        // OBNs? … TSLi?
        let mut mapped = Vec::new();
        if let Some(internal_name) = &object.internal_name {
            mapped.push(n_s(*b"INMs", internal_name));
        }
        mapped.push(n_i(*b"FCLi", object.fill_raw));
        mapped.push(n_i(*b"URSi", object.shape));
        mapped.push(n_f(*b"LFTf", object.frame.x0));
        mapped.push(n_f(*b"TOPf", object.frame.y0));
        mapped.push(n_f(*b"RITf", object.frame.x1));
        mapped.push(n_f(*b"BOTf", object.frame.y1));
        if let Some(name) = &object.name {
            mapped.push(n_s(*b"OBNs", name));
        }
        if let Some(slice_type) = object.slice_type {
            mapped.push(n_i(*b"TSLi", slice_type));
        }
        let children = self.splice_container(mapped, path);
        Ok(n_kids(*b"URLv", children))
    }

    fn write_symbol_instance_object(
        &mut self,
        object: &SymbolInstanceObject,
        path: &str,
    ) -> Result<Node, MapError> {
        // Dictionary order: MSNi MTXv LCKb XFMi OBNs? …
        let mut mapped = vec![n_i(*b"MSNi", object.symbol_number)];
        mapped.push(self.write_matrix(&object.transform, &format!("{path}/MTXv")));
        if object.locked {
            mapped.push(n_b(*b"LCKb", true));
        }
        if let Some(name) = &object.name {
            mapped.push(n_s(*b"OBNs", name));
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

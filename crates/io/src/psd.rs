//! PSD import: parse the Adobe Photoshop file format (version 1,
//! RGB/8-bit) from the public spec and map it onto a fresh
//! [`doc::Document`].
//!
//! The reader is a flat record walk — PSD has no recursive nesting, so
//! unlike the SVG importer it needs no depth gate; its hostile-input
//! story is fixed-layout bounds checking (every read goes through a
//! cursor that refuses to leave the stream), allocation ceilings
//! (compose's 65535px canvas, a 16-megapixel per-plane cap, the
//! 4096-tile bank), and the fault-tolerance policy from decision 0006:
//! unknown blend keys, layer masks, clipping, text layers, and
//! adjustment layers all land as report notes with a best-effort
//! document, never an error (real writers — ImageMagick included —
//! emit non-conformant bytes).
//!
//! Structure mapping: PSD layer records (stored bottom-first, the same
//! z-order the model speaks) become model layers; a top-level group —
//! a bounding divider record (type 3) below its children and an `lsct`
//! header (1/2) above them — becomes a model layer whose cell holds
//! one `GroupObject` spanning the member objects (nested groups nest);
//! a file with no layer records falls back to its merged composite as
//! a single `IMGv` object, exactly like the raster import path.

use doc::{Cell, Document, GroupObject, Layer, LayerList, ObjectData, Page, PageInfo};
use geom::Point;

use crate::{ImportError, Plane};

/// The format name every error carries.
const FORMAT: &str = "psd";

/// Per-plane pixel ceiling: 16 megapixels is far above any sane layer
/// and far below any allocation that could hurt.
const MAX_PLANE_PIXELS: usize = 16_777_216;

/// An imported PSD: the document plus the divergence report.
#[derive(Clone, Debug)]
pub struct ImportedPsd {
    /// The imported document.
    pub document: Document,
    /// Human-readable divergence notes (limitations applied), in walk
    /// order.
    pub notes: Vec<String>,
    /// Layer records imported as raster layers.
    pub layers: usize,
    /// Group pairs imported as structured groups.
    pub groups: usize,
    /// Text layers flattened to raster pixels.
    pub texts_as_raster: usize,
    /// Layer masks dropped (channel id −2).
    pub masks_dropped: usize,
    /// Clipped layers imported unclipped.
    pub clipped_layers: usize,
    /// Unknown blend keys imported as normal.
    pub unknown_blends: usize,
    /// Whether the merged composite was parsed (and used as the
    /// document when the file carries no layers).
    pub composite: bool,
}

/// A bounds-checked big-endian cursor over the stream.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn u8(&mut self) -> Result<u8, ImportError> {
        let byte = *self.bytes.get(self.at).ok_or_else(truncated)?;
        self.at += 1;
        Ok(byte)
    }

    fn u16(&mut self) -> Result<u16, ImportError> {
        let two = self.take(2)?;
        Ok(u16::from_be_bytes([two[0], two[1]]))
    }

    fn i16(&mut self) -> Result<i16, ImportError> {
        Ok(self.u16()? as i16)
    }

    fn u32(&mut self) -> Result<u32, ImportError> {
        let four = self.take(4)?;
        Ok(u32::from_be_bytes([four[0], four[1], four[2], four[3]]))
    }

    fn i32(&mut self) -> Result<i32, ImportError> {
        Ok(self.u32()? as i32)
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], ImportError> {
        let stop = self.at.checked_add(count).ok_or_else(truncated)?;
        let slice = self.bytes.get(self.at..stop).ok_or_else(truncated)?;
        self.at = stop;
        Ok(slice)
    }

    fn tag(&mut self, want: &[u8]) -> Result<bool, ImportError> {
        Ok(self.take(want.len())? == want)
    }

    /// Consumes a u32-length-prefixed section.
    fn skip_length_prefixed(&mut self) -> Result<(), ImportError> {
        let length = self.u32()? as usize;
        self.take(length)?;
        Ok(())
    }

    /// Reads a u32 section length without crossing it; `None` for a
    /// declared length of zero (an absent layer section). Positions
    /// the cursor at the section body.
    fn section(&mut self) -> Result<Option<usize>, ImportError> {
        let length = self.u32()? as usize;
        if length == 0 {
            return Ok(None);
        }
        let end = self.at.checked_add(length).ok_or_else(truncated)?;
        if end > self.bytes.len() {
            return Err(truncated());
        }
        Ok(Some(end))
    }
}

fn truncated() -> ImportError {
    ImportError::Truncated {
        format: FORMAT,
        needed: "more section bytes than the stream holds",
    }
}

/// Decodes a PSD into a document: layers as model layers, groups as
/// structured group objects, and the merged composite as the whole
/// document when no layers exist.
///
/// # Errors
/// [`ImportError::Truncated`] for short streams,
/// [`ImportError::Unsupported`] for out-of-scope features (PSB,
/// non-8-bit depths, non-RGB color modes, ZIP channels, oversized
/// canvases or planes), [`ImportError::Decode`] for malformed records.
pub fn decode(bytes: &[u8]) -> Result<ImportedPsd, ImportError> {
    let mut report = ImportedPsd {
        document: Document::default(),
        notes: Vec::new(),
        layers: 0,
        groups: 0,
        texts_as_raster: 0,
        masks_dropped: 0,
        clipped_layers: 0,
        unknown_blends: 0,
        composite: false,
    };

    let mut reader = Reader::new(bytes);
    let (width, height, channels) = parse_header(&mut reader)?;
    report.document.info.frame_count = 1;
    report.document.info.canvas.width = f64::from(width);
    report.document.info.canvas.height = f64::from(height);

    // Section 2: color mode data (skipped by length).
    reader.skip_length_prefixed()?;
    // Section 3: image resources (skipped by length).
    reader.skip_length_prefixed()?;

    // Section 4: layer and mask information. A missing section or a
    // zero layer count means the merged composite is the document.
    let composite_at;
    let mut any_layers = false;
    match reader.section()? {
        None => composite_at = reader.at,
        Some(layer_section_end) => {
            composite_at = layer_section_end;
            if let Some(info_end) = reader.section()? {
                let count_raw = reader.i16()?;
                if count_raw < 0 {
                    report.notes.push(
                        "merged-image alpha carries transparency (negative layer count)".to_owned(),
                    );
                }
                let count = count_raw.unsigned_abs() as usize;
                if count > 0 {
                    let records = parse_layer_records(&mut reader, info_end, count, &mut report)?;
                    build_model(&mut report, bytes, &records, width, height)?;
                    any_layers = true;
                }
            }
        }
    }

    // Section 5: the merged composite — validated after the layer
    // section; without layers it becomes the whole document. The spec
    // prefixes it with a u32 length; some real writers (ag-psd) emit
    // the compression marker directly. An exact length match settles
    // the prefixed layout; otherwise the length-less form is tried
    // first and the prefixed form has the final word.
    reader.at = composite_at;
    let remaining = bytes.len().saturating_sub(composite_at);
    let mut probe = Reader::new(bytes);
    probe.at = composite_at;
    let exact_prefix = remaining >= 4 && probe.u32()? as usize == remaining - 4;
    let plane = if exact_prefix {
        composite_plane(&mut reader, width, height, channels, true)?
    } else {
        match composite_plane(&mut reader, width, height, channels, false) {
            Ok(plane) => plane,
            Err(_) => {
                reader.at = composite_at;
                composite_plane(&mut reader, width, height, channels, true)?
            }
        }
    };
    report.composite = true;
    if !any_layers {
        build_composite_document(&mut report, width, height, &plane)?;
    }

    Ok(report)
}

/// Parses the fixed 26-byte header into `(width, height, channels)`
/// with the scope gates from decision 0006.
fn parse_header(reader: &mut Reader<'_>) -> Result<(u32, u32, u16), ImportError> {
    if !reader.tag(b"8BPS")? {
        return Err(ImportError::Decode {
            format: FORMAT,
            reason: "signature is not 8BPS".to_owned(),
        });
    }
    let version = reader.u16()?;
    if version != 1 {
        return Err(ImportError::Unsupported {
            format: FORMAT,
            reason: if version == 2 {
                "PSB (large-document) variant is not supported".to_owned()
            } else {
                format!("unknown version {version}")
            },
        });
    }
    reader.take(6)?; // reserved
    let channels = reader.u16()?;
    let height = reader.u32()?;
    let width = reader.u32()?;
    let depth = reader.u16()?;
    let mode = reader.u16()?;
    if depth != 8 {
        return Err(ImportError::Unsupported {
            format: FORMAT,
            reason: format!("{depth}-bit depth (only 8-bit RGB is supported)"),
        });
    }
    if mode != 3 {
        let name = match mode {
            0 => "bitmap",
            1 => "grayscale",
            2 => "indexed",
            4 => "CMYK",
            7 => "multichannel",
            8 => "duotone",
            9 => "lab",
            _ => "unknown",
        };
        return Err(ImportError::Unsupported {
            format: FORMAT,
            reason: format!("color mode {mode} ({name}) — only RGB is supported"),
        });
    }
    if width == 0 || height == 0 {
        return Err(ImportError::Unsupported {
            format: FORMAT,
            reason: format!("{width}x{height} canvas is empty"),
        });
    }
    if width > 65535 || height > 65535 {
        return Err(ImportError::Unsupported {
            format: FORMAT,
            reason: format!("{width}x{height} canvas exceeds the 65535px compose ceiling"),
        });
    }
    Ok((width, height, channels))
}

/// A parsed layer record, before model mapping.
struct LayerRecord {
    top: i32,
    left: i32,
    width: u32,
    height: u32,
    /// `(channel id, compression, stream offset, stream length)` —
    /// the offset and length include the 2-byte compression marker;
    /// lengths under 2 carry no stream at all.
    channels: Vec<(i16, u16, usize, usize)>,
    blend: [u8; 4],
    opacity: u8,
    clipping: u8,
    visible: bool,
    name: String,
    /// `lsct` section type: 1/2 opens a group, 3 terminates it.
    section: Option<i32>,
    /// The record carries Photoshop text (`TySh`).
    is_text: bool,
    /// The record is an adjustment layer (an adjustment-only info
    /// block was present).
    is_adjustment: bool,
}

/// Parses `count` layer records starting at the cursor, stopping at
/// `end`. Records are stored bottom-first. Channel streams arrive in
/// either of the two layouts real writers emit: interleaved after
/// each record's extra data (ImageMagick) or as one block after all
/// records (the spec layout Photoshop and ag-psd write). The stream
/// is walked interleaved first; any structural failure retries the
/// block layout before giving up. Compression markers and the mask
/// census are resolved once the layout is settled.
fn parse_layer_records(
    reader: &mut Reader<'_>,
    end: usize,
    count: usize,
    report: &mut ImportedPsd,
) -> Result<Vec<LayerRecord>, ImportError> {
    let entry = reader.at;
    let mut records = match walk_records(reader, end, count, true) {
        Ok(records) => records,
        Err(_) => {
            reader.at = entry;
            walk_records(reader, end, count, false)?
        }
    };
    for record in &mut records {
        for (id, compression, start, length) in &mut record.channels {
            if *length < 2 {
                continue; // an empty stream carries no marker
            }
            let mut probe = Reader::new(reader.bytes);
            probe.at = *start;
            *compression = probe.u16()?;
            if *compression > 3 {
                return Err(ImportError::Unsupported {
                    format: FORMAT,
                    reason: format!("channel compression {} (ZIP) is not supported", compression),
                });
            }
            if *id == -2 {
                report.masks_dropped += 1;
            }
        }
    }
    Ok(records)
}

/// One layout pass over the records. `interleaved` places each
/// record's channel streams right after its extra data; otherwise the
/// streams form one block after the last record, in record and
/// channel-info order. Stream data is only located here — compression
/// probing happens in [`parse_layer_records`].
fn walk_records(
    reader: &mut Reader<'_>,
    end: usize,
    count: usize,
    interleaved: bool,
) -> Result<Vec<LayerRecord>, ImportError> {
    let mut records = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        if reader.at + 16 > end {
            return Err(truncated());
        }
        let top = reader.i32()?;
        let left = reader.i32()?;
        let bottom = reader.i32()?;
        let right = reader.i32()?;
        let channel_count = reader.u16()? as usize;
        if channel_count > 64 {
            return Err(ImportError::Decode {
                format: FORMAT,
                reason: format!("layer claims {channel_count} channels"),
            });
        }
        let mut info = Vec::with_capacity(channel_count);
        for _ in 0..channel_count {
            let id = reader.i16()?;
            let length = reader.u32()? as usize;
            info.push((id, length));
        }
        if !reader.tag(b"8BIM")? {
            return Err(ImportError::Decode {
                format: FORMAT,
                reason: "layer blend signature is not 8BIM".to_owned(),
            });
        }
        let blend = reader.take(4)?;
        let mut blend_key = [0u8; 4];
        blend_key.copy_from_slice(blend);
        let opacity = reader.u8()?;
        let clipping = reader.u8()?;
        let flags = reader.u8()?;
        reader.take(1)?; // filler
        let extra_length = reader.u32()? as usize;
        let extra_end = reader
            .at
            .checked_add(extra_length)
            .ok_or_else(truncated)?
            .min(end);
        let extra = parse_extra_data(reader, extra_end)?;

        // Stream positions: interleaved consumes them here; the block
        // layout defers to the pass after the record loop. A stream
        // is a u16 compression marker plus the packed rows, so data
        // starts 2 bytes into the length (filled in later).
        let mut channels = Vec::with_capacity(info.len());
        for (id, length) in info {
            let start = if interleaved {
                let start = reader.at;
                if start.checked_add(length).is_none_or(|stop| stop > end) {
                    return Err(truncated());
                }
                reader.at = start + length;
                start
            } else {
                0
            };
            channels.push((id, 0, start, length));
        }

        records.push(LayerRecord {
            top,
            left,
            width: (right - left).max(0) as u32,
            height: (bottom - top).max(0) as u32,
            channels,
            blend: blend_key,
            opacity,
            clipping,
            visible: flags & 0x02 == 0,
            name: extra.name,
            section: extra.section,
            is_text: extra.is_text,
            is_adjustment: extra.is_adjustment,
        });
    }
    if !interleaved {
        // The block starts where the last record's extra data ended.
        let mut at = reader.at;
        for record in &mut records {
            for (_id, _compression, start, length) in &mut record.channels {
                if at.checked_add(*length).is_none_or(|stop| stop > end) {
                    return Err(truncated());
                }
                *start = at;
                at += *length;
            }
        }
    }
    Ok(records)
}

struct ExtraData {
    name: String,
    section: Option<i32>,
    is_text: bool,
    is_adjustment: bool,
}

/// Parses a record's extra data block: mask data, blending ranges,
/// pascal name (padded to 4 measured from its length byte), then
/// additional-info blocks.
fn parse_extra_data(reader: &mut Reader<'_>, end: usize) -> Result<ExtraData, ImportError> {
    reader.skip_length_prefixed()?; // layer mask data
    reader.skip_length_prefixed()?; // blending ranges
    let name_len = usize::from(reader.u8()?);
    let name_bytes = reader.take(name_len)?;
    let name = String::from_utf8_lossy(name_bytes).into_owned();
    let consumed = 1 + name_len;
    reader.take((4 - (consumed % 4)) % 4)?;

    let mut extra = ExtraData {
        name,
        section: None,
        is_text: false,
        is_adjustment: false,
    };

    // Additional info blocks until the record ends. Each: an `8BIM`
    // (or `8B64`) signature, a 4-byte key, a u32 length, and the data
    // padded to a multiple of 4.
    while reader.at + 12 <= end {
        if !reader.tag(b"8BIM")? && !reader.tag(b"8B64")? {
            break; // no more well-formed blocks; what we captured stays
        }
        let mut key = [0u8; 4];
        key.copy_from_slice(reader.take(4)?);
        let length = reader.u32()? as usize;
        let data_start = reader.at;
        let data_end = data_start
            .checked_add(length)
            .ok_or_else(truncated)?
            .min(end);
        apply_info_block(reader.bytes, &key, data_start, data_end, &mut extra);
        reader.at = data_end;
        reader.take((4 - ((data_end - data_start) % 4)) % 4)?;
    }
    // Legacy group convention: the pair is named in the pascal name.
    if extra.section.is_none() {
        if extra.name == "<Layer set>" {
            extra.section = Some(1);
        } else if extra.name == "</Layer set>" {
            extra.section = Some(3);
        }
    }
    Ok(extra)
}

/// Applies one additional-info block to the accumulating record.
/// Everything here is best-effort: a malformed block loses a name or
/// a marker, never the import.
fn apply_info_block(bytes: &[u8], key: &[u8; 4], start: usize, end: usize, extra: &mut ExtraData) {
    let _ = end;
    let mut probe = Reader::new(bytes);
    probe.at = start;
    match key {
        b"luni" => {
            // Unicode layer name: u32 count + UTF-16BE units.
            if let Ok(count) = probe.u32() {
                let mut units = Vec::with_capacity((count as usize).min(1024));
                for _ in 0..(count as usize).min(1024) {
                    match probe.u16() {
                        Ok(unit) => units.push(unit),
                        Err(_) => break,
                    }
                }
                if let Ok(text) = String::from_utf16(&units) {
                    extra.name = text;
                }
            }
        }
        b"lsct" => {
            if let Ok(section) = probe.i32() {
                extra.section = Some(section);
            }
        }
        b"TySh" => extra.is_text = true,
        // Adjustment layers announce themselves through their effect
        // blocks (solid color, gradient, curves, …).
        b"SoCo" | b"GdFl" | b"PtFl" | b"brit" | b"levl" | b"curv" | b"blnc" | b"blwh" | b"phfl"
        | b"mixr" | b"nvrt" | b"post" | b"sel " => extra.is_adjustment = true,
        _ => {}
    }
}

/// The blend-mode mapping result for one record.
enum Blend {
    /// Maps exactly to a model `BLDi` value.
    Mapped(i64),
    /// Group pass-through: semantically different from any model
    /// group blend; the caller notes it and lands on 0.
    PassThrough,
    /// No model equivalent: the caller notes it and lands on 0.
    Unknown,
}

/// The mechanical PSD key → model mapping (the inverse of
/// `compose::blend_value`'s importable rows).
fn blend(key: &[u8; 4]) -> Blend {
    match key {
        b"pass" => Blend::PassThrough,
        b"norm" => Blend::Mapped(0),
        b"mul " => Blend::Mapped(2),
        b"diff" => Blend::Mapped(3),
        b"over" => Blend::Mapped(4),
        b"scrn" => Blend::Mapped(5),
        b"dark" => Blend::Mapped(9),
        b"lite" => Blend::Mapped(10),
        b"hue " => Blend::Mapped(11),
        b"sat " => Blend::Mapped(12),
        b"colr" => Blend::Mapped(13),
        b"lum " => Blend::Mapped(14),
        b"smud" => Blend::Mapped(32),
        b"hLit" => Blend::Mapped(33),
        b"sLit" => Blend::Mapped(34),
        b"div " => Blend::Mapped(36),
        b"idiv" => Blend::Mapped(37),
        _ => Blend::Unknown,
    }
}

/// Resolves a blend key to its model value, reporting the divergences.
fn blend_of(report: &mut ImportedPsd, key: &[u8; 4], name: &str) -> i64 {
    match blend(key) {
        Blend::Mapped(value) => value,
        Blend::PassThrough => {
            report.notes.push(format!(
                "group {name:?} pass-through imports as normal group blend"
            ));
            0
        }
        Blend::Unknown => {
            report.unknown_blends += 1;
            report.notes.push(format!(
                "blend mode {:?} (layer {name:?}) has no model equivalent; imports as normal",
                String::from_utf8_lossy(key)
            ));
            0
        }
    }
}

/// A group header record's carried semantics.
struct GroupShell {
    name: String,
    opacity: u8,
    blend: [u8; 4],
    visible: bool,
}

/// Maps parsed records onto the document: bottom-first layers, groups
/// as structured group objects, everything as `IMGv` raster pixels.
/// Top-level records each become a model layer; group members live
/// only inside the group object.
fn build_model(
    report: &mut ImportedPsd,
    bytes: &[u8],
    records: &[LayerRecord],
    width: u32,
    height: u32,
) -> Result<(), ImportError> {
    report.document.info.frame_count = 1;
    report.document.info.canvas.width = f64::from(width);
    report.document.info.canvas.height = f64::from(height);

    // Open groups: semantics plus the member ids gathered so far.
    let mut stack: Vec<(GroupShell, Vec<doc::ObjectId>)> = Vec::new();
    let mut layers: Vec<Layer> = Vec::new();

    for record in records {
        if record.is_adjustment {
            report.notes.push(format!(
                "adjustment layer {:?} has no pixels of its own and is skipped",
                record.name
            ));
            continue;
        }
        match record.section {
            // Records are bottom-first: a group is a bounding divider
            // (type 3) below its children and a header (1/2) above
            // them, and the header carries the group's real name,
            // blend, opacity, and visibility.
            Some(3) => {
                stack.push((
                    GroupShell {
                        name: record.name.clone(),
                        opacity: record.opacity,
                        blend: record.blend,
                        visible: record.visible,
                    },
                    Vec::new(),
                ));
            }
            Some(1 | 2) => {
                let Some((_, children)) = stack.pop() else {
                    report.notes.push(format!(
                        "group header {:?} has no bounding divider and is skipped",
                        record.name
                    ));
                    continue;
                };
                let shell = GroupShell {
                    name: record.name.clone(),
                    opacity: record.opacity,
                    blend: record.blend,
                    visible: record.visible,
                };
                report.groups += 1;
                if record.section == Some(2) {
                    report.notes.push(format!(
                        "group {:?} imports open (collapsed state dropped)",
                        record.name
                    ));
                }
                let blend = blend_of(report, &shell.blend, &shell.name);
                let group_id = report
                    .document
                    .registry
                    .insert(ObjectData::Group(GroupObject {
                        group_type: 0,
                        name: non_empty(&shell.name),
                        children,
                        opacity: per_mille(shell.opacity),
                        blend,
                        visible: shell.visible,
                        locked: false,
                        mask_hidden: false,
                        effects: Vec::new(),
                    }));
                attach_to_scope(
                    &mut stack,
                    &mut layers,
                    &shell.name,
                    shell.visible,
                    group_id,
                );
            }
            _ => {
                if let Some(image_id) = import_layer_pixels(report, bytes, record)? {
                    attach_to_scope(
                        &mut stack,
                        &mut layers,
                        &record.name,
                        record.visible,
                        image_id,
                    );
                }
            }
        }
    }
    for (shell, _) in std::mem::take(&mut stack) {
        report
            .notes
            .push(format!("group {:?} never terminated", shell.name));
    }

    let canvas = report.document.info.canvas.clone();
    report.document.pages.push(Page {
        info: PageInfo {
            version: report.document.info.version,
            revision: report.document.info.revision,
            frame_count: 1,
            frame_count_duplicate: 1,
            request_id: 0,
            canvas,
            onion_before: 0,
            onion_after: 0,
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        grid: None,
        guides: Default::default(),
        layers: LayerList { layers },
    });
    Ok(())
}

/// Attaches a finished object id to the innermost open group's
/// children, or — at top level — starts a new model layer for it.
fn attach_to_scope(
    stack: &mut [(GroupShell, Vec<doc::ObjectId>)],
    layers: &mut Vec<Layer>,
    name: &str,
    visible: bool,
    id: doc::ObjectId,
) {
    match stack.last_mut() {
        Some((_, children)) => children.push(id),
        None => {
            let name = if name.is_empty() {
                format!("Layer {}", layers.len() + 1)
            } else {
                name.to_owned()
            };
            layers.push(Layer {
                name,
                display: visible,
                cells: vec![Cell {
                    locked: false,
                    visible: true,
                    elements: vec![id],
                }],
                ..Layer::default()
            });
        }
    }
}

/// Decodes one raster layer record's channels into pixels and
/// registers the `IMGv` object (positioned at the record's offset,
/// carrying the layer's opacity/blend/visibility). `Ok(None)` marks a
/// zero-size record, which contributes nothing.
fn import_layer_pixels(
    report: &mut ImportedPsd,
    bytes: &[u8],
    record: &LayerRecord,
) -> Result<Option<doc::ObjectId>, ImportError> {
    let width = record.width as usize;
    let height = record.height as usize;
    if width == 0 || height == 0 {
        report.notes.push(format!(
            "layer {:?} has an empty rectangle and is skipped",
            record.name
        ));
        return Ok(None);
    }
    if width
        .checked_mul(height)
        .is_none_or(|pixels| pixels > MAX_PLANE_PIXELS)
    {
        return Err(ImportError::Unsupported {
            format: FORMAT,
            reason: format!("layer rectangle {width}x{height} exceeds the 16-megapixel cap"),
        });
    }
    if record.is_text {
        report.texts_as_raster += 1;
        report.notes.push(format!(
            "text layer {:?} imports as raster pixels",
            record.name
        ));
    }
    if record.clipping & 1 == 1 {
        report.clipped_layers += 1;
        report
            .notes
            .push(format!("layer {:?} imports unclipped", record.name));
    }
    let pixels = layer_pixels(bytes, record)?;
    let blend = blend_of(report, &record.blend, &record.name);
    let document = &mut report.document;
    let image_id = crate::raster::attach_image(
        document,
        record.width,
        record.height,
        &pixels,
        non_empty(&record.name),
    )?;
    if let Some(ObjectData::Image(image)) = document.registry.get_mut(image_id) {
        image.location = Point::new(f64::from(record.left), f64::from(record.top));
        image.opacity = per_mille(record.opacity);
        image.blend = blend;
        image.visible = record.visible;
    }
    report.layers += 1;
    Ok(Some(image_id))
}

/// Extracts one layer's straight RGBA8 pixels from its channel
/// streams: R/G/B land in order, channel −1 is alpha, everything else
/// (masks, spot colors) is dropped and reported by the caller.
fn layer_pixels(bytes: &[u8], record: &LayerRecord) -> Result<Vec<u8>, ImportError> {
    let width = record.width as usize;
    let height = record.height as usize;
    let mut rgba = vec![255u8; width * height * 4];
    for (id, compression, start, length) in &record.channels {
        let offset = match id {
            0 => 0,
            1 => 1,
            2 => 2,
            -1 => 3,
            _ => continue,
        };
        if *length < 2 {
            continue; // no marker, no data
        }
        let mut stream = Reader::new(bytes);
        stream.at = start + 2; // past the compression marker
        let plane = match compression {
            0 => stream.take(width * height)?.to_vec(),
            1 => rle_plane(&mut stream, width, height)?,
            other => {
                return Err(ImportError::Unsupported {
                    format: FORMAT,
                    reason: format!("channel compression {other} (ZIP) is not supported"),
                });
            }
        };
        for (index, value) in plane.iter().enumerate() {
            if let Some(slot) = rgba.get_mut(index * 4 + offset) {
                *slot = *value;
            }
        }
    }
    Ok(rgba)
}

/// Decodes the merged composite into a straight RGBA plane (also the
/// no-layers fallback's pixels). With `prefixed` the section opens
/// with its u32 length; either way a compression marker follows, then
/// the channels in R,G,B,(A) order.
fn composite_plane(
    reader: &mut Reader<'_>,
    width: u32,
    height: u32,
    channels: u16,
    prefixed: bool,
) -> Result<Vec<u8>, ImportError> {
    let width = width as usize;
    let height = height as usize;
    let pixels = width * height;
    if pixels > MAX_PLANE_PIXELS {
        return Err(ImportError::Unsupported {
            format: FORMAT,
            reason: format!("composite {width}x{height} exceeds the 16-megapixel cap"),
        });
    }
    if prefixed {
        // The section's own u32 length; a zero means "rest of
        // stream". Either way the cursor lands on the body.
        let _ = reader.section()?;
    }
    let compression = reader.u16()?;
    let count = usize::from(channels.clamp(3, 4));
    let mut planes: Vec<Vec<u8>> = vec![vec![255u8; pixels]; 4];
    match compression {
        0 => {
            for plane in planes.iter_mut().take(count) {
                let raw = reader.take(pixels)?;
                plane.copy_from_slice(raw);
            }
        }
        1 => {
            // The composite shares one byte-count table across all
            // channels (rows x channels entries), then one packed
            // stream — the spec's image-data layout, confirmed against
            // ImageMagick and ag-psd output. Layer channels differ:
            // each carries its own table inside its own length.
            let mut counts = Vec::with_capacity(height * count);
            for _ in 0..height * count {
                counts.push(reader.u16()?);
            }
            for channel in 0..count {
                planes[channel] = rle_rows(
                    reader,
                    width,
                    &counts[channel * height..(channel + 1) * height],
                )?;
            }
        }
        other => {
            return Err(ImportError::Unsupported {
                format: FORMAT,
                reason: format!("composite compression {other} (ZIP) is not supported"),
            });
        }
    }
    let mut rgba = vec![0u8; pixels * 4];
    for (offset, plane) in planes.iter().enumerate() {
        for (index, value) in plane.iter().enumerate() {
            if let Some(slot) = rgba.get_mut(index * 4 + offset) {
                *slot = *value;
            }
        }
    }
    Ok(rgba)
}

/// Decodes one PackBits RLE plane at the cursor: a per-scanline u16
/// byte-count table, then the packed rows (the layer-channel layout).
/// Runs that overshoot a row are truncated to the row width; the
/// cursor stops after the last declared row.
fn rle_plane(reader: &mut Reader<'_>, width: usize, height: usize) -> Result<Vec<u8>, ImportError> {
    let mut counts = Vec::with_capacity(height);
    for _ in 0..height {
        counts.push(reader.u16()?);
    }
    rle_rows(reader, width, &counts)
}

/// Decodes packed rows for one channel from pre-read byte counts —
/// the composite's shape, where all channels' counts sit in one table
/// before a single packed stream.
fn rle_rows(reader: &mut Reader<'_>, width: usize, counts: &[u16]) -> Result<Vec<u8>, ImportError> {
    let mut out = Vec::with_capacity(width * counts.len());
    for count in counts {
        let row_start = out.len();
        let mut produced = 0usize;
        let mut budget = usize::from(*count);
        while produced < width && budget > 0 {
            let control = reader.u8()? as i8;
            budget -= 1;
            if control >= 0 {
                let run = control as usize + 1;
                let source = reader.take(run)?;
                out.extend_from_slice(source);
                budget = budget.saturating_sub(run);
                produced += run;
            } else if control != -128 {
                let run = (1 - i32::from(control)) as usize;
                let value = reader.u8()?;
                budget = budget.saturating_sub(1);
                out.extend(std::iter::repeat_n(value, run));
                produced += run;
            }
        }
        out.truncate(row_start + width);
    }
    Ok(out)
}

/// Builds the no-layers fallback document from composite pixels.
fn build_composite_document(
    report: &mut ImportedPsd,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), ImportError> {
    Plane::new(width, height, rgba).map_err(|error| ImportError::Decode {
        format: FORMAT,
        reason: error.to_string(),
    })?;
    let document = &mut report.document;
    let image_id = crate::raster::attach_image(document, width, height, rgba, None)?;
    crate::raster::build_single_object_page(document, document.info.canvas.clone(), vec![image_id]);
    Ok(())
}

/// PSD u8 opacity → model per-mille (rounding).
fn per_mille(opacity: u8) -> i64 {
    i64::from((u32::from(opacity) * 1000 + 127) / 255)
}

/// `Option<String>` that drops empty names (the model wants `None`
/// over an empty string).
fn non_empty(name: &str) -> Option<String> {
    (!name.is_empty()).then(|| name.to_owned())
}

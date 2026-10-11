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

use crate::{ExportError, ImportError, Plane};

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

// ------------------------------------------------------------------ //
// The writer (M8 task 6): layered RGB/8 PSD, maximize compatibility.
// One record per object, groups as the bottom-first divider/header
// pair, RLE channel data (one table + packed rows per channel), and
// the merged composite with its spec'd u32 length prefix.
// ------------------------------------------------------------------ //

/// Maximum group nesting the writer walks (model files are house-made,
/// but the registry accepts anything a `.fw.png` carried).
const MAX_GROUP_DEPTH: usize = 64;

/// Maximum layer records: PSD stores the count in a *signed* u16.
const MAX_RECORDS: usize = 32_767;

/// One prepared layer record awaiting serialization.
struct Record {
    name: String,
    top: i32,
    left: i32,
    bottom: i32,
    right: i32,
    blend: [u8; 4],
    opacity: u8,
    flags: u8,
    /// `lsct` section type: `Some(3)` bounding divider, `Some(1)`
    /// group header, `None` for a plain raster layer.
    section: Option<i32>,
    /// The channel streams: `(id, marker + byte-count table + packed
    /// rows)` in R, G, B, A order.
    channels: Vec<(i16, Vec<u8>)>,
}

impl Record {
    /// A group divider (the record that sits *below* the children).
    /// Real writers flag it hidden, so it never shows as an empty
    /// layer in other apps' stacks.
    fn divider() -> Self {
        Self {
            name: "</Layer group>".to_owned(),
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
            blend: *b"norm",
            opacity: 255,
            flags: 0b10,
            section: Some(3),
            channels: Vec::new(),
        }
    }
}

/// Encodes page `page` (state 0) of `document` as a layered RGB/8 PSD.
/// Raster objects write their bank pixels at their bounds; groups
/// become the divider/header pair; every other object flattens through
/// compose into a full-canvas raster. The merged composite always
/// follows (maximize compatibility).
///
/// # Errors
/// [`ExportError::Unsupported`] for canvases outside the format's
/// reach, unrenderable documents, missing bank tiles, oversized
/// planes, or nesting/record counts beyond the writer's caps;
/// [`ExportError::PlaneSize`] for internal pixel-count mismatches.
pub fn encode(document: &Document, page: usize) -> Result<Vec<u8>, ExportError> {
    let page_data = document.pages.get(page).ok_or(ExportError::Unsupported {
        format: FORMAT,
        reason: format!("page {page} is outside the document"),
    })?;
    let width_f = page_data.info.canvas.width.round();
    let height_f = page_data.info.canvas.height.round();
    if width_f < 1.0 || height_f < 1.0 {
        return Err(ExportError::Unsupported {
            format: FORMAT,
            reason: "canvas rounds to fewer than one pixel on an axis".to_owned(),
        });
    }
    if width_f > f64::from(u16::MAX) || height_f > f64::from(u16::MAX) {
        return Err(ExportError::Unsupported {
            format: FORMAT,
            reason: format!(
                "canvas {}x{} exceeds the PSD u16 extent",
                page_data.info.canvas.width, page_data.info.canvas.height
            ),
        });
    }
    let (width, height) = (width_f as u32, height_f as u32);

    // Bottom-first record walk: model layers in stored order, one
    // record per cell element, groups expanded in place.
    let mut records: Vec<Record> = Vec::new();
    for layer in &page_data.layers.layers {
        let Some(cell) = layer.cells.first() else {
            continue;
        };
        for id in &cell.elements {
            let shown = layer.display;
            push_object(
                document,
                *id,
                shown,
                0,
                &page_data.info.canvas,
                &mut records,
            )?;
            if records.len() > MAX_RECORDS {
                return Err(ExportError::Unsupported {
                    format: FORMAT,
                    reason: format!("more than {MAX_RECORDS} layer records"),
                });
            }
        }
    }

    // The merged composite: the full page through compose, straight
    // alpha, three RLE channels pooled into one byte-count table.
    let rendered =
        compose::render(document, page, 0).map_err(|error| ExportError::Unsupported {
            format: FORMAT,
            reason: format!("composite render failed: {error}"),
        })?;
    if rendered.width != width as u16 || rendered.height != height as u16 {
        return Err(ExportError::Unsupported {
            format: FORMAT,
            reason: format!(
                "composite rendered {}x{}, canvas is {width}x{height}",
                rendered.width, rendered.height
            ),
        });
    }
    let composite = composite_section(&rendered.pixels, width, height)?;

    Ok(assemble(&records, width, height, &composite))
}

/// Appends the records for one object (recursing through groups).
fn push_object(
    document: &Document,
    id: doc::ObjectId,
    shown: bool,
    depth: usize,
    page_canvas: &doc::Canvas,
    records: &mut Vec<Record>,
) -> Result<(), ExportError> {
    let Some(object) = document.registry.get(id) else {
        return Err(ExportError::Unsupported {
            format: FORMAT,
            reason: format!("object {id:?} is dangling in the registry"),
        });
    };
    match object {
        ObjectData::Group(group) => {
            if depth >= MAX_GROUP_DEPTH {
                return Err(ExportError::Unsupported {
                    format: FORMAT,
                    reason: format!("groups nest deeper than {MAX_GROUP_DEPTH}"),
                });
            }
            // Bottom-first order: the bounding divider sits BELOW the
            // children, so it leads the run; the header closes it.
            records.push(Record::divider());
            for child in &group.children {
                push_object(
                    document,
                    *child,
                    shown && group.visible,
                    depth + 1,
                    page_canvas,
                    records,
                )?;
            }
            records.push(group_record(group, shown));
        }
        ObjectData::Image(image) => records.push(image_record(document, image, shown)?),
        other => records.push(flattened_record(document, id, other, shown, page_canvas)?),
    }
    Ok(())
}

/// The group header record — the one *above* the children, carrying
/// the group's real name, blend, opacity, and visibility.
fn group_record(group: &doc::GroupObject, shown: bool) -> Record {
    Record {
        name: group.name.clone().unwrap_or_else(|| "Group".to_owned()),
        top: 0,
        left: 0,
        bottom: 0,
        right: 0,
        blend: blend_key(group.blend),
        opacity: opacity_byte(group.opacity),
        flags: flag_byte(shown && group.visible),
        section: Some(1), // open folder; collapsed state is not carried
        channels: Vec::new(),
    }
}

/// The raster record for an `IMGv`: straight-alpha pixels extracted
/// from the tile bank at the object's bounds.
fn image_record(
    document: &Document,
    image: &doc::ImageObject,
    shown: bool,
) -> Result<Record, ExportError> {
    let (width, height) = (image.pixels.0.max(0), image.pixels.1.max(0));
    if width == 0 || height == 0 {
        return Err(ExportError::Unsupported {
            format: FORMAT,
            reason: "image object has an empty pixel extent".to_owned(),
        });
    }
    if width as u64 * height as u64 > MAX_PLANE_PIXELS as u64 {
        return Err(ExportError::Unsupported {
            format: FORMAT,
            reason: format!("image {width}x{height} exceeds the plane cap"),
        });
    }
    if image.tile_size != raster::TILE_SIDE as i64 {
        return Err(ExportError::Unsupported {
            format: FORMAT,
            reason: format!("tile size {} is not the bank's 128", image.tile_size),
        });
    }
    let left = rect_edge(image.location.x);
    let top = rect_edge(image.location.y);
    let right = rect_edge(image.location.x + width as f64);
    let bottom = rect_edge(image.location.y + height as f64);
    let (width, height) = (width as usize, height as usize);
    let mut rgba = vec![0u8; width * height * 4];
    let cols = width.div_ceil(raster::TILE_SIDE);
    let rows = height.div_ceil(raster::TILE_SIDE);
    for row in 0..rows {
        for col in 0..cols {
            let reference =
                image
                    .tiles
                    .get(row * cols + col)
                    .ok_or_else(|| ExportError::Unsupported {
                        format: FORMAT,
                        reason: "image object references fewer tiles than its extent".to_owned(),
                    })?;
            for y in 0..raster::TILE_SIDE {
                let py = row * raster::TILE_SIDE + y;
                if py >= height {
                    break;
                }
                for x in 0..raster::TILE_SIDE {
                    let px = col * raster::TILE_SIDE + x;
                    if px >= width {
                        break;
                    }
                    let pixel = tile_pixel(document, reference, x, y)?;
                    let at = (py * width + px) * 4;
                    rgba[at..at + 4].copy_from_slice(&pixel);
                }
            }
        }
    }
    record_from_pixels(
        image.name.clone().unwrap_or_else(|| "Layer".to_owned()),
        top,
        left,
        bottom,
        right,
        image.blend,
        image.opacity,
        shown && image.visible,
        &rgba,
        width,
    )
}

/// One straight-alpha pixel out of the tile bank.
fn tile_pixel(
    document: &Document,
    reference: &doc::TileRef,
    x: usize,
    y: usize,
) -> Result<[u8; 4], ExportError> {
    let at = (y * raster::TILE_SIDE + x) * 4;
    match reference {
        doc::TileRef::Tile(id) => {
            let tile = document
                .tiles
                .get(*id)
                .ok_or_else(|| ExportError::Unsupported {
                    format: FORMAT,
                    reason: format!("tile {id} is not in the bank"),
                })?;
            let premultiplied = &tile.pixels()[at..at + 4];
            Ok(straight_pixel(premultiplied))
        }
        doc::TileRef::Matte(raw) => {
            // TMCi is an ARGB raw int; straight alpha as stored.
            let value = u32::try_from(*raw).unwrap_or(0);
            Ok([
                ((value >> 16) & 0xff) as u8,
                ((value >> 8) & 0xff) as u8,
                (value & 0xff) as u8,
                ((value >> 24) & 0xff) as u8,
            ])
        }
    }
}

/// Un-premultiplies one pixel (compose's straight-alpha policy).
fn straight_pixel(premultiplied: &[u8]) -> [u8; 4] {
    let [r, g, b, a] = [
        premultiplied[0],
        premultiplied[1],
        premultiplied[2],
        premultiplied[3],
    ];
    if a == 0 || a == 255 {
        [r, g, b, a]
    } else {
        let un = |c: u8| ((u16::from(c) * 255) / u16::from(a)) as u8;
        [un(r), un(g), un(b), a]
    }
}

/// Renders one non-raster object alone (a probe document holding just
/// that element) and flattens it to a full-canvas raster record.
fn flattened_record(
    document: &Document,
    id: doc::ObjectId,
    object: &ObjectData,
    shown: bool,
    canvas: &doc::Canvas,
) -> Result<Record, ExportError> {
    let width_f = canvas.width.round();
    let height_f = canvas.height.round();
    let (width, height) = (width_f as usize, height_f as usize);
    if width == 0 || height == 0 {
        return Err(ExportError::Unsupported {
            format: FORMAT,
            reason: "canvas is empty; nothing to flatten onto".to_owned(),
        });
    }
    let mut probe = document.clone();
    probe.pages.clear();
    crate::raster::build_single_object_page(&mut probe, canvas.clone(), vec![id]);
    let rendered = compose::render(&probe, 0, 0).map_err(|error| ExportError::Unsupported {
        format: FORMAT,
        reason: format!("object flatten render failed: {error}"),
    })?;
    let rgba = compose::straight_alpha(&rendered.pixels);
    let name = object_name(object).unwrap_or_else(|| "Layer".to_owned());
    // Opacity is deliberately not read here — the render already baked
    // it into the pixels, so the record writes opaque (below).
    let (blend, visible) = match object {
        ObjectData::Path(path) => (path.blend, path.visible),
        ObjectData::Text(text) => (text.blend, text.visible),
        _ => (0, true),
    };
    // The lone-object render bakes the object's opacity into its
    // pixels (alpha against the empty backdrop), so the record writes
    // opaque and keeps only the blend key live — writing both would
    // dim twice on read-back. Slices/symbols carry no blend fields.
    record_from_pixels(
        name,
        0,
        0,
        height as i32,
        width as i32,
        blend,
        1000,
        shown && visible,
        &rgba,
        width,
    )
}

/// The object's `OBNs`, when present and non-empty.
fn object_name(object: &ObjectData) -> Option<String> {
    match object {
        ObjectData::Path(path) => path.name.clone(),
        ObjectData::Text(text) => text.name.clone(),
        ObjectData::Image(image) => image.name.clone(),
        ObjectData::Group(group) => group.name.clone(),
        ObjectData::Slice(slice) => slice.name.clone(),
        ObjectData::SymbolInstance(symbol) => symbol.name.clone(),
    }
}

/// Builds a four-channel RLE record from a straight-alpha plane.
#[allow(clippy::too_many_arguments)]
fn record_from_pixels(
    name: String,
    top: i32,
    left: i32,
    bottom: i32,
    right: i32,
    blend: i64,
    opacity: i64,
    visible: bool,
    rgba: &[u8],
    width: usize,
) -> Result<Record, ExportError> {
    let pixels = rgba.len() / 4;
    if rgba.len() != pixels * 4 || width == 0 || pixels % width != 0 {
        return Err(ExportError::PlaneSize {
            width: width as u32,
            height: (pixels / width.max(1)) as u32,
            len: rgba.len(),
        });
    }
    let height = pixels / width;
    // Planes are channel-major: all red rows, then green, blue, alpha.
    let mut planes: [Vec<u8>; 4] = std::array::from_fn(|_| vec![0u8; pixels]);
    for (index, pixel) in rgba.chunks_exact(4).enumerate() {
        for (channel, value) in pixel.iter().enumerate() {
            planes[channel][index] = *value;
        }
    }
    let channels = planes
        .iter()
        .zip([0i16, 1, 2, -1])
        .map(|(plane, id)| (id, rle_channel(width, height, plane)))
        .collect();
    Ok(Record {
        name,
        top,
        left,
        bottom,
        right,
        blend: blend_key(blend),
        opacity: opacity_byte(opacity),
        flags: flag_byte(visible),
        section: None,
        channels,
    })
}

/// PackBits-encodes one scanline (2+ equal bytes become a run;
/// everything else rides in literals of up to 128).
fn pack_line(row: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(row.len() + row.len() / 2 + 2);
    let mut at = 0;
    while at < row.len() {
        let mut run = 1;
        while at + run < row.len() && row[at + run] == row[at] && run < 128 {
            run += 1;
        }
        if run >= 2 {
            out.push((1 - run as i8) as u8);
            out.push(row[at]);
            at += run;
        } else {
            let start = at;
            at += 1;
            while at < row.len()
                && at - start < 128
                && (at + 1 >= row.len() || row[at] != row[at + 1])
            {
                at += 1;
            }
            out.push((at - start - 1) as u8);
            out.extend_from_slice(&row[start..at]);
        }
    }
    out
}

/// One channel's stream: the RLE marker, a per-scanline byte-count
/// table, then the packed rows (the layer-channel layout — each
/// channel's data is self-contained inside its own length).
fn rle_channel(width: usize, height: usize, plane: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + height * 2 + plane.len());
    out.extend_from_slice(&1u16.to_be_bytes());
    let mut packed = Vec::with_capacity(plane.len());
    for row in 0..height {
        let line = pack_line(&plane[row * width..(row + 1) * width]);
        out.extend_from_slice(&(line.len() as u16).to_be_bytes());
        packed.extend_from_slice(&line);
    }
    out.extend_from_slice(&packed);
    out
}

/// The merged-composite section body: u16 compression, ONE pooled
/// byte-count table (rows x channels), one packed stream — written
/// length-less, the layout every real consumer expects (see below).
fn composite_section(
    premultiplied: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, ExportError> {
    let width = width as usize;
    let height = height as usize;
    let pixels = width * height;
    if premultiplied.len() != pixels * 4 {
        return Err(ExportError::PlaneSize {
            width: width as u32,
            height: height as u32,
            len: premultiplied.len(),
        });
    }
    // Three planes (the header declares 3 channels): the composite is
    // validation-only for layered files and the flat-document fallback
    // reads it the same way.
    let mut planes: [Vec<u8>; 3] = std::array::from_fn(|_| vec![0u8; pixels]);
    for (index, pixel) in premultiplied.chunks_exact(4).enumerate() {
        let pixel = straight_pixel(pixel);
        planes[0][index] = pixel[0];
        planes[1][index] = pixel[1];
        planes[2][index] = pixel[2];
    }
    let mut packed = Vec::with_capacity(pixels * 3);
    let mut out = Vec::with_capacity(2 + height * 6 + pixels * 3);
    out.extend_from_slice(&1u16.to_be_bytes());
    for plane in &planes {
        for row in 0..height {
            let line = pack_line(&plane[row * width..(row + 1) * width]);
            out.extend_from_slice(&(line.len() as u16).to_be_bytes());
            packed.extend_from_slice(&line);
        }
    }
    out.extend_from_slice(&packed);
    // No u32 length prefix: the spec allows one, but ag-psd,
    // ImageMagick, and PIL all write/read the composite bare — the
    // prefixed form made PIL fail the composite by exactly 4 bytes.
    // The reader accepts both.
    Ok(out)
}

/// Serializes the header, the layer section over `records`, and the
/// composite into the final byte stream.
fn assemble(records: &[Record], width: u32, height: u32, composite: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"8BPS");
    out.extend_from_slice(&1u16.to_be_bytes()); // version: PSD
    out.extend_from_slice(&[0; 6]); // reserved
    out.extend_from_slice(&3u16.to_be_bytes()); // channels
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&8u16.to_be_bytes()); // depth
    out.extend_from_slice(&3u16.to_be_bytes()); // RGB
    section(&mut out, &[]); // color mode data
    section(&mut out, &[]); // image resources

    // Layer-info: count, records, then the channel-stream block (the
    // spec layout — all streams after all records).
    let mut info = Vec::new();
    info.extend_from_slice(&(records.len() as u16).to_be_bytes());
    for record in records {
        write_record_head(&mut info, record);
    }
    for record in records {
        for (_, stream) in &record.channels {
            info.extend_from_slice(stream);
        }
    }
    // Layer-and-mask section: layer-info, then the global layer mask
    // info (empty), wrapped.
    let mut inner = section_bytes(&info);
    inner.extend_from_slice(&0u32.to_be_bytes());
    let layer_and_mask = section_bytes(&inner);
    out.extend_from_slice(&layer_and_mask);
    out.extend_from_slice(composite);
    out
}

/// One record's fixed head: rect, channel infos, blend/opacity/flags,
/// and the extra-data block (mask 0, ranges 0, pascal name, `luni`,
/// `lsct` for groups).
fn write_record_head(out: &mut Vec<u8>, record: &Record) {
    out.extend_from_slice(&record.top.to_be_bytes());
    out.extend_from_slice(&record.left.to_be_bytes());
    out.extend_from_slice(&record.bottom.to_be_bytes());
    out.extend_from_slice(&record.right.to_be_bytes());
    out.extend_from_slice(&(record.channels.len() as u16).to_be_bytes());
    for (id, stream) in &record.channels {
        out.extend_from_slice(&id.to_be_bytes());
        out.extend_from_slice(&(stream.len() as u32).to_be_bytes());
    }
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(&record.blend);
    out.push(record.opacity);
    out.push(0); // clipping: never written (no model clipping pair)
    out.push(record.flags);
    out.push(0); // filler

    let mut extra = Vec::new();
    extra.extend_from_slice(&0u32.to_be_bytes()); // no layer mask
    extra.extend_from_slice(&0u32.to_be_bytes()); // no blending ranges
    extra.extend_from_slice(&pascal(&record.name));
    // luni: the Unicode name (the reader's preferred source).
    let mut luni = (record.name.encode_utf16().count() as u32)
        .to_be_bytes()
        .to_vec();
    for unit in record.name.encode_utf16() {
        luni.extend_from_slice(&unit.to_be_bytes());
    }
    block(&mut extra, b"luni", &luni);
    if let Some(section) = record.section {
        let mut lsct = section.to_be_bytes().to_vec();
        if section != 3 {
            // Group headers carry the blend key + subtype (the
            // Photoshop-compatible form ag-psd writes).
            lsct.extend_from_slice(b"8BIM");
            lsct.extend_from_slice(&record.blend);
            lsct.extend_from_slice(&0u32.to_be_bytes());
        }
        block(&mut extra, b"lsct", &lsct);
    }
    out.extend_from_slice(&section_bytes(&extra));
}

/// An `8BIM` info block: signature, key, length, body padded to 4.
fn block(out: &mut Vec<u8>, key: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(key);
    out.extend_from_slice(&section_bytes(body));
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

/// A pascal string: length byte + bytes, padded so the total is a
/// multiple of 4.
fn pascal(name: &str) -> Vec<u8> {
    let bytes = &name.as_bytes()[..name.len().min(255)];
    let mut out = Vec::with_capacity(bytes.len() + 4);
    out.push(bytes.len() as u8);
    out.extend_from_slice(bytes);
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out
}

/// Length-prefixed section bytes.
fn section_bytes(body: &[u8]) -> Vec<u8> {
    let mut out = (body.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(body);
    out
}

/// Appends a length-prefixed section.
fn section(out: &mut Vec<u8>, body: &[u8]) {
    out.extend_from_slice(&section_bytes(body));
}

/// A model blend value's PSD key (the exact inverse of the reader's
/// table); values with no PSD equivalent land on `normal` — the
/// checklist records the limitation.
fn blend_key(value: i64) -> [u8; 4] {
    match value {
        2 => *b"mul ",
        3 => *b"diff",
        4 => *b"over",
        5 => *b"scrn",
        9 => *b"dark",
        10 => *b"lite",
        11 => *b"hue ",
        12 => *b"sat ",
        13 => *b"colr",
        14 => *b"lum ",
        32 => *b"smud",
        33 => *b"hLit",
        34 => *b"sLit",
        36 => *b"div ",
        37 => *b"idiv",
        _ => *b"norm",
    }
}

/// Per-mille opacity to the record's u8: `(per mille * 255 + 500) /
/// 1000`, half-up — the exact inverse of the reader's widening.
fn opacity_byte(per_mille: i64) -> u8 {
    let clamped = per_mille.clamp(0, 1000);
    ((clamped * 255 + 500) / 1000) as u8
}

/// Visibility to the record's flags byte: bit 1 set means hidden.
fn flag_byte(visible: bool) -> u8 {
    if visible { 0 } else { 0b10 }
}

/// Rounds a canvas coordinate into the record's i32 rect edges.
fn rect_edge(coordinate: f64) -> i32 {
    let rounded = coordinate.round();
    if rounded > f64::from(i32::MAX) {
        i32::MAX
    } else if rounded < f64::from(i32::MIN) {
        i32::MIN
    } else {
        rounded as i32
    }
}

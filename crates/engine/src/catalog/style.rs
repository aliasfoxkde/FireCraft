//! Style-library commands: the CS6 Styles panel verbs. The library
//! itself is `Document.styles` (`MKBv`'s `ZSLv`, one `ZSTv` bundle per
//! entry); these commands apply a bundle onto objects and write an
//! object's appearance back into the library.
//!
//! Command ids drop the `Style` infix the CS6 DOM verbs carry
//! (`applyStyle`, `redefineStyleByIndex`, …) the same way
//! `object.setFill` drops it from `setFill` — the catalog area already
//! names the thing. Targets are named explicitly in the payload: the
//! DOM verbs act on "the selection", and the engine's commands carry
//! their targets so a journal replays without ambient state.
//!
//! Application scope: an object-style bundle is copied onto paths
//! (paint, fill handles, brush placement, fill-on-top, effects). Text
//! styles (`ZSOi` `0x1f`) carry a text-attribute block onto text
//! objects — that half waits on the M6 text engine's style surface and
//! is a recorded follow-up.

use base64::Engine as _;
use doc::{Document, ObjectData, ObjectId, PaintStyle, StyleEntry, StyleText};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the style-library commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(StyleApply)?;
    registry.register(StyleRedefineByIndex)?;
    registry.register(StyleRedefineByName)?;
    registry.register(StyleRedefineFromSelection)?;
    registry.register(StyleImportLibrary)?;
    registry.register(StyleImportLibraryTiles)?;
    registry.register(StyleScaleStyles)
}

/// The appearance half of a [`StyleEntry`] a path carries: everything
/// `style.apply` copies and the redefine verbs capture back.
struct Appearance {
    paint: PaintStyle,
    fill_start: Option<geom::Point>,
    fill_end: Option<geom::Point>,
    fill_third: Option<geom::Point>,
    brush_placement: Option<i64>,
    fill_on_top: Option<bool>,
    effects: Vec<doc::Effect>,
}

fn path_appearance(object: &ObjectData) -> Option<Appearance> {
    let ObjectData::Path(path) = object else {
        return None;
    };
    Some(Appearance {
        paint: path.paint.clone().unwrap_or(PaintStyle {
            stroke_raw: 0,
            fill_raw: 0,
            fill: None,
        }),
        fill_start: path.fill_start,
        fill_end: path.fill_end,
        fill_third: path.fill_third,
        brush_placement: path.brush_placement,
        fill_on_top: path.fill_on_top,
        effects: path.effects.clone(),
    })
}

fn apply_appearance(path: &mut doc::PathObject, appearance: &Appearance) {
    path.paint = Some(appearance.paint.clone());
    path.fill_start = appearance.fill_start;
    path.fill_end = appearance.fill_end;
    path.fill_third = appearance.fill_third;
    path.brush_placement = appearance.brush_placement;
    path.fill_on_top = appearance.fill_on_top;
    path.effects = appearance.effects.clone();
}

/// `style.apply` — the CS6 DOM verb `applyStyle`: copies one library
/// bundle onto the named targets. Effects replace the targets' live
/// effects wholesale (the panel applies the style, it does not merge).
/// The whole selection refuses atomically if any target is not a path.
struct StyleApply;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct StyleApplyPayload {
    /// Style-library index to apply.
    index: usize,
    /// Target object ids (the DOM verb's selection, named).
    ids: Vec<ObjectId>,
}

impl TypedCommand for StyleApply {
    type Payload = StyleApplyPayload;

    fn name(&self) -> &'static str {
        "style.apply"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "style.apply";
        if payload.ids.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "no target objects".to_owned(),
            });
        }
        let style =
            document
                .styles
                .get(payload.index)
                .ok_or_else(|| EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "style index {} out of range ({} styles)",
                        payload.index,
                        document.styles.len()
                    ),
                })?;
        let appearance = Appearance {
            paint: style.paint.clone(),
            fill_start: Some(style.fill_start),
            fill_end: Some(style.fill_end),
            fill_third: Some(style.fill_third),
            brush_placement: Some(style.brush_placement),
            fill_on_top: Some(style.fill_on_top),
            effects: style.effects.clone(),
        };
        // Capture before mutating; a mixed selection refuses whole.
        let before: Vec<ObjectData> = payload
            .ids
            .iter()
            .map(|id| access::object(document, NAME, *id).cloned())
            .collect::<Result<_, _>>()?;
        for object in &before {
            if !matches!(object, ObjectData::Path(_)) {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: "style application targets paths".to_owned(),
                });
            }
        }
        for (id, _) in payload.ids.iter().zip(&before) {
            let slot =
                document
                    .registry
                    .get_mut(*id)
                    .ok_or_else(|| EngineError::DeltaMismatch {
                        detail: format!("object {} vanished mid-command", id.index()),
                    })?;
            let ObjectData::Path(path) = slot else {
                return Err(EngineError::DeltaMismatch {
                    detail: format!("object {} changed kind mid-command", id.index()),
                });
            };
            apply_appearance(path, &appearance);
        }
        Ok(Delta::Group(
            payload
                .ids
                .iter()
                .zip(before)
                .map(|(id, object)| Delta::Object {
                    id: *id,
                    inverse: ObjectInverse::Mutated {
                        object: Box::new(object),
                    },
                })
                .collect(),
        ))
    }
}

/// Writes `source`'s appearance back into `document.styles[index]`:
/// paint, fill handles (when the path carries them), brush placement,
/// fill-on-top, and effects. Name/flags/kind and the text block stay
/// as they were — the CS6 verbs redefine appearance, not identity.
fn redefine_at(
    document: &mut Document,
    name: &'static str,
    index: usize,
    source: ObjectId,
) -> Result<Delta, EngineError> {
    if index >= document.styles.len() {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!(
                "style index {index} out of range ({} styles)",
                document.styles.len()
            ),
        });
    }
    let appearance = path_appearance(access::object(document, name, source)?).ok_or_else(|| {
        EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("object {} is not a path", source.index()),
        }
    })?;
    let before = document.styles.clone();
    let entry = &mut document.styles[index];
    entry.paint = appearance.paint;
    if let Some(point) = appearance.fill_start {
        entry.fill_start = point;
    }
    if let Some(point) = appearance.fill_end {
        entry.fill_end = point;
    }
    if let Some(point) = appearance.fill_third {
        entry.fill_third = point;
    }
    if let Some(placement) = appearance.brush_placement {
        entry.brush_placement = placement;
    }
    if let Some(on_top) = appearance.fill_on_top {
        entry.fill_on_top = on_top;
    }
    entry.effects = appearance.effects;
    Ok(Delta::Styles { before })
}

/// The library index of the first style named `name`.
fn index_of_name(document: &Document, name: &str) -> Option<usize> {
    document.styles.iter().position(|style| style.name == name)
}

/// `style.redefineByIndex` — the CS6 DOM verb `redefineStyleByIndex`:
/// the named object's appearance becomes the style at `index`.
struct StyleRedefineByIndex;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct StyleRedefineByIndexPayload {
    /// Style-library index to redefine.
    index: usize,
    /// The object whose appearance is captured.
    source: ObjectId,
}

impl TypedCommand for StyleRedefineByIndex {
    type Payload = StyleRedefineByIndexPayload;

    fn name(&self) -> &'static str {
        "style.redefineByIndex"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "style.redefineByIndex";
        redefine_at(document, NAME, payload.index, payload.source)
    }
}

/// `style.redefineByName` — the CS6 DOM verb `redefineStyleByName`:
/// the named object's appearance becomes the first style carrying
/// `name`.
struct StyleRedefineByName;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct StyleRedefineByNamePayload {
    /// Style-library name to redefine (first match wins).
    name: String,
    /// The object whose appearance is captured.
    source: ObjectId,
}

impl TypedCommand for StyleRedefineByName {
    type Payload = StyleRedefineByNamePayload;

    fn name(&self) -> &'static str {
        "style.redefineByName"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "style.redefineByName";
        let index =
            index_of_name(document, &payload.name).ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("no style named {:?}", payload.name),
            })?;
        redefine_at(document, NAME, index, payload.source)
    }
}

/// `style.redefineFromSelection` — the CS6 DOM verb
/// `redefineStyleFromSelection`: defines `name` from the object's
/// appearance, replacing an existing style of that name or appending a
/// new one (object style, `ZSOi` 8; the modal corpus flags `0x1ff` and
/// default text block).
struct StyleRedefineFromSelection;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct StyleRedefineFromSelectionPayload {
    /// Style name to define or replace.
    name: String,
    /// The object whose appearance is captured.
    source: ObjectId,
}

impl TypedCommand for StyleRedefineFromSelection {
    type Payload = StyleRedefineFromSelectionPayload;

    fn name(&self) -> &'static str {
        "style.redefineFromSelection"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "style.redefineFromSelection";
        if payload.name.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "a style needs a name".to_owned(),
            });
        }
        if let Some(index) = index_of_name(document, &payload.name) {
            return redefine_at(document, NAME, index, payload.source);
        }
        let appearance = path_appearance(access::object(document, NAME, payload.source)?)
            .ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} is not a path", payload.source.index()),
            })?;
        let entry = StyleEntry {
            name: payload.name.clone(),
            flags: 0x1ff,
            kind: 8,
            paint: appearance.paint,
            fill_start: appearance.fill_start.unwrap_or(geom::Point::ZERO),
            fill_end: appearance.fill_end.unwrap_or(geom::Point::ZERO),
            fill_third: appearance.fill_third.unwrap_or(geom::Point::ZERO),
            brush_placement: appearance.brush_placement.unwrap_or(0),
            fill_on_top: appearance.fill_on_top.unwrap_or(true),
            effects: appearance.effects,
            text: StyleText::default(),
        };
        let before = document.styles.clone();
        document.styles.push(entry);
        Ok(Delta::Styles { before })
    }
}

/// `style.importLibrary` — appends whole entries to the library: the
/// host path for opening a CS6 style library (`.stl`). The entries
/// ride the payload (like `library.importSymbol`'s definition) so a
/// journal replays the import; each entry's preserved texture
/// subtrees stay host-side in the unknown bag, keyed under the entry's
/// final `ZSTv` index.
struct StyleImportLibrary;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct StyleImportLibraryPayload {
    /// The entries to append, file order.
    entries: Vec<StyleEntry>,
}

impl TypedCommand for StyleImportLibrary {
    type Payload = StyleImportLibraryPayload;

    fn name(&self) -> &'static str {
        "style.importLibrary"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        if payload.entries.is_empty() {
            return Ok(Delta::Noop);
        }
        let before = document.styles.clone();
        document.styles.extend(payload.entries.iter().cloned());
        Ok(Delta::Styles { before })
    }
}

/// `style.importLibraryTiles` — joins a style library's `mkBT` rasters
/// to the document tile bank under explicit bank ids, so the imported
/// records' pattern fills (and their bagged texture subtrees) point at
/// tiles this document actually holds. The wire form is straight RGBA8
/// (the `TileImage::to_rgba8` export); premultiplication happens here,
/// where the bank's premultiplied contract lives. Ids are chosen
/// host-side against the live bank; a colliding id is accepted only
/// when the content matches — a journal replay of the same import is
/// a no-op, a doctored one is refused.
struct StyleImportLibraryTiles;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct StyleImportLibraryTilesPayload {
    /// The library's tiles, already re-keyed to their final bank ids.
    tiles: Vec<StyleLibraryTile>,
}

/// One `mkBT` raster on the import wire.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct StyleLibraryTile {
    /// The bank id the entries' `TIDi` slots name.
    id: u32,
    /// 128×128 straight RGBA8, base64 (`TILE_BYTES` bytes decoded).
    rgba: String,
}

impl TypedCommand for StyleImportLibraryTiles {
    type Payload = StyleImportLibraryTilesPayload;

    fn name(&self) -> &'static str {
        "style.importLibraryTiles"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "style.importLibraryTiles";
        // Validate every tile before mutating any — a refused call
        // must leave the bank exactly as it was.
        let mut fresh: Vec<(u32, [u8; raster::TILE_BYTES])> = Vec::new();
        for tile in &payload.tiles {
            let rgba = base64::engine::general_purpose::STANDARD
                .decode(&tile.rgba)
                .map_err(|error| EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("tile {} is not base64: {error}", tile.id),
                })?;
            let rgba: [u8; raster::TILE_BYTES] =
                rgba.try_into()
                    .map_err(|len: Vec<u8>| EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: format!(
                            "tile {} carries {} bytes, a tile is {}",
                            tile.id,
                            len.len(),
                            raster::TILE_BYTES
                        ),
                    })?;
            let mut premultiplied = rgba;
            for px in premultiplied.chunks_exact_mut(4) {
                let a = u32::from(px[3]);
                for channel in px.iter_mut().take(3) {
                    *channel = ((u32::from(*channel) * a + 127) / 255) as u8;
                }
            }
            if let Some(existing) = document.tiles.get(tile.id) {
                if existing.pixels()[..] == premultiplied[..] {
                    continue; // journal replay of the same import
                }
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("bank slot {} already holds different pixels", tile.id),
                });
            }
            fresh.push((tile.id, premultiplied));
        }
        for (tile_id, pixels) in &fresh {
            document
                .tiles
                .insert(*tile_id, raster::Tile::from_rgba8(*pixels));
        }
        Ok(Delta::Bank {
            added: fresh.into_iter().map(|(tile_id, _)| tile_id).collect(),
        })
    }
}

/// `style.scaleStyles` — the CS6 `scaleStyles` verb: multiplies every
/// library entry's text size (`PTSf`) by the factor — the slot that
/// follows object scaling. Effect parameters stay untouched (they are
/// strings on the wire, semantics unmodeled). The chosen reading is
/// pinned pending corpus playback; a counterexample surfaces as an
/// encode diff.
struct StyleScaleStyles;
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct StyleScaleStylesPayload {
    /// Multiplier on every style's point size.
    scale: f64,
}

impl TypedCommand for StyleScaleStyles {
    type Payload = StyleScaleStylesPayload;

    fn name(&self) -> &'static str {
        "style.scaleStyles"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "style.scaleStyles";
        if !payload.scale.is_finite() || payload.scale <= 0.0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("scale {} is not a positive number", payload.scale),
            });
        }
        if document.styles.is_empty() || payload.scale == 1.0 {
            return Ok(Delta::Noop);
        }
        let before = document.styles.clone();
        for entry in &mut document.styles {
            entry.text.size *= payload.scale;
        }
        Ok(Delta::Styles { before })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::error::EngineError;
    use crate::registry::CommandRegistry;
    use doc::{
        Cell, Contour, Document, Effect, EffectProperty, EffectValue, Fill, FillShape, Layer,
        LayerList, Page, PageInfo, PathObject, PathPoint,
    };
    use serde_json::json;

    fn point(x: f64, y: f64) -> PathPoint {
        PathPoint {
            anchor: geom::Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        }
    }

    /// A square path carrying `paint`, `handles`, and one effect — the
    /// source redefine verbs capture and the apply verb overwrites.
    fn square(x0: f64, y0: f64, x1: f64, y1: f64, paint: Option<PaintStyle>) -> ObjectData {
        ObjectData::Path(PathObject {
            contours: vec![Contour {
                closed: true,
                point_count: 4,
                points: vec![point(x0, y0), point(x1, y0), point(x1, y1), point(x0, y1)],
            }],
            primitive: None,
            even_odd: false,
            paint,
            fill_start: Some(geom::Point::new(4.0, 4.0)),
            fill_end: Some(geom::Point::new(12.0, 4.0)),
            fill_third: None,
            brush_placement: Some(1),
            fill_on_top: Some(false),
            locked: false,
            visible: true,
            opacity: 1000,
            blend: 0,
            name: Some("square".to_owned()),
            effects: vec![Effect {
                category: "Shadow and Glow".to_owned(),
                internal_name: "dropShadow".to_owned(),
                properties: vec![EffectProperty {
                    key: "gaussian_blur_radius".to_owned(),
                    value: EffectValue::Text("5".to_owned()),
                }],
            }],
        })
    }

    /// One page, one layer, one cell holding the objects.
    fn document_with(objects: Vec<ObjectData>) -> (Document, Vec<ObjectId>) {
        let mut document = Document::default();
        let ids: Vec<ObjectId> = objects
            .iter()
            .map(|object| document.registry.insert(object.clone()))
            .collect();
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
            layers: LayerList {
                layers: vec![Layer {
                    name: "Layer 0".to_owned(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: ids.clone(),
                    }],
                    ..Layer::default()
                }],
            },
            grid: None,
            guides: Default::default(),
            export: None,
        });
        (document, ids)
    }

    /// The fixture library: two entries, the first a solid pastel with
    /// handles and an effect, the second sharing the first's name
    /// target for redefine-by-name's first-match rule — distinct names
    /// would not exercise it.
    fn library() -> Vec<StyleEntry> {
        vec![
            StyleEntry {
                name: "Pastels 001".to_owned(),
                flags: 0x1ff,
                kind: 8,
                paint: PaintStyle {
                    stroke_raw: 0xff004422,
                    fill_raw: 0xffa24b7f,
                    fill: Some(Fill {
                        shape: FillShape::Solid,
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
                fill_start: geom::Point::new(0.0, 0.0),
                fill_end: geom::Point::new(32.0, 0.0),
                fill_third: geom::Point::new(32.0, 16.0),
                brush_placement: 2,
                fill_on_top: true,
                effects: vec![Effect {
                    category: "Shadow and Glow".to_owned(),
                    internal_name: "dropShadow".to_owned(),
                    properties: vec![
                        EffectProperty {
                            key: "EffectMoaID".to_owned(),
                            value: EffectValue::Text("guid".to_owned()),
                        },
                        EffectProperty {
                            key: "gaussian_blur_radius".to_owned(),
                            value: EffectValue::Text("5".to_owned()),
                        },
                    ],
                }],
                text: StyleText::default(),
            },
            StyleEntry {
                name: "Pastels 001".to_owned(),
                flags: 0x1ff,
                kind: 8,
                paint: PaintStyle {
                    stroke_raw: 1,
                    fill_raw: 2,
                    fill: None,
                },
                fill_start: geom::Point::ZERO,
                fill_end: geom::Point::ZERO,
                fill_third: geom::Point::ZERO,
                brush_placement: 0,
                fill_on_top: false,
                effects: Vec::new(),
                text: StyleText::default(),
            },
        ]
    }

    fn document_bytes(document: &Document) -> Vec<u8> {
        serde_json::to_vec(document).unwrap()
    }

    fn apply_by_name(
        document: &mut Document,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<Delta, EngineError> {
        let mut registry = CommandRegistry::new();
        install(&mut registry).unwrap();
        registry.execute(document, name, payload)
    }

    fn path_paint(document: &Document, id: ObjectId) -> PaintStyle {
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => path.paint.clone().unwrap(),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn apply_copies_the_bundle_and_undo_restores() {
        let (mut document, ids) = document_with(vec![
            square(
                0.0,
                0.0,
                8.0,
                8.0,
                Some(PaintStyle {
                    stroke_raw: 0x11,
                    fill_raw: 0x22,
                    fill: None,
                }),
            ),
            square(4.0, 4.0, 12.0, 12.0, None),
        ]);
        document.styles = library();
        let before = document_bytes(&document);

        let delta = apply_by_name(
            &mut document,
            "style.apply",
            json!({ "index": 0, "ids": [ids[0], ids[1]] }),
        )
        .unwrap();

        for id in ids {
            let paint = path_paint(&document, id);
            assert_eq!(paint.fill_raw, 0xffa24b7f);
            assert_eq!(paint.stroke_raw, 0xff004422);
            assert!(paint.fill.is_some());
            let path = match document.registry.get(id).unwrap() {
                ObjectData::Path(path) => path,
                other => panic!("wrong object kind: {other:?}"),
            };
            assert_eq!(path.fill_start.unwrap(), geom::Point::new(0.0, 0.0));
            assert_eq!(path.fill_end.unwrap(), geom::Point::new(32.0, 0.0));
            assert_eq!(path.fill_third.unwrap(), geom::Point::new(32.0, 16.0));
            assert_eq!(path.brush_placement, Some(2));
            assert_eq!(path.fill_on_top, Some(true));
            assert_eq!(path.effects.len(), 1);
            assert_eq!(path.effects[0].internal_name, "dropShadow");
        }

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn apply_refuses_atomically() {
        let (mut document, ids) = document_with(vec![
            square(0.0, 0.0, 8.0, 8.0, None),
            ObjectData::Image(doc::ImageObject {
                location: geom::Point::ZERO,
                offset: geom::Point::ZERO,
                pixels: (4, 4),
                tile_size: 128,
                tiles: Vec::new(),
                name: None,
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                effects: Vec::new(),
            }),
        ]);
        document.styles = library();
        let before = document_bytes(&document);

        for (name, payload) in [
            ("index out of range", json!({ "index": 7, "ids": [ids[0]] })),
            ("no targets", json!({ "index": 0, "ids": [] })),
            (
                "mixed selection",
                json!({ "index": 0, "ids": [ids[0], ids[1]] }),
            ),
            (
                "stale id",
                json!({ "index": 0, "ids": [doc::ObjectId::from_parts(99, 0)] }),
            ),
        ] {
            let error = apply_by_name(&mut document, "style.apply", payload)
                .err()
                .unwrap_or_else(|| panic!("{name}: expected a refusal"));
            assert!(
                matches!(
                    error,
                    EngineError::InvalidState { .. } | EngineError::MissingTarget { .. }
                ),
                "{name}: {error:?}"
            );
            assert_eq!(document_bytes(&document), before, "{name} mutated");
        }
    }

    #[test]
    fn redefine_by_index_writes_the_appearance_back() {
        let (mut document, ids) = document_with(vec![square(
            0.0,
            0.0,
            8.0,
            8.0,
            Some(PaintStyle {
                stroke_raw: 0xaa,
                fill_raw: 0xbb,
                fill: None,
            }),
        )]);
        document.styles = vec![StyleEntry {
            name: "keep me".to_owned(),
            flags: 7,
            kind: 8,
            paint: PaintStyle {
                stroke_raw: 0,
                fill_raw: 0,
                fill: None,
            },
            fill_start: geom::Point::ZERO,
            fill_end: geom::Point::ZERO,
            fill_third: geom::Point::ZERO,
            brush_placement: 0,
            fill_on_top: false,
            effects: Vec::new(),
            text: StyleText::default(),
        }];
        let before = document_bytes(&document);

        let delta = apply_by_name(
            &mut document,
            "style.redefineByIndex",
            json!({ "index": 0, "source": ids[0] }),
        )
        .unwrap();

        let entry = &document.styles[0];
        assert_eq!(entry.name, "keep me");
        assert_eq!(entry.flags, 7, "identity stays, appearance goes");
        assert_eq!(entry.paint.stroke_raw, 0xaa);
        assert_eq!(entry.paint.fill_raw, 0xbb);
        assert_eq!(entry.fill_start, geom::Point::new(4.0, 4.0));
        assert_eq!(entry.fill_end, geom::Point::new(12.0, 4.0));
        assert_eq!(entry.fill_third, geom::Point::ZERO, "absent handle stays");
        assert_eq!(entry.brush_placement, 1);
        assert!(!entry.fill_on_top);
        assert_eq!(entry.effects.len(), 1);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn redefine_by_name_hits_the_first_match_only() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0, None)]);
        document.styles = library();

        apply_by_name(
            &mut document,
            "style.redefineByName",
            json!({ "name": "Pastels 001", "source": ids[0] }),
        )
        .unwrap();

        assert_eq!(document.styles[0].paint.fill_raw, 0, "square carries none");
        assert_eq!(document.styles[1].paint.fill_raw, 2, "second stays");
    }

    #[test]
    fn redefine_from_selection_upserts() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0, None)]);
        document.styles = library();
        let count = document.styles.len();

        apply_by_name(
            &mut document,
            "style.redefineFromSelection",
            json!({ "name": "Fresh", "source": ids[0] }),
        )
        .unwrap();
        assert_eq!(document.styles.len(), count + 1);
        let entry = document.styles.last().unwrap();
        assert_eq!(entry.name, "Fresh");
        assert_eq!(entry.kind, 8);
        assert_eq!(entry.flags, 0x1ff);
        assert_eq!(entry.text, StyleText::default());
        assert_eq!(entry.effects.len(), 1);

        // Same name again replaces instead of appending.
        apply_by_name(
            &mut document,
            "style.redefineFromSelection",
            json!({ "name": "Fresh", "source": ids[0] }),
        )
        .unwrap();
        assert_eq!(document.styles.len(), count + 1);
    }

    #[test]
    fn redefine_refuses() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0, None)]);
        document.styles = library();
        let before = document_bytes(&document);

        for (name, command, payload) in [
            (
                "byIndex out of range",
                "style.redefineByIndex",
                json!({ "index": 9, "source": ids[0] }),
            ),
            (
                "byName missing",
                "style.redefineByName",
                json!({ "name": "Nope", "source": ids[0] }),
            ),
            (
                "empty name",
                "style.redefineFromSelection",
                json!({ "name": "", "source": ids[0] }),
            ),
            (
                "source not a path",
                "style.redefineByIndex",
                json!({ "index": 0, "source": doc::ObjectId::from_parts(99, 0) }),
            ),
        ] {
            let error = apply_by_name(&mut document, command, payload)
                .err()
                .unwrap_or_else(|| panic!("{name}: expected a refusal"));
            assert!(
                matches!(
                    error,
                    EngineError::InvalidState { .. } | EngineError::MissingTarget { .. }
                ),
                "{name}: {error:?}"
            );
            assert_eq!(document_bytes(&document), before, "{name} mutated");
        }
    }

    #[test]
    fn scale_styles_multiplies_sizes_and_undo_restores() {
        let (mut document, _ids) = document_with(vec![]);
        document.styles = library();

        let noop =
            apply_by_name(&mut document, "style.scaleStyles", json!({ "scale": 1.0 })).unwrap();
        assert!(matches!(noop, Delta::Noop), "scale 1 changes nothing");

        let delta =
            apply_by_name(&mut document, "style.scaleStyles", json!({ "scale": 2.0 })).unwrap();
        assert_eq!(document.styles[0].text.size, 24.0);
        assert_eq!(document.styles[1].text.size, 24.0);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.styles[0].text.size, 12.0);

        let (mut empty, _ids) = document_with(vec![]);
        let noop = apply_by_name(&mut empty, "style.scaleStyles", json!({ "scale": 2.0 })).unwrap();
        assert!(matches!(noop, Delta::Noop), "empty library changes nothing");

        let before = document_bytes(&document);
        for scale in [0.0, -1.0] {
            let error = apply_by_name(
                &mut document,
                "style.scaleStyles",
                json!({ "scale": scale }),
            )
            .err()
            .unwrap_or_else(|| panic!("scale {scale}: expected a refusal"));
            assert!(matches!(error, EngineError::InvalidState { .. }));
            assert_eq!(document_bytes(&document), before);
        }
    }

    #[test]
    fn import_appends_and_undo_restores() {
        let (mut document, _ids) = document_with(vec![]);
        document.styles = library();
        let before = document_bytes(&document);
        let imported: Vec<StyleEntry> = (0..2)
            .map(|index| StyleEntry {
                name: format!("imported {index}"),
                ..library()[0].clone()
            })
            .collect();

        let delta = apply_by_name(
            &mut document,
            "style.importLibrary",
            serde_json::json!({ "entries": imported }),
        )
        .unwrap();
        assert_eq!(document.styles.len(), 4);
        assert_eq!(document.styles[2].name, "imported 0", "appends in order");

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);

        let noop = apply_by_name(
            &mut document,
            "style.importLibrary",
            serde_json::json!({ "entries": [] }),
        )
        .unwrap();
        assert!(matches!(noop, Delta::Noop));
    }

    #[test]
    fn payloads_refuse_unknown_or_missing_fields() {
        let (mut document, _ids) = document_with(vec![]);
        document.styles = library();
        for (name, command, payload) in [
            (
                "unknown key",
                "style.apply",
                json!({ "index": 0, "ids": [], "x": 1 }),
            ),
            ("missing ids", "style.apply", json!({ "index": 0 })),
            ("missing scale", "style.scaleStyles", json!({})),
        ] {
            let error = apply_by_name(&mut document, command, payload)
                .err()
                .unwrap_or_else(|| panic!("{name}: expected BadPayload"));
            assert!(
                matches!(error, EngineError::BadPayload { .. }),
                "{name}: {error:?}"
            );
        }
    }
}

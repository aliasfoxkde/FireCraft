//! The typed document model: pages, layers, states (as per-layer cells),
//! and the object union. Every field names the node it maps to (see
//! `docs/planning/firecraft/M3-doc-model.md`); fields whose byte layout
//! is still unpinned (raw color ints) say so in their docs.

use crate::registry::{ObjectId, Registry};
use geom::{Affine, Point, Rect};

/// A document: the `MKBv` node plus its pages (`PDCv`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    /// Scalars and canvas of the document node itself.
    pub info: DocumentInfo,
    /// The document-level layer list (`MKBv`'s own `LYLv` — the reduced
    /// page-1 / shared content copy).
    pub shared_layers: LayerList,
    /// Document grid (`GRDv`).
    pub grid: Grid,
    /// Document guides (`GDSv`).
    pub guides: Guides,
    /// Pages in document order (`PDCv` per page).
    pub pages: Vec<Page>,
    /// Symbol definitions (`MSLv` → `MSTv`).
    pub symbols: Vec<SymbolDefinition>,
    /// Flat object registry everything else points into.
    pub registry: Registry,
}

/// Scalars shared by the document node and pages.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentInfo {
    /// `PGNs` — the document node's legacy page name (the reduced
    /// page-1 copy inside `MKBv`).
    pub name: Option<String>,
    /// `VRSi` — format version.
    pub version: Option<i64>,
    /// `VRMi` — format revision.
    pub revision: Option<i64>,
    /// `FRCi` — frame (state) count. Cell *i* of every layer belongs to
    /// state *i*.
    pub frame_count: i64,
    /// `RQNi` — undocumented counter, preserved as-is.
    pub request_id: i64,
    /// Canvas geometry and background.
    pub canvas: Canvas,
    /// `OSBi` — onion-skinning frames shown before the current one.
    pub onion_before: i64,
    /// `OSAi` — onion-skinning frames shown after the current one.
    pub onion_after: i64,
}

impl Default for DocumentInfo {
    fn default() -> Self {
        Self {
            name: None,
            version: None,
            revision: None,
            frame_count: 1,
            request_id: 0,
            canvas: Canvas::default(),
            onion_before: 0,
            onion_after: 0,
        }
    }
}

/// Canvas geometry (`XLCf YLCf WIDf HITf RESf RSUi BGCi`).
#[derive(Clone, Debug, PartialEq)]
pub struct Canvas {
    /// `XLCf` — canvas x location.
    pub x: f64,
    /// `YLCf` — canvas y location.
    pub y: f64,
    /// `WIDf` — canvas width.
    pub width: f64,
    /// `HITf` — canvas height.
    pub height: f64,
    /// `RESf` — resolution.
    pub resolution: f64,
    /// `RSUi` — resolution unit.
    pub resolution_unit: i64,
    /// `BGCi` — background color as a raw int; the byte layout varies by
    /// context and is unpinned until real-file corpus evidence exists.
    pub background_raw: i64,
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            resolution: 72.0,
            resolution_unit: 0,
            background_raw: 0xFF_FF_FF,
        }
    }
}

/// One page (`PDCv`).
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    /// Page identity and canvas (`VRSi`/`VRMi`/`FRCi` x2/canvas fields).
    pub info: PageInfo,
    /// `PGNs` — page name.
    pub name: Option<String>,
    /// `IMPb` — is the master page.
    pub is_master: bool,
    /// `PILb` — page is linked.
    pub is_linked: bool,
    /// `PIDs` — page id.
    pub page_id: Option<String>,
    /// `LXDs` — last export directory.
    pub last_export_dir: Option<String>,
    /// `LXFs` — last export filename.
    pub last_export_file: Option<String>,
    /// Page layer list (`LYLv`).
    pub layers: LayerList,
    /// Page grid (`GRDv`).
    pub grid: Option<Grid>,
    /// Page guides (`GDSv`).
    pub guides: Guides,
}

/// Page-level scalars. Pages carry the frame count twice (`FRCi` x2 —
/// current frame count and total); both are kept so writes stay
/// faithful.
#[derive(Clone, Debug, PartialEq)]
pub struct PageInfo {
    /// `VRSi`.
    pub version: Option<i64>,
    /// `VRMi`.
    pub revision: Option<i64>,
    /// First `FRCi` — frame (state) count.
    pub frame_count: i64,
    /// Second `FRCi` — the duplicate the format carries.
    pub frame_count_duplicate: i64,
    /// `RQNi`.
    pub request_id: i64,
    /// Canvas geometry and background.
    pub canvas: Canvas,
    /// `OSBi`.
    pub onion_before: i64,
    /// `OSAi`.
    pub onion_after: i64,
}

/// An ordered layer list (`LYLv`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerList {
    /// Layers, bottom-first (file order).
    pub layers: Vec<Layer>,
}

/// One layer (`LAYv`). Per-state content lives in [`Layer::cells`]:
/// cell *i* belongs to state *i* of the owning page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layer {
    /// `LNMs` — layer name.
    pub name: String,
    /// `BKGb` — is the background layer.
    pub is_background: bool,
    /// `URLb` — layer carries URL data.
    pub has_url: bool,
    /// `MSLb` — special layer.
    pub is_special: bool,
    /// `MCLb` — frame-changing layer.
    pub frame_changing: bool,
    /// `DISb` — display (vs smart-layer reference display).
    pub display: bool,
    /// `LLLi` — nesting level.
    pub nesting_level: Option<i64>,
    /// `CLLv` — one cell per state, in state order.
    pub cells: Vec<Cell>,
    /// `VIFv` — per-state visibility override, when present.
    pub visibility_per_frame: Option<Vec<bool>>,
}

/// One animation cell (`CELv`): the element list of one state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    /// `LCKb` — cell is locked.
    pub locked: bool,
    /// `VISb` — cell is visible.
    pub visible: bool,
    /// `ELMv` — object ids, bottom-first (z-order is list order).
    pub elements: Vec<ObjectId>,
}

/// Grid (`GRDv`).
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    /// `GOXf`/`GOYf` — origin offset.
    pub offset: Point,
    /// `GSXf`/`GSYf` — cell size.
    pub size: Point,
    /// `GCLi` — grid color as a raw int (layout unpinned).
    pub color_raw: i64,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            offset: Point::ZERO,
            size: Point::new(24.0, 24.0),
            color_raw: 0,
        }
    }
}

/// Guides (`GDSv`): horizontal and vertical guide coordinates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Guides {
    /// `GDLb` — guides are locked.
    pub locked: bool,
    /// `GDKi` — guide color as a raw int (layout unpinned).
    pub color_raw: i64,
    /// `GDHv` → `GDCf`* — horizontal guide positions.
    pub horizontal: Vec<f64>,
    /// `GDVv` → `GDCf`* — vertical guide positions.
    pub vertical: Vec<f64>,
}

/// A symbol definition (`MSTv`), referenced by `ALSv` instances via
/// [`SymbolInstanceObject::symbol_number`].
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolDefinition {
    /// `UNMs` — user-facing name.
    pub name: String,
    /// `MSNi` — unique symbol number.
    pub symbol_number: i64,
    /// `MOXf`/`MOYf` — origin.
    pub origin: Point,
    /// Symbol frame count (`FRCi`).
    pub frame_count: i64,
    /// Symbol layer list (`LYLv`).
    pub layers: LayerList,
    /// Symbol canvas background (`BGCi`, raw layout).
    pub background_raw: i64,
}

/// `PTHv` — a vector path.
#[derive(Clone, Debug, PartialEq)]
pub struct PathObject {
    /// `PBLv`/`PBPv` — the editable geometry: one entry per contour.
    pub contours: Vec<Contour>,
    /// `PRIv` — the primitive source this path was created from, when
    /// the path is still live-editable as a primitive.
    pub primitive: Option<PrimitiveSource>,
    /// `EOFb` — even-odd fill rule.
    pub even_odd: bool,
}

/// One contour (`PBPv`): a closed flag plus bezier points.
#[derive(Clone, Debug, PartialEq)]
pub struct Contour {
    /// `ISCb` — contour is closed.
    pub closed: bool,
    /// `PPCi` — point count.
    pub point_count: i64,
    /// `PBTv`* — points, in path order.
    pub points: Vec<PathPoint>,
}

/// One bezier point (`PBTv`): anchor plus optional control handles.
#[derive(Clone, Debug, PartialEq)]
pub struct PathPoint {
    /// `XLCf`/`YLCf` — anchor.
    pub anchor: Point,
    /// `XPCf`/`YPCf` — primary (in) control.
    pub primary: Option<Point>,
    /// `XSCf`/`YSCf` — secondary (out) control.
    pub secondary: Option<Point>,
    /// `CRVb` — the point is a curve point.
    pub is_curve: bool,
}

/// `PRIv` — the primitive a path was created from.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveSource {
    /// `RCTv` — the primitive rectangle.
    pub rect: Rect,
    /// `RDSf` — border radius.
    pub radius: f64,
    /// `MTXv` — primitive transform. Projective rows must be affine
    /// (`M02 == M12 == 0`, `M22 == 1`); anything else stays bagged.
    pub transform: Affine,
    /// `RDMb` — maintain border radius on resize.
    pub maintain_radius: bool,
}

/// `TXTv` — a text object. The deep text engine is M6; M3 maps the
/// frame geometry and preserves the run subtree verbatim in the
/// unknown bag.
#[derive(Clone, Debug, PartialEq)]
pub struct TextObject {
    /// `LFTf TOPf RITf BOTf` — text frame bounds.
    pub frame: Rect,
    /// `MTXv` — transform (affine rows enforced like [`PrimitiveSource`]).
    pub transform: Option<Affine>,
}

/// `GRPv` — a group, including mask-structured groups.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupObject {
    /// `GRTi` — group type.
    pub group_type: i64,
    /// `OBNs` — object name.
    pub name: Option<String>,
    /// `ELMv` — child object ids, bottom-first.
    pub children: Vec<ObjectId>,
    /// `OPAi` — opacity, 0..1000 (1000 = opaque).
    pub opacity: i64,
    /// `BLDi` — blend mode (M1 mode table).
    pub blend: i64,
    /// `VISb` — visible.
    pub visible: bool,
    /// `LCKb` — locked.
    pub locked: bool,
    /// `MNAb` — mask group with the mask image itself hidden.
    pub mask_hidden: bool,
}

/// `IMGv` — a raster image backed by `mkBT` tiles.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageObject {
    /// `XLCf`/`YLCf` — location.
    pub location: Point,
    /// `XOFf`/`YOFf` — offset.
    pub offset: Point,
    /// `WPXi`/`HPXi` — pixel size.
    pub pixels: (i64, i64),
    /// `TSZi` — tile size (128 or 256).
    pub tile_size: i64,
    /// `TILv` — tile references, left-to-right top-to-bottom.
    pub tiles: Vec<TileRef>,
    /// `OBNs` — object name.
    pub name: Option<String>,
    /// `OPAi` — opacity, 0..1000.
    pub opacity: i64,
    /// `BLDi` — blend mode.
    pub blend: i64,
    /// `VISb` — visible.
    pub visible: bool,
    /// `LCKb` — locked.
    pub locked: bool,
}

/// One entry in a `TILv`: a tile is *either* an `mkBT` reference *or* a
/// solid matte color (`TMCi` substitutes for `TIDi` when a tile area
/// needs only a flat color).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileRef {
    /// `TIDi` — tile index into the file's `mkBT` chunks.
    Tile(u32),
    /// `TMCi` — solid tile color as a raw int; the dictionary records
    /// this context as ARGB (`0x00000000..0xffffffff`).
    Matte(i64),
}

/// `URLv` — a slice or hotspot.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceObject {
    /// `INMs` — internal name.
    pub internal_name: Option<String>,
    /// `LFTf TOPf RITf BOTf` — slice bounds.
    pub frame: Rect,
    /// `URSi` — hotspot shape.
    pub shape: i64,
    /// `FCLi` — fill color as a raw int (layout unpinned).
    pub fill_raw: i64,
    /// `OBNs` — object name.
    pub name: Option<String>,
    /// `TSLi` — type of slice.
    pub slice_type: Option<i64>,
}

/// `ALSv` — a symbol instance.
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolInstanceObject {
    /// `MSNi` — the referenced [`SymbolDefinition::symbol_number`].
    pub symbol_number: i64,
    /// `MTXv` — instance transform.
    pub transform: Affine,
    /// `LCKb` — instance is locked.
    pub locked: bool,
    /// `OBNs` — object name.
    pub name: Option<String>,
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::registry::ObjectData;

    /// A path object shared by registry/mapping tests.
    pub(crate) fn sample_path() -> ObjectData {
        ObjectData::Path(PathObject {
            contours: vec![Contour {
                closed: true,
                point_count: 4,
                points: vec![
                    PathPoint {
                        anchor: Point::new(0.0, 0.0),
                        primary: None,
                        secondary: None,
                        is_curve: false,
                    },
                    PathPoint {
                        anchor: Point::new(10.0, 0.0),
                        primary: None,
                        secondary: None,
                        is_curve: false,
                    },
                    PathPoint {
                        anchor: Point::new(10.0, 10.0),
                        primary: None,
                        secondary: None,
                        is_curve: false,
                    },
                    PathPoint {
                        anchor: Point::new(0.0, 10.0),
                        primary: None,
                        secondary: None,
                        is_curve: false,
                    },
                ],
            }],
            primitive: Some(PrimitiveSource {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 2.0,
                transform: Affine::IDENTITY,
                maintain_radius: true,
            }),
            even_odd: false,
        })
    }

    #[test]
    fn defaults_are_stable() {
        let info = DocumentInfo::default();
        assert_eq!(info.frame_count, 1);
        assert_eq!(info.canvas.resolution, 72.0);
        let grid = Grid::default();
        assert_eq!(grid.size, Point::new(24.0, 24.0));
    }
}

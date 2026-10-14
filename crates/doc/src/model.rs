//! The typed document model: pages, layers, states (as per-layer cells),
//! and the object union. Every field names the node it maps to (see
//! `docs/planning/firecraft/M3-doc-model.md`); fields whose byte layout
//! is still unpinned (raw color ints) say so in their docs.

use crate::registry::{ObjectId, Registry};
use fwpng::nodetree::Node;
use geom::{Affine, Point, Rect};

/// One live effect: one `EFLv` child of a path/text/group/image
/// object — its category (`CATs`), internal name (`INMs`), and the
/// `EPSv` property entries. The effect's identity is the
/// `EffectMoaID` property (a GUID string); the readable name lives in
/// the internal name.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Effect {
    /// `CATs` — UI category (e.g. "Shadow and Glow"). Empty when the
    /// node is absent (writers skip it, so the round trip is exact).
    pub category: String,
    /// `INMs` — internal name (e.g. "dropShadow").
    pub internal_name: String,
    /// `EPSv` dictionary entries in file order.
    pub properties: Vec<EffectProperty>,
}

/// One `DCEv` entry inside an effect's `EPSv`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EffectProperty {
    /// `DCKs` key (e.g. "EffectMoaID", "gaussian_blur_radius").
    pub key: String,
    pub value: EffectValue,
}

/// The value half of a `DCEv`. The dictionary allows three value node
/// types; the model reads the common one and preserves the rest
/// verbatim (M3 rule: round-trip never waits on semantics).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EffectValue {
    /// `DCVs` — a string value. Numeric effect parameters arrive as
    /// decimal strings in the file; they stay strings here until a
    /// consumer interprets them.
    Text(String),
    /// `GPLv` (curve grab points) / `GDTv` (nested dictionary) — kept
    /// byte-exact, uninterpreted.
    Raw(Node),
}

/// A document: the `MKBv` node plus its pages (`PDCv`).
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// `mkBT` tile payloads the hosts loaded, keyed by the `TIDi` the
    /// `TILv` references point at. The node tree itself stays
    /// reference-faithful — payloads live here so bitmap edits (M7)
    /// and renders share one copy-on-write bank.
    pub tiles: TileBank,
    /// The active marquee/lasso/wand selection, in image pixel space.
    /// Editing state, never file content: the format has no selection
    /// node and the writer never reads this field.
    pub selection: Option<SelectionMask>,
    /// The "Selective JPEG mask" — a named stash of the bitmap-mode
    /// selection (`mask.saveJPEG` / `mask.restoreJPEG`). Editing
    /// state, never file content: the format has no node for it and
    /// the writer never reads this field.
    pub jpeg_mask: Option<SelectionMask>,
    /// Named saved bitmap selections (`dom.saveSelection` family).
    /// Editing state, never file content: the format has no node for
    /// it and the writer never reads this field.
    #[cfg_attr(feature = "serde", serde(default))]
    pub saved_selections: Vec<SavedSelection>,
    /// `EXPv` — the document-level export defaults. `None` when the
    /// file carries no document-level `EXPv`; the writer emits the node
    /// only when settings exist.
    pub export: Option<ExportOptions>,
    /// Document-level style library (`MKBv`'s `ZSLv`). Empty when the
    /// file carries none; the writer emits the node only when styles
    /// exist. A document that carries its styles on an embedded page
    /// instead has them bagged with that page.
    pub styles: Vec<StyleEntry>,
    /// Shared-layer definitions (`MKBv`'s `LSMv` → `LSEv`*). Pages
    /// reference them through [`Layer::shared_link`] links (`LSAv`).
    /// Empty when the file shares nothing; the writer emits the node
    /// only when definitions exist (decision 0022).
    pub shared_layer_defs: Vec<SharedLayerDef>,
    /// The object (vector/element) selection — CS6's `fw.selection`.
    /// Editing state, never file content: the format has no node for
    /// it and the writer never reads this field. Order is canonical
    /// document walk order (page, layer, cell, element position —
    /// decision 0024), not pick order. Distinct from [`Document::selection`]
    /// (the bitmap-mode pixel mask; CS6 keeps the two modes mutually
    /// exclusive in the UI, the model carries them independently).
    #[cfg_attr(feature = "serde", serde(default))]
    pub object_selection: Vec<ObjectId>,
}

/// An `LSAv` layer-list entry: a reference to a shared-layer
/// definition. Lives on [`Layer::shared_link`]; a link layer carries no
/// cells — its content resolves through the `LSEv` its `UIDs` names
/// (decision 0022).
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SharedLayerLink {
    /// `UIDs` — the referenced definition's unique id.
    pub uid: String,
    /// `ISPb` — always false on a link (`true` marks the definition).
    pub is_primary: bool,
}

/// An `LSEv`: one shared-layer definition of `LSMv` (decision 0022).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SharedLayerDef {
    /// `UIDs` — the unique id pages reference via `LSAv`.
    pub uid: String,
    /// `ISPb` — true: this is the primary definition.
    pub is_primary: bool,
    /// The `LAYv` layer body (the shared content, one cell per state).
    pub layer: Layer,
}

/// The document's tile payload bank: `TIDi` → premultiplied RGBA8
/// tile bytes. Hosts populate it from a file's `mkBT` chunks (after
/// ARGB→RGBA8 conversion); commands edit tiles in place through
/// copy-on-write clones, so capturing one in a delta is a pointer bump.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TileBank {
    tiles: std::collections::BTreeMap<u32, raster::Tile>,
}

impl TileBank {
    /// An empty bank.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stored tile for `id`, when the host loaded it.
    #[must_use]
    pub fn get(&self, id: u32) -> Option<&raster::Tile> {
        self.tiles.get(&id)
    }

    /// Stores (or replaces) the payload for `id`.
    pub fn insert(&mut self, id: u32, tile: raster::Tile) {
        self.tiles.insert(id, tile);
    }

    /// Drops the payload for `id`, returning it when present.
    pub fn remove(&mut self, id: u32) -> Option<raster::Tile> {
        self.tiles.remove(&id)
    }

    /// Loaded tile ids in ascending order.
    pub fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.tiles.keys().copied()
    }

    /// Loaded tile count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Whether nothing is loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}

/// An alpha-plane selection: one `0..=255` coverage byte per image
/// pixel (255 = fully selected), row-major, image pixel space. The
/// plane always spans the whole target image so masked ops need no
/// offset bookkeeping.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SelectionMask {
    /// Image width the plane spans.
    pub width: u32,
    /// Image height the plane spans.
    pub height: u32,
    /// `width * height` coverage bytes.
    pub alpha: Vec<u8>,
}

impl SelectionMask {
    /// A fully cleared plane over `width` × `height`.
    #[must_use]
    pub fn empty(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            alpha: vec![
                0;
                (u64::from(width) * u64::from(height))
                    .try_into()
                    .unwrap_or(0)
            ],
        }
    }

    /// Coverage byte at `(x, y)`, `None` outside the plane.
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> Option<u8> {
        let index = self.index_of(x, y)?;
        self.alpha.get(index).copied()
    }

    /// Sets the coverage byte at `(x, y)`; out-of-plane writes are
    /// ignored (clip semantics for mask builders).
    pub fn set(&mut self, x: u32, y: u32, coverage: u8) {
        if let Some(index) = self.index_of(x, y) {
            self.alpha[index] = coverage;
        }
    }

    fn index_of(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        usize::try_from(u64::from(y) * u64::from(self.width) + u64::from(x)).ok()
    }
}

/// A named saved bitmap selection (`dom.saveSelection` /
/// `dom.restoreSelection`). Editing state, never file content: the
/// format has no node for it and the writer never reads this struct.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SavedSelection {
    /// The user-chosen name (CS6 default: "default").
    pub name: String,
    /// The stored coverage plane.
    pub mask: SelectionMask,
}

/// Scalars shared by the document node and pages.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// `EXPv` — this page's export settings. Its `FNLv` frame
    /// configurations are where `PageInfo::frame_durations` persists.
    pub export: Option<ExportOptions>,
}

/// Page-level scalars. Pages carry the frame count twice (`FRCi` x2 —
/// current frame count and total); both are kept so writes stay
/// faithful.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Per-state display delay in hundredths of a second — the `FNDi`
    /// Frame Duration unit — indexed by state. Entries past the vec
    /// (or an empty vec) mean "no explicit delay recorded"; consumers
    /// pick their own default (the GIF exporter documents its 10).
    /// Persists through the page's `EXPv`/`FNLv` mapping.
    pub frame_durations: Vec<i64>,
}

/// An ordered layer list (`LYLv`).
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LayerList {
    /// Layers, bottom-first (file order).
    pub layers: Vec<Layer>,
}

/// One layer (`LAYv`). Per-state content lives in [`Layer::cells`]:
/// cell *i* belongs to state *i* of the owning page.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// `CLLv` — one cell per state, in state order. Empty on a shared
    /// link layer ([`Layer::shared_link`]): the definition owns the
    /// content (decision 0022).
    pub cells: Vec<Cell>,
    /// `VIFv` — per-state visibility override, when present.
    pub visibility_per_frame: Option<Vec<bool>>,
    /// `LSAv` payload when this slot references a shared-layer
    /// definition instead of carrying a layer body (decision 0022).
    pub shared_link: Option<SharedLayerLink>,
}

/// One animation cell (`CELv`): the element list of one state.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// `MSYi` — symbol type. Graphic symbols omit the node rather than
/// storing a zero, so [`SymbolType::Graphic`] maps to absence on the
/// wire; `1` is a button, `2` an animation. Unobserved values keep
/// their raw int so a definition still round-trips byte-exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SymbolType {
    Graphic,
    Button,
    Animation,
    /// An `MSYi` value outside the observed set, preserved verbatim.
    Other(i64),
}

impl SymbolType {
    /// Raw `MSYi` payload; `None` when the node is omitted (graphic).
    pub fn raw(self) -> Option<i64> {
        match self {
            SymbolType::Graphic => None,
            SymbolType::Button => Some(1),
            SymbolType::Animation => Some(2),
            SymbolType::Other(raw) => Some(raw),
        }
    }

    /// Classifies an `MSYi` payload (absent node → [`SymbolType::Graphic`]).
    pub fn from_raw(raw: Option<i64>) -> SymbolType {
        match raw {
            None => SymbolType::Graphic,
            Some(1) => SymbolType::Button,
            Some(2) => SymbolType::Animation,
            Some(other) => SymbolType::Other(other),
        }
    }
}

/// `NGSv` — nine-slice guides on a symbol definition. Inset values
/// are measured from the shape edges; `shape_bounds`/`limits` are
/// relative to the symbol origin.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NineSliceGuides {
    /// `NGLb` — guides locked to the shape.
    pub locked: bool,
    /// `NGKi` — guide-line colour, raw layout.
    pub color_raw: i64,
    /// `NGCf`* — control values in dictionary order: top, bottom,
    /// left, right.
    pub controls: [f64; 4],
    /// `NGMv` — bounds of the symbol's content.
    pub shape_bounds: Rect,
    /// `NGVv` — bounds of the space the guides may move in.
    pub limits: Rect,
}

/// A symbol definition (`MSTv`), referenced by `ALSv` instances via
/// [`SymbolInstanceObject::symbol_number`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// `MSYi`? — symbol type; [`SymbolType::Graphic`] when the node
    /// is absent.
    pub symbol_type: SymbolType,
    /// `NGSv`? — nine-slice guides.
    pub nine_slice: Option<NineSliceGuides>,
    /// `ATSb`? — auto-slice the button (`setButtonAutoSlice`).
    /// Presence-tracked like [`SymbolDefinition::nine_slice`]: `None`
    /// records that the file carried no node; the booleans mean the
    /// same thing on graphic definitions, which carry the defaults.
    pub auto_slice: Option<bool>,
    /// `IDSb`? — include the Down state (`setButtonIncludeDownState`).
    pub include_down_state: Option<bool>,
    /// `DVLb`? — show the Down state on load
    /// (`setButtonShowDownOnLoad`).
    pub show_down_on_load: Option<bool>,
    /// `IODb`? — include the Over-While-Down state
    /// (`setButtonIncludeOverWhileDownState`).
    pub include_over_while_down: Option<bool>,
    /// Rich-symbol script source (`JSVv` under `MSTv`, split into
    /// `JSSs` pieces on the wire). The instance re-derivation runs it
    /// through the scripting host with `Widget.opCode` 1 (defaults)
    /// or 2 (apply) — `None` for a plain symbol.
    pub op_code: Option<String>,
}

/// Paint style carried by a path's `PATv`. M4 maps the two solid
/// colours; M15 decodes the `FPLv` fill subtree (`FSXi`/`FSHi` kind,
/// texture blend, feather, stamping, hard edge, `FGLv` gradient
/// stops, dither colours). `TXBv`/`TXFv`/`BPLv` and the unmodeled
/// `FPLv` children (`FPTv` tile, seed/hard-edge flags, names) ride in
/// the unknown bag.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PaintStyle {
    /// `BCLi` — brush (stroke) colour, ABGR layout.
    pub stroke_raw: i64,
    /// `FCLi` — fill colour, ABGR layout.
    pub fill_raw: i64,
    /// `FPLv`? — fill definition beyond the solid colour; `None` when
    /// the pattern carries no subtree (or one that could not be
    /// modeled — the whole subtree then rides the bag).
    pub fill: Option<Fill>,
}

/// One named style: a `ZSTv` record inside the style library list
/// (`ZSLv`). A style is a property bundle over the same paint, fill
/// handle, effect, and text-attribute slots an object carries — the
/// CS6 Styles panel applies it by copying the bundle onto the
/// selection. Grammar ground truth is the CS6 style-library corpus
/// (15 `.stl` files, 1069 records, one fixed child spine with `EFLv`
/// and `CFTs` optional). The two `IMGv` texture slots per record ride
/// the unknown bag: pattern fills are unmodeled everywhere (task-5
/// recorded gaps), and the slots splice back verbatim.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StyleEntry {
    /// `ZSNs` — the style's panel name ("Pastels 001").
    pub name: String,
    /// `ZSFi` — record flags. Corpus-constant `0x1ff`; semantics
    /// unverified, preserved as-is.
    pub flags: i64,
    /// `ZSOi` — record kind: `8` marks object styles, `0x1f` marks
    /// text styles (across the CS6 library corpus the split is exact:
    /// every `Text *` library carries `0x1f` exclusively, every other
    /// library `8`).
    pub kind: i64,
    /// `PATv` — the paint bundle: brush/fill colours plus the `FPLv`
    /// fill definition (see [`PaintStyle`]).
    pub paint: PaintStyle,
    /// `PSXf`/`PSYf` — the fill's start handle.
    pub fill_start: Point,
    /// `PEXf`/`PEYf` — the fill's end handle.
    pub fill_end: Point,
    /// `PFXf`/`PFYf` — the fill's third handle (elliptical squash).
    pub fill_third: Point,
    /// `BRPi` — brush placement (same slot as a path's).
    pub brush_placement: i64,
    /// `FOTb` — fill on top of the brush (same slot as a path's).
    pub fill_on_top: bool,
    /// `EFLv`* — live effects carried by the style, file order. The
    /// CS6 libraries mostly carry a placeholder effect named
    /// `UNUSED`/`Untitled` in this slot; it round-trips verbatim.
    pub effects: Vec<Effect>,
    /// The text-attribute block (`FALi`…`TOSi`): what a text style
    /// applies onto a text selection. Object styles carry the slots
    /// too (at defaults) — they are part of the record spine.
    pub text: StyleText,
}

/// The text-attribute half of a [`StyleEntry`] — the fixed `FALi`
/// `FATi` `FARi` `FABi` `FONs` [`CFTs`] `PTSf` `BOLb` `ITLb` `UNDb`
/// `JSTi` `HSCf` `RKNf` `LEDf` `LDMi` `ATKb` `TIAb` `TAAi` `TA1i`
/// `TA2i` `TOSi` spine in file order. Field semantics mirror the
/// text-object slots of the same FourCC where those are decoded
/// (`FONs`, `PTSf`, `BOLb`, `ITLb`, `UNDb`, `JSTi`, `HSCf`, `RKNf`,
/// `LEDf`, `LDMi`); the rest are preserved raw with their corpus
/// values recorded.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StyleText {
    /// `FALi` — text-box left margin.
    pub margin_left: i64,
    /// `FATi` — text-box top margin.
    pub margin_top: i64,
    /// `FARi` — text-box right margin.
    pub margin_right: i64,
    /// `FABi` — text-box bottom margin.
    pub margin_bottom: i64,
    /// `FONs` — font family.
    pub font: String,
    /// `CFTs`? — font category. Absent in 30 of the 1069 corpus
    /// records; `None` there.
    pub font_category: Option<String>,
    /// `PTSf` — font point size.
    pub size: f64,
    /// `BOLb` — bold.
    pub bold: bool,
    /// `ITLb` — italic.
    pub italic: bool,
    /// `UNDb` — underline.
    pub underline: bool,
    /// `JSTi` — justification.
    pub justification: i64,
    /// `HSCf` — horizontal scale.
    pub horizontal_scale: f64,
    /// `RKNf` — kerning.
    pub kerning: f64,
    /// `LEDf` — line height.
    pub leading: f64,
    /// `LDMi` — line-height unit.
    pub leading_unit: i64,
    /// `ATKb` — auto-kern.
    pub auto_kern: bool,
    /// `TIAb` — corpus-constant `1`; semantics unverified, preserved
    /// as-is.
    pub anti_alias_enabled: bool,
    /// `TAAi` — anti-alias mode (same slot as a text object's):
    /// corpus `0`, with `1`/`3` on a few text styles.
    pub anti_alias: i64,
    /// `TA1i` — unverified pair slot; corpus pairs with `TA2i` as
    /// (`0xc0`, `0x40`) or (`0xff`, `0`). Preserved as-is.
    pub attr_1: i64,
    /// `TA2i` — unverified pair slot; see [`StyleText::attr_1`].
    pub attr_2: i64,
    /// `TOSi` — unverified slot; corpus `8` or `4` (8 alongside the
    /// (`0xc0`, `0x40`) pair, 4 alongside (`0xff`, `0`)). Preserved
    /// as-is.
    pub tab_size: i64,
}

impl Default for StyleText {
    /// The modal corpus record: zero margins, no font, 12 pt at 100%
    /// scale, single leading, and the unverified slots at their majority
    /// values (`TIAb` 1, `TA1i`/`TA2i` (`0xc0`, `0x40`), `TOSi` 8).
    /// This is what a newly defined style carries before the host
    /// touches the text half.
    fn default() -> Self {
        Self {
            margin_left: 0,
            margin_top: 0,
            margin_right: 0,
            margin_bottom: 0,
            font: String::new(),
            font_category: None,
            size: 12.0,
            bold: false,
            italic: false,
            underline: false,
            justification: 0,
            horizontal_scale: 100.0,
            kerning: 0.0,
            leading: 1.0,
            leading_unit: 0,
            auto_kern: true,
            anti_alias_enabled: true,
            anti_alias: 0,
            attr_1: 0xc0,
            attr_2: 0x40,
            tab_size: 8,
        }
    }
}

/// How a fill derives its pixels (`FSXi`; `FSHi` is the
/// backwards-compatible narrower enumeration — value 3 means
/// "radial or contour" there and reads as [`FillShape::Radial`],
/// which the extended files disambiguate).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FillShape {
    /// `0` — one flat colour (the `FCLi` slot).
    Solid,
    /// `2` — ramp along the start→end fill vector.
    Linear,
    /// `3` — ramp out from a center point.
    Radial,
    /// `4` — ramp around the center's angle.
    Conical,
    /// `5` — satin sheen.
    Satin,
    /// `7` — pinched radial.
    Pinch,
    /// `8` — folded bands.
    Folds,
    /// `9` — squashed radial (the third fill handle sets the ellipse).
    Elliptical,
    /// `10` — rectangular rings.
    Rectangular,
    /// `12` — bar bands.
    Bars,
    /// `13` — ripple interference.
    Ripple,
    /// `14` — wave interference.
    Waves,
    /// `15` — tiled bitmap (`FPTv`).
    Pattern,
    /// `16` — four-colour web dither.
    WebDither,
    /// `17` — contour (concentric rings stepped from the ramp).
    Contour,
}

/// One `FGIv` gradient control point: `FGPf` position 0..=1 plus
/// `FGCi` colour (ABGR layout; the alpha byte is ignored on the RGB
/// ramp — coverage is the alpha ramp's job, on `FG1v`).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GradientStop {
    /// `FGPf` — normalized position along the ramp.
    pub position: f64,
    /// `FGCi` — ABGR colour.
    pub color_raw: i64,
}

/// A `FGLv` fill gradient: the three control-point lists the format
/// carries. `rgb`/`alpha` are the modern `FGYv` split (which ramp a
/// file actually uses is not observable from the dictionary alone —
/// the renderer merges both and lets `alpha` set coverage); `vertices`
/// is the legacy `FGVv` list, carried verbatim. Names (`CATs`/`INMs`)
/// ride the bag.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Gradient {
    /// `FGVv` — legacy vertex list.
    pub vertices: Vec<GradientStop>,
    /// `FG0v` — RGB ramp.
    pub rgb: Vec<GradientStop>,
    /// `FG1v` — alpha ramp (`FGCi` alpha byte is the coverage).
    pub alpha: Vec<GradientStop>,
}

/// Web dither colours (`FD1i`..`FD4i`, `FDTb`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dither {
    /// The up-to-four dither colours; `None` when the slot is absent.
    pub colors: [Option<i64>; 4],
    /// `FDTb` — dither transparent.
    pub transparent: Option<bool>,
}

/// A `FPLv` fill definition. Named slots and identity metadata that
/// the model does not interpret (`CATs`/`INMs`/`UNMs`, `RDOb`, `FETi`,
/// `FRRi`) ride the unknown bag under the `FPLv` path and splice back
/// verbatim.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fill {
    /// `FSXi` (preferred) or `FSHi` — the fill kind.
    pub shape: FillShape,
    /// Which node carried `shape`: `true` writes `FSXi` (a bagged
    /// `FSHi` splices back), `false` writes `FSHi` and no `FSXi` —
    /// the file never carried the extended kind.
    pub shape_extended: bool,
    /// `FTBi` — texture blending, per mille 0..=1000.
    pub texture_blend: i64,
    /// `FEFi` — feather fill amount.
    pub feather: i64,
    /// `FSMi` — stamping mode: 1 blends transparent, 2 blends onto
    /// white. `None` when the file carries no slot.
    pub stamping: Option<i64>,
    /// `FRDb` — hard (non-anti-aliased) edge.
    pub hard_edge: bool,
    /// `FGLv`? — gradient control points; `None` for fills without a
    /// gradient subtree.
    pub gradient: Option<Gradient>,
    /// `FD1i`..`FD4i`/`FDTb`? — web dither slots.
    pub dither: Option<Dither>,
    /// `FPTv`? — the pattern tile (`INMs` + `IMGv`); `None` for fills
    /// without a pattern subtree.
    pub pattern: Option<PatternTile>,
}

/// `FPTv` — one pattern fill's tile: the name plus the `IMGv` raster
/// (pixel extent, tile side, `TILv` references into the document's
/// `mkBT` bank).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PatternTile {
    /// `INMs` — internal name; user-facing `UNMs` rides the bag.
    pub name: Option<String>,
    /// `WPXi`/`HPXi` — the tile raster's pixel extent.
    pub pixels: (i64, i64),
    /// `TSZi` — tile side (128 or 256).
    pub tile_size: i64,
    /// `TILv` — tile references, left-to-right top-to-bottom.
    pub tiles: Vec<TileRef>,
}

/// `PTHv` — a vector path.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PathObject {
    /// `PBLv`/`PBPv` — the editable geometry: one entry per contour.
    pub contours: Vec<Contour>,
    /// `PRIv` — the primitive source this path was created from, when
    /// the path is still live-editable as a primitive.
    pub primitive: Option<PrimitiveSource>,
    /// `EOFb` — even-odd fill rule.
    pub even_odd: bool,
    /// `PATv`? — paint style; `None` when the path carries no pattern.
    pub paint: Option<PaintStyle>,
    /// `PSXf`/`PSYf` — fill point 1 (start).
    pub fill_start: Option<Point>,
    /// `PEXf`/`PEYf` — fill point 2 (end).
    pub fill_end: Option<Point>,
    /// `PFXf`/`PFYf` — fill point 3 (third handle).
    pub fill_third: Option<Point>,
    /// `BRPi` — brush placement (0 inside, 1 middle, 2 outside).
    pub brush_placement: Option<i64>,
    /// `FOTb` — fill over stroke.
    pub fill_on_top: Option<bool>,
    /// `LCKb` — path is locked.
    pub locked: bool,
    /// `VISb` — path is visible.
    pub visible: bool,
    /// `OPAi` — opacity, per mille.
    pub opacity: i64,
    /// `BLDi` — blend mode.
    pub blend: i64,
    /// `OBNs` — object name.
    pub name: Option<String>,
    /// `EFLv`* — live effects, file order.
    pub effects: Vec<Effect>,
}

impl PathObject {
    /// A neutral, visible path over `contours`: opaque, normal blend,
    /// nonzero fill, no paint — the shape every geometry generator
    /// starts from (booleans, Auto Shapes). Paint and metadata are the
    /// caller's to attach.
    pub fn neutral(contours: Vec<Contour>) -> Self {
        Self {
            contours,
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
        }
    }
}

/// One contour (`PBPv`): a closed flag plus bezier points.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// `TFSv` — one style set shared by the [`TextRun`]s that follow it.
/// Every field is optional: the dictionary marks the TFSv children with
/// `*` (zero or more), so a file may omit any of them and the reader
/// must too. Unmapped styling (gradient fills, `CFTs` typefaces) stays
/// bagged at the container level.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextStyle {
    /// `FONs` — font family.
    pub font: Option<String>,
    /// `PTSf` — font point size.
    pub size: Option<f64>,
    /// `BOLb` — bold.
    pub bold: Option<bool>,
    /// `ITLb` — italic.
    pub italic: Option<bool>,
    /// `UNDb` — underline (begin/end run marker).
    pub underline: Option<bool>,
    /// `HSCf` — horizontal scale, percent as written.
    pub horizontal_scale: Option<f64>,
    /// `RKNf` — kerning.
    pub kerning: Option<f64>,
    /// `BLSf` — baseline shift.
    pub baseline_shift: Option<f64>,
    /// `LEDf` — line height, in [`TextStyle::leading_unit`]s.
    pub leading: Option<f64>,
    /// `LDMi` — line height unit.
    pub leading_unit: Option<i64>,
    /// `JSTi` — justification.
    pub justification: Option<i64>,
    /// `FCLi` — fill colour, raw int (same ABGR layout as
    /// [`PaintStyle`]).
    pub fill_raw: Option<i64>,
    /// `PINf` — paragraph indent.
    pub paragraph_indent: Option<f64>,
    /// `PSBf` — paragraph space before.
    pub space_before: Option<f64>,
    /// `PSAf` — paragraph space after.
    pub space_after: Option<f64>,
}

/// `TRNs` — one run of characters sharing the enclosing [`TextStyle`].
/// The dictionary does not document the run payload's node shape and no
/// corpus sample exists yet; FireCraft emits the characters as a string
/// node and accepts a string node back (decision 0007). Anything else
/// rides in the unknown bag.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextRun {
    /// The run's characters.
    pub text: String,
}

/// `TAPv` — text along a path (flow 2): the guide path is embedded in
/// the text object, not a reference to another element. No corpus file
/// with `TAPv` exists yet, so the child order the writer emits is
/// Fireworks-unverified and pinned by the synthetic round-trip fixture;
/// corpus evidence refines it when real exports land.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextOnPath {
    /// `TAMi` — orientation mode: 0 rotate with the path, 1 upright,
    /// 2 skew vertical (vertical strokes stay vertical), 3 skew
    /// horizontal (baseline stays horizontal).
    pub alignment: Option<i64>,
    /// `CHOf` — offset in pixels along the path where the text begins.
    pub offset: f64,
    /// `PTHv`/`PBLv` — the guide path contours. The node's styling
    /// children (paint and friends) stay bagged: the guide path renders
    /// as nothing itself.
    pub path: Vec<Contour>,
}

/// `TXTv` — a text object: a frame, one formatted-string style set with
/// its runs, and the object-level plumbing every element carries.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextObject {
    /// `LFTf TOPf RITf BOTf` — text frame bounds.
    pub frame: Rect,
    /// `TFSv` — the mapped style set; `None` when the file carries no
    /// `TFSv` (style copies at `TXTv` level stay bagged).
    pub style: Option<TextStyle>,
    /// `TFSv`→`TRNs`* — the runs, in file order.
    pub runs: Vec<TextRun>,
    /// `TFTi` — flow: 0 normal, 1 fit to width, 2 follow path, 3 fit
    /// inside shape.
    pub flow: Option<i64>,
    /// `TAPv` — the text-on-path carrier (flow 2): orientation mode,
    /// start offset, and the embedded guide path.
    pub on_path: Option<TextOnPath>,
    /// `TAAi` — anti-alias preset (0 none/smooth .. 3 system, 4 custom).
    pub anti_alias: Option<i64>,
    /// `ATEb` — auto-enlarge.
    pub auto_enlarge: Option<bool>,
    /// `ATKb` — auto-kern.
    pub auto_kern: Option<bool>,
    /// `OBNs` — object name.
    pub name: Option<String>,
    /// `OPAi` — opacity, per mille.
    pub opacity: i64,
    /// `BLDi` — blend mode.
    pub blend: i64,
    /// `VISb` — visible.
    pub visible: bool,
    /// `LCKb` — locked.
    pub locked: bool,
    /// `MTXv` — transform (affine rows enforced like [`PrimitiveSource`]).
    pub transform: Option<Affine>,
    /// `EFLv`* — live effects, file order.
    pub effects: Vec<Effect>,
}

impl TextObject {
    /// A neutral, visible text object over `frame`: no style, no runs,
    /// opaque, normal blend — the object `text.createText` starts from.
    pub fn new(frame: Rect) -> Self {
        Self {
            frame,
            style: None,
            runs: Vec::new(),
            flow: None,
            on_path: None,
            anti_alias: None,
            auto_enlarge: None,
            auto_kern: None,
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            transform: None,
            effects: Vec::new(),
        }
    }
}

/// `GRPv` — a group, including mask-structured groups.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// `LNKb` — mask-group link. `None` = node absent, which the
    /// dictionary defines as linked (`true` is never written);
    /// `Some(false)` = the content moves independently of the mask.
    pub mask_linked: Option<bool>,
    /// `EFLv`* — live effects, file order.
    pub effects: Vec<Effect>,
}

/// `IMGv` — a raster image backed by `mkBT` tiles.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// `EFLv`* — live effects, file order.
    pub effects: Vec<Effect>,
}

/// One entry in a `TILv`: a tile is *either* an `mkBT` reference *or* a
/// solid matte color (`TMCi` substitutes for `TIDi` when a tile area
/// needs only a flat color).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TileRef {
    /// `TIDi` — tile index into the file's `mkBT` chunks.
    Tile(u32),
    /// `TMCi` — solid tile color as a raw int; the dictionary records
    /// this context as ARGB (`0x00000000..0xffffffff`).
    Matte(i64),
}

/// `EXPv` — the export/optimization settings one target carries. The
/// dictionary gives it three homes: document (`MKBv`), page (`PDCv`),
/// and slice (`URLv`/`SLIv` — the per-slice override table behind
/// selective JPEG quality). Every field is `Option`d: `None` means the
/// file did not record it, and the writer emits only recorded fields —
/// a mapping must never mint settings a document never had. Children
/// the dictionary documents but this struct does not carry (the HTML
/// family, `PALv`, `MCTi`, export-area `LR*i`, scale `X/YSZi`, …) stay
/// preserved verbatim in the unknown bag under the `EXPv` path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExportOptions {
    /// `FMTi` — 0 gif, 1 jpeg, 2 png, 3 bitmap/pict/tiff/wbmp
    /// (disambiguated by `MCTi`), 4 animated gif.
    pub format: Option<i64>,
    /// `FCLi` — matte color, ABGR raw (`0..=0xffff_fff`) for formats
    /// that cannot carry alpha.
    pub matte_raw: Option<i64>,
    /// `JPQi` — JPEG quality 0..=100.
    pub jpeg_quality: Option<i64>,
    /// `JPSi` — JPEG smoothing 0..=100. Recorded; no smoothing stage
    /// exists in the codec yet.
    pub jpeg_smoothing: Option<i64>,
    /// `JPOi` — JPEG color setting. Recorded; codec mapping unpinned
    /// until corpus evidence.
    pub jpeg_colour: Option<i64>,
    /// `JPPb` — progressive JPEG scans.
    pub jpeg_progressive: Option<bool>,
    /// `GFIb` — interlaced output (PNG today; GIF interlace has no
    /// codec path yet).
    pub interlaced: Option<bool>,
    /// `GLTi` — GIF loss 0..=128. Recorded; lossy GIF quantization is
    /// not implemented.
    pub gif_loss: Option<i64>,
    /// `PLTi` — palette type (raw dictionary value).
    pub palette_type: Option<i64>,
    /// `PLDi` — palette dither level 0..=100; the animated-GIF
    /// exporter's error-diffusion amount.
    pub dither: Option<i64>,
    /// `PLOb` — palette pruning.
    pub palette_pruning: Option<bool>,
    /// `PTNi` — palette transparency type.
    pub transparency_type: Option<i64>,
    /// `FLPi` — frame loop: `0` infinite, `-1` (the dictionary's
    /// `4294967295`) never, `n` repeat n times.
    pub frame_loop: Option<i64>,
    /// `AACb` — crop animated frames to their ink bounding box.
    pub anim_crop: Option<bool>,
    /// `AADb` — animated inter-frame diff.
    pub anim_diff: Option<bool>,
}

/// `URLv` — a slice or hotspot.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SliceObject {
    /// `INMs` — internal name.
    pub internal_name: Option<String>,
    /// `LFTf TOPf RITf BOTf` — slice bounds.
    pub frame: Rect,
    /// `URSi` — hotspot shape: 0 rectangle, 1 ellipse, 2 polygon.
    pub shape: i64,
    /// `FCLi` — fill color as a raw int (layout unpinned).
    pub fill_raw: i64,
    /// `OBNs` — object name.
    pub name: Option<String>,
    /// `TSLi` — type of slice: 0 foreground, 2 background.
    pub slice_type: Option<i64>,
    /// `A2Ts` — the anchor link URL. The dictionary labels this node
    /// "Anchor Link Alt Text" and `ALTs` "Anchor Link Target", but
    /// `ALTs` spells the HTML `alt` attribute; the labels read as the
    /// analyser's guesses. Mapped per node mnemonics (decision 0017) —
    /// a round-trip is byte-comparable under either reading, so corpus
    /// evidence can re-label these without a format change.
    pub link: Option<String>,
    /// `ALTs` — alt text (the HTML `alt` attribute).
    pub alt: Option<String>,
    /// `FILs` — the auto-naming label (the element the exporter's
    /// naming scheme substitutes when present).
    pub label: Option<String>,
    /// `TDTs` — cell (`TD`) text, the payload of a background slice
    /// exported as HTML table text.
    pub td_text: Option<String>,
    /// `PBPv` — polygon outline (same layout as path contours).
    /// Empty for rectangle/ellipse slices, which carry only `frame`.
    pub contours: Vec<Contour>,
    /// `MSNi`? — the symbol definition this slice belongs to (a
    /// button's auto-slice points back at its definition; decision
    /// 0036). `None` when the file carried no node.
    pub symbol_number: Option<i64>,
    /// `CBTb` + the cell-background family — the background-slice
    /// export knobs. `None` (and no nodes on the wire) for foreground
    /// slices.
    pub background: Option<SliceBackground>,
    /// `BEVv` — behaviors. Only `BETi` is typed (the dictionary
    /// documents the layout only for popup menus, `BETi` 9); every
    /// `BEHv`'s remaining children ride the unknown bag verbatim.
    pub behaviors: Vec<SliceBehavior>,
    /// `DTAv` — per-slice script data (the same `DCEv` key/value
    /// machinery as instances). Export-facing behavior parameters
    /// (swap target/frame, nav group) live here — see decision 0017.
    pub custom_data: std::collections::BTreeMap<String, String>,
    /// `EXPv` — this slice's export override (the selective-JPEG
    /// quality table's home; every field `None` when the file carries
    /// an `EXPv` with none of the mapped children).
    pub export: Option<ExportOptions>,
}

/// The background-slice export knobs (`CBTb` + family under `URLv`).
/// Only present on the wire when recorded; every field is the raw
/// dictionary value.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SliceBackground {
    /// `CBTb` — background slice (set when `TSLi` is 2). `None` when
    /// the file recorded no `CBTb`; the node is presence-tracked so a
    /// recorded `false` survives the round trip.
    pub is_background: Option<bool>,
    /// `CBRi` — repeat: 0 no-repeat, 1 repeat-both, 2 repeat-x,
    /// 3 repeat-y.
    pub repeat: Option<i64>,
    /// `CBAi` — attachment: 0 fixed, 1 scroll.
    pub attachment: Option<i64>,
    /// `CBHi` — horizontal position: 0 left, 1 center, 2 right,
    /// 3 value (`CHVi`).
    pub h_position: Option<i64>,
    /// `CHVi` — horizontal value for `CBHi` 3.
    pub h_value: Option<i64>,
    /// `CBVi` — vertical position: 0 top, 1 center, 2 bottom,
    /// 3 value (`CVVi`).
    pub v_position: Option<i64>,
    /// `CVVi` — vertical value for `CBVi` 3.
    pub v_value: Option<i64>,
}

/// One entry of a slice's `BEVv` behavior list. `BETi` is the only
/// node the dictionary documents across behavior types (9 = popup
/// menu, whose payload structure is known but is a non-goal here);
/// the rest of each `BEHv` is preserved verbatim in the unknown bag.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SliceBehavior {
    /// `BETi` — raw behavior type (9 = popup menu).
    pub kind: i64,
}

/// The animation parameter family carried by a symbol instance
/// (`ALSv`): the span of frames the composer generates from the
/// definition, and how the instance moves/scales/fades across it.
/// Presence is keyed on `ANNi` — a graphic instance carries none of
/// these nodes.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolAnimation {
    /// `ANNi` — number of frames to generate (smooth transition
    /// through the parameters below).
    pub frames: i64,
    /// `ANXf` — x translation across the span (px).
    pub x_shift: f64,
    /// `ANYf` — y translation across the span (px).
    pub y_shift: f64,
    /// `ASCf` — scaling across the span (percent).
    pub scaling: f64,
    /// `ANRi`? — clockwise rotation across the span (degrees,
    /// signed; negative counter-clockwise).
    pub rotation: Option<i64>,
    /// `ASOi`? — start opacity, per mille.
    pub start_opacity: Option<i64>,
    /// `AEOi`? — end opacity, per mille.
    pub end_opacity: Option<i64>,
}

/// `ALSv` — a symbol instance.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolInstanceObject {
    /// `MSNi` — the referenced [`SymbolDefinition::symbol_number`].
    pub symbol_number: i64,
    /// `MTXv` — instance transform.
    pub transform: Affine,
    /// `LCKb` — instance is locked.
    pub locked: bool,
    /// `OBNs` — object name.
    pub name: Option<String>,
    /// `ANNi` + family — animation parameters; `None` (and no nodes
    /// on the wire) for a plain instance.
    pub animation: Option<SymbolAnimation>,
    /// `DTAv` — per-instance custom data: one `DCEv` (`DCKs` key +
    /// `DCVs` string) per entry, canonical order. Rich-symbol
    /// re-derivation keeps its parameter array (the corpus's
    /// `customData["currentValues"]`, JSON-marshalled) here.
    pub custom_data: std::collections::BTreeMap<String, String>,
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
            paint: Some(PaintStyle {
                stroke_raw: 0xFF_00_00_FF,
                fill_raw: 0x00_FF_00_FF,
                fill: None,
            }),
            fill_start: None,
            fill_end: None,
            fill_third: None,
            brush_placement: Some(0),
            fill_on_top: None,
            locked: false,
            visible: true,
            opacity: 1000,
            blend: 0,
            name: None,
            effects: Vec::new(),
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

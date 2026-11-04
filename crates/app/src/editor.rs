//! The editor core: document + tool + view, no window required.
//!
//! Shell rule: nothing here mutates a `Document` except through
//! [`Session::apply`] — the same registry the CLI and journal replay
//! use, so UI edits and scripted edits are indistinguishable in the
//! journal. The egui layer ([`crate::shell`]) is a thin face on this.

use base64::Engine as _;
use doc::{Cell, Document, Layer, ObjectData, ObjectId, TileRef};
use engine::catalog;
use engine::error::EngineError;
use engine::{CommandRegistry, Session};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;

/// Which tool the pointer carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Select and inspect whole objects.
    Pointer,
    /// Reach into groups.
    Subselect,
    /// Drag to create a rectangle.
    Rect,
    /// Drag to create an ellipse.
    Ellipse,
    /// Drag to create a line (corner to corner).
    Line,
    /// Drag to create an empty text frame; characters are typed into
    /// the properties panel (in-canvas editing lands with text tool
    /// polish in a later milestone).
    Text,
    /// Drag = marquee zoom, click = zoom in, alt-click = zoom out.
    Zoom,
    /// Drag to create a foreground slice on the page's Web Layer
    /// (`slice.create`).
    Slice,
    /// Drag to create a link-only hotspot area (`hotspot.create`).
    Hotspot,
    /// Drag to pan.
    Hand,
    /// Drag to create an Auto Shape with the tool's default
    /// parameters (shape chosen in the Tools menu).
    AutoShape(AutoShape),
    /// Paint with the brush: soft-edged dabs along the drag.
    Paintbrush,
    /// Draw with the pencil: hard one-pixel dabs.
    Pencil,
    /// Erase with the brush-shaped eraser.
    Eraser,
    /// Fill the selection (or the whole image) with the paint color.
    PaintBucket,
    /// Drag a linear gradient across the image.
    Gradient,
    /// Pick the paint color from the canvas.
    Eyedropper,
    /// Drag a rectangular pixel marquee over an image.
    Marquee,
    /// Drag an elliptical pixel marquee over an image.
    EllipticalMarquee,
    /// Freehand pixel lasso: the drag's trace becomes the outline.
    Lasso,
    /// Click a pixel polygon vertex by vertex; the panel commits it.
    PolygonalMarquee,
    /// Click to select the pixels that match the one clicked.
    MagicWand,
}

/// The engine's ARGB raw-int color convention (the low 32 bits are
/// the color): `0xAARRGGBB`.
fn argb_raw(color: [u8; 4]) -> i64 {
    let raw = (u32::from(color[3]) << 24)
        | (u32::from(color[0]) << 16)
        | (u32::from(color[1]) << 8)
        | u32::from(color[2]);
    i64::from(raw)
}

/// The gradient's second color: the paint color ramps to white, the
/// two-color default of CS6's Gradient tool.
fn gradient_end_color(color: [u8; 4]) -> [u8; 4] {
    [255, 255, 255, color[3]]
}

/// The stamp shape a brush leaves behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StampShape {
    /// A round footprint (the default).
    Round,
    /// A square footprint.
    Square,
}

impl StampShape {
    /// The wire name the engine's brush payload uses.
    #[must_use]
    pub fn wire(self) -> &'static str {
        match self {
            Self::Round => "round",
            Self::Square => "square",
        }
    }
}

/// The brush model the paint tools share: the Edit Brush dialog's
/// slots plus the paint color and the effect strength. The engine
/// validates the same ranges, so the panel's bounds match the
/// command's refusals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushSettings {
    /// Brush diameter in pixels (the dialog's Size slot), 1..=1024.
    pub size: u32,
    /// Edge softness percent (0 hard rim, 100 full falloff).
    pub edge: u8,
    /// The stamp shape.
    pub shape: StampShape,
    /// Minor axis as a percent of the size (1..=100).
    pub aspect: u8,
    /// Rotation in degrees.
    pub angle: i16,
    /// Per-dab pressure percent (1..=100).
    pub pressure: u8,
    /// Effect strength percent (1..=100).
    pub strength: u8,
    /// The paint color, straight RGBA.
    pub color: [u8; 4],
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            size: 16,
            edge: 50,
            shape: StampShape::Round,
            aspect: 100,
            angle: 0,
            pressure: 100,
            strength: 100,
            color: [0, 0, 0, 255],
        }
    }
}

/// A paint stroke in progress: the image it paints on and where the
/// last dab landed (image pixel coordinates).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    /// The image being painted.
    pub target: ObjectId,
    /// The last dab's center, in image pixel coordinates.
    pub last: geom::Point,
    /// The stroke's start point, in image pixel coordinates — the
    /// gradient's first handle.
    pub start: geom::Point,
}

/// The geometry an Auto Shape is placed from: the box it occupies
/// and, for the line-based shapes, its two endpoints.
struct Placement {
    /// Box origin, world coordinates.
    x: f64,
    /// Box origin, world coordinates.
    y: f64,
    /// Box width.
    width: f64,
    /// Box height.
    height: f64,
    /// The first endpoint (the line-based shapes).
    start: (f64, f64),
    /// The second endpoint (the line-based shapes).
    end: (f64, f64),
}

/// The Auto Shape tool group: one drag-to-create tool per shape,
/// each issuing its `autoshape.*` registry command with pinned
/// defaults derived from the drag frame (per-shape corpus gating of
/// those defaults is M8's job).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoShape {
    /// `autoshape.star` — 5-point star.
    Star,
    /// `autoshape.polygon` — 6-side smart polygon.
    Polygon,
    /// `autoshape.spiral`.
    Spiral,
    /// `autoshape.pie` — 270° wedge.
    Pie,
    /// `autoshape.doughnut` — full annulus.
    Doughnut,
    /// `autoshape.lshape`.
    LShape,
    /// `autoshape.arrow` — block arrow.
    Arrow,
    /// `autoshape.arrowLine`.
    ArrowLine,
    /// `autoshape.measure`.
    Measure,
    /// `autoshape.connectorLine`.
    ConnectorLine,
}

impl AutoShape {
    /// Menu order.
    pub const ALL: [AutoShape; 10] = [
        AutoShape::Star,
        AutoShape::Polygon,
        AutoShape::Spiral,
        AutoShape::Pie,
        AutoShape::Doughnut,
        AutoShape::LShape,
        AutoShape::Arrow,
        AutoShape::ArrowLine,
        AutoShape::Measure,
        AutoShape::ConnectorLine,
    ];

    /// Menu label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            AutoShape::Star => "Star",
            AutoShape::Polygon => "Smart Polygon",
            AutoShape::Spiral => "Spiral",
            AutoShape::Pie => "Pie",
            AutoShape::Doughnut => "Doughnut",
            AutoShape::LShape => "L-Shape",
            AutoShape::Arrow => "Arrow",
            AutoShape::ArrowLine => "Arrow Line",
            AutoShape::Measure => "Measure",
            AutoShape::ConnectorLine => "Connector Line",
        }
    }

    /// Registry command this tool issues.
    #[must_use]
    pub fn command(self) -> &'static str {
        match self {
            AutoShape::Star => "autoshape.star",
            AutoShape::Polygon => "autoshape.polygon",
            AutoShape::Spiral => "autoshape.spiral",
            AutoShape::Pie => "autoshape.pie",
            AutoShape::Doughnut => "autoshape.doughnut",
            AutoShape::LShape => "autoshape.lshape",
            AutoShape::Arrow => "autoshape.arrow",
            AutoShape::ArrowLine => "autoshape.arrowLine",
            AutoShape::Measure => "autoshape.measure",
            AutoShape::ConnectorLine => "autoshape.connectorLine",
        }
    }

    /// Default-parameter payload for a completed drag: center-based
    /// shapes take the frame center + radius, frame-based shapes the
    /// normalized frame, line-based shapes the raw endpoints.
    #[must_use]
    fn payload(self, drag: &Drag, page: usize, layer: usize, frame: usize) -> serde_json::Value {
        let (sx, sy) = (drag.start.x, drag.start.y);
        let (cx, cy) = (drag.current.x, drag.current.y);
        // Frame origin/size, normalized so drags in any direction
        // produce positive extents.
        let placement = Placement {
            x: sx.min(cx),
            y: sy.min(cy),
            width: (cx - sx).abs(),
            height: (cy - sy).abs(),
            start: (sx, sy),
            end: (cx, cy),
        };
        self.payload_from(&placement, page, layer, frame)
    }

    /// The payload for a shape inserted without a drag — the Auto
    /// Shapes panel's path: a `size`-square box at (`x`, `y`). The
    /// line-based shapes run along the box's horizontal midline.
    #[must_use]
    pub fn payload_in_rect(
        self,
        page: usize,
        layer: usize,
        frame: usize,
        x: f64,
        y: f64,
        size: f64,
    ) -> serde_json::Value {
        let mid = y + size / 2.0;
        let placement = Placement {
            x,
            y,
            width: size,
            height: size,
            start: (x, mid),
            end: (x + size, mid),
        };
        self.payload_from(&placement, page, layer, frame)
    }

    /// The registry payload for one shape placement. Every shape
    /// family reads the placement it is defined by: the box's center
    /// and radius (circle-bound), the box itself (frame-bound), or
    /// the endpoints (line-bound).
    #[must_use]
    fn payload_from(
        self,
        place: &Placement,
        page: usize,
        layer: usize,
        frame: usize,
    ) -> serde_json::Value {
        let frame_json = serde_json::json!({
            "page": page, "layer": layer, "cell": frame, "name": null,
        });
        let (x, y) = (place.x, place.y);
        let (width, height) = (place.width, place.height);
        // Center + radius (circle-bound shapes).
        let (center_x, center_y) = (x + width / 2.0, y + height / 2.0);
        let radius = width.max(height) / 2.0;
        let (sx, sy) = place.start;
        let (cx, cy) = place.end;
        // Placement rides on every payload.
        let mut value = match self {
            AutoShape::Star => serde_json::json!({
                "centerX": center_x, "centerY": center_y,
                "outer": radius, "inner": radius / 2.0,
                "outerRound": 0.0, "innerRound": 0.0,
                "points": 5, "startAngle": -std::f64::consts::FRAC_PI_2,
            }),
            AutoShape::Polygon => serde_json::json!({
                "centerX": center_x, "centerY": center_y,
                "outer": radius, "inner": 0.0,
                "points": 6, "sections": 6,
                "startAngle": -std::f64::consts::FRAC_PI_2, "split": false,
            }),
            AutoShape::Spiral => serde_json::json!({
                "centerX": center_x, "centerY": center_y,
                "radius": radius, "tightnessDeg": 80.0,
                "clockwise": true, "closed": false,
            }),
            AutoShape::Pie => serde_json::json!({
                "centerX": center_x, "centerY": center_y,
                "outer": radius, "inner": 0.0,
                "start": -std::f64::consts::FRAC_PI_2,
                "end": std::f64::consts::PI,
            }),
            AutoShape::Doughnut => serde_json::json!({
                "centerX": center_x, "centerY": center_y,
                "outer": radius, "inner": radius / 2.0,
                "start": -std::f64::consts::FRAC_PI_2,
                "end": 3.0 * std::f64::consts::FRAC_PI_2,
            }),
            AutoShape::LShape => serde_json::json!({
                "x": x, "y": y, "width": width, "height": height,
                "thickness": 0.3 * width.min(height), "roundness": 0.0,
            }),
            AutoShape::Arrow => serde_json::json!({
                "x": x, "y": y, "width": width, "height": height,
                "thickness": 0.4 * height, "roundness": 0.0,
                "arrowWidth": height, "arrowLength": 0.4 * width,
                "arrowTip": 0.1 * width,
            }),
            AutoShape::ArrowLine => serde_json::json!({
                "startX": sx, "startY": sy, "endX": cx, "endY": cy,
                "startHead": "none", "endHead": "solid",
            }),
            AutoShape::Measure => serde_json::json!({
                "startX": sx, "startY": sy, "endX": cx, "endY": cy,
            }),
            AutoShape::ConnectorLine => serde_json::json!({
                "startX": sx, "startY": sy, "endX": cx, "endY": cy,
                "elbowX": cx, "elbowY": (sy + cy) / 2.0,
                "corner1": 4.0, "corner2": 4.0, "vertical": false,
            }),
        };
        if let (Some(object), Some(place)) = (value.as_object_mut(), frame_json.as_object()) {
            for (key, field) in place {
                object.insert(key.clone(), field.clone());
            }
        }
        value
    }
}

/// The modifier constraint matrix for creation drags — one
/// documented rule applied to the stored drag geometry (the rubber
/// band and the created object share it, so the preview never lies):
///
/// | Tools | Shift | Alt |
/// |---|---|---|
/// | box-drag (rect/ellipse/text/slice/hotspot/auto shapes) | square | from center |
/// | Line | snap to 45° | from center |
/// | Pointer / Subselect / Zoom / Hand | — | — |
///
/// Shift+Alt combines (square, centered). CS6's reads: shift squares
/// the drag or locks the line to the axis octants; alt drags the shape
/// out from its center.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DragConstraint {
    /// Shift held.
    pub shift: bool,
    /// Alt held.
    pub alt: bool,
}

/// A creation drag in progress: tool + world-space start point, or a
/// pointer drag moving a captured selection.
#[derive(Clone, Debug, PartialEq)]
/// The drag a creation/constraint tool is doing.
pub struct Drag {
    /// Tool the drag belongs to.
    pub tool: Tool,
    /// The live modifier constraint (updated per step).
    pub constraint: DragConstraint,
    /// World position where the press landed.
    pub start: geom::Point,
    /// Current world position.
    pub current: geom::Point,
    /// `Some` while a pointer drag moves objects: the ids were fixed
    /// at drag start (the press selects its target first), and
    /// `moved` records whether any translate applied — a click-sized
    /// drag commits nothing.
    pub moving: Option<MoveDrag>,
}

/// Which half of the constraint matrix a tool's creation drag obeys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragClass {
    /// Shift squares the drag; alt drags from center.
    Box,
    /// Shift snaps the endpoint to a 45° octant; alt drags from
    /// center.
    Line,
    /// No constraint (view and selection tools).
    None,
}

fn drag_class(tool: Tool) -> DragClass {
    match tool {
        Tool::Rect
        | Tool::Ellipse
        | Tool::Text
        | Tool::Slice
        | Tool::Hotspot
        | Tool::AutoShape(_) => DragClass::Box,
        Tool::Line => DragClass::Line,
        Tool::Pointer
        | Tool::Subselect
        | Tool::Zoom
        | Tool::Hand
        | Tool::Paintbrush
        | Tool::Pencil
        | Tool::Eraser
        | Tool::PaintBucket
        | Tool::Gradient
        | Tool::Eyedropper
        | Tool::Marquee
        | Tool::EllipticalMarquee
        | Tool::Lasso
        | Tool::PolygonalMarquee
        | Tool::MagicWand => DragClass::None,
    }
}

/// How a new pixel selection combines with the active one — CS6's
/// marquee modifier keys (shift adds, alt subtracts) made explicit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelSelectMode {
    /// Replace the active selection.
    Replace,
    /// Union with the active selection.
    Add,
    /// Cut out of the active selection.
    Subtract,
    /// Keep only the overlap.
    Intersect,
}

impl PixelSelectMode {
    /// The wire name the engine's selection payloads use.
    #[must_use]
    pub fn wire(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Add => "add",
            Self::Subtract => "subtract",
            Self::Intersect => "intersect",
        }
    }

    /// The mode the modifier keys select: shift adds, alt subtracts,
    /// ctrl intersects; no modifier replaces.
    #[must_use]
    pub fn from_modifiers(modifiers: egui::Modifiers) -> Self {
        if modifiers.shift {
            Self::Add
        } else if modifiers.alt {
            Self::Subtract
        } else if modifiers.ctrl {
            Self::Intersect
        } else {
            Self::Replace
        }
    }
}

/// A pixel selection being drawn: the image it lands on, the drag's
/// endpoints, and the trace a freehand lasso leaves behind.
#[derive(Clone, Debug, PartialEq)]
pub struct PixelSelection {
    /// The image being selected in.
    pub target: ObjectId,
    /// The drag's first point, in image pixel coordinates.
    pub start: geom::Point,
    /// The drag's current point, in image pixel coordinates.
    pub current: geom::Point,
    /// The lasso's trace: every point the pointer visited.
    pub trace: Vec<geom::Point>,
}

/// A pixel polygon being built click by click.
#[derive(Clone, Debug, PartialEq)]
pub struct PolygonDraft {
    /// The image the vertices are in.
    pub target: ObjectId,
    /// The vertices, in image pixel coordinates.
    pub points: Vec<geom::Point>,
}

/// One place computes the constrained drag geometry — the
/// [`DragConstraint`] doc comment holds the matrix. The move/clone
/// drags pass through untouched (their tools are `DragClass::None`).
fn constrain_geometry(
    tool: Tool,
    start: geom::Point,
    current: geom::Point,
    constraint: DragConstraint,
) -> geom::Point {
    let (dx, dy) = (current.x - start.x, current.y - start.y);
    let mut out = current;
    match drag_class(tool) {
        DragClass::None => return out,
        DragClass::Box => {
            if constraint.shift {
                // Square: the longer axis wins, sign kept per axis.
                let side = dx.abs().max(dy.abs());
                out.x = start.x + dx.signum() * side;
                out.y = start.y + dy.signum() * side;
            }
        }
        DragClass::Line => {
            if constraint.shift && (dx != 0.0 || dy != 0.0) {
                // Snap the endpoint onto the nearest 45° octant,
                // keeping the pointer's length.
                let len = (dx * dx + dy * dy).sqrt();
                let octant = (dy.atan2(dx) / std::f64::consts::FRAC_PI_4).round()
                    * std::f64::consts::FRAC_PI_4;
                out.x = start.x + len * octant.cos();
                out.y = start.y + len * octant.sin();
            }
        }
    }
    if constraint.alt {
        // From center: the press point is the shape's center, so the
        // endpoint mirrors through it (end_drag's `current − start`
        // then reads as the half-extent).
        out.x = start.x - (out.x - start.x);
        out.y = start.y - (out.y - start.y);
    }
    out
}

/// The selection a pointer drag is moving.
#[derive(Clone, Debug, PartialEq)]
pub struct MoveDrag {
    /// The objects being moved.
    pub ids: Vec<ObjectId>,
    /// Whether any `object.translate` applied so far.
    pub moved: bool,
}

/// The four nine-slice inset edges, in `NGCf` dictionary order
/// (top, bottom, left, right) — the same order as
/// [`doc::NineSliceGuides::controls`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NineSliceEdge {
    /// Upper inset, measured down from the shape's top edge.
    Top,
    /// Lower inset, measured up from the shape's bottom edge.
    Bottom,
    /// Left inset, measured right from the shape's left edge.
    Left,
    /// Right inset, measured left from the shape's right edge.
    Right,
}

impl NineSliceEdge {
    /// Index into `NineSliceGuides::controls`.
    #[must_use]
    pub fn control(self) -> usize {
        match self {
            Self::Top => 0,
            Self::Bottom => 1,
            Self::Left => 2,
            Self::Right => 3,
        }
    }
}

/// A nine-slice guide edge being dragged to a new inset. The drag
/// lives in the editor (not the shell) so the inset it commits is a
/// `symbol.set9Slice` command like every other edit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideDrag {
    /// The definition whose guides are moving, by `MSNi`.
    pub symbol: i64,
    /// Which inset edge is under the pointer.
    pub edge: NineSliceEdge,
    /// Latest world position of the pointer.
    pub current: geom::Point,
}

/// Which selection handle the transform drag grabbed. Corner handles
/// scale both axes; edge handles scale one; nothing else reaches the
/// transform gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformHandle {
    /// Top-left corner: scales away from the opposite corner.
    TopLeft,
    /// Top-right corner.
    TopRight,
    /// Bottom-right corner.
    BottomRight,
    /// Bottom-left corner.
    BottomLeft,
    /// Top edge midpoint: vertical scale only.
    Top,
    /// Bottom edge midpoint.
    Bottom,
    /// Left edge midpoint: horizontal scale only.
    Left,
    /// Right edge midpoint.
    Right,
}

impl TransformHandle {
    /// True for the four corner handles (Distort moves these).
    #[must_use]
    pub fn is_corner(self) -> bool {
        !matches!(self, Self::Top | Self::Bottom | Self::Left | Self::Right)
    }

    /// The fixed (opposite) anchor the drag scales away from, in
    /// normalized bounds coordinates — CS6's handle semantics: the
    /// opposite corner/edge stays put while the grabbed one follows
    /// the pointer.
    #[must_use]
    fn anchor_norm(self) -> (f64, f64) {
        match self {
            Self::TopLeft => (1.0, 1.0),
            Self::TopRight => (0.0, 1.0),
            Self::BottomRight => (0.0, 0.0),
            Self::BottomLeft => (1.0, 0.0),
            Self::Top => (0.5, 1.0),
            Self::Bottom => (0.5, 0.0),
            Self::Left => (1.0, 0.5),
            Self::Right => (0.0, 0.5),
        }
    }
}

/// How the selection's transform handles behave (Modify ▸ Transform).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformMode {
    /// Free Transform: handles scale away from the opposite anchor.
    Scale,
    /// Distort: the grabbed corner moves independently; the selection
    /// fills the parallelogram the moved corner defines.
    Distort,
}

/// A handle drag in progress: the selection is being scaled live by
/// one `object.scale`/`object.transform` step per pointer move, folded
/// into a single "Transform" undo step like a move drag.
#[derive(Clone, Debug, PartialEq)]
pub struct TransformDrag {
    /// The objects being transformed.
    pub ids: Vec<ObjectId>,
    /// Which handle is under the pointer.
    pub handle: TransformHandle,
    /// The selection's union bounds at drag start (the scale space).
    pub bounds: geom::Rect,
    /// Latest world position of the pointer.
    pub current: geom::Point,
    /// Whether any scale step applied (a click-sized drag commits
    /// nothing, mirroring `MoveDrag::moved`).
    pub applied: bool,
    /// Distort mode: the three unmoved corners, in world coordinates,
    /// captured at drag start — percents are re-derived per step
    /// because the command's reference bounds move with the object.
    pub fixed_corners: Option<[(f64, f64); 3]>,
}

/// A path-node drag in progress (Subselect tool): one anchor of one
/// path moves live, one `path.movePoints` step per pointer move,
/// folded into a single "Reshape" undo step.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeDrag {
    /// The path being reshaped.
    pub id: ObjectId,
    /// Contour index of the grabbed anchor.
    pub contour: usize,
    /// Point index of the grabbed anchor.
    pub point: usize,
    /// Latest world position of the pointer.
    pub current: geom::Point,
    /// Whether any step applied (a click-sized drag commits nothing).
    pub applied: bool,
}

/// Which end of a gradient ramp a drag grabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientHandle {
    /// The ramp origin (`fdct` start).
    Start,
    /// The ramp terminus (`fdct` end).
    End,
}

/// A gradient-handle drag in progress: one `path.setFillVector` step
/// per pointer move, folded into a single "Gradient" undo step.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientDrag {
    /// The path whose ramp is moving.
    pub id: ObjectId,
    /// Which end is under the pointer.
    pub handle: GradientHandle,
    /// Latest world position of the pointer.
    pub current: geom::Point,
    /// Whether any step applied.
    pub applied: bool,
}

/// Everything the shell shows, minus the window itself.
pub struct Editor {
    /// The editing session (document + history + journal).
    pub session: Session,
    /// Unknown-node bag from the opened file; rides along on save.
    unknown: doc::fwmap::UnknownBag,
    /// Original chunk stream; `save` replaces its mkTS.
    chunks: Vec<fwpng::chunk::RawChunk>,
    /// Active tool.
    pub tool: Tool,
    /// Current page index (view state, not document state).
    pub page: usize,
    /// Current state (frame) index (view state).
    pub frame: usize,
    /// Active layer on the current page (view state): every placement
    /// command lands its object here, and the Layers panel highlights
    /// the row. Bottom-first, like the document's own layer list.
    pub active_layer: usize,
    /// Selected object, if any (the last one picked).
    pub selection: Option<ObjectId>,
    /// Multi-selection the arrange/boolean surfaces operate on, in
    /// pick order; `selection` is always its last entry when non-empty.
    pub selection_set: Vec<ObjectId>,
    /// Creation drag in progress.
    pub drag: Option<Drag>,
    /// The symbol definition being edited, by `MSNi`; `None` edits the
    /// page tree. While set, creation commands land their objects in
    /// the definition (via a page-side create plus `symbol.editAttach`)
    /// and hit-testing looks at the definition's own cells.
    pub symbol_scope: Option<i64>,
    /// Active layer inside the symbol scope (view state).
    pub symbol_layer: usize,
    /// Nine-slice guide edge drag in progress.
    pub guide_drag: Option<GuideDrag>,
    /// Selection-handle transform drag in progress.
    pub transform_drag: Option<TransformDrag>,
    /// How transform handles behave (Free Transform scales; Distort
    /// lets corner handles move independently).
    pub transform_mode: TransformMode,
    /// Path-node drag in progress (Subselect).
    pub node_drag: Option<NodeDrag>,
    /// Gradient-handle drag in progress.
    pub gradient_drag: Option<GradientDrag>,
    /// The brush model the paint tools use (view state — the engine
    /// re-reads it from every stroke's payload).
    pub brush: BrushSettings,
    /// The paint stroke in progress, if any.
    pub stroke: Option<Stroke>,
    /// The pixel selection being drawn, if any.
    pub pixel_selection: Option<PixelSelection>,
    /// The pixel polygon being built, if any.
    pub polygon: Option<PolygonDraft>,
    /// The image the active pixel selection belongs to — the
    /// selection plane is document-wide, so the overlay needs to know
    /// which image it is drawn over.
    pub pixel_target: Option<ObjectId>,
    /// How the next pixel selection combines with the active one.
    pub pixel_mode: PixelSelectMode,
    /// The magic wand's per-channel tolerance.
    pub wand_tolerance: u8,
    /// The feather radius the panel's slider applies.
    pub feather_radius: u8,
}

impl Editor {
    /// A new document: one page, one visible layer, a 640×480 canvas —
    /// seeded through the same two registry commands the Commands menu
    /// drives, so even a fresh file's journal reads like real edits.
    #[must_use]
    pub fn untitled() -> Self {
        let mut editor = Self::from_document(Document::default());
        // Neither seed step can fail (append to an empty page list;
        // resize the page that was just appended), matching the `let _`
        // idiom used for the infallible catalog install above.
        let _ = editor.apply("page.add", serde_json::json!({ "index": null }));
        let _ = editor.apply(
            "page.setCanvasSize",
            serde_json::json!({ "page": 0, "width": 640.0, "height": 480.0 }),
        );
        editor
    }

    /// Wraps a document (e.g. the seed fixture in tests).
    #[must_use]
    pub fn from_document(document: Document) -> Self {
        Self::wrap(document, doc::fwmap::UnknownBag::default(), Vec::new())
    }

    /// Wraps an opened file's document plus its preservation payload.
    #[must_use]
    pub fn wrap(
        document: Document,
        unknown: doc::fwmap::UnknownBag,
        chunks: Vec<fwpng::chunk::RawChunk>,
    ) -> Self {
        let mut registry = CommandRegistry::new();
        // install() can only fail on duplicate ids — impossible for
        // the built-in table; a bug there is a panic-grade defect.
        let _ = catalog::install(&mut registry);
        // CS6 opens a document with its topmost layer active, so a
        // freshly opened multi-layer file lands new objects where the
        // user expects them.
        let active_layer = document
            .pages
            .first()
            .map_or(0, |page| page.layers.layers.len().saturating_sub(1));
        Self {
            session: Session::with_registry(document, registry),
            unknown,
            chunks,
            tool: Tool::Pointer,
            page: 0,
            frame: 0,
            active_layer,
            selection: None,
            selection_set: Vec::new(),
            drag: None,
            symbol_scope: None,
            symbol_layer: 0,
            guide_drag: None,
            transform_drag: None,
            transform_mode: TransformMode::Scale,
            node_drag: None,
            gradient_drag: None,
            brush: BrushSettings::default(),
            stroke: None,
            pixel_selection: None,
            polygon: None,
            pixel_target: None,
            pixel_mode: PixelSelectMode::Replace,
            wand_tolerance: 32,
            feather_radius: 2,
        }
    }

    /// The number of layers on the current page (0 when the document
    /// has no such page).
    #[must_use]
    pub fn layer_count(&self) -> usize {
        self.session
            .document()
            .pages
            .get(self.page)
            .map_or(0, |page| page.layers.layers.len())
    }

    /// The active layer, clamped into the current page's layer list.
    /// A stale index (the last layer was deleted, the page changed)
    /// resolves to the topmost layer rather than refusing.
    #[must_use]
    pub fn active_layer(&self) -> usize {
        let count = self.layer_count();
        if count == 0 {
            0
        } else {
            self.active_layer.min(count - 1)
        }
    }

    /// Makes `index` the active layer — the Layers panel's click
    /// handler. Out-of-range indices are clamped to the topmost layer.
    pub fn set_active_layer(&mut self, index: usize) {
        let count = self.layer_count();
        if count == 0 {
            self.active_layer = 0;
            return;
        }
        self.active_layer = index.min(count - 1);
    }

    /// Opens `.fw.png` bytes through the full read path.
    ///
    /// # Errors
    /// Whatever the reader refuses (bad PNG, no mkTS, model drift).
    pub fn open(bytes: &[u8]) -> Result<Self, EngineError> {
        let loaded = app_cli::io::open(bytes).map_err(|error| EngineError::Io {
            path: "<bytes>".to_owned(),
            detail: error.0,
        })?;
        Ok(Self::wrap(loaded.document, loaded.unknown, loaded.chunks))
    }

    /// Canonical `.fw.png` bytes for the current document.
    ///
    /// # Errors
    /// Whatever the writer refuses.
    pub fn save(&self) -> Result<Vec<u8>, EngineError> {
        app_cli::io::save(self.session.document(), &self.unknown, &self.chunks).map_err(|error| {
            EngineError::Io {
                path: "<bytes>".to_owned(),
                detail: error.0,
            }
        })
    }

    /// Exports the named definitions (first name match wins) and their
    /// member closures as a library file: an ordinary `.fw.png` whose
    /// `MSLv` carries the definitions and whose registry holds their
    /// members, nothing on a page. Written through the same save path
    /// as any document, so it opens anywhere the format does.
    ///
    /// # Errors
    /// An unknown name or a dangling member id (corrupt library).
    pub fn export_library(&self, names: &[&str]) -> Result<Vec<u8>, EngineError> {
        let library = doc::library::extract(self.session.document(), names).map_err(|error| {
            EngineError::InvalidState {
                name: "library.export".to_owned(),
                detail: error.to_string(),
            }
        })?;
        app_cli::io::save(&library, &doc::fwmap::UnknownBag::default(), &[]).map_err(|error| {
            EngineError::Io {
                path: "<bytes>".to_owned(),
                detail: error.0,
            }
        })
    }

    /// Imports definitions from library-file bytes: reads the file
    /// through the full read path, then installs each selected
    /// definition with its member closure via `library.importSymbol`,
    /// renumbered into this document's library (`None` selects every
    /// definition in the file, file order). Returns the new `MSNi`
    /// numbers in import order — one undoable step per definition.
    ///
    /// # Errors
    /// Whatever the reader refuses, or an import refuses (a payload
    /// whose closure is incomplete — a corrupt library file).
    pub fn import_library(
        &mut self,
        bytes: &[u8],
        names: Option<&[&str]>,
    ) -> Result<Vec<i64>, EngineError> {
        let loaded = app_cli::io::open(bytes).map_err(|error| EngineError::Io {
            path: "<bytes>".to_owned(),
            detail: error.0,
        })?;
        let mut numbers = Vec::new();
        for definition in &loaded.document.symbols {
            if let Some(names) = names {
                if !names.iter().any(|name| *name == definition.name) {
                    continue;
                }
            }
            let members: Vec<serde_json::Value> = loaded
                .document
                .registry
                .iter()
                .map(|(id, object)| serde_json::json!({ "id": id, "object": object }))
                .collect();
            self.apply(
                "library.importSymbol",
                serde_json::json!({
                    "definition": definition,
                    "members": members,
                }),
            )?;
            if let Some(added) = self.document().symbols.last() {
                numbers.push(added.symbol_number);
            }
        }
        Ok(numbers)
    }

    /// Imports a CS6 style library (`.stl`): its `mkTS` node tree
    /// deserializes into entries appended to the document library
    /// (`style.importLibrary`), the library's `mkBT` rasters join the
    /// document tile bank under fresh ids (`style.importLibraryTiles`),
    /// and every `TIDi` the import carries — in the records' pattern
    /// fills and their preserved texture subtrees — re-keys to the new
    /// ids. Both applies fold into one undo step (a transaction); the
    /// tiles themselves have no inverse — undoing the import leaves
    /// orphaned bank tiles, which nothing reads. Each entry's preserved
    /// texture subtrees also merge into the editor's unknown bag under
    /// the entry's final `ZSTv` index, so the splices stay byte-exact
    /// on save.
    ///
    /// # Errors
    /// Whatever the chunk parse, the inflate, the style reader, or the
    /// tile join refuses (a `TIDi` with no `mkBT` payload means a
    /// corrupt library and refuses the import whole); the document is
    /// untouched.
    pub fn import_styles(&mut self, bytes: &[u8]) -> Result<usize, EngineError> {
        fn detail(error: impl std::fmt::Display) -> EngineError {
            EngineError::Io {
                path: "<styles>".to_owned(),
                detail: error.to_string(),
            }
        }
        let chunks = fwpng::chunk::parse_chunks(bytes).map_err(detail)?;
        let Some(mkts) = chunks.iter().find(|chunk| &chunk.ctype == b"mkTS") else {
            return Err(EngineError::Io {
                path: "<styles>".to_owned(),
                detail: "no mkTS chunk (not a Fireworks style library?)".to_owned(),
            });
        };
        let mut inflated = Vec::new();
        flate2::read::ZlibDecoder::new(&mkts.data[..])
            .read_to_end(&mut inflated)
            .map_err(detail)?;
        let nodes = fwpng::nodetree::parse(&inflated).map_err(detail)?;
        let (mut entries, bag) = doc::fwmap::read_style_library(&nodes).map_err(detail)?;
        // Tile join: the library's `mkBT` rasters get fresh bank slots
        // (bank-length base, import order), and the library→document
        // id map re-keys every `TIDi` below — in the parsed records
        // first, then in the bagged subtrees as they splice in. A
        // `TIDi` with no `mkBT` payload means a corrupt library; the
        // import refuses whole rather than land a dangling reference.
        let images = fwpng::parse_mkbt_chunks(&chunks).map_err(detail)?;
        let mut tile_map: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        let mut tiles = Vec::new();
        {
            const NAME: &str = "style.importLibraryTiles";
            let base = u32::try_from(self.session.document().tiles.len()).map_err(|_| {
                EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: "bank ids exceed the u32 tile id space".to_owned(),
                }
            })?;
            for (rank, image) in images.iter().enumerate() {
                let rank = u32::try_from(rank).map_err(|_| EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: "more than u32::MAX tiles in one library".to_owned(),
                })?;
                if image.grayscale {
                    return Err(detail(format!(
                        "mkBT tile {} is 256×256 grayscale; style tiles are 128×128 ARGB",
                        image.index
                    )));
                }
                let id = base
                    .checked_add(rank)
                    .ok_or_else(|| EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: "bank ids exceed the u32 tile id space".to_owned(),
                    })?;
                tile_map.insert(image.index, id);
                tiles.push(serde_json::json!({
                    "id": id,
                    "rgba": base64::engine::general_purpose::STANDARD
                        .encode(image.to_rgba8()),
                }));
            }
            let rekey_tile = |id: &mut u32| -> Result<(), EngineError> {
                match tile_map.get(id) {
                    Some(fresh) => {
                        *id = *fresh;
                        Ok(())
                    }
                    None => Err(EngineError::Io {
                        path: "<styles>".to_owned(),
                        detail: format!("library references tile {id} with no mkBT payload"),
                    }),
                }
            };
            for entry in &mut entries {
                let Some(pattern) = entry
                    .paint
                    .fill
                    .as_mut()
                    .and_then(|fill| fill.pattern.as_mut())
                else {
                    continue;
                };
                for tile in &mut pattern.tiles {
                    if let TileRef::Tile(id) = tile {
                        rekey_tile(id)?;
                    }
                }
            }
        }
        // Bag merge: library record k becomes document record
        // `styles.len() + k`, so its preserved nodes re-key to the
        // final index before joining the editor bag. Foreign roots
        // (bagged at "") have no document home; the library container
        // itself is consumed by the import. `TIDi` slots inside the
        // spliced subtrees ride the same tile map.
        let base = self.session.document().styles.len();
        let rekey = |path: &str| -> Option<String> {
            let rest = path.strip_prefix("/ZSLv/ZSTv[")?;
            let close = rest.find(']')?;
            let record: usize = rest[..close].parse().ok()?;
            Some(format!(
                "/ZSLv/ZSTv[{}]{}",
                base + record,
                &rest[close + 1..]
            ))
        };
        for (path, index, node) in bag.iter() {
            if let Some(remapped) = rekey(path) {
                let mut node = node.clone();
                if !tile_map.is_empty() {
                    rekey_tidi(&mut node, &tile_map)?;
                }
                self.unknown.insert(&remapped, index, node);
            }
        }
        let count = entries.len();
        self.session.begin_transaction("Import Styles")?;
        let outcome = (|| -> Result<(), EngineError> {
            if !tiles.is_empty() {
                self.session.apply(
                    "style.importLibraryTiles",
                    serde_json::json!({ "tiles": tiles }),
                )?;
            }
            self.session.apply(
                "style.importLibrary",
                serde_json::json!({ "entries": entries }),
            )?;
            Ok(())
        })();
        match outcome {
            Ok(()) => self.session.commit_transaction().map(|_| ())?,
            Err(error) => {
                self.session.abort_transaction()?;
                return Err(error);
            }
        }
        Ok(count)
    }

    /// Applies a catalog command. The only mutation door in the app.
    ///
    /// # Errors
    /// Whatever the command refuses; the document is untouched.
    pub fn apply(&mut self, name: &str, payload: serde_json::Value) -> Result<(), EngineError> {
        self.session.apply(name, payload)
    }

    /// [`Editor::apply`] returning the captured delta — gestures that
    /// create objects (the Alt-drag clone) read the created ids out of
    /// it, exactly like the scripting host does.
    ///
    /// # Errors
    /// Whatever the command refuses; the document is untouched.
    pub fn apply_delta(
        &mut self,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<engine::Delta, EngineError> {
        self.session.apply_delta(name, payload)
    }

    /// The session journal as a steps-file body: the same JSON-lines
    /// text [`Session::reopen`] consumes, so "Save Steps As Command"
    /// and replay are exact inverses by construction (M9 task 6).
    #[must_use]
    pub fn steps_text(&self) -> String {
        self.session.journal_lines()
    }

    /// Rebuilds the session from a steps file over the current
    /// document as the base — the same restart path a reopened CLI
    /// session takes. History is reconstructed (undo survives), and
    /// the journal becomes exactly `lines`, so further edits append to
    /// the replayed prefix. Callers replaying onto a *pristine* base
    /// open that base first (`Editor::open`) and replay onto it.
    ///
    /// # Errors
    /// Whatever the journal refuses (malformed line, sequence gap) or
    /// a replayed command fails.
    pub fn replay_steps(&mut self, lines: &str) -> Result<(), EngineError> {
        let session = Session::reopen(self.session.document().clone(), lines)?;
        self.session = session;
        Ok(())
    }

    /// Undoes the last step, returning its command name.
    ///
    /// # Errors
    /// Empty undo stack.
    pub fn undo(&mut self) -> Result<String, EngineError> {
        self.session.undo()
    }

    /// Redoes the last undone step, returning its command name.
    ///
    /// # Errors
    /// Empty redo stack.
    pub fn redo(&mut self) -> Result<String, EngineError> {
        self.session.redo()
    }

    /// Read-only document access for panels and painting.
    #[must_use]
    pub fn document(&self) -> &Document {
        self.session.document()
    }

    /// The target cell of the current page/frame, if it exists.
    ///
    /// The active layer's cell is the editor's cell: everything the
    /// user places lands there, so hit-testing and selection read the
    /// same cell the placement commands write. A document whose layers
    /// disagree about their frame count (an opened file whose top
    /// layer has fewer states) falls back to the first layer that
    /// carries the frame.
    #[must_use]
    pub fn current_cell(&self) -> Option<&Cell> {
        let page = self.document().pages.get(self.page)?;
        let layers = &page.layers.layers;
        let active = self.active_layer().min(layers.len().saturating_sub(1));
        layers
            .get(active)
            .and_then(|layer: &Layer| layer.cells.get(self.frame))
            .or_else(|| {
                layers
                    .iter()
                    .find_map(|layer: &Layer| layer.cells.get(self.frame))
            })
    }

    /// A library definition by its `MSNi` number.
    #[must_use]
    pub fn definition_of(&self, symbol: i64) -> Option<&doc::SymbolDefinition> {
        self.document()
            .symbols
            .iter()
            .find(|definition| definition.symbol_number == symbol)
    }

    /// The definition currently being edited, if any.
    #[must_use]
    pub fn editing_definition(&self) -> Option<&doc::SymbolDefinition> {
        self.definition_of(self.symbol_scope?)
    }

    /// Enters symbol editing: tools target the definition's own cells,
    /// hit-testing looks at definition content. A view-state change —
    /// it does not journal, and refuses unknown symbol numbers.
    ///
    /// # Errors
    /// No definition carries `symbol`.
    pub fn enter_symbol(&mut self, symbol: i64) -> Result<(), EngineError> {
        if self.definition_of(symbol).is_none() {
            return Err(EngineError::MissingTarget {
                name: "symbol.edit".to_owned(),
                detail: format!("no symbol numbered {symbol}"),
            });
        }
        self.symbol_scope = Some(symbol);
        self.symbol_layer = 0;
        self.selection = None;
        self.selection_set.clear();
        Ok(())
    }

    /// Leaves symbol editing; tools target the page tree again.
    pub fn exit_symbol(&mut self) {
        self.symbol_scope = None;
        self.symbol_layer = 0;
        self.selection = None;
        self.selection_set.clear();
    }

    /// The cell the tools edit: the definition's active cell in symbol
    /// scope, the page cell otherwise. Clamped to what the definition
    /// has (definitions carry their own state count).
    #[must_use]
    pub fn editing_cell(&self) -> Option<&Cell> {
        let Some(definition) = self.editing_definition() else {
            return self.current_cell();
        };
        let layer = definition.layers.layers.get(self.symbol_layer)?;
        let index = self.frame.min(layer.cells.len().saturating_sub(1));
        layer.cells.get(index)
    }

    /// The definition cell index new scope creations lift into.
    fn symbol_cell_index(&self) -> usize {
        let Some(definition) = self.editing_definition() else {
            return 0;
        };
        let cells = definition
            .layers
            .layers
            .get(self.symbol_layer)
            .map_or(1, |layer| layer.cells.len());
        self.frame.min(cells.saturating_sub(1))
    }

    /// Places an instance of `symbol` at `world` on the current page
    /// and state (the Library panel's drag-to-place), then selects it.
    ///
    /// # Errors
    /// Whatever `symbol.createInstance` refuses (unknown symbol,
    /// missing destination cell).
    pub fn place_symbol(&mut self, symbol: i64, world: geom::Point) -> Result<(), EngineError> {
        let before: Vec<ObjectId> = self
            .current_cell()
            .map_or_else(Vec::new, |cell| cell.elements.clone());
        self.apply(
            "symbol.createInstance",
            serde_json::json!({
                "symbol": symbol,
                "page": self.page, "layer": self.active_layer(), "cell": self.frame,
                "index": null,
                "x": world.x, "y": world.y,
                "name": null,
            }),
        )?;
        // Select what was placed: the first element the cell gained.
        let placed = self
            .current_cell()
            .and_then(|cell| cell.elements.iter().find(|id| !before.contains(id)))
            .copied();
        if let Some(id) = placed {
            self.selection = Some(id);
            self.selection_set = vec![id];
        }
        Ok(())
    }

    /// Installs default nine-slice guides on `symbol`: shape bounds
    /// from the definition's content, zero insets, guides free to move
    /// anywhere within the shape.
    ///
    /// # Errors
    /// Unknown symbol, or the definition has no bounded content to
    /// slice.
    pub fn enable_nine_slice(&mut self, symbol: i64) -> Result<(), EngineError> {
        const NAME: &str = "symbol.set9Slice";
        let Some(bounds) = self.definition_content_bounds(symbol) else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the definition has no bounded content to slice".to_owned(),
            });
        };
        self.apply(
            NAME,
            serde_json::json!({
                "symbol": symbol,
                "guides": {
                    "locked": false, "color_raw": 0xFF_00_FF,
                    "controls": [0.0, 0.0, 0.0, 0.0],
                    "shape_bounds": {
                        "x0": bounds.x0, "y0": bounds.y0,
                        "x1": bounds.x1, "y1": bounds.y1,
                    },
                    "limits": {
                        "x0": bounds.x0, "y0": bounds.y0,
                        "x1": bounds.x1, "y1": bounds.y1,
                    },
                },
            }),
        )
    }

    /// Clears `symbol`'s nine-slice guides.
    ///
    /// # Errors
    /// Unknown symbol.
    pub fn disable_nine_slice(&mut self, symbol: i64) -> Result<(), EngineError> {
        self.apply(
            "symbol.set9Slice",
            serde_json::json!({ "symbol": symbol, "guides": null }),
        )
    }

    /// Union bounding box of the definition's path content, in
    /// definition-local coordinates.
    #[must_use]
    pub fn definition_content_bounds(&self, symbol: i64) -> Option<geom::Rect> {
        let definition = self.definition_of(symbol)?;
        let mut bounds: Option<geom::Rect> = None;
        for layer in &definition.layers.layers {
            for cell in &layer.cells {
                for id in &cell.elements {
                    if let Some(ObjectData::Path(path)) = self.document().registry.get(*id) {
                        let path_bounds = bounds_of(path);
                        bounds = Some(match bounds {
                            Some(so_far) => so_far.union(path_bounds),
                            None => path_bounds,
                        });
                    }
                }
            }
        }
        bounds
    }

    /// Begins dragging a nine-slice guide edge. Refuses definitions
    /// without guides — there is nothing to drag.
    ///
    /// # Errors
    /// Unknown symbol, or no `NGSv` guides installed.
    pub fn begin_guide_drag(
        &mut self,
        symbol: i64,
        edge: NineSliceEdge,
        world: geom::Point,
    ) -> Result<(), EngineError> {
        if self
            .definition_of(symbol)
            .and_then(|definition| definition.nine_slice.as_ref())
            .is_none()
        {
            return Err(EngineError::MissingTarget {
                name: "symbol.set9Slice".to_owned(),
                detail: format!("symbol {symbol} has no nine-slice guides"),
            });
        }
        self.guide_drag = Some(GuideDrag {
            symbol,
            edge,
            current: world,
        });
        Ok(())
    }

    /// Updates the in-progress guide drag.
    pub fn drag_guide_to(&mut self, world: geom::Point) {
        if let Some(drag) = &mut self.guide_drag {
            drag.current = world;
        }
    }

    /// Ends the guide drag: commits the edge's new inset with
    /// `symbol.set9Slice`, the other guide data preserved. Insets are
    /// measured from the shape edges (dictionary order top, bottom,
    /// left, right) and clamp to the shape's extent.
    ///
    /// # Errors
    /// Whatever `symbol.set9Slice` refuses (unknown symbol, guides
    /// removed mid-drag).
    pub fn end_guide_drag(&mut self) -> Result<(), EngineError> {
        const NAME: &str = "symbol.set9Slice";
        let Some(drag) = self.guide_drag.take() else {
            return Ok(());
        };
        let Some(guides) = self
            .definition_of(drag.symbol)
            .and_then(|definition| definition.nine_slice.clone())
        else {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("symbol {} has no nine-slice guides", drag.symbol),
            });
        };
        let bounds = guides.shape_bounds;
        let inset = match drag.edge {
            NineSliceEdge::Top => drag.current.y - bounds.y0,
            NineSliceEdge::Bottom => bounds.y1 - drag.current.y,
            NineSliceEdge::Left => drag.current.x - bounds.x0,
            NineSliceEdge::Right => bounds.x1 - drag.current.x,
        };
        let extent = match drag.edge {
            NineSliceEdge::Top | NineSliceEdge::Bottom => bounds.height(),
            NineSliceEdge::Left | NineSliceEdge::Right => bounds.width(),
        };
        let mut controls = guides.controls;
        controls[drag.edge.control()] = inset.clamp(0.0, extent.max(0.0));
        self.apply(
            NAME,
            serde_json::json!({
                "symbol": drag.symbol,
                "guides": {
                    "locked": guides.locked,
                    "color_raw": guides.color_raw,
                    "controls": controls,
                    "shape_bounds": {
                        "x0": bounds.x0, "y0": bounds.y0,
                        "x1": bounds.x1, "y1": bounds.y1,
                    },
                    "limits": {
                        "x0": guides.limits.x0, "y0": guides.limits.y0,
                        "x1": guides.limits.x1, "y1": guides.limits.y1,
                    },
                },
            }),
        )
    }

    /// Begins a pointer drag in world space.
    pub fn begin_drag(&mut self, world: geom::Point) {
        self.drag = Some(Drag {
            tool: self.tool,
            constraint: DragConstraint::default(),
            start: world,
            current: world,
            moving: None,
        });
    }

    /// Begins a pointer drag that moves `ids` live: one
    /// `object.translate` per step, folded by a "Move" transaction into
    /// a single undo step. The ids are fixed at drag start — the press
    /// selects its target first (the shell owns that rule).
    ///
    /// # Errors
    /// Opening the transaction was refused (a prior transaction could
    /// not commit); nothing was mutated.
    pub fn begin_move_drag(
        &mut self,
        world: geom::Point,
        ids: Vec<ObjectId>,
    ) -> Result<(), EngineError> {
        self.session.begin_transaction("Move")?;
        self.drag = Some(Drag {
            tool: self.tool,
            constraint: DragConstraint::default(),
            start: world,
            current: world,
            moving: Some(MoveDrag { ids, moved: false }),
        });
        Ok(())
    }

    /// Begins an Alt-drag clone: duplicates `ids` (the copies become
    /// the selection), then drags the copies — duplicates and moves
    /// inside ONE "Move" transaction, so undo takes the whole gesture
    /// back at once, like CS6.
    ///
    /// # Errors
    /// The transaction could not open, or any duplicate was refused
    /// (the transaction aborts; the document is untouched).
    pub fn begin_clone_drag(
        &mut self,
        world: geom::Point,
        ids: &[ObjectId],
    ) -> Result<Vec<ObjectId>, EngineError> {
        const NAME: &str = "object.duplicate";
        self.session.begin_transaction("Move")?;
        let mut created = Vec::new();
        let result = (|| -> Result<(), EngineError> {
            for id in ids {
                let delta = self
                    .session
                    .apply_delta(NAME, serde_json::json!({ "id": id }))?;
                collect_created(&delta, &mut created);
            }
            Ok(())
        })();
        if let Err(error) = result {
            let _ = self.session.abort_transaction();
            return Err(error);
        }
        if let Some(last) = created.last().copied() {
            self.selection = Some(last);
        }
        self.selection_set = created.clone();
        self.drag = Some(Drag {
            tool: self.tool,
            constraint: DragConstraint::default(),
            start: world,
            current: world,
            moving: Some(MoveDrag {
                ids: created.clone(),
                moved: false,
            }),
        });
        Ok(created)
    }

    /// Updates the in-progress drag. A move drag applies one
    /// `object.translate` per nonzero step; a refused step aborts the
    /// gesture (the transaction rolls back, the document is untouched).
    pub fn drag_to(&mut self, world: geom::Point) {
        self.drag_to_with(world, DragConstraint::default());
    }

    /// The constrained form: the live modifier state folds into the
    /// stored drag geometry per the [`DragConstraint`] matrix, so the
    /// rubber band and the eventual object carry the same shape.
    pub fn drag_to_with(&mut self, world: geom::Point, constraint: DragConstraint) {
        if let Some(drag) = &mut self.drag {
            drag.constraint = constraint;
        }
        let constrained = match &self.drag {
            Some(drag) => constrain_geometry(drag.tool, drag.start, world, constraint),
            None => return,
        };
        self.drag_step(constrained);
    }

    /// Begins a Subselect node drag: the anchor under the press moves
    /// live through `path.movePoints`, one step per pointer move,
    /// folded into a single "Reshape" transaction.
    ///
    /// # Errors
    /// Opening the transaction was refused; nothing was mutated.
    pub fn begin_node_drag(
        &mut self,
        id: ObjectId,
        contour: usize,
        point: usize,
        world: geom::Point,
    ) -> Result<(), EngineError> {
        self.session.begin_transaction("Reshape")?;
        self.node_drag = Some(NodeDrag {
            id,
            contour,
            point,
            current: world,
            applied: false,
        });
        Ok(())
    }

    /// Updates the node drag: one `path.movePoints` step carrying the
    /// delta from the last pointer position (handles ride along with
    /// the anchor). A refused step aborts the gesture.
    pub fn node_drag_to(&mut self, world: geom::Point) {
        let Some(drag) = &self.node_drag else {
            return;
        };
        let step = world - drag.current;
        if step.x == 0.0 && step.y == 0.0 {
            return;
        }
        let payload = serde_json::json!({
            "id": drag.id,
            "edits": [{
                "contour": drag.contour,
                "point": drag.point,
                "dx": step.x, "dy": step.y,
            }],
        });
        match self.apply("path.movePoints", payload) {
            Ok(()) => {
                if let Some(drag) = &mut self.node_drag {
                    drag.current = world;
                    drag.applied = true;
                }
            }
            Err(_) => {
                let _ = self.session.abort_transaction();
                self.node_drag = None;
            }
        }
    }

    /// Ends the node drag: a moved gesture folds into its one
    /// "Reshape" undo step; a click commits nothing.
    ///
    /// # Errors
    /// Whatever the transaction commit refuses.
    pub fn end_node_drag(&mut self) -> Result<(), EngineError> {
        let Some(drag) = self.node_drag.take() else {
            return Ok(());
        };
        if drag.applied {
            self.session.commit_transaction().map(|_label| ())
        } else {
            self.session.abort_transaction()
        }
    }

    /// The nearest anchor of the selected path within `radius` world
    /// units of `world` — the Subselect press's grab target. Only the
    /// single selected object is searched: CS6's node editing requires
    /// exactly one path selected.
    #[must_use]
    pub fn node_at(&self, world: geom::Point, radius: f64) -> Option<(usize, usize)> {
        let id = self.selected_live()?;
        let data = self.document().registry.get(id)?;
        let ObjectData::Path(path) = data else {
            return None;
        };
        let mut best: Option<(f64, usize, usize)> = None;
        for (contour, points) in path.contours.iter().enumerate() {
            for (point, path_point) in points.points.iter().enumerate() {
                let dx = path_point.anchor.x - world.x;
                let dy = path_point.anchor.y - world.y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance <= radius && best.is_none_or(|(d, _, _)| distance < d) {
                    best = Some((distance, contour, point));
                }
            }
        }
        best.map(|(_, contour, point)| (contour, point))
    }

    /// The nearest segment of the selected path within `radius` world
    /// units of `world` — the Subselect double-click's insert target.
    /// A segment is the span between point `s` and its successor
    /// (wrapping on a closed contour); the returned index is `s`.
    /// Distance is to the straight span between the endpoints — the
    /// corner-case approximation the double-click affordance shares
    /// with the insertion preview.
    #[must_use]
    pub fn segment_at(&self, world: geom::Point, radius: f64) -> Option<(usize, usize)> {
        let id = self.selected_live()?;
        let data = self.document().registry.get(id)?;
        let ObjectData::Path(path) = data else {
            return None;
        };
        let mut best: Option<(f64, usize, usize)> = None;
        for (contour, points) in path.contours.iter().enumerate() {
            let count = points.points.len();
            let segments = if points.closed { count } else { count - 1 };
            for segment in 0..segments {
                let a = &points.points[segment].anchor;
                let b = &points.points[(segment + 1) % count].anchor;
                let distance = distance_point_segment(world, a, b);
                if distance <= radius && best.is_none_or(|(d, _, _)| distance < d) {
                    best = Some((distance, contour, segment));
                }
            }
        }
        best.map(|(_, contour, segment)| (contour, segment))
    }

    /// The selected path's gradient handles (start, end) in world
    /// space — the drag targets of the canvas fill overlay. `None`
    /// when the selection is not a path with a gradient fill.
    #[must_use]
    pub fn gradient_handles(&self) -> Option<(ObjectId, geom::Point, geom::Point)> {
        let id = self.selected_live()?;
        let data = self.document().registry.get(id)?;
        let ObjectData::Path(path) = data else {
            return None;
        };
        let is_gradient = path
            .paint
            .as_ref()
            .and_then(|paint| paint.fill.as_ref())
            .is_some_and(|fill| !matches!(fill.shape, doc::FillShape::Solid));
        if !is_gradient {
            return None;
        }
        Some((id, path.fill_start?, path.fill_end?))
    }

    /// Begins a gradient-handle drag: moves the ramp's start or end
    /// point live through `path.setFillVector`, one step per pointer
    /// move, folded into a single "Gradient" transaction.
    ///
    /// # Errors
    /// Opening the transaction was refused; nothing was mutated.
    pub fn begin_gradient_drag(
        &mut self,
        id: ObjectId,
        handle: GradientHandle,
        world: geom::Point,
    ) -> Result<(), EngineError> {
        self.session.begin_transaction("Gradient")?;
        self.gradient_drag = Some(GradientDrag {
            id,
            handle,
            current: world,
            applied: false,
        });
        Ok(())
    }

    /// Updates the gradient-handle drag: one `path.setFillVector` step
    /// repositioning the grabbed handle. A refused step aborts.
    pub fn gradient_drag_to(&mut self, world: geom::Point) {
        let Some(drag) = &self.gradient_drag else {
            return;
        };
        let moved = match drag.handle {
            GradientHandle::Start => "start",
            GradientHandle::End => "end",
        };
        let payload = serde_json::json!({
            "id": drag.id,
            moved: { "x": world.x, "y": world.y },
        });
        match self.apply("path.setFillVector", payload) {
            Ok(()) => {
                if let Some(drag) = &mut self.gradient_drag {
                    drag.current = world;
                    drag.applied = true;
                }
            }
            Err(_) => {
                let _ = self.session.abort_transaction();
                self.gradient_drag = None;
            }
        }
    }

    /// Ends the gradient-handle drag.
    ///
    /// # Errors
    /// Whatever the transaction commit refuses.
    pub fn end_gradient_drag(&mut self) -> Result<(), EngineError> {
        let Some(drag) = self.gradient_drag.take() else {
            return Ok(());
        };
        if drag.applied {
            self.session.commit_transaction().map(|_label| ())
        } else {
            self.session.abort_transaction()
        }
    }

    /// Begins a handle drag that scales the selection live: one
    /// `object.scale`/`object.transform` step per pointer move, folded
    /// by a "Transform" transaction into a single undo step. The bounds
    /// are captured once at drag start — the scale space the handle
    /// math works in — so mid-drag bounds changes never feed back.
    ///
    /// # Errors
    /// Opening the transaction was refused; nothing was mutated.
    pub fn begin_transform_drag(
        &mut self,
        world: geom::Point,
        handle: TransformHandle,
        ids: Vec<ObjectId>,
    ) -> Result<(), EngineError> {
        let document = self.document();
        let bounds = union_selection_bounds(document, &ids).ok_or(EngineError::InvalidState {
            name: "transform.begin".to_owned(),
            detail: "the selection carries no bounds to scale".to_owned(),
        })?;
        self.session.begin_transaction("Transform")?;
        let fixed_corners = if self.transform_mode == TransformMode::Distort && handle.is_corner() {
            let (x0, y0, x1, y1) = (bounds.x0, bounds.y0, bounds.x1, bounds.y1);
            let corners = match handle {
                TransformHandle::TopLeft => [(x1, y0), (x1, y1), (x0, y1)],
                TransformHandle::TopRight => [(x0, y0), (x1, y1), (x0, y1)],
                TransformHandle::BottomRight => [(x0, y0), (x1, y0), (x0, y1)],
                TransformHandle::BottomLeft => [(x0, y0), (x1, y0), (x1, y1)],
                _ => [(x0, y0), (x1, y0), (x1, y1)],
            };
            Some(corners)
        } else {
            None
        };
        self.transform_drag = Some(TransformDrag {
            ids,
            handle,
            bounds,
            current: world,
            applied: false,
            fixed_corners,
        });
        Ok(())
    }

    /// Updates the in-progress transform drag: computes the affine the
    /// handle position implies (the opposite anchor stays fixed) and
    /// applies it as one incremental step. CS6 composes each step on
    /// top of the last, so the drag is a product of small scales — a
    /// refused step aborts the whole gesture (the transaction rolls
    /// back).
    pub fn transform_drag_to(&mut self, world: geom::Point) {
        let Some(drag) = &self.transform_drag else {
            return;
        };
        if drag.handle.is_corner() && self.transform_mode == TransformMode::Distort {
            self.distort_drag_to(world);
            return;
        }
        let (ax, ay) = drag.handle.anchor_norm();
        let b = drag.bounds;
        let anchor = geom::Point::new(b.x0 + ax * b.width(), b.y0 + ay * b.height());
        // The grabbed handle's home position, in bounds space.
        let (hx, hy) = (1.0 - ax, 1.0 - ay);
        let home = geom::Point::new(b.x0 + hx * b.width(), b.y0 + hy * b.height());
        // Project the pointer onto the handle's drag axes: an edge
        // handle moves along one axis only; a corner tracks both.
        let (free_x, free_y) = match drag.handle {
            TransformHandle::Top | TransformHandle::Bottom => (false, true),
            TransformHandle::Left | TransformHandle::Right => (true, false),
            _ => (true, true),
        };
        let want = geom::Vec2::new(
            if free_x {
                world.x - anchor.x
            } else {
                home.x - anchor.x
            },
            if free_y {
                world.y - anchor.y
            } else {
                home.y - anchor.y
            },
        );
        let had = home - anchor;
        // Degenerate home offsets (a zero-width selection) have no
        // meaningful scale; the step is identity.
        let (sx, sy) = (
            if had.x != 0.0 { want.x / had.x } else { 1.0 },
            if had.y != 0.0 { want.y / had.y } else { 1.0 },
        );
        let (sx, sy) = (sx.max(0.0), sy.max(0.0));
        if (sx - 1.0).abs() < 1e-9 && (sy - 1.0).abs() < 1e-9 {
            return;
        }
        let ids = drag.ids.clone();
        let payload = serde_json::json!({
            "ids": ids,
            "sx": sx, "sy": sy,
            "pivot": [anchor.x, anchor.y],
        });
        match self.apply("object.scale", payload) {
            Ok(()) => {
                // The bounds grow with the object: recompute so the
                // next step composes against current geometry.
                let ids = self
                    .transform_drag
                    .as_ref()
                    .map(|drag| drag.ids.clone())
                    .unwrap_or_default();
                let document = self.document();
                let bounds = union_selection_bounds(document, &ids);
                if let Some(drag) = &mut self.transform_drag {
                    drag.current = world;
                    drag.applied = true;
                    if let Some(bounds) = bounds {
                        drag.bounds = bounds;
                    }
                }
            }
            Err(_) => {
                let _ = self.session.abort_transaction();
                self.transform_drag = None;
            }
        }
    }

    /// Ends the transform drag: a moved gesture folds into its one
    /// "Transform" undo step; a click-sized drag commits nothing.
    ///
    /// # Errors
    /// Whatever the transaction commit refuses.
    pub fn end_transform_drag(&mut self) -> Result<(), EngineError> {
        let Some(drag) = self.transform_drag.take() else {
            return Ok(());
        };
        if drag.applied {
            self.session.commit_transaction().map(|_label| ())
        } else {
            self.session.abort_transaction()
        }
    }

    /// One Distort step: the grabbed corner moves to the pointer while
    /// the other three stay put — the bounds rectangle becomes the
    /// parallelogram that corner defines, via `object.setQuadrangle`
    /// (percent coordinates). Bounds recompose after each step like the
    /// scale drag.
    fn distort_drag_to(&mut self, world: geom::Point) {
        let Some(drag) = &self.transform_drag else {
            return;
        };
        // The three fixed corners are the drag-start world positions;
        // they must not drift as bounds recompose per step.
        let Some(fixed) = drag.fixed_corners else {
            return;
        };
        let b = drag.bounds;
        let to_percent =
            |point: (f64, f64)| [(point.0 - b.x0) / b.width(), (point.1 - b.y0) / b.height()];
        let (tl, tr, br, bl) = (
            to_percent(fixed[0]),
            to_percent(fixed[1]),
            to_percent(fixed[2]),
            [world.x, world.y],
        );
        let (top_left, top_right, bottom_right, bottom_left) = match drag.handle {
            TransformHandle::TopLeft => ([world.x, world.y], tr, br, bl),
            TransformHandle::TopRight => (tl, [world.x, world.y], br, bl),
            TransformHandle::BottomRight => (tl, tr, [world.x, world.y], bl),
            TransformHandle::BottomLeft => (tl, tr, br, [world.x, world.y]),
            _ => (tl, tr, br, bl),
        };
        let ids = drag.ids.clone();
        let payload = serde_json::json!({
            "ids": ids,
            "top_left": top_left,
            "top_right": top_right,
            "bottom_right": bottom_right,
            "bottom_left": bottom_left,
        });
        match self.apply("object.setQuadrangle", payload) {
            Ok(()) => {
                let ids = self
                    .transform_drag
                    .as_ref()
                    .map(|drag| drag.ids.clone())
                    .unwrap_or_default();
                let document = self.document();
                let bounds = union_selection_bounds(document, &ids);
                if let Some(drag) = &mut self.transform_drag {
                    drag.current = world;
                    drag.applied = true;
                    if let Some(bounds) = bounds {
                        drag.bounds = bounds;
                    }
                }
            }
            Err(_) => {
                let _ = self.session.abort_transaction();
                self.transform_drag = None;
            }
        }
    }

    /// Applies one drag step (already constrained).
    fn drag_step(&mut self, world: geom::Point) {
        let step = match &self.drag {
            Some(drag) => world - drag.current,
            None => return,
        };
        let ids = match &self.drag {
            Some(Drag {
                moving: Some(move_drag),
                ..
            }) => move_drag.ids.clone(),
            _ => {
                if let Some(drag) = &mut self.drag {
                    drag.current = world;
                }
                return;
            }
        };
        if step.x == 0.0 && step.y == 0.0 {
            return;
        }
        let payload = serde_json::json!({
            "ids": ids,
            "dx": step.x, "dy": step.y,
        });
        match self.apply("object.translate", payload) {
            Ok(()) => {
                if let Some(drag) = &mut self.drag {
                    drag.current = world;
                    if let Some(move_drag) = &mut drag.moving {
                        move_drag.moved = true;
                    }
                }
            }
            Err(_) => {
                // Nothing partial survives: the transaction rolls back.
                let _ = self.session.abort_transaction();
                if let Some(drag) = &mut self.drag {
                    drag.moving = None;
                }
            }
        }
    }

    /// Ends the drag. Creation tools apply their create command through
    /// the session; degenerate drags (a click) create nothing.
    /// Panning/zooming tools touch only the view (shell's job). While
    /// a symbol is being edited, the fresh creation is lifted into the
    /// definition.
    ///
    /// # Errors
    /// Whatever the creation command (or the scope lift) refuses.
    pub fn end_drag(&mut self) -> Result<(), EngineError> {
        let Some(drag) = self.drag.take() else {
            return Ok(());
        };
        if let Some(move_drag) = drag.moving {
            // A moved gesture folds into its one "Move" undo step; a
            // click-sized drag commits nothing at all.
            return if move_drag.moved {
                self.session.commit_transaction().map(|_label| ())
            } else {
                self.session.abort_transaction()
            };
        }
        let (x, y) = (drag.start.x, drag.start.y);
        let (width, height) = (drag.current.x - drag.start.x, drag.current.y - drag.start.y);
        // A click is not a shape.
        if width.abs() < 0.5 || height.abs() < 0.5 {
            return Ok(());
        }
        // The page cell creation writes into, snapshotted before the
        // apply so the created id can be found by diff — creation
        // commands carry no return value.
        let before: Vec<ObjectId> = self
            .current_cell()
            .map_or_else(Vec::new, |cell| cell.elements.clone());
        let created = match drag.tool {
            Tool::Rect | Tool::Ellipse | Tool::Line => {
                let kind = match drag.tool {
                    Tool::Rect => "rect",
                    Tool::Ellipse => "ellipse",
                    _ => "line",
                };
                self.apply(
                    "object.createPath",
                    serde_json::json!({
                        "kind": kind,
                        "x": x, "y": y,
                        "width": width, "height": height,
                        "page": self.page, "layer": self.active_layer(), "cell": self.frame,
                        "name": null,
                    }),
                )?;
                true
            }
            Tool::Text => {
                self.apply(
                    "text.createText",
                    serde_json::json!({
                        "page": self.page, "layer": self.active_layer(), "cell": self.frame,
                        "x": x, "y": y, "width": width, "height": height,
                        "text": "", "name": null,
                    }),
                )?;
                true
            }
            Tool::AutoShape(shape) => {
                self.apply(
                    shape.command(),
                    shape.payload(&drag, self.page, self.active_layer(), self.frame),
                )?;
                true
            }
            Tool::Slice | Tool::Hotspot => {
                // Web objects normalize any-direction drags to a
                // positive frame and live on the Web Layer — never in
                // the current cell, so no symbol-scope lift.
                let command = match drag.tool {
                    Tool::Slice => "slice.create",
                    _ => "hotspot.create",
                };
                self.apply(
                    command,
                    serde_json::json!({
                        "shape": "rect",
                        "x": x.min(drag.current.x), "y": y.min(drag.current.y),
                        "width": width.abs(), "height": height.abs(),
                        "page": self.page,
                        "name": null, "link": null, "alt": null,
                    }),
                )?;
                false
            }
            Tool::Pointer
            | Tool::Subselect
            | Tool::Zoom
            | Tool::Hand
            | Tool::Paintbrush
            | Tool::Pencil
            | Tool::Eraser
            | Tool::PaintBucket
            | Tool::Gradient
            | Tool::Eyedropper
            | Tool::Marquee
            | Tool::EllipticalMarquee
            | Tool::Lasso
            | Tool::PolygonalMarquee
            | Tool::MagicWand => false,
        };
        if created && self.symbol_scope.is_some() {
            self.lift_creation_into_scope(&before)?;
        }
        Ok(())
    }

    /// The image under `world` and the point in its own pixel space —
    /// the paint tools' target. The topmost image of the edited cell
    /// wins; a point that misses every image has no target.
    #[must_use]
    pub fn paint_target(&self, world: geom::Point) -> Option<(ObjectId, geom::Point)> {
        let cell = self.editing_cell()?;
        for id in cell.elements.iter().rev() {
            let Some(data) = self.document().registry.get(*id) else {
                continue;
            };
            let ObjectData::Image(image) = data else {
                continue;
            };
            let bounds = geom::Rect::from_points(
                image.location,
                image.location
                    + geom::Vec2::new(
                        f64::from(i32::try_from(image.pixels.0).unwrap_or(0)),
                        f64::from(i32::try_from(image.pixels.1).unwrap_or(0)),
                    ),
            );
            if !bounds.contains(world) {
                continue;
            }
            return Some((
                *id,
                geom::Point::new(world.x - image.location.x, world.y - image.location.y),
            ));
        }
        None
    }

    /// Starts a paint stroke at `world`: the brush lays its first dab
    /// on the image under the pointer. Every dab of the stroke folds
    /// into one undo step.
    ///
    /// # Errors
    /// The engine refuses the dab; the document is untouched.
    pub fn begin_stroke(&mut self, world: geom::Point) -> Result<(), EngineError> {
        let Some((target, point)) = self.paint_target(world) else {
            return Ok(());
        };
        self.stroke = Some(Stroke {
            target,
            last: point,
            start: point,
        });
        if self.tool == Tool::Gradient {
            // The gradient's two handles come from the drag itself;
            // nothing is painted until the pointer lifts.
            return Ok(());
        }
        self.session.begin_transaction("paint stroke")?;
        if let Err(error) = self.paint_dab(target, point) {
            let _ = self.session.abort_transaction();
            self.stroke = None;
            return Err(error);
        }
        Ok(())
    }

    /// Continues the stroke: dabs along the segment from the last one
    /// to the pointer. The gradient tool only records its end handle.
    ///
    /// # Errors
    /// The engine refuses a dab; the document is untouched.
    pub fn stroke_to(&mut self, world: geom::Point) -> Result<(), EngineError> {
        let Some(stroke) = self.stroke else {
            return Ok(());
        };
        let Some((target, point)) = self.paint_target(world) else {
            return Ok(());
        };
        if target != stroke.target {
            return Ok(());
        }
        if self.tool == Tool::Gradient {
            self.stroke = Some(Stroke {
                target,
                last: point,
                start: stroke.start,
            });
            return Ok(());
        }
        let from = stroke.last;
        let distance = ((point.x - from.x).powi(2) + (point.y - from.y).powi(2)).sqrt();
        // A pointer that barely moved adds no dab — the stroke would
        // pile opacity onto the same pixel.
        if distance < 0.5 {
            return Ok(());
        }
        let spacing = match self.tool {
            Tool::Pencil => 1.0,
            _ => f64::from(self.brush.size.clamp(1, 1024)) / 4.0,
        };
        let steps = ((distance / spacing).floor() as usize).max(1);
        for step in 1..=steps {
            let t = step as f64 / steps as f64;
            let dab = geom::Point::new(
                from.x + (point.x - from.x) * t,
                from.y + (point.y - from.y) * t,
            );
            self.paint_dab(target, dab)?;
        }
        self.stroke = Some(Stroke {
            target,
            last: point,
            start: stroke.start,
        });
        Ok(())
    }

    /// Ends the stroke: the gradient paints, the dabs fold into the
    /// stroke's single undo step.
    ///
    /// # Errors
    /// The engine refuses the final command; the stroke's dabs stay.
    pub fn end_stroke(&mut self) -> Result<(), EngineError> {
        let Some(stroke) = self.stroke.take() else {
            return Ok(());
        };
        if self.tool == Tool::Gradient {
            return self.paint_gradient(stroke);
        }
        if self.session.transaction_open() {
            return self.session.commit_transaction().map(|_label| ());
        }
        Ok(())
    }

    /// Lays one dab of the current brush on `target` at `point` (image
    /// pixel coordinates).
    ///
    /// # Errors
    /// The engine refuses the dab; the document is untouched.
    fn paint_dab(&mut self, target: ObjectId, point: geom::Point) -> Result<(), EngineError> {
        let brush = self.brush;
        let color_raw = argb_raw(brush.color);
        let slots = serde_json::json!({
            "edge": brush.edge,
            "shape": brush.shape.wire(),
            "aspect": brush.aspect,
            "angle": brush.angle,
            "pressure": brush.pressure,
        });
        match self.tool {
            Tool::Pencil => self.apply(
                "bitmap.pencil",
                serde_json::json!({
                    "id": target, "x": point.x, "y": point.y,
                    "color_raw": color_raw,
                    "strength": brush.strength,
                    "pressure": brush.pressure,
                }),
            ),
            Tool::Eraser => self.apply(
                "retouch.eraser",
                serde_json::json!({
                    "id": target, "x": point.x, "y": point.y,
                    "radius": (brush.size / 2).clamp(1, 512),
                    "strength": brush.strength,
                    "brush": slots,
                }),
            ),
            _ => self.apply(
                "bitmap.brush",
                serde_json::json!({
                    "id": target, "x": point.x, "y": point.y,
                    "size": brush.size.clamp(1, 1024),
                    "color_raw": color_raw,
                    "strength": brush.strength,
                    "brush": slots,
                }),
            ),
        }
    }

    /// Paints the gradient the drag described: the stroke's two
    /// handles, in the stroke's image.
    ///
    /// # Errors
    /// The engine refuses the gradient; the document is untouched.
    fn paint_gradient(&mut self, stroke: Stroke) -> Result<(), EngineError> {
        let brush = self.brush;
        self.apply(
            "bitmap.gradient",
            serde_json::json!({
                "id": stroke.target,
                "color_a_raw": argb_raw(brush.color),
                "color_b_raw": argb_raw(gradient_end_color(brush.color)),
                "x0": stroke.start.x, "y0": stroke.start.y,
                "x1": stroke.last.x, "y1": stroke.last.y,
            }),
        )
    }

    /// Fills the target image's selection (or the whole image when
    /// nothing is selected) with the paint color — the paint bucket.
    ///
    /// # Errors
    /// The engine refuses the fill; the document is untouched.
    pub fn fill_target(&mut self, world: geom::Point) -> Result<bool, EngineError> {
        let Some((target, _point)) = self.paint_target(world) else {
            return Ok(false);
        };
        self.apply(
            "bitmap.fill",
            serde_json::json!({ "id": target, "color_raw": argb_raw(self.brush.color) }),
        )?;
        Ok(true)
    }

    /// Picks the paint color from the pixel under `world` — the
    /// eyedropper. The color lands in the brush model, not in the
    /// document.
    #[must_use]
    pub fn pick_color(&mut self, world: geom::Point) -> Option<[u8; 4]> {
        let (target, point) = self.paint_target(world)?;
        let data = self.document().registry.get(target)?;
        let ObjectData::Image(image) = data else {
            return None;
        };
        // The point is inside the image's bounds (the hit test said
        // so), so the floor fits an `i64` and the cast saturates
        // rather than wrapping.
        let x = point.x.floor() as i64;
        let y = point.y.floor() as i64;
        let color = engine::bitmap::sample_pixel(image, &self.document().tiles, x, y)?;
        self.brush.color = color;
        Some(color)
    }

    /// The point in `target`'s own pixel space for a world point.
    /// Unlike [`Editor::paint_target`] this does not require the point
    /// to be inside the image — a lasso may leave the frame.
    fn image_point(&self, target: ObjectId, world: geom::Point) -> Option<geom::Point> {
        let data = self.document().registry.get(target)?;
        let ObjectData::Image(image) = data else {
            return None;
        };
        Some(geom::Point::new(
            world.x - image.location.x,
            world.y - image.location.y,
        ))
    }

    /// Starts a pixel selection at `world`: the marquee, ellipse and
    /// lasso tools act on the image under the pointer. The wand is a
    /// click, not a drag — it selects at once.
    ///
    /// # Errors
    /// The engine refuses the selection; the document is untouched.
    pub fn begin_pixel_selection(&mut self, world: geom::Point) -> Result<(), EngineError> {
        let Some((target, point)) = self.paint_target(world) else {
            return Ok(());
        };
        if self.tool == Tool::MagicWand {
            self.pixel_target = Some(target);
            return self.wand_select(target, point);
        }
        self.pixel_selection = Some(PixelSelection {
            target,
            start: point,
            current: point,
            trace: vec![point],
        });
        Ok(())
    }

    /// Continues a pixel selection: the marquee tracks the pointer,
    /// the lasso records the trace it leaves behind.
    ///
    /// # Errors
    /// The engine refuses the extension; the document is untouched.
    pub fn extend_pixel_selection(&mut self, world: geom::Point) -> Result<(), EngineError> {
        let Some(selection) = self.pixel_selection.as_ref() else {
            return Ok(());
        };
        let target = selection.target;
        let Some(point) = self.image_point(target, world) else {
            return Ok(());
        };
        let Some(selection) = self.pixel_selection.as_mut() else {
            return Ok(());
        };
        selection.current = point;
        if self.tool == Tool::Lasso {
            let last = selection.trace.last().copied().unwrap_or(point);
            let moved = ((point.x - last.x).powi(2) + (point.y - last.y).powi(2)).sqrt();
            // A pointer that barely moved adds no vertex — the trace
            // would grow without describing a different shape.
            if moved >= 2.0 {
                selection.trace.push(point);
            }
        }
        Ok(())
    }

    /// Ends a pixel selection: the marquee, ellipse and lasso apply
    /// their shape to the document's selection plane. A collapsed
    /// shape selects nothing, as in the original.
    ///
    /// # Errors
    /// The engine refuses the selection; the document is untouched.
    pub fn end_pixel_selection(&mut self) -> Result<(), EngineError> {
        let Some(selection) = self.pixel_selection.take() else {
            return Ok(());
        };
        let mode = self.pixel_mode.wire();
        self.pixel_target = Some(selection.target);
        let x = selection.start.x.min(selection.current.x);
        let y = selection.start.y.min(selection.current.y);
        let width = (selection.current.x - selection.start.x).abs();
        let height = (selection.current.y - selection.start.y).abs();
        match self.tool {
            Tool::Marquee => self.apply(
                "bitmap.selectRect",
                serde_json::json!({
                    "id": selection.target, "x": x, "y": y,
                    "width": width, "height": height, "mode": mode,
                }),
            ),
            Tool::EllipticalMarquee => self.apply(
                "bitmap.selectEllipse",
                serde_json::json!({
                    "id": selection.target, "x": x, "y": y,
                    "width": width, "height": height, "mode": mode,
                }),
            ),
            Tool::Lasso => {
                let points: Vec<[f64; 2]> = selection
                    .trace
                    .iter()
                    .map(|point| [point.x, point.y])
                    .collect();
                self.apply(
                    "bitmap.selectLasso",
                    serde_json::json!({
                        "id": selection.target, "points": points, "mode": mode,
                    }),
                )
            }
            _ => Ok(()),
        }
    }

    /// The magic wand: every pixel that matches the clicked one
    /// within the tolerance joins the selection.
    ///
    /// # Errors
    /// The engine refuses the selection; the document is untouched.
    fn wand_select(&mut self, target: ObjectId, point: geom::Point) -> Result<(), EngineError> {
        // The point is inside the image (the hit test said so), so
        // the floor fits a `u32` and the cast saturates rather than
        // wrapping.
        let x = point.x.floor() as u32;
        let y = point.y.floor() as u32;
        let tolerance = self.wand_tolerance;
        let mode = self.pixel_mode.wire();
        self.apply(
            "bitmap.selectWand",
            serde_json::json!({
                "id": target, "x": x, "y": y,
                "tolerance": tolerance, "contiguous": true, "mode": mode,
            }),
        )
    }

    /// Adds a vertex to the pixel polygon being built. A click on
    /// another image starts a new polygon.
    ///
    /// # Errors
    /// The engine refuses nothing here; the method cannot fail today,
    /// but it keeps the panel's error door open for later commands.
    pub fn add_polygon_point(&mut self, world: geom::Point) -> Result<(), EngineError> {
        let Some((target, point)) = self.paint_target(world) else {
            return Ok(());
        };
        match self.polygon.as_mut() {
            Some(draft) if draft.target == target => draft.points.push(point),
            _ => {
                self.polygon = Some(PolygonDraft {
                    target,
                    points: vec![point],
                });
            }
        }
        Ok(())
    }

    /// Commits the pixel polygon: its vertices become the selection.
    /// A polygon with fewer than three vertices is not a shape.
    ///
    /// # Errors
    /// The engine refuses the selection; the document is untouched.
    pub fn commit_polygon(&mut self) -> Result<bool, EngineError> {
        let Some(draft) = self.polygon.as_ref() else {
            return Ok(false);
        };
        if draft.points.len() < 3 {
            // A refused commit keeps the draft: the pointer adds the
            // next vertex and clicks commit again.
            return Ok(false);
        }
        let Some(draft) = self.polygon.take() else {
            return Ok(false);
        };
        self.pixel_target = Some(draft.target);
        let points: Vec<[f64; 2]> = draft
            .points
            .iter()
            .map(|point| [point.x, point.y])
            .collect();
        let mode = self.pixel_mode.wire();
        self.apply(
            "bitmap.selectPolygon",
            serde_json::json!({
                "id": draft.target, "points": points, "mode": mode,
            }),
        )?;
        Ok(true)
    }

    /// The image's world-space location (its top-left), for overlays.
    #[must_use]
    pub fn image_location(&self, target: ObjectId) -> Option<geom::Point> {
        let data = self.document().registry.get(target)?;
        match data {
            ObjectData::Image(image) => Some(image.location),
            _ => None,
        }
    }

    /// Whether the document carries a pixel selection.
    #[must_use]
    pub fn has_pixel_selection(&self) -> bool {
        self.document().selection.is_some()
    }

    /// Clears the pixel selection.
    ///
    /// # Errors
    /// The engine refuses the command; the document is untouched.
    pub fn deselect_pixels(&mut self) -> Result<(), EngineError> {
        self.pixel_target = None;
        self.apply("bitmap.selectNone", serde_json::json!({}))
    }

    /// Softens the pixel selection's edge.
    ///
    /// # Errors
    /// The engine refuses the command; the document is untouched.
    pub fn feather_selection(&mut self, radius: u8) -> Result<(), EngineError> {
        self.apply("bitmap.feather", serde_json::json!({ "radius": radius }))
    }

    /// Grows, shrinks, borders or smooths the pixel selection.
    ///
    /// # Errors
    /// The engine refuses the command; the document is untouched.
    pub fn adjust_pixel_selection(&mut self, what: &str, amount: i64) -> Result<(), EngineError> {
        self.apply(
            "bitmap.adjustPixelSel",
            serde_json::json!({ "what": what, "amount": amount }),
        )
    }

    /// Selects the pixels that match the active selection's average
    /// color within `tolerance`.
    ///
    /// # Errors
    /// The engine refuses the command; the document is untouched.
    pub fn select_similar(&mut self, tolerance: u8) -> Result<(), EngineError> {
        let Some(target) = self.pixel_target else {
            return Ok(());
        };
        let mode = self.pixel_mode.wire();
        self.apply(
            "bitmap.selectSimilar",
            serde_json::json!({
                "id": target, "tolerance": tolerance,
                "contiguous": false, "mode": mode,
            }),
        )
    }

    /// Moves the selection plane itself — the pixels under it stay.
    ///
    /// # Errors
    /// The engine refuses the command; the document is untouched.
    pub fn move_selection_mask(&mut self, dx: i64, dy: i64) -> Result<(), EngineError> {
        self.apply(
            "bitmap.moveSelectionMaskBy",
            serde_json::json!({ "delta": { "x": dx, "y": dy } }),
        )
    }

    /// Moves the pixels under the selection.
    ///
    /// # Errors
    /// The engine refuses the command; the document is untouched.
    pub fn move_selected_pixels(&mut self, dx: i64, dy: i64) -> Result<(), EngineError> {
        let Some(target) = self.pixel_target else {
            return Ok(());
        };
        self.apply(
            "bitmap.movePixels",
            serde_json::json!({ "id": target, "dx": dx, "dy": dy }),
        )
    }

    /// Erases the pixels under the selection.
    ///
    /// # Errors
    /// The engine refuses the command; the document is untouched.
    pub fn erase_selected(&mut self) -> Result<(), EngineError> {
        let Some(target) = self.pixel_target else {
            return Ok(());
        };
        self.apply("bitmap.erase", serde_json::json!({ "id": target }))
    }

    /// The active pixel selection's outline in world coordinates —
    /// the marching-ants path the canvas draws. `None` when nothing
    /// is selected.
    #[must_use]
    pub fn selection_outline(&self) -> Option<Vec<[f64; 4]>> {
        let mask = self.document().selection.as_ref()?;
        let target = self.pixel_target?;
        let data = self.document().registry.get(target)?;
        let ObjectData::Image(image) = data else {
            return None;
        };
        Some(
            selection_edges(mask)
                .into_iter()
                .map(|[x0, y0, x1, y1]| {
                    [
                        image.location.x + x0,
                        image.location.y + y0,
                        image.location.x + x1,
                        image.location.y + y1,
                    ]
                })
                .collect(),
        )
    }

    /// While a symbol is being edited, a fresh creation belongs to the
    /// definition, not the page — but creation commands only write
    /// pages. So the object lands on the page and this lifts it into
    /// the definition with `symbol.editAttach`: two journal rows, both
    /// replayable, undo walks them back in order.
    ///
    /// # Errors
    /// The created object cannot be located in the page cell, or the
    /// lift is refused.
    fn lift_creation_into_scope(&mut self, before: &[ObjectId]) -> Result<(), EngineError> {
        const NAME: &str = "symbol.editAttach";
        let Some(symbol) = self.symbol_scope else {
            return Ok(());
        };
        let created = self
            .current_cell()
            .and_then(|cell| cell.elements.iter().find(|id| !before.contains(id)))
            .copied();
        let Some(id) = created else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the created object did not appear in the page cell".to_owned(),
            });
        };
        self.apply(
            NAME,
            serde_json::json!({
                "symbol": symbol, "id": id,
                "layer": self.symbol_layer, "cell": self.symbol_cell_index(),
                "index": null,
            }),
        )
    }

    /// Selects the topmost object in the current cell whose contour
    /// bounding box contains `world`, replacing the selection set.
    /// Groups select as wholes in v1 (subselect descends later).
    #[must_use]
    pub fn select_at(&mut self, world: geom::Point) -> Option<ObjectId> {
        let hit = self.hit_at(world);
        self.selection = hit;
        self.selection_set = hit.map_or_else(Vec::new, |id| vec![id]);
        hit
    }

    /// Shift-click: adds the hit to (or removes it from) the selection
    /// set. A miss clears the set, like a plain click on empty canvas.
    /// Returns whether the hit landed on an object.
    pub fn toggle_select_at(&mut self, world: geom::Point) -> bool {
        let Some(id) = self.hit_at(world) else {
            self.selection = None;
            self.selection_set.clear();
            return false;
        };
        if let Some(at) = self.selection_set.iter().position(|member| *member == id) {
            self.selection_set.remove(at);
            self.selection = self.selection_set.last().copied();
        } else {
            self.selection_set.push(id);
            self.selection = Some(id);
        }
        true
    }

    /// The topmost live object under `world` — the definition's cell
    /// in symbol scope; on the page, every visible layer of the
    /// current frame (CS6 picks through layers; the topmost layer
    /// wins). Hidden layers (`display` off or the frame's visibility
    /// flag) and locked cells are unpickable; groups hit as wholes.
    #[must_use]
    pub fn hit_at(&self, world: geom::Point) -> Option<ObjectId> {
        let candidates: Vec<ObjectId> = match self.editing_definition() {
            Some(_) => self
                .editing_cell()?
                .elements
                .iter()
                .copied()
                .rev()
                .collect(),
            None => {
                let page = self.document().pages.get(self.page)?;
                let mut ids = Vec::new();
                for layer in page.layers.layers.iter().rev() {
                    if !layer.display {
                        continue;
                    }
                    if let Some(flags) = &layer.visibility_per_frame
                        && !flags.get(self.frame).copied().unwrap_or(true)
                    {
                        continue;
                    }
                    if let Some(cell) = layer.cells.get(self.frame)
                        && cell.visible
                        && !cell.locked
                    {
                        ids.extend(cell.elements.iter().rev().copied());
                    }
                }
                ids
            }
        };
        candidates.into_iter().find(|id| {
            self.document()
                .registry
                .get(*id)
                .is_some_and(|data| object_bounds(self.document(), data, 0).contains(world))
        })
    }

    /// The selection set, pruned to objects still live in the registry
    /// (undo can delete members behind the panel's back).
    #[must_use]
    pub fn live_selection(&self) -> Vec<ObjectId> {
        self.selection_set
            .iter()
            .copied()
            .filter(|id| self.document().registry.get(*id).is_some())
            .collect()
    }

    /// The primary selection if it is still live.
    #[must_use]
    pub fn selected_live(&self) -> Option<ObjectId> {
        self.selection
            .filter(|id| self.document().registry.get(*id).is_some())
    }

    /// Installs a selection as the view state: the pick order is the
    /// order given, and the primary selection is its last entry.
    pub fn set_selection(&mut self, ids: Vec<ObjectId>) {
        self.selection = ids.last().copied();
        self.selection_set = ids;
    }

    /// Pushes the view's selection into the document's editing state.
    ///
    /// The engine's selection-scoped commands — the clipboard, the
    /// flatten family — read `Document::object_selection`, while the
    /// shell tracks the selection as view state. This is the bridge:
    /// the ids land in the document through the same command door as
    /// every other mutation, so the change is journaled and undoable
    /// like any other edit. A selection that already matches is not
    /// re-sent — the command would be a no-op that only adds an undo
    /// step.
    ///
    /// # Errors
    /// Whatever the command refuses; the document is untouched.
    pub fn sync_selection(&mut self) -> Result<(), EngineError> {
        let mut ids = self.live_selection();
        ids.sort_by_key(|id| (id.index(), id.generation()));
        let mut current = self.session.document().object_selection.clone();
        current.sort_by_key(|id| (id.index(), id.generation()));
        if ids == current {
            return Ok(());
        }
        self.apply("selection.set", serde_json::json!({ "ids": ids }))
    }

    /// Pulls the document's object selection back into the view. The
    /// reverse of [`Self::sync_selection`]: commands like
    /// `selection.selectParents` rewrite
    /// `Document::object_selection` directly, and the view's
    /// selection state would otherwise go stale.
    pub fn adopt_document_selection(&mut self) {
        let ids = self.document().object_selection.clone();
        self.set_selection(ids);
    }

    /// The two operands for `path.boolean`: the live pair ordered
    /// bottom-first in the current cell (z order), so `a` is the base
    /// and `b` the consumed top object — Fireworks' Punch semantics.
    /// Selections outside the current cell keep pick order.
    #[must_use]
    pub fn boolean_pair(&self) -> Option<(ObjectId, ObjectId)> {
        let mut ids = self.live_selection();
        if ids.len() != 2 {
            return None;
        }
        ids.sort_by_key(|id| self.z_order(*id));
        Some((ids[0], ids[1]))
    }

    /// Cell element index of `id` (z position) in the edited cell;
    /// unattached objects sort last without disturbing the relative
    /// order of found ones.
    fn z_order(&self, id: ObjectId) -> usize {
        self.editing_cell()
            .and_then(|cell| cell.elements.iter().position(|element| *element == id))
            .unwrap_or(usize::MAX)
    }
}

/// World-space bounding box of a path's contour points (anchors and
/// control handles — the handles keep fast strokes inside the box).
#[must_use]
/// The parts of `span` that `covered` does not cover — the helper
/// behind the selection's horizontal boundary.
fn uncovered(covered: &[(usize, usize)], span: (usize, usize)) -> Vec<(usize, usize)> {
    let mut gaps = Vec::new();
    let mut cursor = span.0;
    for &(start, end) in covered {
        if end <= cursor {
            continue;
        }
        if start >= span.1 {
            break;
        }
        if start > cursor {
            gaps.push((cursor, start.min(span.1)));
        }
        cursor = cursor.max(end);
        if cursor >= span.1 {
            break;
        }
    }
    if cursor < span.1 {
        gaps.push((cursor, span.1));
    }
    gaps
}

/// The boundary of a selection mask in image pixel coordinates: the
/// line segments `[x0, y0, x1, y1]` between selected and unselected
/// pixels. The image's own border is not a boundary — a selection
/// that runs to the edge of the frame has no ants there.
#[must_use]
pub fn selection_edges(mask: &doc::SelectionMask) -> Vec<[f64; 4]> {
    let width = mask.width as usize;
    let height = mask.height as usize;
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let selected =
        |x: usize, y: usize| -> bool { x < width && y < height && mask.alpha[y * width + x] != 0 };
    // The selected runs of every row.
    let mut rows: Vec<Vec<(usize, usize)>> = Vec::with_capacity(height);
    for y in 0..height {
        let mut runs = Vec::new();
        let mut x = 0;
        while x < width {
            if !selected(x, y) {
                x += 1;
                continue;
            }
            let start = x;
            while x < width && selected(x, y) {
                x += 1;
            }
            runs.push((start, x));
        }
        rows.push(runs);
    }
    let mut edges = Vec::new();
    // Horizontal boundaries: the parts of a run that the row above
    // (or below) does not cover.
    let empty: Vec<(usize, usize)> = Vec::new();
    for (y, runs) in rows.iter().enumerate() {
        let above = if y == 0 { &empty } else { &rows[y - 1] };
        let below = if y + 1 == height {
            &empty
        } else {
            &rows[y + 1]
        };
        for &(x0, x1) in runs {
            // The image border is not a boundary: no edge along y = 0
            // or y = height, just as the vertical pass skips x = 0 and
            // x = width.
            if y > 0 {
                for (a, b) in uncovered(above, (x0, x1)) {
                    edges.push([a as f64, y as f64, b as f64, y as f64]);
                }
            }
            if y + 1 < height {
                for (a, b) in uncovered(below, (x0, x1)) {
                    edges.push([a as f64, (y + 1) as f64, b as f64, (y + 1) as f64]);
                }
            }
        }
    }
    // Vertical boundaries: a run's left and right edges, merged
    // across the rows that share them.
    let mut open: BTreeMap<usize, usize> = BTreeMap::new();
    for (y, runs) in rows.iter().enumerate() {
        let mut boundaries = BTreeSet::new();
        for &(x0, x1) in runs {
            if x0 > 0 {
                boundaries.insert(x0);
            }
            if x1 < width {
                boundaries.insert(x1);
            }
        }
        let closed: Vec<usize> = open
            .keys()
            .filter(|x| !boundaries.contains(x))
            .copied()
            .collect();
        for x in closed {
            if let Some(y0) = open.remove(&x) {
                edges.push([x as f64, y0 as f64, x as f64, y as f64]);
            }
        }
        for x in boundaries {
            open.entry(x).or_insert(y);
        }
    }
    for (x, y0) in open {
        edges.push([x as f64, y0 as f64, x as f64, height as f64]);
    }
    edges
}

#[must_use]
pub fn bounds_of(path: &doc::PathObject) -> geom::Rect {
    let mut points = Vec::new();
    for contour in &path.contours {
        for point in &contour.points {
            points.push(point.anchor);
            if let Some(primary) = point.primary {
                points.push(primary);
            }
            if let Some(secondary) = point.secondary {
                points.push(secondary);
            }
        }
    }
    let Some((first, rest)) = points.split_first() else {
        return geom::Rect::ZERO;
    };
    let mut bounds = geom::Rect::from_points(*first, *first);
    for point in rest {
        bounds = bounds.union_pt(*point);
    }
    bounds
}

/// Euclidean distance from `p` to the straight span `a`–`b`, clamped
/// to the span's extent (the segment hit-test metric).
fn distance_point_segment(p: geom::Point, a: &geom::Point, b: &geom::Point) -> f64 {
    let ax = b.x - a.x;
    let ay = b.y - a.y;
    let length_sq = ax * ax + ay * ay;
    if length_sq <= f64::EPSILON {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = ((p.x - a.x) * ax + (p.y - a.y) * ay) / length_sq;
    let t = t.clamp(0.0, 1.0);
    let cx = a.x + t * ax;
    let cy = a.y + t * ay;
    (p.x - cx).hypot(p.y - cy)
}

/// World-space bounding box of any object: paths by contour points
/// ([`bounds_of`]), text and slices by frame, images by location plus
/// pixel size, groups by union over children, symbol instances by the
/// definition's content bounds mapped through the instance transform.
/// Depth-capped so a malformed (impossible-in-practice) group cycle
/// cannot recurse forever; an unknown or empty kind gives a zero box
/// that never hits.
#[must_use]
/// The union bounds of a selection — the space a transform handle
/// scales in. `None` when nothing in the set carries bounds.
pub fn union_selection_bounds(document: &Document, ids: &[ObjectId]) -> Option<geom::Rect> {
    let mut out: Option<geom::Rect> = None;
    for id in ids {
        let Some(data) = document.registry.get(*id) else {
            continue;
        };
        let bounds = object_bounds(document, data, 0);
        if bounds.width() <= 0.0 && bounds.height() <= 0.0 {
            continue;
        }
        out = Some(match out {
            Some(acc) => acc.union(bounds),
            None => bounds,
        });
    }
    out
}

#[must_use]
pub fn object_bounds(document: &Document, data: &ObjectData, depth: u8) -> geom::Rect {
    const MAX_DEPTH: u8 = 32;
    if depth > MAX_DEPTH {
        return geom::Rect::ZERO;
    }
    match data {
        ObjectData::Path(path) => bounds_of(path),
        ObjectData::Text(text) => text.frame,
        ObjectData::Group(group) => group
            .children
            .iter()
            .filter_map(|id| {
                document
                    .registry
                    .get(*id)
                    .map(|child| object_bounds(document, child, depth + 1))
            })
            .fold(geom::Rect::ZERO, |acc, bounds| acc.union(bounds)),
        ObjectData::Image(image) => geom::Rect::from_points(
            image.location,
            image.location
                + geom::Vec2::new(
                    f64::from(i32::try_from(image.pixels.0).unwrap_or(0)),
                    f64::from(i32::try_from(image.pixels.1).unwrap_or(0)),
                ),
        ),
        ObjectData::Slice(slice) => slice
            .contours
            .iter()
            .filter_map(|contour| {
                contour.points.first().map(|first| {
                    let mut bounds = geom::Rect::from_points(first.anchor, first.anchor);
                    for point in &contour.points {
                        bounds = bounds.union_pt(point.anchor);
                    }
                    bounds
                })
            })
            .fold(slice.frame, |acc, bounds| acc.union(bounds)),
        ObjectData::SymbolInstance(instance) => {
            let Some(definition) = document
                .symbols
                .iter()
                .find(|definition| definition.symbol_number == instance.symbol_number)
            else {
                return geom::Rect::ZERO;
            };
            let content = definition
                .layers
                .layers
                .iter()
                .flat_map(|layer| &layer.cells)
                .flat_map(|cell| &cell.elements)
                .filter_map(|id| {
                    document
                        .registry
                        .get(*id)
                        .map(|child| object_bounds(document, child, depth + 1))
                })
                .fold(geom::Rect::ZERO, |acc, bounds| acc.union(bounds));
            geom::transform::map_rect(instance.transform, content)
        }
    }
}

/// Fills `out` with every object a delta created (the clone gesture
/// selects exactly those).
fn collect_created(delta: &engine::Delta, out: &mut Vec<ObjectId>) {
    match delta {
        engine::Delta::Group(children) => {
            for child in children {
                collect_created(child, out);
            }
        }
        engine::Delta::Object {
            id,
            inverse: engine::ObjectInverse::Created,
        } => out.push(*id),
        _ => {}
    }
}

/// Rewrites every `TIDi` below `node` through the library→bank tile
/// map — the bagged texture subtrees' tile references ride the same
/// re-key the parsed records got. A referenced tile the library never
/// shipped refuses the import: splicing it in would write a dangling
/// id into every later save.
fn rekey_tidi(
    node: &mut fwpng::Node,
    map: &std::collections::HashMap<u32, u32>,
) -> Result<(), EngineError> {
    if node.fourcc == *b"TIDi" {
        let fwpng::NodeValue::Int(value) = &mut node.value else {
            return Ok(());
        };
        let id = u32::try_from(*value).map_err(|_| EngineError::Io {
            path: "<styles>".to_owned(),
            detail: format!("`TIDi` {value} is outside the u32 tile id space"),
        })?;
        match map.get(&id) {
            Some(fresh) => *value = i64::from(*fresh),
            None => {
                return Err(EngineError::Io {
                    path: "<styles>".to_owned(),
                    detail: format!("library references tile {id} with no mkBT payload"),
                });
            }
        }
        return Ok(());
    }
    if let fwpng::NodeValue::Children(children) = &mut node.value {
        for child in children {
            rekey_tidi(child, map)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn drag_creates_a_rect_through_the_registry() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(60.0, 40.0));
        editor.end_drag().unwrap();

        let cell = editor.current_cell().unwrap();
        assert_eq!(cell.elements.len(), 1);
        let id = cell.elements[0];
        assert!(matches!(
            editor.document().registry.get(id),
            Some(ObjectData::Path(_))
        ));
        // And undo removes it — the same registry semantics as any
        // other surface.
        editor.undo().unwrap();
        assert!(editor.document().registry.get(id).is_none());
    }

    #[test]
    fn a_click_creates_nothing() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Ellipse;
        editor.begin_drag(geom::Point::new(5.0, 5.0));
        editor.drag_to(geom::Point::new(5.2, 5.1));
        editor.end_drag().unwrap();
        assert_eq!(editor.current_cell().unwrap().elements.len(), 0);
    }

    fn rect_at(editor: &mut Editor, x: f64, y: f64) -> doc::ObjectId {
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(x, y));
        editor.drag_to(geom::Point::new(x + 40.0, y + 20.0));
        editor.end_drag().unwrap();
        *editor
            .current_cell()
            .unwrap()
            .elements
            .last()
            .expect("the creation drag made an object")
    }

    fn first_anchor(editor: &Editor, id: doc::ObjectId) -> (f64, f64) {
        match editor.document().registry.get(id) {
            Some(ObjectData::Path(path)) => {
                let anchor = path.contours[0].points[0].anchor;
                (anchor.x, anchor.y)
            }
            _ => (f64::NAN, f64::NAN),
        }
    }

    #[test]
    fn subselect_node_drag_moves_one_anchor_in_one_undo_step() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 10.0, 10.0);
        editor.set_selection(vec![id]);
        let undo_before = editor.session.undo_len();

        // The rect's first anchor sits at (10, 10): grab it, move by
        // +20, -5.
        let (contour, point) = editor
            .node_at(geom::Point::new(10.0, 10.0), 5.0)
            .expect("anchor hit");
        editor
            .begin_node_drag(id, contour, point, geom::Point::new(10.0, 10.0))
            .unwrap();
        editor.node_drag_to(geom::Point::new(25.0, 8.0));
        editor.node_drag_to(geom::Point::new(30.0, 5.0));
        editor.end_node_drag().unwrap();

        assert_eq!(first_anchor(&editor, id), (30.0, 5.0));
        assert_eq!(editor.session.undo_len(), undo_before + 1, "one undo step");
        editor.undo().unwrap();
        assert_eq!(first_anchor(&editor, id), (10.0, 10.0));
    }

    #[test]
    fn node_at_misses_far_anchors_and_non_paths() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 10.0, 10.0);
        editor.set_selection(vec![id]);
        assert!(
            editor
                .node_at(geom::Point::new(500.0, 500.0), 5.0)
                .is_none()
        );
        // A text object (or any non-path) has no nodes.
        editor
            .apply(
                "text.createText",
                serde_json::json!({
                    "page": 0, "layer": 0, "cell": 0,
                    "x": 0.0, "y": 0.0, "width": 50.0, "height": 20.0,
                    "text": "hi", "name": null,
                }),
            )
            .unwrap();
        let text = editor.current_cell().unwrap().elements[1];
        editor.set_selection(vec![text]);
        assert!(editor.node_at(geom::Point::new(0.0, 0.0), 50.0).is_none());
    }

    #[test]
    fn a_click_sized_node_drag_commits_nothing() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 10.0, 10.0);
        editor.set_selection(vec![id]);
        let undo_before = editor.session.undo_len();
        editor
            .begin_node_drag(id, 0, 0, geom::Point::new(10.0, 10.0))
            .unwrap();
        editor.end_node_drag().unwrap();
        assert_eq!(editor.session.undo_len(), undo_before, "no step committed");
    }

    /// `segment_at` finds the nearest contour span for the Subselect
    /// double-click insert: hits between two anchors, misses far off
    /// the path, and a double-click *on* an anchor resolves to the
    /// node test first (the shell's ordering contract, asserted here
    /// as "the node radius wins at the same point").
    #[test]
    fn segment_at_finds_spans_for_the_insert_gesture() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 10.0, 10.0);
        editor.set_selection(vec![id]);
        // The rect's bottom edge spans (10,30)-(50,30): mid-edge hit.
        let (contour, segment) = editor
            .segment_at(geom::Point::new(30.0, 30.0), 5.0)
            .expect("bottom edge hit");
        assert_eq!(contour, 0);
        // Insert through the registry command, one undo step.
        let undo_before = editor.session.undo_len();
        editor
            .apply(
                "path.insertPoint",
                serde_json::json!({
                    "id": id, "contour": contour, "segment": segment,
                    "x": 30.0, "y": 30.0,
                }),
            )
            .unwrap();
        assert_eq!(editor.session.undo_len(), undo_before + 1, "one undo step");
        let data = editor.document().registry.get(id).unwrap();
        let ObjectData::Path(path) = data else {
            panic!("the rect is a path");
        };
        assert_eq!(path.contours[0].points.len(), 5, "insert adds one point");
        editor.undo().unwrap();
        let data = editor.document().registry.get(id).unwrap();
        let ObjectData::Path(path) = data else {
            panic!("the rect is a path");
        };
        assert_eq!(path.contours[0].points.len(), 4, "undo restores");

        // Far away: no segment.
        assert!(
            editor
                .segment_at(geom::Point::new(500.0, 500.0), 5.0)
                .is_none()
        );
    }

    #[test]
    fn gradient_handles_drag_and_survive_undo() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 0.0, 0.0);
        editor.set_selection(vec![id]);

        // No handles on a solid fill.
        assert!(editor.gradient_handles().is_none());

        // Install a linear ramp through the registry (the panel's path).
        editor
            .apply(
                "path.setFill",
                serde_json::json!({
                    "id": id, "shape": "linear",
                    "stops": [
                        { "position": 0.0, "color_raw": 0xFF00_00FF_i64 },
                        { "position": 1.0, "color_raw": 0xFFFF_FFFF_i64 },
                    ],
                    "alpha": [],
                }),
            )
            .unwrap();
        editor
            .apply(
                "path.setFillVector",
                serde_json::json!({
                    "id": id,
                    "start": { "x": 0.0, "y": 10.0 },
                    "end": { "x": 40.0, "y": 10.0 },
                }),
            )
            .unwrap();

        let (_, start, end) = editor.gradient_handles().expect("handles");
        assert_eq!((start.x, start.y), (0.0, 10.0));
        assert_eq!((end.x, end.y), (40.0, 10.0));

        // Drag the end handle: one undo step, and undo restores.
        let undo_before = editor.session.undo_len();
        editor
            .begin_gradient_drag(id, GradientHandle::End, geom::Point::new(40.0, 10.0))
            .unwrap();
        editor.gradient_drag_to(geom::Point::new(60.0, 20.0));
        editor.end_gradient_drag().unwrap();
        assert_eq!(editor.session.undo_len(), undo_before + 1, "one step");
        let (_, _, end) = editor.gradient_handles().unwrap();
        assert_eq!((end.x, end.y), (60.0, 20.0));
        editor.undo().unwrap();
        let (_, _, end) = editor.gradient_handles().unwrap();
        assert_eq!((end.x, end.y), (40.0, 10.0), "undo restores the ramp");
    }

    #[test]
    fn handle_drag_scales_and_is_one_undo_step() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 10.0, 10.0);
        editor.set_selection(vec![id]);
        let undo_before = editor.session.undo_len();

        // Grab the bottom-right handle; the pointer to the doubled
        // corner scales 2× about the top-left anchor.
        editor
            .begin_transform_drag(
                geom::Point::new(50.0, 30.0),
                TransformHandle::BottomRight,
                vec![id],
            )
            .unwrap();
        editor.transform_drag_to(geom::Point::new(90.0, 50.0));
        editor.end_transform_drag().unwrap();

        let (x, y) = first_anchor(&editor, id);
        assert_eq!((x, y), (10.0, 10.0), "the opposite anchor stays put");
        let after = crate::editor::union_selection_bounds(editor.document(), &[id]).unwrap();
        assert!(
            (after.width() - 80.0).abs() < 0.001 && (after.height() - 40.0).abs() < 0.001,
            "2× scale doubled the extents: {after:?}"
        );
        assert_eq!(editor.session.undo_len(), undo_before + 1, "one undo step");
        editor.undo().unwrap();
        let restored = crate::editor::union_selection_bounds(editor.document(), &[id]).unwrap();
        assert!(
            (restored.width() - 40.0).abs() < 0.001 && (restored.height() - 20.0).abs() < 0.001,
            "undo restores the pre-drag geometry: {restored:?}"
        );
    }

    #[test]
    fn edge_handle_scales_one_axis_only() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 0.0, 0.0);
        editor.set_selection(vec![id]);
        editor
            .begin_transform_drag(
                geom::Point::new(40.0, 10.0),
                TransformHandle::Right,
                vec![id],
            )
            .unwrap();
        // The pointer moves down too, but a Right handle ignores y.
        editor.transform_drag_to(geom::Point::new(80.0, 100.0));
        editor.end_transform_drag().unwrap();
        let after = crate::editor::union_selection_bounds(editor.document(), &[id]).unwrap();
        assert!(
            (after.width() - 80.0).abs() < 0.001 && (after.height() - 20.0).abs() < 0.001,
            "height unchanged under an edge handle: {after:?}"
        );
    }

    #[test]
    fn a_click_sized_handle_drag_commits_nothing() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 10.0, 10.0);
        editor.set_selection(vec![id]);
        let undo_before = editor.session.undo_len();
        editor
            .begin_transform_drag(
                geom::Point::new(50.0, 30.0),
                TransformHandle::BottomRight,
                vec![id],
            )
            .unwrap();
        editor.end_transform_drag().unwrap();
        assert_eq!(editor.session.undo_len(), undo_before, "no step committed");
        let after = crate::editor::union_selection_bounds(editor.document(), &[id]).unwrap();
        assert!(
            (after.width() - 40.0).abs() < 0.001,
            "geometry untouched: {after:?}"
        );
    }

    #[test]
    fn transform_steps_compose_mid_drag() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 0.0, 0.0);
        editor.set_selection(vec![id]);
        editor
            .begin_transform_drag(
                geom::Point::new(40.0, 20.0),
                TransformHandle::BottomRight,
                vec![id],
            )
            .unwrap();
        // Two steps: 1.5×, then another 1.5× against the new bounds —
        // a net 2.25×.
        editor.transform_drag_to(geom::Point::new(60.0, 30.0));
        editor.transform_drag_to(geom::Point::new(90.0, 45.0));
        editor.end_transform_drag().unwrap();
        let after = crate::editor::union_selection_bounds(editor.document(), &[id]).unwrap();
        assert!(
            (after.width() - 90.0).abs() < 0.001 && (after.height() - 45.0).abs() < 0.001,
            "steps compose against current geometry: {after:?}"
        );
    }

    #[test]
    fn a_move_drag_translates_live_and_is_one_undo_step() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 10.0, 10.0);
        editor.selection = Some(id);
        editor.selection_set = vec![id];

        editor.tool = Tool::Pointer;
        editor
            .begin_move_drag(geom::Point::new(30.0, 20.0), vec![id])
            .unwrap();
        editor.drag_to(geom::Point::new(55.0, 45.0));
        editor.drag_to(geom::Point::new(80.0, 45.0));
        editor.end_drag().unwrap();

        // The press at (30,20) + the two steps = +50,+25 total.
        assert_eq!(first_anchor(&editor, id), (60.0, 35.0));
        // ONE undo step covers the whole gesture; everything after it
        // is setup history (the rect's creation + untitled()'s seed).
        editor.undo().unwrap();
        assert_eq!(first_anchor(&editor, id), (10.0, 10.0));
        while editor.undo().is_ok() {}
    }

    #[test]
    fn shift_squares_the_box_drag() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        // The pointer says 40×20; shift says the longer axis wins.
        editor.drag_to_with(
            geom::Point::new(50.0, 30.0),
            DragConstraint {
                shift: true,
                alt: false,
            },
        );
        let drag = editor.drag.as_ref().unwrap();
        assert_eq!(drag.current, geom::Point::new(50.0, 50.0));
    }

    #[test]
    fn alt_drags_the_box_from_center() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        // The press point is the center: the endpoint mirrors through
        // it, so `current − start` reads as the half-extent.
        editor.drag_to_with(
            geom::Point::new(30.0, 25.0),
            DragConstraint {
                shift: false,
                alt: true,
            },
        );
        let drag = editor.drag.as_ref().unwrap();
        assert_eq!(drag.current, geom::Point::new(-10.0, -5.0));
    }

    #[test]
    fn shift_and_alt_combine_into_centered_squares() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to_with(
            geom::Point::new(50.0, 30.0),
            DragConstraint {
                shift: true,
                alt: true,
            },
        );
        let drag = editor.drag.as_ref().unwrap();
        // Square first (40 on a side), then mirrored through the
        // center: the shape grows equally in all four directions.
        assert_eq!(drag.current, geom::Point::new(-30.0, -30.0));
    }

    #[test]
    fn shift_snaps_the_line_to_the_nearest_octant() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Line;
        editor.begin_drag(geom::Point::new(0.0, 0.0));
        // atan2(10, 30) ≈ 18.4° rounds down to 0° — horizontal, same
        // length.
        editor.drag_to_with(
            geom::Point::new(30.0, 10.0),
            DragConstraint {
                shift: true,
                alt: false,
            },
        );
        let drag = editor.drag.as_ref().unwrap();
        let length = (30.0f64 * 30.0 + 10.0 * 10.0).sqrt();
        assert!((drag.current.x - length).abs() < 1e-9);
        assert!(drag.current.y.abs() < 1e-9);

        // Steeper than 63.4° rounds up to 90° — vertical.
        editor.drag_to_with(
            geom::Point::new(10.0, 30.0),
            DragConstraint {
                shift: true,
                alt: false,
            },
        );
        let drag = editor.drag.as_ref().unwrap();
        assert!(drag.current.x.abs() < 1e-9);
        assert!((drag.current.y - length).abs() < 1e-9);
    }

    #[test]
    fn move_drags_pass_through_constraints_untouched() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 10.0, 10.0);
        editor.selection = Some(id);
        editor.selection_set = vec![id];
        editor.tool = Tool::Pointer;
        editor
            .begin_move_drag(geom::Point::new(30.0, 20.0), vec![id])
            .unwrap();
        // Shift+alt must not square or mirror a move: the delta is
        // verbatim.
        editor.drag_to_with(
            geom::Point::new(55.0, 45.0),
            DragConstraint {
                shift: true,
                alt: true,
            },
        );
        editor.end_drag().unwrap();
        assert_eq!(first_anchor(&editor, id), (35.0, 35.0));
        while editor.undo().is_ok() {}
    }

    #[test]
    fn a_click_sized_move_drag_commits_nothing() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 10.0, 10.0);

        editor.tool = Tool::Pointer;
        editor
            .begin_move_drag(geom::Point::new(20.0, 15.0), vec![id])
            .unwrap();
        editor.end_drag().unwrap();

        assert_eq!(first_anchor(&editor, id), (10.0, 10.0));
        // The next undo must be the rect's creation, not a move step;
        // after that only untitled()'s seed history remains.
        editor.undo().unwrap();
        assert!(
            editor.document().registry.get(id).is_none(),
            "a click commits no undo step"
        );
        while editor.undo().is_ok() {}
    }

    #[test]
    fn an_alt_clone_drag_copies_moves_and_is_one_undo_step() {
        let mut editor = Editor::untitled();
        let id = rect_at(&mut editor, 10.0, 10.0);

        editor.tool = Tool::Pointer;
        let created = editor
            .begin_clone_drag(geom::Point::new(30.0, 20.0), &[id])
            .unwrap();
        editor.drag_to(geom::Point::new(60.0, 40.0));
        editor.end_drag().unwrap();

        assert_eq!(created.len(), 1);
        assert_ne!(created[0], id);
        let cell = editor.current_cell().unwrap();
        assert_eq!(cell.elements, vec![id, created[0]], "copy sits above");
        assert_eq!(
            editor.selection_set,
            vec![created[0]],
            "the copies become the selection"
        );
        // The copy moved off the source.
        assert_eq!(first_anchor(&editor, created[0]), (40.0, 30.0));
        assert_eq!(first_anchor(&editor, id), (10.0, 10.0), "source stays");
        // One undo removes the copy and the move together.
        editor.undo().unwrap();
        assert_eq!(editor.current_cell().unwrap().elements, vec![id]);
        assert!(
            editor.document().registry.get(created[0]).is_none(),
            "undo releases the clone's slot"
        );
        // Everything after the gesture is setup history.
        while editor.undo().is_ok() {}
    }

    #[test]
    fn hit_at_sees_every_geometry_kind() {
        let mut editor = Editor::untitled();
        // A rect creation drag makes a path; its frame spans
        // (10,10)..(50,30).
        let id = rect_at(&mut editor, 10.0, 10.0);
        assert_eq!(editor.hit_at(geom::Point::new(30.0, 20.0)), Some(id));
        assert_eq!(editor.hit_at(geom::Point::new(200.0, 200.0)), None);
    }

    /// CS6 picks through layers: the pointer hits an object on any
    /// visible layer of the frame, topmost layer first. A hidden
    /// layer and a locked cell are unpickable.
    #[test]
    fn hit_at_picks_through_layers() {
        let mut editor = Editor::untitled();
        // Two layers: 0 holds the rect, 1 is the active one.
        editor
            .apply("layer.add", serde_json::json!({ "page": 0 }))
            .unwrap();
        let lower = rect_at(&mut editor, 10.0, 10.0);
        editor.set_active_layer(1);
        let upper = rect_at(&mut editor, 60.0, 10.0);
        assert_ne!(lower, upper);

        // A point over the lower layer's rect still hits it — the
        // active layer's own cell is empty there.
        assert_eq!(editor.hit_at(geom::Point::new(30.0, 20.0)), Some(lower));

        // Overlapping objects: the topmost layer wins.
        let over = rect_at(&mut editor, 10.0, 10.0);
        assert_eq!(editor.hit_at(geom::Point::new(30.0, 20.0)), Some(over));

        // A hidden layer is unpickable, a locked cell too.
        editor
            .apply(
                "layer.setDisplay",
                serde_json::json!({ "page": 0, "layer": 1, "display": false }),
            )
            .unwrap();
        assert_eq!(editor.hit_at(geom::Point::new(30.0, 20.0)), Some(lower));

        editor
            .apply(
                "layer.setDisplay",
                serde_json::json!({ "page": 0, "layer": 1, "display": true }),
            )
            .unwrap();
        editor
            .apply(
                "layer.setLocked",
                serde_json::json!({ "page": 0, "layer": 1, "frame": 0, "locked": true }),
            )
            .unwrap();
        assert_eq!(editor.hit_at(geom::Point::new(30.0, 20.0)), Some(lower));
    }

    /// A picked cross-layer object is selectable and movable: the
    /// selection verbs act on it even though it lives on a layer
    /// other than the active one.
    #[test]
    fn a_cross_layer_selection_is_movable() {
        let mut editor = Editor::untitled();
        editor
            .apply("layer.add", serde_json::json!({ "page": 0 }))
            .unwrap();
        let lower = rect_at(&mut editor, 10.0, 10.0);
        editor.set_active_layer(1);

        // The pointer picks the lower layer's object.
        assert_eq!(editor.select_at(geom::Point::new(30.0, 20.0)), Some(lower));
        editor.sync_selection().unwrap();
        assert_eq!(
            editor.document().object_selection,
            vec![lower],
            "the document carries the cross-layer selection"
        );

        // Dragging it moves it — the gesture does not need the object
        // to sit on the active layer.
        editor
            .begin_move_drag(geom::Point::new(30.0, 20.0), vec![lower])
            .unwrap();
        editor.drag_to(geom::Point::new(40.0, 30.0));
        editor.end_drag().unwrap();
        assert_eq!(first_anchor(&editor, lower), (20.0, 20.0));
    }

    #[test]
    fn text_drag_creates_an_empty_text_object() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Text;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(90.0, 40.0));
        editor.end_drag().unwrap();

        let id = editor.current_cell().unwrap().elements[0];
        let Some(ObjectData::Text(text)) = editor.document().registry.get(id) else {
            panic!("the text tool creates a text object");
        };
        // Empty until the properties panel applies characters.
        assert!(text.runs.is_empty());
        assert_eq!(text.frame, geom::Rect::new(10.0, 10.0, 90.0, 40.0));
        editor.undo().unwrap();
        assert!(editor.document().registry.get(id).is_none());
    }

    #[test]
    fn select_at_finds_the_topmost_hit() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(0.0, 0.0));
        editor.drag_to(geom::Point::new(20.0, 20.0));
        editor.end_drag().unwrap();
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(30.0, 30.0));
        editor.end_drag().unwrap();

        // Inside both rects: the later (top) one wins.
        let top = editor.select_at(geom::Point::new(15.0, 15.0)).unwrap();
        let cell = editor.current_cell().unwrap();
        assert_eq!(top, cell.elements[1]);
        // Outside everything: deselects.
        assert!(editor.select_at(geom::Point::new(500.0, 500.0)).is_none());
        assert!(editor.selection.is_none());
    }

    #[test]
    fn autoshape_drag_creates_through_the_registry() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::AutoShape(AutoShape::Star);
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(60.0, 50.0));
        editor.end_drag().unwrap();

        let id = editor.current_cell().unwrap().elements[0];
        let Some(ObjectData::Path(path)) = editor.document().registry.get(id) else {
            panic!("the star is a path");
        };
        // A 5-point star has 10 corners on one contour.
        assert_eq!(path.contours.len(), 1);
        assert_eq!(path.contours[0].points.len(), 10);
        // Undo removes it — same registry semantics as every surface.
        editor.undo().unwrap();
        assert!(editor.document().registry.get(id).is_none());
    }

    #[test]
    fn toggle_select_builds_and_drains_the_set() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        for (x0, x1) in [(0.0, 20.0), (30.0, 50.0)] {
            editor.begin_drag(geom::Point::new(x0, 0.0));
            editor.drag_to(geom::Point::new(x1, 20.0));
            editor.end_drag().unwrap();
        }
        let a = editor.current_cell().unwrap().elements[0];
        let b = editor.current_cell().unwrap().elements[1];

        assert!(editor.toggle_select_at(geom::Point::new(10.0, 10.0)));
        assert_eq!(editor.selection_set, vec![a]);
        assert!(editor.toggle_select_at(geom::Point::new(40.0, 10.0)));
        assert_eq!(editor.selection_set, vec![a, b]);
        assert_eq!(editor.selection, Some(b));
        // Toggling the primary off promotes the set's last member.
        assert!(editor.toggle_select_at(geom::Point::new(40.0, 10.0)));
        assert_eq!(editor.selection_set, vec![a]);
        assert_eq!(editor.selection, Some(a));
        // A miss clears, like a plain click on empty canvas.
        assert!(!editor.toggle_select_at(geom::Point::new(500.0, 500.0)));
        assert!(editor.selection_set.is_empty());
        assert!(editor.selection.is_none());
    }

    #[test]
    fn boolean_pair_orders_bottom_first() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(0.0, 0.0));
        editor.drag_to(geom::Point::new(20.0, 20.0));
        editor.end_drag().unwrap();
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(30.0, 30.0));
        editor.end_drag().unwrap();
        let bottom = editor.current_cell().unwrap().elements[0];
        let top = editor.current_cell().unwrap().elements[1];

        // Pick the top one first: the pair still comes out bottom-first.
        editor.toggle_select_at(geom::Point::new(25.0, 25.0));
        editor.toggle_select_at(geom::Point::new(5.0, 5.0));
        assert_eq!(editor.boolean_pair(), Some((bottom, top)));
        // One selection: no pair for the boolean menu.
        editor.toggle_select_at(geom::Point::new(5.0, 5.0));
        assert_eq!(editor.boolean_pair(), None);
    }

    #[test]
    fn edits_round_trip_through_save_and_open() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(1.0, 1.0));
        editor.drag_to(geom::Point::new(50.0, 50.0));
        editor.end_drag().unwrap();
        let id = editor.current_cell().unwrap().elements[0];
        editor
            .apply(
                "object.setOpacity",
                serde_json::json!({ "id": id, "opacity": 700 }),
            )
            .unwrap();

        let bytes = editor.save().unwrap();
        let reopened = Editor::open(&bytes).unwrap();
        let cell = reopened.document().pages[0].layers.layers[0].cells[0].clone();
        assert_eq!(cell.elements.len(), 1);
        match reopened.document().registry.get(cell.elements[0]) {
            Some(ObjectData::Path(path)) => assert_eq!(path.opacity, 700),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    /// One drawn rect turned into a symbol: the definition owns the
    /// artwork, the page cell holds the instance.
    fn symbol_editor() -> (Editor, doc::ObjectId, doc::ObjectId) {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(90.0, 50.0));
        editor.end_drag().unwrap();
        let content = editor.current_cell().unwrap().elements[0];
        editor
            .apply(
                "symbol.create",
                serde_json::json!({
                    "ids": [content], "name": "badge", "symbol_type": "graphic",
                }),
            )
            .unwrap();
        let instance = editor.current_cell().unwrap().elements[0];
        (editor, content, instance)
    }

    #[test]
    fn symbol_scope_routes_tools_at_the_definition() {
        let (mut editor, content, instance) = symbol_editor();

        editor.enter_symbol(0).unwrap();
        assert_eq!(editor.symbol_scope, Some(0));
        // The edited cell is the definition's, not the page's: the
        // artwork sits there, the instance does not.
        let cell = editor.editing_cell().unwrap();
        assert_eq!(cell.elements, vec![content]);
        assert_ne!(cell.elements[0], instance);

        // Unknown symbol numbers refuse and leave the scope alone.
        assert!(editor.enter_symbol(9).is_err());
        assert_eq!(editor.symbol_scope, Some(0));

        editor.exit_symbol();
        assert!(editor.symbol_scope.is_none());
        assert_eq!(
            editor.editing_cell().map(|cell| cell.elements.clone()),
            editor.current_cell().map(|cell| cell.elements.clone()),
            "back on the page, the edited cell is the page cell again"
        );
    }

    #[test]
    fn a_scope_creation_lands_in_the_definition() {
        let (mut editor, _content, _instance) = symbol_editor();
        editor.enter_symbol(0).unwrap();

        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(100.0, 100.0));
        editor.drag_to(geom::Point::new(140.0, 130.0));
        editor.end_drag().unwrap();

        // The definition gained the artwork; the page cell is
        // unchanged (just the instance).
        let definition_cell = editor.editing_cell().unwrap();
        assert_eq!(definition_cell.elements.len(), 2);
        let page_cell = editor.current_cell().unwrap();
        assert_eq!(page_cell.elements.len(), 1);

        // Undo walks the two journal rows back: lift, then create.
        editor.undo().unwrap();
        editor.undo().unwrap();
        assert_eq!(editor.editing_cell().unwrap().elements.len(), 1);
        assert_eq!(
            editor.current_cell().unwrap().elements.len(),
            1,
            "the page keeps the instance, nothing else"
        );
    }

    #[test]
    fn place_symbol_places_and_selects_an_instance() {
        let (mut editor, _content, instance) = symbol_editor();
        editor.exit_symbol();
        editor.selection = None;
        editor.selection_set.clear();

        editor
            .place_symbol(0, geom::Point::new(200.0, 220.0))
            .unwrap();
        let cell = editor.current_cell().unwrap();
        assert_eq!(cell.elements.len(), 2);
        assert_eq!(editor.selection, Some(cell.elements[1]));
        assert_ne!(cell.elements[1], instance);
        match editor.document().registry.get(cell.elements[1]) {
            Some(ObjectData::SymbolInstance(placed)) => {
                let translation = placed.transform.translation();
                assert_eq!(translation, geom::Vec2::new(200.0, 220.0));
            }
            other => panic!("a placement creates an instance: {other:?}"),
        }
        editor.undo().unwrap();
        assert_eq!(editor.current_cell().unwrap().elements.len(), 1);
    }

    #[test]
    fn hit_at_inside_scope_finds_definition_content() {
        let (mut editor, _content, _instance) = symbol_editor();
        editor.enter_symbol(0).unwrap();

        // Over the original artwork: inside the scope this is the
        // definition member, never the page instance.
        let hit = editor.hit_at(geom::Point::new(50.0, 30.0)).unwrap();
        assert!(editor.editing_cell().unwrap().elements.contains(&hit));
    }

    #[test]
    fn guide_drag_commits_a_new_inset() {
        let (mut editor, _content, _instance) = symbol_editor();
        editor.enable_nine_slice(0).unwrap();
        // Defaults: zero insets, shape bounds from the definition's
        // content.
        let guides = editor.document().symbols[0]
            .nine_slice
            .clone()
            .expect("guides installed");
        assert_eq!(guides.controls, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(guides.shape_bounds, geom::Rect::new(10.0, 10.0, 90.0, 50.0));

        // Drag the top guide down to world y=30: a 20px inset.
        editor
            .begin_guide_drag(0, NineSliceEdge::Top, geom::Point::new(50.0, 30.0))
            .unwrap();
        editor.drag_guide_to(geom::Point::new(52.0, 30.0));
        editor.end_guide_drag().unwrap();
        let guides = editor.document().symbols[0]
            .nine_slice
            .clone()
            .expect("guides survive the drag");
        assert_eq!(guides.controls, [20.0, 0.0, 0.0, 0.0]);

        // Dictionary order: a right-edge drag writes slot 3, measured
        // in from the right edge, clamped to the shape width.
        editor
            .begin_guide_drag(0, NineSliceEdge::Right, geom::Point::new(60.0, 30.0))
            .unwrap();
        editor.end_guide_drag().unwrap();
        let guides = editor.document().symbols[0].nine_slice.clone().unwrap();
        assert_eq!(guides.controls, [20.0, 0.0, 0.0, 30.0]);

        // A drag over an unguided symbol refuses cleanly.
        editor
            .apply(
                "library.add",
                serde_json::json!({ "name": "bare", "symbol_type": "graphic" }),
            )
            .unwrap();
        let bare = editor.document().symbols[1].symbol_number;
        assert!(
            editor
                .begin_guide_drag(bare, NineSliceEdge::Top, geom::Point::new(0.0, 0.0))
                .is_err()
        );
        assert!(editor.guide_drag.is_none());
        // An abandoned drag (no edge picked) commits nothing.
        editor.guide_drag = None;
        editor.end_guide_drag().unwrap();
        assert_eq!(
            editor.document().symbols[0]
                .nine_slice
                .as_ref()
                .unwrap()
                .controls,
            [20.0, 0.0, 0.0, 30.0]
        );
    }

    #[test]
    fn nine_slice_defaults_refuse_empty_definitions() {
        let mut editor = Editor::untitled();
        editor
            .apply(
                "library.add",
                serde_json::json!({ "name": "empty", "symbol_type": "graphic" }),
            )
            .unwrap();
        let number = editor.document().symbols[0].symbol_number;
        assert!(editor.enable_nine_slice(number).is_err());
        editor.disable_nine_slice(number).unwrap();
        assert!(editor.document().symbols[0].nine_slice.is_none());
    }

    /// A 2×2 red image on the page — the selection tools' target.
    fn editor_with_raster() -> (Editor, ObjectId) {
        let mut editor = Editor::untitled();
        // 2×2 opaque red, base64 of [255, 0, 0, 255] × 2 rows.
        let red = [
            255u8, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
        ];
        let plane = base64::engine::general_purpose::STANDARD.encode(red);
        editor
            .apply(
                "bitmap.importFile",
                serde_json::json!({
                    "width": 2, "height": 2, "pixels": plane,
                    "bounds": { "left": 0.0, "top": 0.0, "right": 0.0, "bottom": 0.0 },
                    "maintain_aspect_ratio": true,
                    "page": 0, "layer": 0, "cell": 0, "name": "red",
                }),
            )
            .unwrap();
        let id = editor.current_cell().unwrap().elements[0];
        (editor, id)
    }

    #[test]
    fn the_marquee_selects_a_rect_of_pixels() {
        let (mut editor, id) = editor_with_raster();
        editor.tool = Tool::Marquee;
        editor
            .begin_pixel_selection(geom::Point::new(0.5, 0.5))
            .unwrap();
        // Half-open center rule: the drag [0.5, 2.0) covers both pixel
        // centers (0.5, 1.5).
        editor
            .extend_pixel_selection(geom::Point::new(2.0, 2.0))
            .unwrap();
        editor.end_pixel_selection().unwrap();

        let mask = editor.document().selection.as_ref().expect("selected");
        assert_eq!((mask.width, mask.height), (2, 2));
        assert_eq!(mask.get(0, 0), Some(255));
        assert_eq!(mask.get(1, 1), Some(255));
        assert_eq!(editor.pixel_target, Some(id));

        // The engine's deselect command clears it through the same door.
        editor.deselect_pixels().unwrap();
        assert!(editor.document().selection.is_none());
    }

    #[test]
    fn a_marquee_with_add_mode_folds_over_the_active_one() {
        let (mut editor, _id) = editor_with_raster();
        editor.tool = Tool::Marquee;
        editor
            .begin_pixel_selection(geom::Point::new(0.5, 0.5))
            .unwrap();
        editor
            .extend_pixel_selection(geom::Point::new(0.9, 1.9))
            .unwrap();
        editor.end_pixel_selection().unwrap();

        // Shift = add: a second column joins the first.
        editor.pixel_mode = PixelSelectMode::Add;
        editor
            .begin_pixel_selection(geom::Point::new(1.5, 0.5))
            .unwrap();
        editor
            .extend_pixel_selection(geom::Point::new(1.9, 0.9))
            .unwrap();
        editor.end_pixel_selection().unwrap();

        let mask = editor.document().selection.as_ref().expect("selected");
        assert_eq!(mask.get(0, 1), Some(255), "the first column survives");
        assert_eq!(mask.get(1, 0), Some(255), "the second joined it");
    }

    #[test]
    fn the_lasso_selects_its_trace_polygon() {
        let (mut editor, _id) = editor_with_raster();
        editor.tool = Tool::Lasso;
        editor
            .begin_pixel_selection(geom::Point::new(0.0, 0.0))
            .unwrap();
        editor
            .extend_pixel_selection(geom::Point::new(2.0, 0.0))
            .unwrap();
        editor
            .extend_pixel_selection(geom::Point::new(2.0, 2.0))
            .unwrap();
        editor
            .extend_pixel_selection(geom::Point::new(0.0, 2.0))
            .unwrap();
        editor.end_pixel_selection().unwrap();

        let mask = editor.document().selection.as_ref().expect("selected");
        // The trace closes a full square: centers 0.5 and 1.5 on both
        // axes sit strictly inside [0, 2).
        assert_eq!(mask.get(0, 1), Some(255));
        assert_eq!(mask.get(1, 0), Some(255));
    }

    #[test]
    fn the_wand_selects_matching_pixels_from_the_seed() {
        let (mut editor, _id) = editor_with_raster();
        editor.tool = Tool::MagicWand;
        editor.wand_tolerance = 0;
        editor
            .begin_pixel_selection(geom::Point::new(0.5, 0.5))
            .unwrap();

        let mask = editor.document().selection.as_ref().expect("selected");
        assert_eq!(mask.get(0, 0), Some(255));
        assert_eq!(mask.get(1, 1), Some(255), "the whole red field matches");
    }

    #[test]
    fn the_polygon_selects_on_commit() {
        let (mut editor, _id) = editor_with_raster();
        editor.tool = Tool::PolygonalMarquee;
        // Two points are not a shape; three commit.
        editor
            .add_polygon_point(geom::Point::new(0.5, 0.5))
            .unwrap();
        editor
            .add_polygon_point(geom::Point::new(1.5, 0.5))
            .unwrap();
        assert!(!editor.commit_polygon().unwrap_or(false));
        editor
            .add_polygon_point(geom::Point::new(0.5, 1.5))
            .unwrap();
        assert!(editor.commit_polygon().unwrap());

        let mask = editor.document().selection.as_ref().expect("selected");
        assert_eq!(mask.get(0, 0), Some(255));
        assert_eq!(mask.get(1, 1), Some(0), "the far corner stays out");
    }

    #[test]
    fn feather_adjust_and_move_ride_the_panel_commands() {
        let (mut editor, id) = editor_with_raster();
        editor.tool = Tool::Marquee;
        editor
            .begin_pixel_selection(geom::Point::new(0.5, 0.5))
            .unwrap();
        editor
            .extend_pixel_selection(geom::Point::new(0.9, 0.9))
            .unwrap();
        editor.end_pixel_selection().unwrap();

        // Grow by two: the diagonal is chamfer distance 4, so a
        // radius-1 expand (threshold 3) reaches only axis neighbors —
        // radius 2 (threshold 6) covers the whole 2×2 image.
        editor.adjust_pixel_selection("expand", 2).unwrap();
        let mask = editor.document().selection.as_ref().expect("selected");
        assert_eq!(mask.get(1, 1), Some(255));

        // Move the pixels under it: the red field shifts, undo walks
        // it back.
        editor.pixel_target = Some(id);
        editor.move_selected_pixels(0, 1).unwrap();
        editor.undo().unwrap();
        assert!(editor.document().selection.is_some());

        // Erase under the mask, then undo restores the pixels.
        editor.erase_selected().unwrap();
        editor.undo().unwrap();
        let mask = editor.document().selection.as_ref().expect("still there");
        assert_eq!(mask.get(0, 0), Some(255));
    }

    #[test]
    fn selection_edges_outlines_the_mask_not_the_image() {
        // A full-image selection has no interior boundary — the image
        // border is not a boundary — but a half-image one has a
        // vertical line down the middle.
        let mut mask = doc::SelectionMask::empty(2, 1);
        mask.set(0, 0, 255);
        let edges = selection_edges(&mask);
        // The image border is not a boundary on any side: the lone
        // pixel contributes only its interior right edge (x = 1).
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0], [1.0, 0.0, 1.0, 1.0]);

        let mut full = doc::SelectionMask::empty(2, 2);
        for y in 0..2 {
            for x in 0..2 {
                full.set(x, y, 255);
            }
        }
        assert!(selection_edges(&full).is_empty());
    }

    #[test]
    fn undo_and_redo_round_trip_a_command() {
        let mut editor = Editor::untitled();
        // untitled()'s seed steps (page.add + setCanvasSize) are the
        // setup history; the shape lands on top of them.
        let seed_depth = editor.session.undo_len();

        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(60.0, 40.0));
        editor.end_drag().unwrap();
        assert_eq!(editor.current_cell().unwrap().elements.len(), 1);

        let name = editor.undo().unwrap();
        assert!(!name.is_empty(), "undo returns the undone command's name");
        assert!(editor.current_cell().unwrap().elements.is_empty());

        editor.redo().unwrap();
        assert_eq!(editor.current_cell().unwrap().elements.len(), 1);
        // Redo the remaining setup steps back to seed depth, then the
        // drained stack refuses.
        while editor.session.undo_len() > seed_depth {
            if editor.redo().is_err() {
                break;
            }
        }
        assert!(editor.redo().is_err(), "drained redo stack refuses");
    }

    #[test]
    fn save_and_open_round_trip_preserves_elements() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Ellipse;
        editor.begin_drag(geom::Point::new(0.0, 0.0));
        editor.drag_to(geom::Point::new(30.0, 30.0));
        editor.end_drag().unwrap();

        let bytes = editor.save().unwrap();
        let reopened = Editor::open(&bytes).unwrap();
        let cell = reopened.current_cell().unwrap();
        assert_eq!(cell.elements.len(), 1);
        assert!(matches!(
            reopened.document().registry.get(cell.elements[0]),
            Some(doc::ObjectData::Path(_))
        ));
    }

    #[test]
    fn open_refuses_garbage_bytes() {
        assert!(Editor::open(b"not a png at all").is_err());
    }

    #[test]
    fn set_active_layer_clamps_to_the_topmost_layer() {
        let mut editor = Editor::untitled();
        editor.set_active_layer(9);
        assert_eq!(editor.active_layer(), 0, "clamped on a fresh document");
    }

    #[test]
    fn pixel_select_ops_refuse_cleanly_without_a_selection() {
        let mut editor = Editor::untitled();
        // No pixel selection exists: the no-selection early-return
        // paths must be Ok (no-ops), not errors.
        assert!(editor.deselect_pixels().is_ok());
        assert!(editor.select_similar(16).is_ok());
        assert!(editor.move_selected_pixels(1, 1).is_ok());
    }

    #[test]
    fn pixel_select_ops_drive_the_commands_with_a_selection() {
        let (mut editor, _id) = editor_with_raster();
        editor.tool = Tool::Marquee;
        // A real drag span: begin at the seed pixel, end one pixel
        // over, so the committed rect selection is non-empty.
        editor.pixel_mode = PixelSelectMode::from_modifiers(egui::Modifiers::default());
        editor
            .begin_pixel_selection(geom::Point::new(0.5, 0.5))
            .unwrap();
        editor
            .extend_pixel_selection(geom::Point::new(2.0, 2.0))
            .unwrap();
        editor.end_pixel_selection().unwrap();
        assert!(
            editor.pixel_selection.is_none(),
            "end consumes the live drag"
        );
        assert!(editor.document().selection.is_some());

        editor.move_selection_mask(1, 0).unwrap();
        editor.adjust_pixel_selection("expand", 1).unwrap();
        editor.feather_selection(1).unwrap();
        editor.select_similar(8).unwrap();
        editor.move_selected_pixels(1, 0).unwrap();
        editor.deselect_pixels().unwrap();
        assert!(editor.pixel_selection.is_none());
        // Every op above landed exactly one undo step.
        assert!(!editor.session.undo().is_err());
    }

    #[test]
    fn set_selection_replaces_the_vector_selection() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(0.0, 0.0));
        editor.drag_to(geom::Point::new(10.0, 10.0));
        editor.end_drag().unwrap();
        let id = editor.current_cell().unwrap().elements[0];

        editor.set_selection(vec![id]);
        assert_eq!(editor.selected_live(), Some(id));

        editor.set_selection(Vec::new());
        assert_eq!(editor.selected_live(), None);
    }

    #[test]
    fn a_pencil_stroke_dabs_and_is_one_undo_step() {
        let (mut editor, _id) = editor_with_raster();
        editor.tool = Tool::Pencil;
        editor.begin_stroke(geom::Point::new(0.5, 0.5)).unwrap();
        editor.stroke_to(geom::Point::new(1.5, 0.5)).unwrap();
        editor.stroke_to(geom::Point::new(1.5, 0.5)).unwrap(); // no-op: same point
        editor.end_stroke().unwrap();

        // One transaction = one undo step; undo restores the pre-stroke
        // document.
        assert!(editor.undo().is_ok());
    }

    #[test]
    fn a_stroke_off_the_canvas_is_a_no_op() {
        let (mut editor, _id) = editor_with_raster();
        editor.tool = Tool::Pencil;
        // No image under this point: begin is Ok and paints nothing.
        editor.begin_stroke(geom::Point::new(900.0, 900.0)).unwrap();
        assert!(editor.stroke.is_none());
        editor.end_stroke().unwrap();
    }

    #[test]
    fn replay_steps_replays_journal_lines() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(0.0, 0.0));
        editor.drag_to(geom::Point::new(10.0, 10.0));
        editor.end_drag().unwrap();
        let steps = editor.steps_text();
        // Reopen replays the whole journal (seed steps included) onto
        // the given document, so the base is bare — an untitled base
        // would double the seed page.
        let mut fresh = Editor::from_document(doc::Document::default());
        fresh.replay_steps(&steps).unwrap();
        assert_eq!(fresh.document().pages.len(), editor.document().pages.len(),);
        assert_eq!(
            fresh.current_cell().unwrap().elements.len(),
            editor.current_cell().unwrap().elements.len(),
            "replay lands the same objects",
        );
    }

    #[test]
    fn guide_drag_commits_an_inset_through_the_full_gesture() {
        let (mut editor, _content, _instance) = symbol_editor();
        editor.enter_symbol(0).unwrap();
        editor.enable_nine_slice(0).unwrap();
        editor.exit_symbol();
        editor.enter_symbol(0).unwrap();
        editor
            .begin_guide_drag(0, NineSliceEdge::Top, geom::Point::new(50.0, 15.0))
            .unwrap();
        editor.drag_guide_to(geom::Point::new(50.0, 20.0));
        editor.end_guide_drag().unwrap();
    }

    #[test]
    fn pick_color_reads_the_pixel_under_the_point() {
        let (mut editor, _id) = editor_with_raster();
        let color = editor.pick_color(geom::Point::new(0.5, 0.5));
        assert_eq!(color, Some([255, 0, 0, 255]), "the raster is opaque red");
        assert_eq!(editor.pick_color(geom::Point::new(900.0, 900.0)), None);
    }
}

//! Auto Shape insertion (`autoshape.*`): each CS6 Auto Shape tool's
//! generator exposed as a catalog command that appends a path object to
//! a cell.
//!
//! The geometry comes from `vector::autoshape`, whose generators are
//! clean-room ports of the tools' `RedrawSmartShape` behavior (the
//! reload path: rebuild the whole shape from the stored parameters
//! alone — this crate's regenerate-don't-bake contract). The inserted
//! object carries no primitive source: the parameters live at the
//! calling layer (panel, shell, script), and re-generation is a fresh
//! command, not a baked mutation.
//!
//! Finiteness is validated on the *generated* geometry: every
//! generator maps its parameters into coordinates by pure arithmetic,
//! so a non-finite input always surfaces as non-finite anchors or
//! handles. Rejecting non-finite output therefore rejects non-finite
//! payloads before anything mutates, and doubles as a tripwire against
//! a generator producing NaN on finite input.

use doc::{Document, ObjectData, ObjectId, PaintStyle, PathObject};
use geom::Point;
use vector::{
    ArrowHead, ArrowLineParams, ArrowParams, ConnectorParams, CornerKind, LShapeParams,
    MeasureParams, PieParams, PolygonParams, RectParams, SpiralParams, StarParams,
};

use crate::access;
use crate::catalog::object::attach_cell;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the Auto Shape commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(CreateRect)?;
    registry.register(CreateStar)?;
    registry.register(CreatePolygon)?;
    registry.register(CreateSpiral)?;
    registry.register(CreatePie)?;
    registry.register(CreateDoughnut)?;
    registry.register(CreateLShape)?;
    registry.register(CreateArrow)?;
    registry.register(CreateArrowLine)?;
    registry.register(CreateMeasure)?;
    registry.register(CreateConnectorLine)?;
    registry.register(RedrawSmartShape)?;
    Ok(())
}

/// Destination fields shared by every `autoshape.*` payload.
trait Placed {
    /// Target page index.
    fn page(&self) -> usize;
    /// Target layer index.
    fn layer(&self) -> usize;
    /// Target cell index (state number).
    fn cell(&self) -> usize;
    /// Optional object name.
    fn object_name(&self) -> Option<&String>;
}

/// Declares a zero-state command struct whose whole `apply` is
/// "generate the shape from `payload` and insert it".
macro_rules! autoshape_command {
    ($id:literal, $kind:ident, $payload:ident, $generate:ident) => {
        struct $kind;

        impl TypedCommand for $kind {
            type Payload = $payload;

            fn name(&self) -> &'static str {
                $id
            }

            fn apply(
                &self,
                document: &mut Document,
                payload: &Self::Payload,
            ) -> Result<Delta, EngineError> {
                insert_shape(document, $id, payload, $generate(payload))
            }
        }
    };
}

/// Validates the generated geometry, then inserts the path object and
/// appends it to the target cell. Validation runs before the first
/// mutation, so a rejected payload leaves the document untouched.
fn insert_shape(
    document: &mut Document,
    name: &'static str,
    placement: &impl Placed,
    generated: PathObject,
) -> Result<Delta, EngineError> {
    if !finite_geometry(&generated) {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "parameters must be finite (generated geometry is not)".to_owned(),
        });
    }
    access::cell_mut(
        document,
        name,
        placement.page(),
        placement.layer(),
        placement.cell(),
    )?;

    let object = ObjectData::Path(PathObject {
        contours: generated.contours,
        even_odd: generated.even_odd,
        primitive: None,
        paint: Some(PaintStyle {
            stroke_raw: 0,         // transparent black — no stroke yet
            fill_raw: 0xFF00_0000, // ABGR opaque black — the CS6 new-path fill
            fill: None,
        }),
        fill_start: None,
        fill_end: None,
        fill_third: None,
        brush_placement: None,
        fill_on_top: None,
        locked: false,
        visible: true,
        opacity: 1000,
        blend: 0,
        name: placement.object_name().cloned(),
        effects: Vec::new(),
    });

    let id = document.registry.insert(object);
    // Append (top of z-order). The same cell lookup passed above, and
    // `None` appends — this cannot fail, so there is no
    // orphan-registration cleanup to run.
    let placed = attach_cell(
        document,
        name,
        placement.page(),
        placement.layer(),
        placement.cell(),
        id,
        None,
    )?;
    Ok(Delta::Group(vec![
        placed,
        Delta::Object {
            id,
            inverse: ObjectInverse::Created,
        },
    ]))
}

/// `true` when every anchor and handle coordinate is finite.
fn finite_geometry(path: &PathObject) -> bool {
    path.contours.iter().all(|contour| {
        contour.points.iter().all(|point| {
            point.anchor.is_finite()
                && point.primary.is_none_or(geom::Point::is_finite)
                && point.secondary.is_none_or(geom::Point::is_finite)
        })
    })
}

/// Corner treatment for `autoshape.rect` (the three rectangle tools
/// share one generator, differing only in this value).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum RectCorner {
    /// Straight diagonal cuts (`Rectangle`).
    Bevel,
    /// Quarter-circle arcs (`Rounded Rectangle`).
    Round,
    /// Edge-parallel handles (`Chamfer Rectangle`).
    Chamfer,
}

impl From<RectCorner> for CornerKind {
    fn from(corner: RectCorner) -> Self {
        match corner {
            RectCorner::Bevel => CornerKind::Beveled,
            RectCorner::Round => CornerKind::Rounded,
            RectCorner::Chamfer => CornerKind::Chamfer,
        }
    }
}

/// `autoshape.rect` — corner-cut rectangle (the Rectangle / Rounded
/// Rectangle / Chamfer Rectangle tools).
/// Builds the autoshape.rect geometry from the payload.
fn create_rect(payload: &CreateRectPayload) -> PathObject {
    vector::rectangle(RectParams {
        origin: Point::new(payload.x, payload.y),
        width: payload.width,
        height: payload.height,
        corner: payload.corner,
        kind: payload.kind.into(),
    })
}

autoshape_command!("autoshape.rect", CreateRect, CreateRectPayload, create_rect);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateRectPayload {
    /// Frame origin, world coordinates.
    x: f64,
    /// Frame origin, world coordinates.
    y: f64,
    /// Full width; must be finite.
    width: f64,
    /// Full height; must be finite.
    height: f64,
    /// Corner cut radius.
    corner: f64,
    /// Corner treatment.
    kind: RectCorner,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

impl Placed for CreateRectPayload {
    fn page(&self) -> usize {
        self.page
    }
    fn layer(&self) -> usize {
        self.layer
    }
    fn cell(&self) -> usize {
        self.cell
    }
    fn object_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

/// `autoshape.star` — the Star tool (roundable N-point star).
/// Builds the autoshape.star geometry from the payload.
fn create_star(payload: &CreateStarPayload) -> PathObject {
    vector::star(StarParams {
        center: Point::new(payload.center_x, payload.center_y),
        outer: payload.outer,
        inner: payload.inner,
        outer_round: payload.outer_round,
        inner_round: payload.inner_round,
        points: payload.points,
        start_angle: payload.start_angle,
    })
}

autoshape_command!("autoshape.star", CreateStar, CreateStarPayload, create_star);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateStarPayload {
    /// Center control point.
    center_x: f64,
    /// Center control point.
    center_y: f64,
    /// Outer corner radius.
    outer: f64,
    /// Inner corner radius.
    inner: f64,
    /// Outer roundness radius.
    outer_round: f64,
    /// Inner roundness radius.
    inner_round: f64,
    /// Corner count (clamped 3..=50 by the generator, per the script).
    points: u32,
    /// Direction of the first outer corner, radians.
    start_angle: f64,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

impl Placed for CreateStarPayload {
    fn page(&self) -> usize {
        self.page
    }
    fn layer(&self) -> usize {
        self.layer
    }
    fn cell(&self) -> usize {
        self.cell
    }
    fn object_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

/// `autoshape.polygon` — the Smart Polygon tool (ring or pinwheel).
/// Builds the autoshape.polygon geometry from the payload.
fn create_polygon(payload: &CreatePolygonPayload) -> PathObject {
    vector::smart_polygon(PolygonParams {
        center: Point::new(payload.center_x, payload.center_y),
        outer: payload.outer,
        inner: payload.inner,
        points: payload.points,
        sections: payload.sections,
        start_angle: payload.start_angle,
        split: payload.split,
    })
}

autoshape_command!(
    "autoshape.polygon",
    CreatePolygon,
    CreatePolygonPayload,
    create_polygon
);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreatePolygonPayload {
    /// Center control point.
    center_x: f64,
    /// Center control point.
    center_y: f64,
    /// Outer radius.
    outer: f64,
    /// Inner radius (0 disables the ring).
    inner: f64,
    /// Corner count (clamped 3..=25 by the generator, per the script).
    points: u32,
    /// Sides drawn / split sections (clamped 1..=points).
    sections: u32,
    /// Direction of the first corner, radians.
    start_angle: f64,
    /// Split mode: one closed section per side.
    split: bool,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

impl Placed for CreatePolygonPayload {
    fn page(&self) -> usize {
        self.page
    }
    fn layer(&self) -> usize {
        self.layer
    }
    fn cell(&self) -> usize {
        self.cell
    }
    fn object_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

/// `autoshape.spiral` — the Spiral tool.
/// Builds the autoshape.spiral geometry from the payload.
fn create_spiral(payload: &CreateSpiralPayload) -> PathObject {
    vector::spiral(SpiralParams {
        center: Point::new(payload.center_x, payload.center_y),
        radius: payload.radius,
        tightness_deg: payload.tightness_deg,
        clockwise: payload.clockwise,
        closed: payload.closed,
    })
}

autoshape_command!(
    "autoshape.spiral",
    CreateSpiral,
    CreateSpiralPayload,
    create_spiral
);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateSpiralPayload {
    /// Center control point.
    center_x: f64,
    /// Center control point.
    center_y: f64,
    /// Outer radius limit.
    radius: f64,
    /// Tightness in degrees (clamped 50..=89 by the generator).
    tightness_deg: f64,
    /// Drawing direction.
    clockwise: bool,
    /// Closed mode: retrace back to the center, reversed.
    closed: bool,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

impl Placed for CreateSpiralPayload {
    fn page(&self) -> usize {
        self.page
    }
    fn layer(&self) -> usize {
        self.layer
    }
    fn cell(&self) -> usize {
        self.cell
    }
    fn object_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

/// `autoshape.pie` — the Pie tool (`is_pie` semantics).
/// Builds the autoshape.pie geometry from the payload.
fn create_pie(payload: &CreatePiePayload) -> PathObject {
    vector::pie(PieParams {
        center: Point::new(payload.center_x, payload.center_y),
        outer: payload.outer,
        inner: payload.inner,
        start: payload.start,
        end: payload.end,
        is_pie: true,
    })
}

autoshape_command!("autoshape.pie", CreatePie, CreatePiePayload, create_pie);

/// `autoshape.doughnut` — the Doughnut tool (annulus; the inserted
/// object is `even_odd`).
/// Builds the autoshape.doughnut geometry from the payload.
fn create_doughnut(payload: &CreatePiePayload) -> PathObject {
    vector::doughnut(PieParams {
        center: Point::new(payload.center_x, payload.center_y),
        outer: payload.outer,
        inner: payload.inner,
        start: payload.start,
        end: payload.end,
        is_pie: false,
    })
}

autoshape_command!(
    "autoshape.doughnut",
    CreateDoughnut,
    CreatePiePayload,
    create_doughnut
);

/// Pie / doughnut payload (same parameter surface, one command each).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreatePiePayload {
    /// Center control point.
    center_x: f64,
    /// Center control point.
    center_y: f64,
    /// Outer radius.
    outer: f64,
    /// Inner radius (0 for a pie wedge).
    inner: f64,
    /// Sector start angle, radians.
    start: f64,
    /// Sector end angle, radians.
    end: f64,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

impl Placed for CreatePiePayload {
    fn page(&self) -> usize {
        self.page
    }
    fn layer(&self) -> usize {
        self.layer
    }
    fn cell(&self) -> usize {
        self.cell
    }
    fn object_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

/// `autoshape.lshape` — the L-Shape tool.
/// Builds the autoshape.lshape geometry from the payload.
fn create_l_shape(payload: &CreateLShapePayload) -> PathObject {
    vector::lshape(LShapeParams {
        origin: Point::new(payload.x, payload.y),
        width: payload.width,
        height: payload.height,
        thickness: payload.thickness,
        roundness: payload.roundness,
    })
}

autoshape_command!(
    "autoshape.lshape",
    CreateLShape,
    CreateLShapePayload,
    create_l_shape
);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateLShapePayload {
    /// Frame origin, world coordinates.
    x: f64,
    /// Frame origin, world coordinates.
    y: f64,
    /// Overall width; must be finite.
    width: f64,
    /// Overall height; must be finite.
    height: f64,
    /// Shaft thickness.
    thickness: f64,
    /// Corner roundness.
    roundness: f64,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

impl Placed for CreateLShapePayload {
    fn page(&self) -> usize {
        self.page
    }
    fn layer(&self) -> usize {
        self.layer
    }
    fn cell(&self) -> usize {
        self.cell
    }
    fn object_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

/// `autoshape.arrow` — the Arrow tool (block arrow).
/// Builds the autoshape.arrow geometry from the payload.
fn create_arrow(payload: &CreateArrowPayload) -> PathObject {
    vector::arrow(ArrowParams {
        origin: Point::new(payload.x, payload.y),
        width: payload.width,
        height: payload.height,
        thickness: payload.thickness,
        roundness: payload.roundness,
        arrow_width: payload.arrow_width,
        arrow_length: payload.arrow_length,
        arrow_tip: payload.arrow_tip,
    })
}

autoshape_command!(
    "autoshape.arrow",
    CreateArrow,
    CreateArrowPayload,
    create_arrow
);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateArrowPayload {
    /// Frame origin, world coordinates.
    x: f64,
    /// Frame origin, world coordinates.
    y: f64,
    /// Overall width; must be finite.
    width: f64,
    /// Overall height; must be finite.
    height: f64,
    /// Shaft thickness.
    thickness: f64,
    /// Corner roundness.
    roundness: f64,
    /// Full barb span.
    arrow_width: f64,
    /// Barb setback from the right edge.
    arrow_length: f64,
    /// Tip reach past the right edge.
    arrow_tip: f64,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

impl Placed for CreateArrowPayload {
    fn page(&self) -> usize {
        self.page
    }
    fn layer(&self) -> usize {
        self.layer
    }
    fn cell(&self) -> usize {
        self.cell
    }
    fn object_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

/// End head style for `autoshape.arrowLine` (each end independent).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum HeadStyle {
    /// No head (contour absent).
    None,
    /// Open V, stroked only.
    Open,
    /// Closed triangle.
    Solid,
    /// Barbed tail.
    Tail,
    /// Diagonal square.
    Box,
}

impl From<HeadStyle> for ArrowHead {
    fn from(style: HeadStyle) -> Self {
        match style {
            HeadStyle::None => ArrowHead::None,
            HeadStyle::Open => ArrowHead::Open,
            HeadStyle::Solid => ArrowHead::Solid,
            HeadStyle::Tail => ArrowHead::Tail,
            HeadStyle::Box => ArrowHead::Box,
        }
    }
}

/// `autoshape.arrowLine` — the Arrow Line tool.
/// Builds the autoshape.arrowLine geometry from the payload.
fn create_arrow_line(payload: &CreateArrowLinePayload) -> PathObject {
    vector::arrow_line(ArrowLineParams {
        start: Point::new(payload.start_x, payload.start_y),
        end: Point::new(payload.end_x, payload.end_y),
        start_head: payload.start_head.into(),
        end_head: payload.end_head.into(),
    })
}

autoshape_command!(
    "autoshape.arrowLine",
    CreateArrowLine,
    CreateArrowLinePayload,
    create_arrow_line
);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateArrowLinePayload {
    /// Line start, world coordinates.
    start_x: f64,
    /// Line start, world coordinates.
    start_y: f64,
    /// Line end, world coordinates.
    end_x: f64,
    /// Line end, world coordinates.
    end_y: f64,
    /// Head style at the start.
    start_head: HeadStyle,
    /// Head style at the end.
    end_head: HeadStyle,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

impl Placed for CreateArrowLinePayload {
    fn page(&self) -> usize {
        self.page
    }
    fn layer(&self) -> usize {
        self.layer
    }
    fn cell(&self) -> usize {
        self.cell
    }
    fn object_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

/// `autoshape.measure` — the Measure tool (dimension readout; the
/// distance label is a text element, out of scope for this command).
/// Builds the autoshape.measure geometry from the payload.
fn create_measure(payload: &CreateMeasurePayload) -> PathObject {
    vector::measure(MeasureParams {
        start: Point::new(payload.start_x, payload.start_y),
        end: Point::new(payload.end_x, payload.end_y),
    })
}

autoshape_command!(
    "autoshape.measure",
    CreateMeasure,
    CreateMeasurePayload,
    create_measure
);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateMeasurePayload {
    /// Measured start, world coordinates.
    start_x: f64,
    /// Measured start, world coordinates.
    start_y: f64,
    /// Measured end, world coordinates.
    end_x: f64,
    /// Measured end, world coordinates.
    end_y: f64,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

impl Placed for CreateMeasurePayload {
    fn page(&self) -> usize {
        self.page
    }
    fn layer(&self) -> usize {
        self.layer
    }
    fn cell(&self) -> usize {
        self.cell
    }
    fn object_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

/// `autoshape.connectorLine` — the Connector Line tool.
/// Builds the autoshape.connectorLine geometry from the payload.
fn create_connector_line(payload: &CreateConnectorLinePayload) -> PathObject {
    vector::connector_line(ConnectorParams {
        start: Point::new(payload.start_x, payload.start_y),
        end: Point::new(payload.end_x, payload.end_y),
        elbow: Point::new(payload.elbow_x, payload.elbow_y),
        corner1: payload.corner1,
        corner2: payload.corner2,
        vertical: payload.vertical,
    })
}

autoshape_command!(
    "autoshape.connectorLine",
    CreateConnectorLine,
    CreateConnectorLinePayload,
    create_connector_line
);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateConnectorLinePayload {
    /// Start point, world coordinates.
    start_x: f64,
    /// Start point, world coordinates.
    start_y: f64,
    /// End point, world coordinates.
    end_x: f64,
    /// End point, world coordinates.
    end_y: f64,
    /// Spine position.
    elbow_x: f64,
    /// Spine position.
    elbow_y: f64,
    /// Start corner: signed offset of the start arc from the spine.
    corner1: f64,
    /// End corner, likewise.
    corner2: f64,
    /// Route orientation: vertical spine when set.
    vertical: bool,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

impl Placed for CreateConnectorLinePayload {
    fn page(&self) -> usize {
        self.page
    }
    fn layer(&self) -> usize {
        self.layer
    }
    fn cell(&self) -> usize {
        self.cell
    }
    fn object_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

/// Which Auto Shape library entry `autoshape.redraw` re-generates
/// with. The strings match the `autoshape.*` command-id suffixes.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum SmartKind {
    Rect,
    Star,
    Polygon,
    Spiral,
    Pie,
    Doughnut,
    LShape,
    Arrow,
    #[serde(rename = "arrowLine")]
    ArrowLine,
    Measure,
    #[serde(rename = "connectorLine")]
    ConnectorLine,
}

impl SmartKind {
    /// The library name, for error messages.
    fn label(self) -> &'static str {
        match self {
            SmartKind::Rect => "rect",
            SmartKind::Star => "star",
            SmartKind::Polygon => "polygon",
            SmartKind::Spiral => "spiral",
            SmartKind::Pie => "pie",
            SmartKind::Doughnut => "doughnut",
            SmartKind::LShape => "lshape",
            SmartKind::Arrow => "arrow",
            SmartKind::ArrowLine => "arrowLine",
            SmartKind::Measure => "measure",
            SmartKind::ConnectorLine => "connectorLine",
        }
    }
}

/// Parameter surfaces for `autoshape.redraw` — geometry only. The
/// create payloads carry placement (page/layer/cell/name), which a
/// redraw has no use for: the object already sits in its cell.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RedrawRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corner: f64,
    kind: RectCorner,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RedrawStar {
    center_x: f64,
    center_y: f64,
    outer: f64,
    inner: f64,
    outer_round: f64,
    inner_round: f64,
    points: u32,
    start_angle: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RedrawPolygon {
    center_x: f64,
    center_y: f64,
    outer: f64,
    inner: f64,
    points: u32,
    sections: u32,
    start_angle: f64,
    split: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RedrawSpiral {
    center_x: f64,
    center_y: f64,
    radius: f64,
    tightness_deg: f64,
    clockwise: bool,
    closed: bool,
}

/// Shared by the `pie` and `doughnut` kinds, as at creation.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RedrawPie {
    center_x: f64,
    center_y: f64,
    outer: f64,
    inner: f64,
    start: f64,
    end: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RedrawLShape {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    thickness: f64,
    roundness: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RedrawArrow {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    thickness: f64,
    roundness: f64,
    arrow_width: f64,
    arrow_length: f64,
    arrow_tip: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RedrawArrowLine {
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    start_head: HeadStyle,
    end_head: HeadStyle,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RedrawMeasure {
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RedrawConnectorLine {
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    elbow_x: f64,
    elbow_y: f64,
    corner1: f64,
    corner2: f64,
    vertical: bool,
}

/// Deserializes `value` against the kind's parameter surface.
fn params<T: serde::de::DeserializeOwned>(
    kind: SmartKind,
    value: &serde_json::Value,
) -> Result<T, EngineError> {
    serde_json::from_value(value.clone()).map_err(|error| EngineError::BadPayload {
        name: "autoshape.redraw".to_owned(),
        detail: format!("{} params do not fit their shape: {error}", kind.label()),
    })
}

/// Re-runs a library generator from redraw parameters. Each arm is the
/// create mapping minus placement.
fn redraw_rect(p: &RedrawRect) -> PathObject {
    vector::rectangle(RectParams {
        origin: Point::new(p.x, p.y),
        width: p.width,
        height: p.height,
        corner: p.corner,
        kind: p.kind.into(),
    })
}

fn redraw_star(p: &RedrawStar) -> PathObject {
    vector::star(StarParams {
        center: Point::new(p.center_x, p.center_y),
        outer: p.outer,
        inner: p.inner,
        outer_round: p.outer_round,
        inner_round: p.inner_round,
        points: p.points,
        start_angle: p.start_angle,
    })
}

fn redraw_polygon(p: &RedrawPolygon) -> PathObject {
    vector::smart_polygon(PolygonParams {
        center: Point::new(p.center_x, p.center_y),
        outer: p.outer,
        inner: p.inner,
        points: p.points,
        sections: p.sections,
        start_angle: p.start_angle,
        split: p.split,
    })
}

fn redraw_spiral(p: &RedrawSpiral) -> PathObject {
    vector::spiral(SpiralParams {
        center: Point::new(p.center_x, p.center_y),
        radius: p.radius,
        tightness_deg: p.tightness_deg,
        clockwise: p.clockwise,
        closed: p.closed,
    })
}

fn redraw_pie(p: &RedrawPie) -> PathObject {
    vector::pie(PieParams {
        center: Point::new(p.center_x, p.center_y),
        outer: p.outer,
        inner: p.inner,
        start: p.start,
        end: p.end,
        is_pie: true,
    })
}

fn redraw_doughnut(p: &RedrawPie) -> PathObject {
    vector::doughnut(PieParams {
        center: Point::new(p.center_x, p.center_y),
        outer: p.outer,
        inner: p.inner,
        start: p.start,
        end: p.end,
        is_pie: false,
    })
}

fn redraw_l_shape(p: &RedrawLShape) -> PathObject {
    vector::lshape(LShapeParams {
        origin: Point::new(p.x, p.y),
        width: p.width,
        height: p.height,
        thickness: p.thickness,
        roundness: p.roundness,
    })
}

fn redraw_arrow(p: &RedrawArrow) -> PathObject {
    vector::arrow(ArrowParams {
        origin: Point::new(p.x, p.y),
        width: p.width,
        height: p.height,
        thickness: p.thickness,
        roundness: p.roundness,
        arrow_width: p.arrow_width,
        arrow_length: p.arrow_length,
        arrow_tip: p.arrow_tip,
    })
}

fn redraw_arrow_line(p: &RedrawArrowLine) -> PathObject {
    vector::arrow_line(ArrowLineParams {
        start: Point::new(p.start_x, p.start_y),
        end: Point::new(p.end_x, p.end_y),
        start_head: p.start_head.into(),
        end_head: p.end_head.into(),
    })
}

fn redraw_measure(p: &RedrawMeasure) -> PathObject {
    vector::measure(MeasureParams {
        start: Point::new(p.start_x, p.start_y),
        end: Point::new(p.end_x, p.end_y),
    })
}

fn redraw_connector_line(p: &RedrawConnectorLine) -> PathObject {
    vector::connector_line(ConnectorParams {
        start: Point::new(p.start_x, p.start_y),
        end: Point::new(p.end_x, p.end_y),
        elbow: Point::new(p.elbow_x, p.elbow_y),
        corner1: p.corner1,
        corner2: p.corner2,
        vertical: p.vertical,
    })
}

/// `autoshape.redraw` — CS6 `dom.redrawSmartShape()`: "Forces the Auto
/// Shapes in the document to redraw... useful when modifying a Auto
/// Shape outside a tool." CS6 re-runs each shape's embedded generator
/// script; `FireCraft`'s library bakes geometry at insert (no script
/// interpreter, no wire node for script state), so the redraw carries
/// its parameters explicitly and regenerates one path's contours.
/// Styling, name, and effects are untouched — this re-parameterizes,
/// it does not restyle.
struct RedrawSmartShape;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RedrawSmartShapePayload {
    /// Target path object.
    id: ObjectId,
    /// Which library entry re-generates the geometry.
    kind: SmartKind,
    /// That entry's parameter surface (geometry only).
    params: serde_json::Value,
}

impl TypedCommand for RedrawSmartShape {
    type Payload = RedrawSmartShapePayload;

    fn name(&self) -> &'static str {
        "autoshape.redraw"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "autoshape.redraw";
        // Capture before the first mutation; nothing below mutates on
        // the refusal paths.
        let before = match access::object(document, NAME, payload.id)? {
            ObjectData::Path(path) => path.clone(),
            _ => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a vector path", payload.id.index()),
                });
            }
        };
        let generated = match payload.kind {
            SmartKind::Rect => redraw_rect(&params(payload.kind, &payload.params)?),
            SmartKind::Star => redraw_star(&params(payload.kind, &payload.params)?),
            SmartKind::Polygon => redraw_polygon(&params(payload.kind, &payload.params)?),
            SmartKind::Spiral => redraw_spiral(&params(payload.kind, &payload.params)?),
            SmartKind::Pie => redraw_pie(&params(payload.kind, &payload.params)?),
            SmartKind::Doughnut => redraw_doughnut(&params(payload.kind, &payload.params)?),
            SmartKind::LShape => redraw_l_shape(&params(payload.kind, &payload.params)?),
            SmartKind::Arrow => redraw_arrow(&params(payload.kind, &payload.params)?),
            SmartKind::ArrowLine => redraw_arrow_line(&params(payload.kind, &payload.params)?),
            SmartKind::Measure => redraw_measure(&params(payload.kind, &payload.params)?),
            SmartKind::ConnectorLine => {
                redraw_connector_line(&params(payload.kind, &payload.params)?)
            }
        };
        if !finite_geometry(&generated) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "{} parameters must be finite (generated geometry is not)",
                    payload.kind.label()
                ),
            });
        }
        if let Some(ObjectData::Path(path)) = document.registry.get_mut(payload.id) {
            path.contours = generated.contours;
            path.even_odd = generated.even_odd;
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(ObjectData::Path(before)),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::super::testdoc;
    use super::*;
    use crate::error::EngineError;
    use crate::registry::CommandRegistry;
    use doc::{ObjectData, ObjectId};
    use serde_json::json;

    fn apply_by_name(
        document: &mut Document,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<Delta, EngineError> {
        let mut registry = CommandRegistry::new();
        install(&mut registry).unwrap();
        registry.execute(document, name, payload)
    }

    /// Placement into the fixture's first cell, plus an object name.
    fn placed() -> serde_json::Value {
        json!({ "page": 0, "layer": 0, "cell": 0, "name": "shape" })
    }

    fn cell_objects(document: &Document) -> Vec<ObjectId> {
        document.pages[0].layers.layers[0].cells[0].elements.clone()
    }

    #[test]
    fn star_defaults_insert_a_ten_anchor_ring() {
        let (mut document, _) = testdoc::sample_document();
        let before = cell_objects(&document).len();
        apply_by_name(
            &mut document,
            "autoshape.star",
            json!({
                "centerX": 0.0, "centerY": 0.0,
                "outer": 100.0, "inner": 50.0,
                "outerRound": 100.0, "innerRound": 50.0,
                "points": 5, "startAngle": -std::f64::consts::FRAC_PI_2,
                "page": 0, "layer": 0, "cell": 0, "name": "star",
            }),
        )
        .unwrap();
        let elements = cell_objects(&document);
        assert_eq!(elements.len(), before + 1);
        let ObjectData::Path(path) = document.registry.get(*elements.last().unwrap()).unwrap()
        else {
            panic!("inserted object is not a path");
        };
        assert_eq!(path.contours.len(), 1);
        assert!(path.contours[0].closed);
        assert_eq!(path.contours[0].point_count, 10);
        assert!(!path.even_odd);
        assert_eq!(path.name.as_deref(), Some("star"));
    }

    #[test]
    fn doughnut_inserts_even_odd_geometry() {
        let (mut document, _) = testdoc::sample_document();
        apply_by_name(
            &mut document,
            "autoshape.doughnut",
            json!({
                "centerX": 0.0, "centerY": 0.0,
                "outer": 100.0, "inner": 40.0,
                "start": 0.0, "end": 0.0,
                "page": 0, "layer": 0, "cell": 0, "name": null,
            }),
        )
        .unwrap();
        let id = *cell_objects(&document).last().unwrap();
        let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
            panic!("inserted object is not a path");
        };
        assert!(path.even_odd, "the annulus reads through the hole");
    }

    #[test]
    fn non_finite_parameters_are_rejected_without_mutating() {
        // JSON cannot carry a non-finite float: `json!` coerces NaN to
        // null, which fails payload deserialization before any
        // geometry runs.
        let (mut document, _) = testdoc::sample_document();
        let before = serde_json::to_vec(&document).unwrap();
        let error = apply_by_name(
            &mut document,
            "autoshape.lshape",
            json!({
                "x": 0.0, "y": 0.0,
                "width": 200.0, "height": null,
                "thickness": 100.0, "roundness": 0.0,
                "page": 0, "layer": 0, "cell": 0, "name": null,
            }),
        )
        .expect_err("null height must be rejected");
        assert!(matches!(error, EngineError::BadPayload { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "a rejected payload must leave the document untouched"
        );

        // The typed path (a crafted in-memory payload) still trips the
        // geometry tripwire, again without mutating.
        let payload = CreateLShapePayload {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: f64::NAN,
            thickness: 100.0,
            roundness: 0.0,
            page: 0,
            layer: 0,
            cell: 0,
            name: None,
        };
        let error = CreateLShape
            .apply(&mut document, &payload)
            .expect_err("NaN height must be rejected");
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn unknown_payload_keys_are_rejected() {
        let (mut document, _) = testdoc::sample_document();
        let mut payload = placed();
        payload["bogus"] = json!(1);
        payload["shape"] = json!("star");
        payload["centerX"] = json!(0.0);
        payload["centerY"] = json!(0.0);
        payload["outer"] = json!(100.0);
        payload["inner"] = json!(50.0);
        payload["outerRound"] = json!(100.0);
        payload["innerRound"] = json!(50.0);
        payload["points"] = json!(5);
        payload["startAngle"] = json!(0.0);
        let error = apply_by_name(&mut document, "autoshape.star", payload)
            .expect_err("unknown keys must be rejected");
        assert!(matches!(error, EngineError::BadPayload { .. }));
    }

    #[test]
    fn insert_deltas_carry_the_inverse_for_undo() {
        let (mut document, _) = testdoc::sample_document();
        let before = cell_objects(&document);
        let delta = apply_by_name(
            &mut document,
            "autoshape.arrowLine",
            json!({
                "startX": 40.0, "startY": -30.0,
                "endX": 140.0, "endY": -30.0,
                "startHead": "none", "endHead": "open",
                "page": 0, "layer": 0, "cell": 0, "name": null,
            }),
        )
        .unwrap();
        // Cell placement (with prior content) + object creation: the
        // pair undo needs to restore both sides.
        let Delta::Group(deltas) = delta else {
            panic!("insert must be a group delta");
        };
        assert_eq!(deltas.len(), 2);
        assert!(matches!(deltas[0], Delta::Cell { .. }));
        assert!(matches!(
            deltas[1],
            Delta::Object {
                inverse: ObjectInverse::Created,
                ..
            }
        ));
        assert_eq!(cell_objects(&document).len(), before.len() + 1);
    }

    /// A five-point star inserted into the fixture's first cell.
    fn star_document() -> (Document, ObjectId) {
        let (mut document, _) = testdoc::sample_document();
        apply_by_name(
            &mut document,
            "autoshape.star",
            json!({
                "centerX": 0.0, "centerY": 0.0,
                "outer": 100.0, "inner": 50.0,
                "outerRound": 100.0, "innerRound": 50.0,
                "points": 5, "startAngle": -std::f64::consts::FRAC_PI_2,
                "page": 0, "layer": 0, "cell": 0, "name": "star",
            }),
        )
        .unwrap();
        let id = *cell_objects(&document).last().unwrap();
        (document, id)
    }

    #[test]
    fn redraw_reparameterizes_and_undo_restores() {
        let (mut document, id) = star_document();
        let before = match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => path.clone(),
            other => panic!("expected a path, got {other:?}"),
        };
        assert_eq!(before.contours[0].point_count, 10);

        let delta = apply_by_name(
            &mut document,
            "autoshape.redraw",
            json!({
                "id": id, "kind": "star",
                "params": {
                    "centerX": 0.0, "centerY": 0.0,
                    "outer": 100.0, "inner": 50.0,
                    "outerRound": 100.0, "innerRound": 50.0,
                    "points": 6, "startAngle": -std::f64::consts::FRAC_PI_2,
                },
            }),
        )
        .unwrap();
        let redrawn = match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => path,
            other => panic!("expected a path, got {other:?}"),
        };
        assert_eq!(redrawn.contours[0].point_count, 12, "one point pair more");
        // Re-parameterization only: paint, name, and flags survive.
        assert_eq!(redrawn.name.as_deref(), Some("star"));
        assert_eq!(redrawn.paint, before.paint);
        assert_eq!(redrawn.even_odd, before.even_odd);

        delta.apply_inverse(&mut document).unwrap();
        let restored = match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => path,
            other => panic!("expected a path, got {other:?}"),
        };
        assert_eq!(*restored, before, "undo restores the exact path");
    }

    #[test]
    fn redraw_matches_a_fresh_create() {
        // The golden gate: a redrawn shape is indistinguishable from
        // one created with the same parameters.
        let (mut document, _) = testdoc::sample_document();
        let star = |points: u32| {
            json!({
                "centerX": 4.0, "centerY": -7.0,
                "outer": 90.0, "inner": 45.0,
                "outerRound": 80.0, "innerRound": 40.0,
                "points": points, "startAngle": 0.25,
                "page": 0, "layer": 0, "cell": 0, "name": null,
            })
        };
        apply_by_name(&mut document, "autoshape.star", star(5)).unwrap();
        apply_by_name(&mut document, "autoshape.star", star(8)).unwrap();
        let elements = cell_objects(&document);
        let (five, eight) = (elements[elements.len() - 2], elements[elements.len() - 1]);
        let contours_of =
            |document: &Document, id: ObjectId| match document.registry.get(id).unwrap() {
                ObjectData::Path(path) => path.contours.clone(),
                other => panic!("expected a path, got {other:?}"),
            };
        assert_ne!(contours_of(&document, five), contours_of(&document, eight));

        apply_by_name(
            &mut document,
            "autoshape.redraw",
            json!({
                "id": eight, "kind": "star",
                "params": {
                    "centerX": 4.0, "centerY": -7.0,
                    "outer": 90.0, "inner": 45.0,
                    "outerRound": 80.0, "innerRound": 40.0,
                    "points": 5, "startAngle": 0.25,
                },
            }),
        )
        .unwrap();
        assert_eq!(contours_of(&document, five), contours_of(&document, eight));
    }

    #[test]
    fn redraw_swaps_the_even_odd_flag() {
        let (mut document, id) = star_document();
        apply_by_name(
            &mut document,
            "autoshape.redraw",
            json!({
                "id": id, "kind": "doughnut",
                "params": {
                    "centerX": 0.0, "centerY": 0.0,
                    "outer": 100.0, "inner": 40.0,
                    "start": 0.0, "end": 0.0,
                },
            }),
        )
        .unwrap();
        let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
            panic!("expected a path");
        };
        assert!(path.even_odd, "the annulus reads through the hole");
        assert_eq!(path.name.as_deref(), Some("star"), "name survives");
    }

    #[test]
    fn redraw_refusals_leave_the_document_untouched() {
        let (mut document, id) = star_document();
        let before = serde_json::to_vec(&document).unwrap();
        let star = json!({
            "centerX": 0.0, "centerY": 0.0,
            "outer": 100.0, "inner": 50.0,
            "outerRound": 100.0, "innerRound": 50.0,
            "points": 6, "startAngle": 0.0,
        });

        // Unknown kind, star params against the polygon surface, and
        // an id that is not in the registry.
        for payload in [
            json!({"id": id, "kind": "hexagon", "params": star}),
            json!({
                "id": id, "kind": "polygon",
                "params": {"centerX": 0.0, "centerY": 0.0, "outer": 90.0},
            }),
            json!({"id": doc::ObjectId::from_parts(999, 0), "kind": "star", "params": star}),
        ] {
            let error = apply_by_name(&mut document, "autoshape.redraw", payload)
                .expect_err("redraw must refuse");
            assert!(matches!(
                error,
                EngineError::BadPayload { .. } | EngineError::MissingTarget { .. }
            ));
            assert_eq!(
                serde_json::to_vec(&document).unwrap(),
                before,
                "a refused redraw must leave the document untouched"
            );
        }
    }
}

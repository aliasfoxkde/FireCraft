//! The editor core: document + tool + view, no window required.
//!
//! Shell rule: nothing here mutates a `Document` except through
//! [`Session::apply`] — the same registry the CLI and journal replay
//! use, so UI edits and scripted edits are indistinguishable in the
//! journal. The egui layer ([`crate::shell`]) is a thin face on this.

use doc::{Cell, Document, Layer, ObjectData, ObjectId};
use engine::catalog;
use engine::error::EngineError;
use engine::{CommandRegistry, Session};

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
    /// Drag = marquee zoom, click = zoom in, alt-click = zoom out.
    Zoom,
    /// Drag to pan.
    Hand,
    /// Drag to create an Auto Shape with the tool's default
    /// parameters (shape chosen in the Tools menu).
    AutoShape(AutoShape),
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
    fn payload(self, drag: Drag, page: usize, frame: usize) -> serde_json::Value {
        let (sx, sy) = (drag.start.x, drag.start.y);
        let (cx, cy) = (drag.current.x, drag.current.y);
        let frame_json = serde_json::json!({
            "page": page, "layer": 0, "cell": frame, "name": null,
        });
        // Frame origin/size, normalized so drags in any direction
        // produce positive extents.
        let (x, y) = (sx.min(cx), sy.min(cy));
        let (width, height) = ((cx - sx).abs(), (cy - sy).abs());
        // Center + radius (circle-bound shapes).
        let (center_x, center_y) = ((sx + cx) / 2.0, (sy + cy) / 2.0);
        let radius = width.max(height) / 2.0;
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

/// A creation drag in progress: tool + world-space start point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    /// Tool the drag belongs to.
    pub tool: Tool,
    /// World position where the press landed.
    pub start: geom::Point,
    /// Current world position.
    pub current: geom::Point,
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
    /// Selected object, if any (the last one picked).
    pub selection: Option<ObjectId>,
    /// Multi-selection the arrange/boolean surfaces operate on, in
    /// pick order; `selection` is always its last entry when non-empty.
    pub selection_set: Vec<ObjectId>,
    /// Creation drag in progress.
    pub drag: Option<Drag>,
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
        Self {
            session: Session::with_registry(document, registry),
            unknown,
            chunks,
            tool: Tool::Pointer,
            page: 0,
            frame: 0,
            selection: None,
            selection_set: Vec::new(),
            drag: None,
        }
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

    /// Applies a catalog command. The only mutation door in the app.
    ///
    /// # Errors
    /// Whatever the command refuses; the document is untouched.
    pub fn apply(&mut self, name: &str, payload: serde_json::Value) -> Result<(), EngineError> {
        self.session.apply(name, payload)
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
    #[must_use]
    pub fn current_cell(&self) -> Option<&Cell> {
        self.document()
            .pages
            .get(self.page)?
            .layers
            .layers
            .iter()
            .find_map(|layer: &Layer| layer.cells.get(self.frame))
    }

    /// Begins a pointer drag in world space.
    pub fn begin_drag(&mut self, world: geom::Point) {
        self.drag = Some(Drag {
            tool: self.tool,
            start: world,
            current: world,
        });
    }

    /// Updates the in-progress drag.
    pub fn drag_to(&mut self, world: geom::Point) {
        if let Some(drag) = &mut self.drag {
            drag.current = world;
        }
    }

    /// Ends the drag. Creation tools apply their create command through
    /// the session; degenerate drags (a click) create nothing.
    /// Panning/zooming tools touch only the view (shell's job).
    ///
    /// # Errors
    /// Whatever the creation command refuses.
    pub fn end_drag(&mut self) -> Result<(), EngineError> {
        let Some(drag) = self.drag.take() else {
            return Ok(());
        };
        let (x, y) = (drag.start.x, drag.start.y);
        let (width, height) = (drag.current.x - drag.start.x, drag.current.y - drag.start.y);
        // A click is not a shape.
        if width.abs() < 0.5 || height.abs() < 0.5 {
            return Ok(());
        }
        match drag.tool {
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
                        "page": self.page, "layer": 0, "cell": self.frame,
                        "name": null,
                    }),
                )
            }
            Tool::AutoShape(shape) => {
                self.apply(shape.command(), shape.payload(drag, self.page, self.frame))
            }
            Tool::Pointer | Tool::Subselect | Tool::Zoom | Tool::Hand => Ok(()),
        }
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

    /// The topmost live path in the current cell hit by `world`.
    #[must_use]
    pub fn hit_at(&self, world: geom::Point) -> Option<ObjectId> {
        let cell = self.current_cell()?;
        let candidates: Vec<ObjectId> = cell.elements.iter().copied().rev().collect();
        candidates.into_iter().find(|id| {
            matches!(
                self.document().registry.get(*id),
                Some(ObjectData::Path(path)) if bounds_of(path).contains(world)
            )
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

    /// Cell element index of `id` (z position); unattached objects
    /// sort last without disturbing the relative order of found ones.
    fn z_order(&self, id: ObjectId) -> usize {
        self.current_cell()
            .and_then(|cell| cell.elements.iter().position(|element| *element == id))
            .unwrap_or(usize::MAX)
    }
}

/// World-space bounding box of a path's contour points (anchors and
/// control handles — the handles keep fast strokes inside the box).
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
}

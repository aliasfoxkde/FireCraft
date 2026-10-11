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
    /// Selected object, if any.
    pub selection: Option<ObjectId>,
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

    /// Ends the drag. Creation tools apply `object.createPath` through
    /// the session; degenerate drags (a click) create nothing.
    /// Panning/zooming tools touch only the view (shell's job).
    ///
    /// # Errors
    /// Whatever the creation command refuses.
    pub fn end_drag(&mut self) -> Result<(), EngineError> {
        let Some(drag) = self.drag.take() else {
            return Ok(());
        };
        let kind = match drag.tool {
            Tool::Rect => "rect",
            Tool::Ellipse => "ellipse",
            Tool::Line => "line",
            _ => return Ok(()),
        };
        let (x, y) = (drag.start.x, drag.start.y);
        let (width, height) = (drag.current.x - drag.start.x, drag.current.y - drag.start.y);
        // A click is not a shape.
        if width.abs() < 0.5 || height.abs() < 0.5 {
            return Ok(());
        }
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

    /// Selects the topmost object in the current cell whose contour
    /// bounding box contains `world`. Groups select as wholes in v1
    /// (subselect descends later).
    #[must_use]
    pub fn select_at(&mut self, world: geom::Point) -> Option<ObjectId> {
        let cell = self.current_cell()?;
        let candidates: Vec<ObjectId> = cell.elements.iter().copied().rev().collect();
        for id in candidates {
            let Some(ObjectData::Path(path)) = self.document().registry.get(id) else {
                continue;
            };
            if bounds_of(path).contains(world) {
                self.selection = Some(id);
                return Some(id);
            }
        }
        self.selection = None;
        None
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

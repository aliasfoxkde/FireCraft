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
    /// The symbol definition being edited, by `MSNi`; `None` edits the
    /// page tree. While set, creation commands land their objects in
    /// the definition (via a page-side create plus `symbol.editAttach`)
    /// and hit-testing looks at the definition's own cells.
    pub symbol_scope: Option<i64>,
    /// Active layer inside the symbol scope (view state).
    pub symbol_layer: usize,
    /// Nine-slice guide edge drag in progress.
    pub guide_drag: Option<GuideDrag>,
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
            symbol_scope: None,
            symbol_layer: 0,
            guide_drag: None,
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

    /// Applies a catalog command. The only mutation door in the app.
    ///
    /// # Errors
    /// Whatever the command refuses; the document is untouched.
    pub fn apply(&mut self, name: &str, payload: serde_json::Value) -> Result<(), EngineError> {
        self.session.apply(name, payload)
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
                "page": self.page, "layer": 0, "cell": self.frame,
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
                        "page": self.page, "layer": 0, "cell": self.frame,
                        "name": null,
                    }),
                )?;
                true
            }
            Tool::Text => {
                self.apply(
                    "text.createText",
                    serde_json::json!({
                        "page": self.page, "layer": 0, "cell": self.frame,
                        "x": x, "y": y, "width": width, "height": height,
                        "text": "", "name": null,
                    }),
                )?;
                true
            }
            Tool::AutoShape(shape) => {
                self.apply(shape.command(), shape.payload(drag, self.page, self.frame))?;
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
            Tool::Pointer | Tool::Subselect | Tool::Zoom | Tool::Hand => false,
        };
        if created && self.symbol_scope.is_some() {
            self.lift_creation_into_scope(&before)?;
        }
        Ok(())
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

    /// The topmost live path in the edited cell hit by `world` — the
    /// definition's cell in symbol scope, the page cell otherwise.
    #[must_use]
    pub fn hit_at(&self, world: geom::Point) -> Option<ObjectId> {
        let cell = self.editing_cell()?;
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
}

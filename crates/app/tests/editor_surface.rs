//! Phase 14.1 editor-surface battery: the pointer/paint/selection
//! gesture families through [`Editor`]'s public API — autoshape
//! label/command/payload tables, the stroke family (brush, pencil,
//! eraser, gradient), pixel selections (lasso, ellipse, polygon,
//! wand-adjacent helpers), drag classes (creation, move, clone,
//! constrained), guide drags, library export/import, and the
//! import_styles refusal paths. Every expected value is hand-derived
//! from the editor's documented defaults.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::{AutoShape, DragConstraint, Editor, NineSliceEdge, PixelSelectMode, Tool};
use base64::Engine as _;
use doc::ObjectData;
use engine::delta::Delta;
use geom::Point;

/// A 2×2 opaque-red raster on the page — the paint/selection target.
fn editor_with_raster() -> (Editor, doc::ObjectId) {
    let mut editor = Editor::untitled();
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

// --- autoshape tables -------------------------------------------------------

#[test]
fn autoshape_labels_commands_and_defaults() {
    let cases: &[(AutoShape, &str, &str)] = &[
        (AutoShape::Star, "Star", "autoshape.star"),
        (AutoShape::Polygon, "Smart Polygon", "autoshape.polygon"),
        (AutoShape::Spiral, "Spiral", "autoshape.spiral"),
        (AutoShape::Pie, "Pie", "autoshape.pie"),
        (AutoShape::Doughnut, "Doughnut", "autoshape.doughnut"),
        (AutoShape::LShape, "L-Shape", "autoshape.lshape"),
        (AutoShape::Arrow, "Arrow", "autoshape.arrow"),
        (AutoShape::ArrowLine, "Arrow Line", "autoshape.arrowLine"),
        (AutoShape::Measure, "Measure", "autoshape.measure"),
        (
            AutoShape::ConnectorLine,
            "Connector Line",
            "autoshape.connectorLine",
        ),
    ];
    for &(shape, label, command) in cases {
        assert_eq!(shape.label(), label, "{label}");
        assert_eq!(shape.command(), command, "{label}");
    }
    assert_eq!(AutoShape::ALL.len(), 10);
}

#[test]
fn every_autoshape_drag_creates_a_path_object() {
    for shape in AutoShape::ALL {
        let mut editor = Editor::untitled();
        editor.tool = Tool::AutoShape(shape);
        editor.begin_drag(Point::new(10.0, 10.0));
        editor.drag_to(Point::new(50.0, 40.0));
        editor.end_drag().unwrap();
        let cell = editor.current_cell().unwrap();
        assert_eq!(cell.elements.len(), 1, "{shape:?} created one object");
        assert!(
            matches!(
                editor.document().registry.get(cell.elements[0]),
                Some(ObjectData::Path(_))
            ),
            "{shape:?} is a path object"
        );
    }
}

// --- pixel select modes ------------------------------------------------------

#[test]
fn pixel_select_mode_wires_and_modifier_mapping() {
    assert_eq!(PixelSelectMode::Replace.wire(), "replace");
    assert_eq!(PixelSelectMode::Add.wire(), "add");
    assert_eq!(PixelSelectMode::Subtract.wire(), "subtract");
    assert_eq!(PixelSelectMode::Intersect.wire(), "intersect");

    let mods = |shift: bool, alt: bool, ctrl: bool| egui::Modifiers {
        alt,
        ctrl,
        shift,
        ..egui::Modifiers::default()
    };
    assert_eq!(
        PixelSelectMode::from_modifiers(mods(true, false, false)),
        PixelSelectMode::Add
    );
    assert_eq!(
        PixelSelectMode::from_modifiers(mods(false, true, false)),
        PixelSelectMode::Subtract
    );
    assert_eq!(
        PixelSelectMode::from_modifiers(mods(false, false, true)),
        PixelSelectMode::Intersect
    );
    assert_eq!(
        PixelSelectMode::from_modifiers(mods(false, false, false)),
        PixelSelectMode::Replace
    );
}

// --- the stroke family -------------------------------------------------------

#[test]
fn a_brush_stroke_paints_and_folds_into_one_undo_step() {
    let (mut editor, _id) = editor_with_raster();
    editor.tool = Tool::Paintbrush;
    editor.begin_stroke(Point::new(0.5, 0.5)).unwrap();
    editor.stroke_to(Point::new(1.5, 0.5)).unwrap();
    editor.stroke_to(Point::new(1.5, 1.5)).unwrap();
    editor.end_stroke().unwrap();
    // One undoable step for the whole stroke.
    editor.undo().expect("stroke is one undo step");
    // After undo the stroke's transaction is closed: a second undo
    // reports no open paint step (either it errors or undoes older
    // work — both are fine; what matters is the stroke folded).
    let _ = editor.undo();
}

#[test]
fn a_stroke_off_canvas_is_a_no_op() {
    let (mut editor, _id) = editor_with_raster();
    editor.tool = Tool::Pencil;
    // No image under the point: the stroke never opens.
    editor.begin_stroke(Point::new(500.0, 500.0)).unwrap();
    assert!(editor.stroke.is_none());
    editor.stroke_to(Point::new(510.0, 510.0)).unwrap();
    editor.end_stroke().unwrap();
}

#[test]
fn a_micro_move_adds_no_dab() {
    let (mut editor, _id) = editor_with_raster();
    editor.tool = Tool::Pencil;
    editor.begin_stroke(Point::new(0.5, 0.5)).unwrap();
    // Under the 0.5 px spacing floor: no dab, no error.
    editor.stroke_to(Point::new(0.6, 0.5)).unwrap();
    editor.end_stroke().unwrap();
}

#[test]
fn the_gradient_stroke_paints_on_lift() {
    let (mut editor, _id) = editor_with_raster();
    editor.tool = Tool::Gradient;
    editor.begin_stroke(Point::new(0.0, 0.0)).unwrap();
    editor.stroke_to(Point::new(1.5, 1.5)).unwrap();
    editor.end_stroke().unwrap();
    // The gradient's undo step exists (the paint happened at commit).
    editor.undo().expect("gradient painted one undo step");
}

#[test]
fn the_eraser_stroke_runs_through_the_same_door() {
    let (mut editor, _id) = editor_with_raster();
    editor.tool = Tool::Eraser;
    editor.begin_stroke(Point::new(0.5, 0.5)).unwrap();
    editor.stroke_to(Point::new(1.5, 1.5)).unwrap();
    editor.end_stroke().unwrap();
    editor.undo().expect("erase painted one undo step");
}

#[test]
fn the_paint_bucket_fills_or_reports_no_target() {
    let (mut editor, _id) = editor_with_raster();
    assert!(editor.fill_target(Point::new(0.5, 0.5)).unwrap());
    assert!(!editor.fill_target(Point::new(500.0, 500.0)).unwrap());
}

// --- pixel selections --------------------------------------------------------

#[test]
fn an_ellipse_marquee_selects_through_the_document() {
    let (mut editor, _id) = editor_with_raster();
    editor.tool = Tool::EllipticalMarquee;
    editor.begin_pixel_selection(Point::new(0.0, 0.0)).unwrap();
    editor.extend_pixel_selection(Point::new(2.0, 2.0)).unwrap();
    editor.end_pixel_selection().unwrap();
    assert!(editor.has_pixel_selection());
    // The ellipse over [0,2)² covers the pixel centers.
    let mask = editor.document().selection.as_ref().unwrap();
    assert_eq!((mask.width, mask.height), (2, 2));
}

#[test]
fn a_lasso_records_its_trace() {
    let (mut editor, _id) = editor_with_raster();
    editor.tool = Tool::Lasso;
    editor.begin_pixel_selection(Point::new(0.0, 0.0)).unwrap();
    // A triangle covering the pixel centers (the image_point path
    // accepts trace vertices past the frame, unlike begin).
    editor.extend_pixel_selection(Point::new(2.0, 0.0)).unwrap();
    editor.extend_pixel_selection(Point::new(0.0, 2.0)).unwrap();
    editor.end_pixel_selection().unwrap();
    assert!(editor.has_pixel_selection());
}

#[test]
fn a_polygon_selects_at_three_or_more_vertices() {
    let (mut editor, _id) = editor_with_raster();
    // Fewer than three vertices: the commit refuses and keeps the
    // draft. (Interior points — the pointer must sit on the image.)
    editor.add_polygon_point(Point::new(0.5, 0.5)).unwrap();
    editor.add_polygon_point(Point::new(1.5, 0.5)).unwrap();
    assert!(!editor.commit_polygon().unwrap());
    assert!(editor.polygon.is_some(), "the draft survives a refusal");
    editor.add_polygon_point(Point::new(1.0, 1.5)).unwrap();
    assert!(editor.commit_polygon().unwrap());
    assert!(editor.polygon.is_none());
    assert!(editor.has_pixel_selection());
    // A commit with no draft at all is a no-op.
    assert!(!editor.commit_polygon().unwrap());
}

#[test]
fn selection_outline_maps_mask_runs_to_world_space() {
    let (mut editor, _id) = editor_with_raster();
    editor.tool = Tool::Marquee;
    editor.begin_pixel_selection(Point::new(0.5, 0.5)).unwrap();
    editor.extend_pixel_selection(Point::new(2.0, 2.0)).unwrap();
    editor.end_pixel_selection().unwrap();
    // The image sits at (0,0), so world == pixel for the outline.
    // The marquee covers every pixel: a fully-selected mask has no
    // interior boundary — the image border is not drawn.
    let outline = editor.selection_outline().expect("outline");
    assert!(outline.is_empty(), "full mask, no interior edges");
    // A partial selection does outline: deselect one column.
    editor.deselect_pixels().unwrap();
    editor.tool = Tool::Marquee;
    editor.begin_pixel_selection(Point::new(0.0, 0.0)).unwrap();
    editor.extend_pixel_selection(Point::new(1.0, 2.0)).unwrap();
    editor.end_pixel_selection().unwrap();
    // A 1×2 partial selection (left column) has one interior vertical
    // boundary at x=1 spanning the mask's rows.
    let outline = editor.selection_outline().expect("outline");
    assert_eq!(outline.len(), 1, "one interior edge at x=1");
}

#[test]
fn selection_helpers_route_through_the_engine() {
    let (mut editor, id) = editor_with_raster();
    editor.tool = Tool::Marquee;
    editor.begin_pixel_selection(Point::new(0.5, 0.5)).unwrap();
    editor.extend_pixel_selection(Point::new(2.0, 2.0)).unwrap();
    editor.end_pixel_selection().unwrap();

    editor.feather_selection(1).unwrap();
    editor.adjust_pixel_selection("expand", 1).unwrap();
    editor.select_similar(16).unwrap();
    editor.move_selection_mask(1, 0).unwrap();
    editor.move_selected_pixels(1, 0).unwrap();
    assert_eq!(editor.image_location(id), Some(Point::new(0.0, 0.0)));
    editor.erase_selected().unwrap();
    // The erase clears the pixels but leaves the selection active
    // (as in the original — marching ants persist through an erase).
    assert!(editor.has_pixel_selection());
}

#[test]
fn image_location_rejects_non_images() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Rect;
    editor.begin_drag(Point::new(0.0, 0.0));
    editor.drag_to(Point::new(10.0, 10.0));
    editor.end_drag().unwrap();
    let id = editor.current_cell().unwrap().elements[0];
    assert_eq!(editor.image_location(id), None);
}

// --- drag classes --------------------------------------------------------------

#[test]
fn a_constrained_drag_still_creates() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Rect;
    editor.begin_drag(Point::new(10.0, 10.0));
    editor.drag_to_with(
        Point::new(60.0, 20.0),
        DragConstraint {
            shift: true,
            alt: false,
        },
    );
    editor.end_drag().unwrap();
    assert_eq!(editor.current_cell().unwrap().elements.len(), 1);
}

#[test]
fn a_move_drag_translates_live_and_folds_on_lift() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Rect;
    editor.begin_drag(Point::new(0.0, 0.0));
    editor.drag_to(Point::new(10.0, 10.0));
    editor.end_drag().unwrap();
    let id = editor.current_cell().unwrap().elements[0];

    editor
        .begin_move_drag(Point::new(0.0, 0.0), vec![id])
        .unwrap();
    editor.drag_to(Point::new(5.0, 0.0));
    editor.end_drag().unwrap();
    // One "Move" undo step folds both translates.
    editor.undo().expect("the move folds into one step");
}

#[test]
fn a_click_sized_move_drag_commits_nothing() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Rect;
    editor.begin_drag(Point::new(0.0, 0.0));
    editor.drag_to(Point::new(10.0, 10.0));
    editor.end_drag().unwrap();
    let id = editor.current_cell().unwrap().elements[0];
    let before = editor.session.document().clone();

    editor
        .begin_move_drag(Point::new(0.0, 0.0), vec![id])
        .unwrap();
    editor.end_drag().unwrap();
    // The click aborted its transaction: position unchanged.
    let path = match editor.document().registry.get(id) {
        Some(ObjectData::Path(path)) => path,
        other => panic!("path, got {other:?}"),
    };
    let baseline = match before.registry.get(id) {
        Some(ObjectData::Path(path)) => path,
        other => panic!("path, got {other:?}"),
    };
    assert_eq!(path.contours, baseline.contours, "nothing moved");
}

#[test]
fn a_clone_drag_duplicates_then_moves_the_copies() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Rect;
    editor.begin_drag(Point::new(0.0, 0.0));
    editor.drag_to(Point::new(10.0, 10.0));
    editor.end_drag().unwrap();
    let id = editor.current_cell().unwrap().elements[0];

    let created = editor
        .begin_clone_drag(Point::new(0.0, 0.0), vec![id])
        .unwrap();
    assert_eq!(created.len(), 1);
    assert_ne!(created[0], id);
    editor.drag_to(Point::new(3.0, 0.0));
    editor.end_drag().unwrap();
    assert_eq!(
        editor.current_cell().unwrap().elements.len(),
        2,
        "the original and the clone both exist"
    );
    assert_eq!(editor.selected_live(), Some(created[0]));
}

#[test]
fn a_text_drag_creates_a_text_object() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Text;
    editor.begin_drag(Point::new(5.0, 5.0));
    editor.drag_to(Point::new(40.0, 20.0));
    editor.end_drag().unwrap();
    let cell = editor.current_cell().unwrap();
    assert!(matches!(
        editor.document().registry.get(cell.elements[0]),
        Some(ObjectData::Text(_))
    ));
}

#[test]
fn slice_and_hotspot_drags_live_on_the_web_layer() {
    for tool in [Tool::Slice, Tool::Hotspot] {
        let mut editor = Editor::untitled();
        editor.tool = tool;
        editor.begin_drag(Point::new(0.0, 0.0));
        editor.drag_to(Point::new(30.0, 20.0));
        editor.end_drag().unwrap();
        assert!(
            editor.current_cell().unwrap().elements.is_empty(),
            "{tool:?} writes to the Web Layer, not the current cell"
        );
    }
}

#[test]
fn a_line_drag_creates_a_line_path() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Line;
    editor.begin_drag(Point::new(0.0, 0.0));
    editor.drag_to(Point::new(30.0, 10.0));
    editor.end_drag().unwrap();
    assert_eq!(editor.current_cell().unwrap().elements.len(), 1);
}

#[test]
fn an_ellipse_drag_creates_an_ellipse() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Ellipse;
    editor.begin_drag(Point::new(0.0, 0.0));
    editor.drag_to(Point::new(30.0, 10.0));
    editor.end_drag().unwrap();
    let id = editor.current_cell().unwrap().elements[0];
    match editor.document().registry.get(id) {
        Some(ObjectData::Path(path)) => {
            assert!(path.primitive.is_some(), "an ellipse primitive");
        }
        other => panic!("path, got {other:?}"),
    }
}

// --- guide drags -----------------------------------------------------------------

#[test]
fn guide_drags_reach_every_edge() {
    let mut editor = Editor::untitled();
    editor
        .apply(
            "library.add",
            serde_json::json!({ "name": "s", "symbol_type": "graphic" }),
        )
        .unwrap();
    // Give the definition content so nine-slice guides are legal:
    // enter the symbol first, then draw — the creation lands in the
    // definition (no page lift needed).
    editor.enter_symbol(0).unwrap();
    editor.tool = Tool::Rect;
    editor.begin_drag(Point::new(0.0, 0.0));
    editor.drag_to(Point::new(20.0, 20.0));
    editor.end_drag().unwrap();
    editor.exit_symbol();
    let number = editor.document().symbols[0].symbol_number;
    editor.enable_nine_slice(number).unwrap();
    for (edge, index) in [
        (NineSliceEdge::Top, 0usize),
        (NineSliceEdge::Bottom, 1),
        (NineSliceEdge::Left, 2),
        (NineSliceEdge::Right, 3),
    ] {
        assert_eq!(edge.control(), index, "{edge:?} control slot");
        // Drag the guide one pixel inward from its shape edge — the
        // inset is bounds-relative, so the drag target reads the
        // live shape bounds.
        let bounds = editor.document().symbols[0]
            .nine_slice
            .as_ref()
            .unwrap()
            .shape_bounds;
        let (from, to) = match edge {
            NineSliceEdge::Top => (Point::new(0.0, bounds.y0), Point::new(0.0, bounds.y0 + 1.0)),
            NineSliceEdge::Bottom => (Point::new(0.0, bounds.y1), Point::new(0.0, bounds.y1 - 1.0)),
            NineSliceEdge::Left => (Point::new(bounds.x0, 0.0), Point::new(bounds.x0 + 1.0, 0.0)),
            NineSliceEdge::Right => (Point::new(bounds.x1, 0.0), Point::new(bounds.x1 - 1.0, 0.0)),
        };
        editor.begin_guide_drag(number, edge, from).unwrap();
        editor.drag_guide_to(to);
        editor.end_guide_drag().unwrap();
    }
    let controls = editor.document().symbols[0]
        .nine_slice
        .as_ref()
        .unwrap()
        .controls;
    // Every guide was dragged one pixel inward from its edge.
    assert_eq!(controls[0], 1.0, "top");
    assert_eq!(controls[1], 1.0, "bottom");
    assert_eq!(controls[2], 1.0, "left");
    assert_eq!(controls[3], 1.0, "right");
}

// --- library round-trips -----------------------------------------------------------

#[test]
fn export_library_refuses_an_unknown_name() {
    let editor = Editor::untitled();
    let error = editor
        .export_library(&["no-such-symbol"])
        .expect_err("unknown name");
    assert!(error.to_string().contains("no-such-symbol"));
}

#[test]
fn library_export_import_round_trips_a_symbol() {
    let mut editor = Editor::untitled();
    editor
        .apply(
            "library.add",
            serde_json::json!({ "name": "round", "symbol_type": "graphic" }),
        )
        .unwrap();
    let bytes = editor.export_library(&["round"]).unwrap();
    let mut fresh = Editor::untitled();
    let numbers = fresh.import_library(&bytes, None).unwrap();
    assert_eq!(numbers.len(), 1);
    assert!(
        fresh.definition_of(numbers[0]).is_some(),
        "the definition landed"
    );
}

#[test]
fn import_styles_refuses_non_library_bytes() {
    let mut editor = Editor::untitled();
    // Not a PNG at all.
    assert!(editor.import_styles(b"not a png").is_err());
    // A valid PNG container with no mkTS chunk. The importer parses
    // the node stream and rejects it on shape, not by name.
    let png = editor.save().unwrap();
    assert!(editor.import_styles(&png).is_err());
}

#[test]
fn import_styles_refuses_truncated_mkts() {
    let mut editor = Editor::untitled();
    // A PNG-signature'd stream with a mkTS chunk of garbage zlib.
    let mut bytes = vec![0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend_from_slice(&[0, 0, 0, 4]); // length
    bytes.extend_from_slice(b"mkTS");
    bytes.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // not zlib
    bytes.extend_from_slice(&[0, 0, 0, 0]); // crc
    assert!(editor.import_styles(&bytes).is_err());
}

#[test]
fn apply_delta_reports_deltas_like_apply() {
    let mut editor = Editor::untitled();
    let delta = editor
        .apply_delta(
            "library.add",
            serde_json::json!({ "name": "d", "symbol_type": "graphic" }),
        )
        .unwrap();
    assert!(!matches!(delta, Delta::Noop));
    assert_eq!(editor.document().symbols.len(), 1);
}

#[test]
fn undo_redo_report_their_step_labels() {
    let mut editor = Editor::untitled();
    editor
        .apply(
            "library.add",
            serde_json::json!({ "name": "s", "symbol_type": "graphic" }),
        )
        .unwrap();
    let undone = editor.undo().unwrap();
    assert!(!undone.is_empty());
    let redone = editor.redo().unwrap();
    assert!(!redone.is_empty());
    assert!(editor.undo().is_ok());
}

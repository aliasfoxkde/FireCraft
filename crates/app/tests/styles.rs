//! The M15 task 6 style-library path, as a permanent gate: a synthetic
//! `.stl` (the `mkTS` style container) imports through
//! [`Editor::import_styles`] — entries join the document library with
//! their preserved subtree bag re-keyed past the base styles, a row of
//! the Styles panel (`style.apply`) paints the imported bundle onto a
//! selection, and the merge survives save → reopen byte-stably. The
//! import is one undoable step (list-capturing `style.importLibrary`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::Editor;
use doc::{Effect, EffectProperty, EffectValue, PaintStyle, StyleEntry, StyleText};
use std::io::Write as _;

/// One solid object style, the modal CS6 record: flags `0x1ff`, kind
/// `8`, a fill + stroke paint bundle, no effects.
fn solid(name: &str, fill_raw: i64) -> StyleEntry {
    StyleEntry {
        name: name.to_owned(),
        flags: 0x1ff,
        kind: 8,
        paint: PaintStyle {
            stroke_raw: 0xFF33_3322,
            fill_raw,
            fill: None,
        },
        fill_start: geom::Point::ZERO,
        fill_end: geom::Point::new(100.0, 0.0),
        fill_third: geom::Point::ZERO,
        brush_placement: 0,
        fill_on_top: true,
        effects: Vec::new(),
        text: StyleText::default(),
    }
}

/// A synthetic `.stl`: two style records — one solid, one carrying a
/// live effect — written through the model's own library writer and
/// wrapped as the `mkTS`+`IEND` chunk stream a Fireworks style
/// library carries.
fn synthetic_stl() -> Vec<u8> {
    let mut shadow = solid("Brass 01", 0xFFC8_A250);
    shadow.effects.push(Effect {
        category: "Shadow and Glow".to_owned(),
        internal_name: "dropShadow".to_owned(),
        properties: vec![EffectProperty {
            key: "gaussian_blur_radius".to_owned(),
            value: EffectValue::Text("4".to_owned()),
        }],
    });
    let entries = vec![shadow, solid("Ink 02", 0xFF20_0000)];
    let (nodes, stats) = doc::fwmap::write_style_library(&entries, &Default::default()).unwrap();
    assert_eq!(stats.unconsumed_unknown, 0, "the writer ate every node");
    let body = fwpng::nodetree::serialize(&nodes);
    let mut compressed = Vec::new();
    flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::default())
        .write_all(&body)
        .unwrap();
    let mut out = Vec::from(fwpng::chunk::PNG_SIGNATURE);
    fwpng::chunk::write_chunk(&mut out, b"mkTS", &compressed);
    fwpng::chunk::write_chunk(&mut out, b"IEND", &[]);
    out
}

/// An editor holding one selected path — the apply target.
fn editor_with_selection() -> (Editor, doc::ObjectId) {
    let mut editor = Editor::untitled();
    editor
        .apply(
            "object.createPath",
            serde_json::json!({
                "kind": "rect",
                "x": 0.0, "y": 0.0, "width": 24.0, "height": 10.0,
                "page": 0, "layer": 0, "cell": 0, "name": "plate",
            }),
        )
        .unwrap();
    let id = *editor.current_cell().unwrap().elements.last().unwrap();
    editor.selection_set = vec![id];
    (editor, id)
}

#[test]
fn import_appends_rekeyed_and_survives_reopen() {
    let mut editor = Editor::untitled();
    assert!(editor.document().styles.is_empty());
    let count = editor.import_styles(&synthetic_stl()).unwrap();
    assert_eq!(count, 2, "both records land");
    assert_eq!(
        editor
            .document()
            .styles
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Brass 01", "Ink 02"]
    );
    assert_eq!(
        editor.undo().unwrap(),
        "Import Styles",
        "one library = one undoable step (tiles + entries fold)"
    );
    assert!(
        editor.document().styles.is_empty(),
        "undo restores the list"
    );

    // Re-import, then push the document through the file boundary:
    // the merged library round-trips byte-stably.
    editor.import_styles(&synthetic_stl()).unwrap();
    let saved = editor.save().unwrap();
    let reopened = Editor::open(&saved).unwrap();
    assert_eq!(reopened.save().unwrap(), saved);
    assert_eq!(reopened.document().styles, editor.document().styles);
}

#[test]
fn apply_paints_an_imported_bundle() {
    let (mut editor, id) = editor_with_selection();
    editor.import_styles(&synthetic_stl()).unwrap();

    editor
        .apply(
            "style.apply",
            serde_json::json!({ "index": 1, "ids": [id] }),
        )
        .unwrap();
    match editor.document().registry.get(id) {
        Some(doc::ObjectData::Path(path)) => {
            let paint = path.paint.as_ref().unwrap();
            assert_eq!(paint.fill_raw, 0xFF20_0000, "the record's fill lands");
            assert_eq!(paint.stroke_raw, 0xFF33_3322, "the record's stroke lands");
        }
        other => panic!("expected the plate path, got {other:?}"),
    }
    assert_eq!(
        editor.undo().unwrap(),
        "style.apply",
        "apply undoes as one step"
    );

    // The effect-carrying record brings its effect along.
    editor
        .apply(
            "style.apply",
            serde_json::json!({ "index": 0, "ids": [id] }),
        )
        .unwrap();
    match editor.document().registry.get(id) {
        Some(doc::ObjectData::Path(path)) => {
            assert_eq!(path.effects.len(), 1, "the effect rides the bundle");
            assert_eq!(path.effects[0].internal_name, "dropShadow");
        }
        other => panic!("expected the plate path, got {other:?}"),
    }
}

#[test]
fn import_keeps_base_styles_separate() {
    // A document that already carries one style: the import re-keys
    // the bag past it and appends — nothing overwritten.
    let (mut editor, id) = editor_with_selection();
    editor
        .apply(
            "style.redefineFromSelection",
            serde_json::json!({ "name": "base", "source": id }),
        )
        .unwrap();
    assert_eq!(editor.document().styles.len(), 1);

    editor.import_styles(&synthetic_stl()).unwrap();
    let names: Vec<_> = editor
        .document()
        .styles
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert_eq!(names, vec!["base", "Brass 01", "Ink 02"]);
    let saved = editor.save().unwrap();
    let reopened = Editor::open(&saved).unwrap();
    assert_eq!(reopened.document().styles.len(), 3, "all three survive");
}

/// The shipped style set (`assets/styles/firecraft-core.stl`,
/// regenerated by `xtask styles-gen`): imports through the same
/// `Editor::import_styles` path the panel drives, and its records
/// paint.
#[test]
fn the_shipped_style_set_imports_and_paints() {
    let bytes = std::fs::read(asset("firecraft-core.stl"))
        .unwrap_or_else(|error| panic!("read asset: {error}"));
    let (mut editor, id) = editor_with_selection();
    let count = editor.import_styles(&bytes).unwrap();
    assert_eq!(count, 6, "the whole set lands");
    let names: Vec<_> = editor
        .document()
        .styles
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "Foundry Ink",
            "Signal Amber",
            "Verdigris Patina",
            "Ember Fade",
            "Paper White",
            "Midnight Radial",
        ]
    );
    // The gradient-backed record carries its FPLv; applying it paints
    // a gradient, not a solid.
    editor
        .apply(
            "style.apply",
            serde_json::json!({ "index": 3, "ids": [id] }),
        )
        .unwrap();
    match editor.document().registry.get(id) {
        Some(doc::ObjectData::Path(path)) => {
            let paint = path.paint.as_ref().unwrap();
            assert!(
                paint.fill.is_some(),
                "the gradient definition rides the record"
            );
        }
        other => panic!("expected the plate path, got {other:?}"),
    }
    // And the shadow-carrying record brings its effect stack.
    editor
        .apply(
            "style.apply",
            serde_json::json!({ "index": 2, "ids": [id] }),
        )
        .unwrap();
    match editor.document().registry.get(id) {
        Some(doc::ObjectData::Path(path)) => {
            assert_eq!(path.effects.len(), 1, "the drop shadow rides the record");
        }
        other => panic!("expected the plate path, got {other:?}"),
    }
}

// --- residual surfaces ------------------------------------------------------------

/// A style record whose fill carries a pattern tile: the `FPTv`
/// subtree names one `mkBT` raster through a `TIDi` reference.
fn patterned(name: &str, tile: u32) -> StyleEntry {
    let mut entry = solid(name, 0xFF30_4050);
    entry.paint.fill = Some(doc::Fill {
        shape: doc::FillShape::Solid,
        shape_extended: false,
        texture_blend: 1000,
        feather: 0,
        stamping: None,
        hard_edge: false,
        gradient: None,
        dither: None,
        pattern: Some(doc::PatternTile {
            name: Some("hatch".to_owned()),
            pixels: (128, 128),
            tile_size: 128,
            tiles: vec![doc::TileRef::Tile(tile)],
        }),
    });
    entry
}

/// A `.stl` carrying one pattern-backed record that references `mkBT`
/// index `tile`, plus the `mkBT` chunk itself.
fn patterned_stl(tile: u32) -> Vec<u8> {
    let entries = vec![patterned("Hatch 01", tile)];
    let (nodes, stats) = doc::fwmap::write_style_library(&entries, &Default::default()).unwrap();
    assert_eq!(stats.unconsumed_unknown, 0, "the writer ate every node");
    let body = fwpng::nodetree::serialize(&nodes);
    let mut compressed = Vec::new();
    flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::default())
        .write_all(&body)
        .unwrap();
    let mut out = Vec::from(fwpng::chunk::PNG_SIGNATURE);
    fwpng::chunk::write_chunk(&mut out, b"mkTS", &compressed);
    for payload in fwpng::tile::encode_mkbt(tile, &tile_plane()).unwrap() {
        fwpng::chunk::write_chunk(&mut out, b"mkBT", &payload);
    }
    fwpng::chunk::write_chunk(&mut out, b"IEND", &[]);
    out
}

/// One 128×128 ARGB tile plane.
fn tile_plane() -> Vec<u8> {
    let mut plane = vec![0u8; fwpng::tile::TILE_STREAM_BYTES];
    for (index, byte) in plane.iter_mut().enumerate() {
        *byte = (index % 251) as u8;
    }
    plane
}

/// A pattern-backed style import re-keys the library's `TIDi` onto
/// the document's tile bank: the reference lands on the fresh bank
/// slot, the raster joins the bank, and the record still applies.
#[test]
fn pattern_tile_import_rekeys_and_joins_the_bank() {
    let mut editor = Editor::untitled();
    assert!(editor.document().tiles.is_empty(), "the bank starts empty");

    let count = editor.import_styles(&patterned_stl(7)).unwrap();
    assert_eq!(count, 1, "the record lands");
    assert_eq!(
        editor.document().tiles.len(),
        1,
        "the library's raster joins the bank"
    );

    let pattern = editor.document().styles[0]
        .paint
        .fill
        .as_ref()
        .and_then(|fill| fill.pattern.as_ref())
        .expect("the pattern subtree survives the import");
    assert_eq!(
        pattern.tiles,
        vec![doc::TileRef::Tile(0)],
        "the library's tile 7 re-keys to the fresh bank slot 0"
    );

    // The record still applies onto a selection, pattern and all.
    let (mut editor, id) = editor_with_selection();
    editor.import_styles(&patterned_stl(7)).unwrap();
    editor
        .apply(
            "style.apply",
            serde_json::json!({ "index": 0, "ids": [id] }),
        )
        .unwrap();
    match editor.document().registry.get(id) {
        Some(doc::ObjectData::Path(path)) => {
            let fill = path.paint.as_ref().unwrap().fill.as_ref().unwrap();
            assert!(fill.pattern.is_some(), "the pattern rides the bundle");
        }
        other => panic!("expected the plate path, got {other:?}"),
    }
}

/// A grayscale `mkBT` (256×256) is not a style tile — the import
/// refuses whole rather than landing a wrong-shaped raster.
#[test]
fn grayscale_tile_refuses_the_import() {
    let mut editor = Editor::untitled();
    let entries = vec![solid("Plain", 0xFF00_0000)];
    let (nodes, _) = doc::fwmap::write_style_library(&entries, &Default::default()).unwrap();
    let body = fwpng::nodetree::serialize(&nodes);
    let mut compressed = Vec::new();
    flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::default())
        .write_all(&body)
        .unwrap();
    let mut out = Vec::from(fwpng::chunk::PNG_SIGNATURE);
    fwpng::chunk::write_chunk(&mut out, b"mkTS", &compressed);
    // A grayscale payload: the header's flag byte at [11] set, and the
    // stream sized for the 256×256 expansion.
    let mut payloads = fwpng::tile::encode_mkbt(3, &tile_plane()).unwrap();
    payloads[0][11] = 1;
    for payload in payloads {
        fwpng::chunk::write_chunk(&mut out, b"mkBT", &payload);
    }
    fwpng::chunk::write_chunk(&mut out, b"IEND", &[]);

    let error = editor.import_styles(&out).expect_err("grayscale refuses");
    let text = error.to_string();
    assert!(
        text.contains("grayscale"),
        "the refusal names the reason: {text}"
    );
    assert!(
        editor.document().styles.is_empty() && editor.document().tiles.is_empty(),
        "the document is untouched"
    );
}

/// A `TIDi` with no `mkBT` payload behind it is a corrupt library —
/// the import refuses whole rather than land a dangling reference.
#[test]
fn dangling_tile_reference_refuses_the_import() {
    let mut editor = Editor::untitled();
    // The record references tile 5, but no `mkBT` chunk rides along.
    let entries = vec![patterned("Hatch 01", 5)];
    let (nodes, _) = doc::fwmap::write_style_library(&entries, &Default::default()).unwrap();
    let body = fwpng::nodetree::serialize(&nodes);
    let mut compressed = Vec::new();
    flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::default())
        .write_all(&body)
        .unwrap();
    let mut out = Vec::from(fwpng::chunk::PNG_SIGNATURE);
    fwpng::chunk::write_chunk(&mut out, b"mkTS", &compressed);
    fwpng::chunk::write_chunk(&mut out, b"IEND", &[]);

    let error = editor
        .import_styles(&out)
        .expect_err("dangling reference refuses");
    let text = error.to_string();
    assert!(
        text.contains("no mkBT payload") || text.contains("tile 5"),
        "the refusal names the dangling reference: {text}"
    );
    assert!(
        editor.document().styles.is_empty(),
        "the document is untouched"
    );
}

/// Replaying a steps file onto a pristine base rebuilds the document
/// the recording was made from — the journal is the whole history.
#[test]
fn replay_steps_onto_a_pristine_base() {
    let mut editor = Editor::untitled();
    editor
        .apply(
            "object.createPath",
            serde_json::json!({
                "kind": "rect",
                "x": 0.0, "y": 0.0, "width": 24.0, "height": 10.0,
                "page": 0, "layer": 0, "cell": 0, "name": "plate",
            }),
        )
        .unwrap();
    let recorded = editor.steps_text();
    assert!(!recorded.is_empty(), "the journal captured the edit");
    let expected = editor.document().clone();

    // A pristine base (the untitled document) replays the recording
    // into the same end state.
    let mut replay = Editor::untitled();
    replay.replay_steps(&recorded).unwrap();
    assert_eq!(replay.document().registry, expected.registry);
    assert_eq!(
        replay.steps_text(),
        recorded,
        "the journal is exactly the recording"
    );
}

/// Nine-slice guides clear cleanly: enabling installs them, disabling
/// takes them away, and the definition keeps the rest of its shape.
#[test]
fn disable_nine_slice_clears_the_guides() {
    let mut editor = Editor::untitled();
    editor
        .apply(
            "library.add",
            serde_json::json!({ "name": "s", "symbol_type": "graphic" }),
        )
        .unwrap();
    editor.enter_symbol(0).unwrap();
    editor.tool = app::editor::Tool::Rect;
    editor.begin_drag(geom::Point::new(0.0, 0.0));
    editor.drag_to(geom::Point::new(20.0, 20.0));
    editor.end_drag().unwrap();
    editor.exit_symbol();

    let number = editor.document().symbols[0].symbol_number;
    editor.enable_nine_slice(number).unwrap();
    assert!(
        editor.document().symbols[0].nine_slice.is_some(),
        "the guides land"
    );

    editor.disable_nine_slice(number).unwrap();
    assert!(
        editor.document().symbols[0].nine_slice.is_none(),
        "the guides clear"
    );

    // And the definition's content survives the round trip.
    assert!(
        editor.definition_content_bounds(number).is_some(),
        "the shape content is untouched"
    );
}

/// Absolute path to a generated style asset.
fn asset(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/styles")
        .join(name)
}

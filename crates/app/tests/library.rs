//! The M10 task 6 stopping condition: library export → import → place
//! round-trips byte-comparably; import undoes to the pre-import bytes;
//! the five re-created Common Library assets (`assets/library/`,
//! original `FireCraft` art from `xtask library-gen`) open, import,
//! place, and render.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::Editor;
use compose::render;
use doc::ObjectData;

/// A document whose definition closure is two members (a plate and a
/// chip), both colored — the smallest import payload with more than
/// one member to re-slot.
fn two_member_editor() -> Editor {
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
    let plate = *editor.current_cell().unwrap().elements.last().unwrap();
    editor
        .apply(
            "object.setFill",
            serde_json::json!({ "id": plate, "color_raw": 0xFF20_4A87i64 }),
        )
        .unwrap();
    editor
        .apply(
            "object.createPath",
            serde_json::json!({
                "kind": "ellipse",
                "x": 6.0, "y": 2.0, "width": 6.0, "height": 6.0,
                "page": 0, "layer": 0, "cell": 0, "name": "chip",
            }),
        )
        .unwrap();
    let chip = *editor.current_cell().unwrap().elements.last().unwrap();
    editor
        .apply(
            "object.setFill",
            serde_json::json!({ "id": chip, "color_raw": 0xFFCC_0000i64 }),
        )
        .unwrap();
    let members = editor.current_cell().unwrap().elements.clone();
    editor
        .apply(
            "symbol.create",
            serde_json::json!({ "ids": members, "name": "kit", "symbol_type": "graphic" }),
        )
        .unwrap();
    editor
}

#[test]
fn export_import_place_round_trips_byte_comparably() {
    let bytes = two_member_editor().export_library(&["kit"]).unwrap();

    // Two fresh documents import the same library bytes and place the
    // same way — the results must be byte-identical.
    let place = |bytes: &[u8]| {
        let mut editor = Editor::untitled();
        let numbers = editor.import_library(bytes, None).unwrap();
        assert_eq!(numbers, vec![0], "one definition, numbered from 0");
        editor
            .apply(
                "symbol.createInstance",
                serde_json::json!({
                    "symbol": 0, "page": 0, "layer": 0, "cell": 0,
                    "x": 12.0, "y": 8.0,
                }),
            )
            .unwrap();
        editor
    };
    let first = place(&bytes);
    let second = place(&bytes);
    assert_eq!(
        first.save().unwrap(),
        second.save().unwrap(),
        "two imports of one library place identically"
    );

    // The placed document survives the file boundary byte-stably.
    let placed_bytes = first.save().unwrap();
    let reopened = Editor::open(&placed_bytes).unwrap();
    assert_eq!(reopened.save().unwrap(), placed_bytes);

    // The imported closure is live and named: the definition's cell
    // lists reference real members carrying the exported artwork.
    let root = first.document().symbols[0].layers.layers[0].cells[0].elements[0];
    let mut names = Vec::new();
    for id in &first.document().symbols[0].layers.layers[0].cells[0].elements {
        match first.document().registry.get(*id) {
            Some(ObjectData::Path(path)) => names.push(path.name.clone().unwrap_or_default()),
            other => panic!("expected a path member, got {other:?}"),
        }
    }
    assert!(names.contains(&"plate".to_owned()) && names.contains(&"chip".to_owned()));
    assert!(first.document().registry.get(root).is_some());
}

#[test]
fn import_undo_restores_the_pre_import_bytes() {
    let bytes = two_member_editor().export_library(&["kit"]).unwrap();
    let mut editor = Editor::untitled();
    let before = serde_json::to_vec(editor.document()).unwrap();

    editor.import_library(&bytes, None).unwrap();
    assert_eq!(editor.document().symbols.len(), 1);
    assert_eq!(
        editor.undo().unwrap(),
        "library.importSymbol",
        "one definition = one undoable step"
    );
    assert!(editor.document().symbols.is_empty());
    assert_eq!(
        serde_json::to_vec(editor.document()).unwrap(),
        before,
        "undo restores the pre-import document exactly"
    );
}

#[test]
fn import_can_pick_definitions_by_name() {
    // Themes carries three definitions; a name filter imports a subset.
    let bytes = std::fs::read(asset("Themes.fw.png")).unwrap();
    let mut editor = Editor::untitled();
    let numbers = editor
        .import_library(&bytes, Some(&["Theme - Ocean"]))
        .unwrap();
    assert_eq!(numbers, vec![0]);
    assert_eq!(editor.document().symbols.len(), 1);
    assert_eq!(editor.document().symbols[0].name, "Theme - Ocean");
    // Four swatches rode along as the definition's closure.
    assert_eq!(
        editor.document().symbols[0].layers.layers[0].cells[0]
            .elements
            .len(),
        4
    );
}

/// Every re-created Common Library asset: opens as a library file,
/// imports, places, and renders — the placed symbol visibly draws.
#[test]
fn the_common_library_assets_open_place_and_render() {
    for name in [
        "Animations.fw.png",
        "Bullets.fw.png",
        "Buttons.fw.png",
        "HaloLite.fw.png",
        "Themes.fw.png",
    ] {
        let bytes =
            std::fs::read(asset(name)).unwrap_or_else(|error| panic!("read asset {name}: {error}"));

        // The library file itself opens and carries its definitions.
        let library = Editor::open(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            !library.document().symbols.is_empty(),
            "{name} carries definitions"
        );

        // Import, place, render: the placed symbol draws over an
        // otherwise untouched canvas.
        let mut editor = Editor::untitled();
        editor.import_library(&bytes, None).unwrap();
        let placed = render(editor.document(), 0, 0).unwrap();
        editor
            .apply(
                "symbol.createInstance",
                serde_json::json!({
                    "symbol": 0, "page": 0, "layer": 0, "cell": 0,
                    "x": 8.0, "y": 8.0,
                }),
            )
            .unwrap();
        let drawn = render(editor.document(), 0, 0).unwrap();
        assert_eq!(
            (placed.width, placed.height),
            (drawn.width, drawn.height),
            "{name}: placement does not resize the canvas"
        );
        assert_ne!(
            placed.pixels, drawn.pixels,
            "{name}: the placed symbol must draw something"
        );
    }
}

/// A rich-symbol asset proves the whole chain survives the library
/// path: Buttons imports with its Widget script, the placed instance
/// re-derives through the M9 host, and the definition members carry
/// the script's defaults.
#[test]
fn the_rich_button_imports_and_rederives() {
    let bytes = std::fs::read(asset("Buttons.fw.png")).unwrap();
    let mut editor = Editor::untitled();
    let numbers = editor.import_library(&bytes, None).unwrap();
    let rich = *numbers.last().unwrap();
    let definition = editor
        .definition_of(rich)
        .unwrap_or_else(|| panic!("definition {rich} missing"));
    assert_eq!(definition.name, "Button - Rich");
    assert!(
        definition
            .op_code
            .as_deref()
            .unwrap_or_default()
            .contains("Widget.GetObjectByName"),
        "the rich definition carries its Widget script across the library path"
    );
    assert_eq!(definition.layers.layers[0].cells[0].elements.len(), 2);

    // Place, then re-derive the defaults pass (opCode 1) through the
    // M9 host: the label member takes the script's default text and
    // the plate member its default color.
    editor
        .apply(
            "symbol.createInstance",
            serde_json::json!({
                "symbol": rich, "page": 0, "layer": 0, "cell": 0,
                "x": 8.0, "y": 8.0,
            }),
        )
        .unwrap();
    let instance = *editor.current_cell().unwrap().elements.last().unwrap();
    let mut engine = scripting::ScriptEngine::new(scripting::Limits::default()).unwrap();
    let report = scripting::rederive_symbol(&mut editor.session, &mut engine, instance, 1);
    assert!(
        report.completed(),
        "defaults pass failed: {:?}",
        report.message
    );
    let document = editor.document();
    let label = document
        .symbols
        .iter()
        .find(|d| d.symbol_number == rich)
        .map(|d| d.layers.layers[0].cells[0].elements[1])
        .unwrap();
    match document.registry.get(label) {
        Some(ObjectData::Text(text)) => assert_eq!(text.runs[0].text, "Button"),
        other => panic!("expected the label text, got {other:?}"),
    }
    let plate = document
        .symbols
        .iter()
        .find(|d| d.symbol_number == rich)
        .map(|d| d.layers.layers[0].cells[0].elements[0])
        .unwrap();
    match document.registry.get(plate) {
        Some(ObjectData::Path(path)) => {
            assert_eq!(
                path.paint.as_ref().unwrap().fill_raw,
                0xFF06_9A4E,
                "#4e9a06 lands as raw ABGR"
            );
        }
        other => panic!("expected the plate path, got {other:?}"),
    }
    // The instance gained its parameter store.
    match document.registry.get(instance) {
        Some(ObjectData::SymbolInstance(data)) => {
            let values = data
                .custom_data
                .get("currentValues")
                .unwrap_or_else(|| panic!("currentValues missing after the defaults pass"));
            assert!(values.contains(r#""name":"label""#));
        }
        other => panic!("expected a symbol instance, got {other:?}"),
    }
}

/// Absolute path to a generated library asset.
fn asset(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/library")
        .join(name)
}

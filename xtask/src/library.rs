//! Common Library asset generation (`cargo run -p xtask -- library-gen`).
//!
//! The five CS6 Common Library files are re-created as ORIGINAL
//! FireCraft assets: vector art built through the engine registry from
//! the observed *categories* of their contents (animations, bullets,
//! buttons, halos, themes) — no Adobe bytes, paths, colors, or names
//! are copied. Each asset is a library file: an ordinary `.fw.png`
//! whose `MSLv` carries the definitions and whose registry holds their
//! member closure, nothing on a page — the same shape
//! `Editor::export_library` writes. `crates/app/tests/library.rs`
//! opens, imports, places, and renders every one.
//!
//! Buttons carries one rich symbol (a definition with a Widget script,
//! `JSVv` under `MSTv`) mirroring the observed rich-symbol presence in
//! CS6's Buttons.png, with FireCraft's own script and artwork.

use crate::corpus::{encode_rgba8, insert_before_iend, zlib};
use crate::util::workspace_root;

/// Output directory, workspace-relative.
pub(crate) const DIR: &str = "assets/library";

/// Canvas the builders work on; the art positions below fit inside it.
const CANVAS: (f64, f64) = (96.0, 64.0);

/// Gate entry point: regenerate the five assets.
pub fn run() -> Result<(), String> {
    let dir = workspace_root().join(DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let assets: [(&str, Result<Vec<u8>, String>); 5] = [
        ("Animations.fw.png", build_animations()),
        ("Bullets.fw.png", build_bullets()),
        ("Buttons.fw.png", build_buttons()),
        ("HaloLite.fw.png", build_halolite()),
        ("Themes.fw.png", build_themes()),
    ];
    let mut written = 0usize;
    for (name, bytes) in &assets {
        std::fs::write(dir.join(name), bytes.as_ref()?)
            .map_err(|e| format!("write {name}: {e}"))?;
        written += 1;
    }
    eprintln!("[xtask] library-gen: {written} asset(s) written to {DIR}/");
    Ok(())
}

/// One engine session with a single empty page — the scratch surface
/// the builders draw members on before `symbol.create` lifts each set
/// into a definition. Leftover placement instances stay on the page;
/// extraction takes only definition closures.
fn harness() -> Result<engine::Session, String> {
    let mut registry = engine::CommandRegistry::new();
    engine::catalog::install(&mut registry).map_err(|e| format!("catalog install: {e}"))?;
    let mut session = engine::Session::with_registry(doc::Document::default(), registry);
    apply(
        &mut session,
        "page.add",
        serde_json::json!({ "index": null }),
    )?;
    apply(
        &mut session,
        "page.setCanvasSize",
        serde_json::json!({ "page": 0, "width": CANVAS.0, "height": CANVAS.1 }),
    )?;
    Ok(session)
}

fn apply(
    session: &mut engine::Session,
    name: &str,
    payload: serde_json::Value,
) -> Result<(), String> {
    session
        .apply(name, payload)
        .map_err(|e| format!("{name}: {e}"))
}

/// The `ids` payload for the newest `count` cell elements.
fn take_tail(session: &engine::Session, count: usize) -> Vec<serde_json::Value> {
    let elements = &session.document().pages[0].layers.layers[0].cells[0].elements;
    elements[elements.len() - count..]
        .iter()
        .map(|id| serde_json::json!({ "index": id.index(), "generation": id.generation() }))
        .collect()
}

/// A filled rectangle member at `x,y`.
fn rect(
    session: &mut engine::Session,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: i64,
    name: &str,
) -> Result<(), String> {
    apply(
        session,
        "object.createPath",
        serde_json::json!({
            "kind": "rect", "x": x, "y": y, "width": w, "height": h,
            "page": 0, "layer": 0, "cell": 0, "name": name,
        }),
    )?;
    let id = newest(session)?;
    apply(
        session,
        "object.setFill",
        serde_json::json!({ "id": id, "color_raw": fill }),
    )
}

/// A filled ellipse member at `x,y`.
fn ellipse(
    session: &mut engine::Session,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: i64,
    name: &str,
) -> Result<(), String> {
    apply(
        session,
        "object.createPath",
        serde_json::json!({
            "kind": "ellipse", "x": x, "y": y, "width": w, "height": h,
            "page": 0, "layer": 0, "cell": 0, "name": name,
        }),
    )?;
    let id = newest(session)?;
    apply(
        session,
        "object.setFill",
        serde_json::json!({ "id": id, "color_raw": fill }),
    )
}

/// A text member at `x,y`.
fn text(
    session: &mut engine::Session,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    chars: &str,
    name: &str,
) -> Result<(), String> {
    apply(
        session,
        "text.createText",
        serde_json::json!({
            "x": x, "y": y, "width": w, "height": h,
            "text": chars, "page": 0, "layer": 0, "cell": 0, "name": name,
        }),
    )
}

/// The id payload of the cell's newest element.
fn newest(session: &engine::Session) -> Result<serde_json::Value, String> {
    let elements = &session.document().pages[0].layers.layers[0].cells[0].elements;
    let id = elements
        .last()
        .ok_or_else(|| "no object in cell".to_owned())?;
    Ok(serde_json::json!({
        "index": id.index(),
        "generation": id.generation(),
    }))
}

/// Lifts the newest `count` members into a named definition.
fn become_symbol(
    session: &mut engine::Session,
    count: usize,
    name: &str,
    symbol_type: &str,
) -> Result<(), String> {
    apply(
        session,
        "symbol.create",
        serde_json::json!({
            "ids": take_tail(session, count),
            "name": name,
            "symbol_type": symbol_type,
        }),
    )
}

/// Extracts every definition into the pageless library shape and wraps
/// it as a `.fw.png`.
fn finish(session: &engine::Session) -> Result<Vec<u8>, String> {
    let names: Vec<&str> = session
        .document()
        .symbols
        .iter()
        .map(|definition| definition.name.as_str())
        .collect();
    let library =
        doc::library::extract(session.document(), &names).map_err(|e| format!("extract: {e}"))?;
    let (tree, stats) = doc::fwmap::write(&library, &doc::fwmap::UnknownBag::default())
        .map_err(|e| format!("doc write: {e}"))?;
    if stats.unconsumed_unknown != 0 {
        return Err(format!(
            "doc write left {} unknown node(s)",
            stats.unconsumed_unknown
        ));
    }
    let payload = fwpng::nodetree::serialize(&tree);
    let standard = encode_rgba8(96, 64, |_, _| [255, 255, 255, 255])?;
    insert_before_iend(
        &standard,
        &[(b"mkTS", zlib(&payload)?), (b"mkBS", zlib(&payload)?)],
    )
}

/// CSS `#RRGGBB` to opaque raw ABGR — the same layout the engine's
/// `color_raw` payloads take.
#[allow(clippy::integer_division)]
fn abgr(css: &str) -> i64 {
    let hex = css.trim_start_matches('#');
    let r = u32::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u32::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u32::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (0xFF00_0000u64 | (u64::from(b) << 16) | (u64::from(g) << 8) | u64::from(r)) as i64
}

/// `Animations.fw.png`: three animation-source symbols — original
/// marks (ring, bar, disc) standing in for the observed class of
/// looping artwork.
fn build_animations() -> Result<Vec<u8>, String> {
    let mut session = harness()?;
    ellipse(&mut session, 8.0, 8.0, 12.0, 12.0, abgr("#3465a4"), "mark")?;
    become_symbol(&mut session, 1, "Anim - Spin", "animation")?;

    rect(&mut session, 36.0, 10.0, 14.0, 6.0, abgr("#f57900"), "mark")?;
    become_symbol(&mut session, 1, "Anim - Bounce", "animation")?;

    ellipse(&mut session, 68.0, 8.0, 10.0, 10.0, abgr("#73d216"), "mark")?;
    become_symbol(&mut session, 1, "Anim - Pulse", "animation")?;
    finish(&session)
}

/// `Bullets.fw.png`: five small list marks.
fn build_bullets() -> Result<Vec<u8>, String> {
    let mut session = harness()?;
    rect(&mut session, 6.0, 8.0, 6.0, 6.0, abgr("#cc0000"), "mark")?;
    become_symbol(&mut session, 1, "Bullet - Square", "graphic")?;

    ellipse(&mut session, 28.0, 8.0, 6.0, 6.0, abgr("#4e9a06"), "mark")?;
    become_symbol(&mut session, 1, "Bullet - Circle", "graphic")?;

    rect(&mut session, 48.0, 10.0, 10.0, 2.0, abgr("#06989a"), "mark")?;
    become_symbol(&mut session, 1, "Bullet - Dash", "graphic")?;

    rect(&mut session, 68.0, 8.0, 6.0, 6.0, abgr("#75507b"), "mark")?;
    become_symbol(&mut session, 1, "Bullet - Diamond", "graphic")?;

    ellipse(&mut session, 88.0, 8.0, 5.0, 5.0, abgr("#c4a000"), "mark")?;
    become_symbol(&mut session, 1, "Bullet - Dot", "graphic")?;
    finish(&session)
}

/// The Widget script for `Button - Rich`: label + color parameters,
/// defaults on opCode 1, apply on opCode 2 — FireCraft's own script in
/// the corpus-shape contract.
const BUTTON_SCRIPT: &str = r##"
var defaults = [
    { name: "label", type: "text", value: "Button" },
    { name: "color", type: "color", value: "#4e9a06" }
];
var values = Widget.elem.customData["currentValues"];
if (Widget.opCode === 1) {
    values = defaults;
    Widget.elem.customData["currentValues"] = defaults;
}
for (var i = 0; i < values.length; i++) {
    var v = values[i];
    if (v.name === "label") { Widget.GetObjectByName("label").textChars = v.value; }
    if (v.name === "color") { Widget.GetObjectByName("plate").pathAttributes.fillColor = v.value; }
}
"##;

/// `Buttons.fw.png`: two plain buttons and one rich symbol — the
/// rich-script carrier mirrors the observed `JSVv` in CS6's
/// Buttons.png, with original artwork and script.
fn build_buttons() -> Result<Vec<u8>, String> {
    let mut session = harness()?;

    rect(&mut session, 6.0, 8.0, 32.0, 12.0, abgr("#888a85"), "plate")?;
    text(&mut session, 12.0, 11.0, 20.0, 6.0, "Button", "label")?;
    become_symbol(&mut session, 2, "Button - Default", "graphic")?;

    rect(
        &mut session,
        46.0,
        8.0,
        32.0,
        12.0,
        abgr("#3465a4"),
        "plate",
    )?;
    text(&mut session, 54.0, 11.0, 16.0, 6.0, "OK", "label")?;
    become_symbol(&mut session, 2, "Button - Primary", "graphic")?;

    rect(
        &mut session,
        6.0,
        30.0,
        32.0,
        12.0,
        abgr("#4e9a06"),
        "plate",
    )?;
    text(&mut session, 12.0, 33.0, 20.0, 6.0, "Button", "label")?;
    become_symbol(&mut session, 2, "Button - Rich", "graphic")?;
    // The script rides the definition (`MSNi` 2 — the third created).
    apply(
        &mut session,
        "symbol.setScript",
        serde_json::json!({ "symbol": 2, "source": BUTTON_SCRIPT }),
    )?;
    finish(&session)
}

/// `HaloLite.fw.png`: three soft-glow approximations in plain vector.
fn build_halolite() -> Result<Vec<u8>, String> {
    let mut session = harness()?;
    ellipse(&mut session, 6.0, 6.0, 16.0, 16.0, abgr("#fce94f"), "ring")?;
    become_symbol(&mut session, 1, "Halo - Ring", "graphic")?;

    ellipse(&mut session, 36.0, 4.0, 18.0, 18.0, abgr("#729fcf"), "disc")?;
    become_symbol(&mut session, 1, "Halo - Disc", "graphic")?;

    rect(&mut session, 68.0, 4.0, 5.0, 20.0, abgr("#eeeeec"), "beam")?;
    become_symbol(&mut session, 1, "Halo - Beam", "graphic")?;
    finish(&session)
}

/// `Themes.fw.png`: three swatch rows — each definition a four-member
/// closure, so imports exercise the multi-member remap.
fn build_themes() -> Result<Vec<u8>, String> {
    let mut session = harness()?;

    for (column, (name, swatches)) in [
        (
            "Theme - Ember",
            ["#a40000", "#cc4400", "#e07830", "#f0c090"],
        ),
        (
            "Theme - Ocean",
            ["#204a87", "#3465a4", "#729fcf", "#c8d8f0"],
        ),
        (
            "Theme - Forest",
            ["#4e9a06", "#73d216", "#8ae234", "#c8f0a8"],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let x0 = 6.0 + f64::from(column as u16) * 24.0;
        for (row, color) in swatches.into_iter().enumerate() {
            rect(
                &mut session,
                x0,
                8.0 + f64::from(row as u16) * 11.0,
                18.0,
                9.0,
                abgr(color),
                "swatch",
            )?;
        }
        become_symbol(&mut session, 4, name, "graphic")?;
    }
    finish(&session)
}

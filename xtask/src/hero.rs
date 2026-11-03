//! Hero-card document generation (`cargo run -p xtask -- hero-gen`).
//!
//! Builds the demo document the README/release card shows: a
//! FireCraft-branded canvas with a gradient bar, vector shapes, and
//! type, written through the same engine registry the app drives.
//! The output is a normal `.fw.png`, so the app opens it with
//! File ▸ Open exactly like any other document.

use crate::corpus::{encode_rgba8, insert_before_iend, zlib};
use crate::util::workspace_root;

/// Output path, workspace-relative.
pub(crate) const OUT: &str = "docs/img/firecraft-demo.fw.png";

/// Canvas the card is drawn on.
const CANVAS: (f64, f64) = (960.0, 540.0);

/// Brand palette (the `FireCraft` hues).
const ORANGE: i64 = 0xFF_E8_7A_2E;
const BLUE: i64 = 0xFF_2F_6F_D8;
const TEAL: i64 = 0xFF_1E_9C_94;
const INK: i64 = 0xFF_22_28_33;
const PAPER: i64 = 0xFF_F4_F1_EA;

/// Gate entry point: regenerate the hero document.
pub fn run() -> Result<(), String> {
    let path = workspace_root().join(OUT);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    let bytes = build()?;
    std::fs::write(&path, &bytes).map_err(|e| format!("write {OUT}: {e}"))?;
    eprintln!("[xtask] hero-gen: wrote {OUT}");
    Ok(())
}

fn build() -> Result<Vec<u8>, String> {
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

    // Paper backdrop with a soft warm wash, so the card reads as a
    // document rather than a bare canvas.
    rect(
        &mut session,
        0.0,
        0.0,
        CANVAS.0,
        CANVAS.1,
        PAPER,
        "backdrop",
    )?;

    // Brand band: a wide orange-to-blue linear gradient.
    let band = shape(
        &mut session,
        "rect",
        0.0,
        0.0,
        CANVAS.0,
        132.0,
        "brand band",
    )?;
    gradient(
        &mut session,
        &band,
        "linear",
        ORANGE,
        BLUE,
        (0.0, 0.0),
        (CANVAS.0, 132.0),
    )?;

    // Three vector shapes across the lower half.
    ellipse(&mut session, 96.0, 232.0, 176.0, 176.0, TEAL, "disc")?;
    shape(&mut session, "rect", 330.0, 268.0, 240.0, 140.0, "slab").and_then(|id| {
        apply(
            &mut session,
            "object.setFill",
            serde_json::json!({ "id": id, "color_raw": INK }),
        )
    })?;
    ellipse(&mut session, 630.0, 236.0, 190.0, 190.0, ORANGE, "dot")?;

    // The wordmark, set in the bundled Liberation Sans.
    text(
        &mut session,
        64.0,
        44.0,
        560.0,
        56.0,
        "FireCraft",
        "wordmark",
    )?;
    text(
        &mut session,
        64.0,
        172.0,
        620.0,
        28.0,
        "Fireworks CS6, re-implemented in Rust",
        "tagline",
    )?;
    text(
        &mut session,
        64.0,
        462.0,
        700.0,
        24.0,
        "Vector + bitmap .fw.png documents, in the browser and on the desktop",
        "footnote",
    )?;

    let (tree, stats) = doc::fwmap::write(session.document(), &doc::fwmap::UnknownBag::default())
        .map_err(|e| format!("doc write: {e}"))?;
    if stats.unconsumed_unknown != 0 {
        return Err(format!(
            "doc write left {} unknown node(s)",
            stats.unconsumed_unknown
        ));
    }
    let payload = fwpng::nodetree::serialize(&tree);
    let standard = encode_rgba8(CANVAS.0 as u32, CANVAS.1 as u32, |_, _| {
        [244, 241, 234, 255]
    })?;
    insert_before_iend(
        &standard,
        &[(b"mkTS", zlib(&payload)?), (b"mkBS", zlib(&payload)?)],
    )
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

/// Creates a path shape and returns its element id.
fn shape(
    session: &mut engine::Session,
    kind: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    name: &str,
) -> Result<serde_json::Value, String> {
    apply(
        session,
        "object.createPath",
        serde_json::json!({
            "kind": kind, "x": x, "y": y, "width": width, "height": height,
            "page": 0, "layer": 0, "cell": 0, "name": name,
        }),
    )?;
    newest(session)
}

fn rect(
    session: &mut engine::Session,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill: i64,
    name: &str,
) -> Result<(), String> {
    let id = shape(session, "rect", x, y, width, height, name)?;
    apply(
        session,
        "object.setFill",
        serde_json::json!({ "id": id, "color_raw": fill }),
    )
}

fn ellipse(
    session: &mut engine::Session,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill: i64,
    name: &str,
) -> Result<(), String> {
    let id = shape(session, "ellipse", x, y, width, height, name)?;
    apply(
        session,
        "object.setFill",
        serde_json::json!({ "id": id, "color_raw": fill }),
    )
}

fn text(
    session: &mut engine::Session,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    chars: &str,
    name: &str,
) -> Result<(), String> {
    apply(
        session,
        "text.createText",
        serde_json::json!({
            "x": x, "y": y, "width": width, "height": height,
            "text": chars, "page": 0, "layer": 0, "cell": 0, "name": name,
        }),
    )
}

fn gradient(
    session: &mut engine::Session,
    id: &serde_json::Value,
    shape: &str,
    from: i64,
    to: i64,
    (sx, sy): (f64, f64),
    (ex, ey): (f64, f64),
) -> Result<(), String> {
    apply(
        session,
        "path.setFill",
        serde_json::json!({
            "id": id,
            "shape": shape,
            "stops": [
                { "position": 0.0, "color_raw": from },
                { "position": 1.0, "color_raw": to },
            ],
        }),
    )?;
    apply(
        session,
        "path.setFillVector",
        serde_json::json!({
            "id": id,
            "start": { "x": sx, "y": sy },
            "end": { "x": ex, "y": ey },
        }),
    )
}

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

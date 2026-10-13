//! Generates `assets/styles/firecraft-core.stl`: the original FireCraft
//! style set — six named object styles built through the engine's own
//! style surface (`path.setFill`/`filter.add` paint members, then
//! `style.redefineFromSelection` lifts each appearance into a record)
//! and written by the model's style-library writer into the `mkTS`
//! container a Fireworks `.stl` carries.
//!
//! Original work throughout — the CS6 `.stl` libraries under `source/`
//! are analysis input only and are never copied or shipped (asset rule;
//! see `docs/ATTRIBUTION.md`). This asset proves the write path with a
//! committed artifact and rides the corpus round-trip battery.

use crate::corpus::zlib;
use crate::util::workspace_root;
use fwpng::chunk::{PNG_SIGNATURE, write_chunk};

pub(crate) const DIR: &str = "assets/styles";

/// Gate entry point: regenerate the style set.
pub fn run() -> Result<(), String> {
    let dir = workspace_root().join(DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let bytes = build()?;
    std::fs::write(dir.join("firecraft-core.stl"), &bytes)
        .map_err(|e| format!("write firecraft-core.stl: {e}"))?;
    eprintln!("[xtask] styles-gen: 1 style set written to {DIR}/");
    Ok(())
}

/// Builds the set: one engine session paints six member rects (solid,
/// gradient, and effect-carrying), then redefines one style per
/// appearance. The document's style list is the payload.
fn build() -> Result<Vec<u8>, String> {
    let mut registry = engine::CommandRegistry::new();
    engine::catalog::install(&mut registry).map_err(|e| format!("catalog install: {e}"))?;
    let mut session = engine::Session::with_registry(doc::Document::default(), registry);
    fn apply(
        session: &mut engine::Session,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        session
            .apply(name, payload)
            .map_err(|e| format!("styles asset: {name}: {e}"))
    }
    apply(
        &mut session,
        "page.add",
        serde_json::json!({ "index": null }),
    )?;
    apply(
        &mut session,
        "page.setCanvasSize",
        serde_json::json!({ "page": 0, "width": 24.0, "height": 120.0 }),
    )?;
    // One member rect per style row, bottom-up; each gets its paint
    // before the define pass lifts it into a named record.
    let rows: [(&str, i64, &str); 6] = [
        ("Foundry Ink", 0xFF2B_303B, ""),
        ("Signal Amber", 0xFFE8_9B2C, ""),
        ("Verdigris Patina", 0xFF3E_8A76i64, "drop"),
        ("Ember Fade", 0xFFD4_5A28i64, "fade"),
        ("Paper White", 0xFFF4_F2EC, ""),
        ("Midnight Radial", 0xFF14_181Fi64, "radial"),
    ];
    for (name, fill_raw, treatment) in rows {
        apply(
            &mut session,
            "object.createPath",
            serde_json::json!({
                "kind": "rect",
                "x": 2.0, "y": 2.0, "width": 20.0, "height": 16.0,
                "page": 0, "layer": 0, "cell": 0, "name": name,
            }),
        )?;
        let id = newest(&session)?;
        apply(
            &mut session,
            "object.setFill",
            serde_json::json!({ "id": id, "color_raw": fill_raw }),
        )?;
        match treatment {
            "drop" => apply(
                &mut session,
                "filter.add",
                serde_json::json!({
                    "id": id, "name": "dropShadow", "category": "Shadow and Glow",
                    "params": [
                        { "key": "EffectMoaID", "value": "{a7944db8-6ce2-11d1-8c76000502701850}" },
                        { "key": "gaussian_blur_radius", "value": "4.000000" },
                    ],
                }),
            )?,
            "fade" => {
                apply(
                    &mut session,
                    "path.setFill",
                    serde_json::json!({
                        "id": id, "shape": "linear",
                        "stops": [
                            { "position": 0.0, "color_raw": 0xFFD4_5A28i64 },
                            { "position": 1.0, "color_raw": 0xFF7A_2E10i64 },
                        ],
                    }),
                )?;
                apply(
                    &mut session,
                    "path.setFillVector",
                    serde_json::json!({
                        "id": id,
                        "start": { "x": 2.0, "y": 10.0 },
                        "end": { "x": 22.0, "y": 10.0 },
                    }),
                )?;
            }
            "radial" => {
                apply(
                    &mut session,
                    "path.setFill",
                    serde_json::json!({
                        "id": id, "shape": "radial",
                        "stops": [
                            { "position": 0.0, "color_raw": 0xFF4C_566Ai64 },
                            { "position": 1.0, "color_raw": 0xFF14_181Fi64 },
                        ],
                    }),
                )?;
                apply(
                    &mut session,
                    "path.setFillVector",
                    serde_json::json!({
                        "id": id,
                        "start": { "x": 12.0, "y": 10.0 },
                        "end": { "x": 22.0, "y": 20.0 },
                    }),
                )?;
            }
            _ => {}
        }
        apply(
            &mut session,
            "style.redefineFromSelection",
            serde_json::json!({ "name": name, "source": id }),
        )?;
    }
    let entries = session.document().styles.clone();
    if entries.len() != rows.len() {
        return Err(format!(
            "styles asset: {} record(s), expected {}",
            entries.len(),
            rows.len()
        ));
    }
    let (nodes, stats) =
        doc::fwmap::write_style_library(&entries, &doc::fwmap::UnknownBag::default())
            .map_err(|e| format!("styles asset: style write: {e}"))?;
    if stats.unconsumed_unknown != 0 {
        return Err(format!(
            "styles asset: style write left {} unknown node(s)",
            stats.unconsumed_unknown
        ));
    }
    let payload = fwpng::nodetree::serialize(&nodes);
    let mut file = Vec::from(PNG_SIGNATURE);
    write_chunk(&mut file, b"mkTS", &zlib(&payload)?);
    write_chunk(&mut file, b"IEND", &[]);
    Ok(file)
}

/// The id the last engine command allocated: the cell's newest element.
fn newest(session: &engine::Session) -> Result<serde_json::Value, String> {
    let document = session.document();
    let elements = &document.pages[0].layers.layers[0].cells[0].elements;
    let id = elements
        .last()
        .ok_or_else(|| "styles asset: no object in cell".to_owned())?;
    Ok(serde_json::json!({
        "index": id.index(),
        "generation": id.generation(),
    }))
}

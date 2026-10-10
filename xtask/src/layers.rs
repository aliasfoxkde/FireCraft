//! Layer gate: workspace crates are organized in dependency tiers and the
//! gate fails when an edge points the wrong way or a crate is unregistered.
//!
//! ```text
//! L0  fwpng, geom, color, raster      foundations, zero cross-deps
//! L1  doc                             document model
//! L2  vector, filters, text, ops      feature engines
//! L3  compose, gpu, engine            rendering + orchestration
//! L4  io                              import/export codecs
//! L5  app, app-cli, automation        shells
//! L6  testkit                         test-only, may depend on anything
//! L7  xtask                           verification tooling
//! ```

use crate::util::cargo_metadata;

const LAYERS: &[(&str, u8)] = &[
    ("fwpng", 0),
    ("geom", 0),
    ("color", 0),
    ("raster", 0),
    ("doc", 1),
    ("vector", 2),
    ("filters", 2),
    ("text", 2),
    ("ops", 2),
    ("compose", 3),
    ("gpu", 3),
    ("engine", 3),
    ("io", 4),
    ("app", 5),
    ("app-cli", 5),
    ("automation", 5),
    ("testkit", 6),
    ("xtask", 7),
];

fn tier_of(name: &str) -> Option<u8> {
    LAYERS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, tier)| *tier)
}

/// Checks every workspace package's path dependencies against the tier
/// table.
pub fn run() -> Result<(), String> {
    let metadata = cargo_metadata()?;
    let Some(packages) = metadata["packages"].as_array() else {
        return Err("cargo metadata: missing packages array".into());
    };
    let mut errors = Vec::new();
    for package in packages {
        let name = package["name"].as_str().unwrap_or_default();
        let Some(tier) = tier_of(name) else {
            errors.push(format!(
                "crate `{name}` is not registered in the xtask layer table"
            ));
            continue;
        };
        let Some(deps) = package["dependencies"].as_array() else {
            continue;
        };
        for dep in deps {
            let dep_name = dep["name"].as_str().unwrap_or_default();
            if !dep["path"].is_string() {
                continue; // registry deps are outside the layer system
            }
            if name == "fwpng" {
                errors.push(format!(
                    "fwpng must stay standalone: found path dependency on `{dep_name}`"
                ));
                continue;
            }
            let Some(dep_tier) = tier_of(dep_name) else {
                errors.push(format!(
                    "`{name}` depends on `{dep_name}`, which is not registered in the layer table"
                ));
                continue;
            };
            // testkit (L6) may depend on anything; everything else must
            // point downward or sideways.
            if tier != 6 && dep_tier > tier {
                errors.push(format!(
                    "layer violation: `{name}` (L{tier}) depends on `{dep_name}` (L{dep_tier})"
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

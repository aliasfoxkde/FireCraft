//! Effect floor: zero unknown live-effect GUIDs over the observed
//! corpus (M14 task 4).
//!
//! The passthrough floor the M7 milestone took on as debt: a live
//! effect whose `EffectMoaID` is outside `compose::RECOGNIZED_GUIDS`
//! rides through unfiltered. Over the enumerated surface that must
//! never happen silently — every Fireworks file we pin or harvest is
//! scanned, and one unrecognized GUID fails the gate loudly. The floor
//! is over observed files, not a promise about third-party Photoshop
//! plugin GUIDs a future file might carry; when one appears, the fix
//! is a row in the enumeration (`docs/research/fw-code-notes.md`) and
//! a kernel, not a quiet skip.
//!
//! Scanned sets:
//!
//! - every `corpus/MANIFEST.tsv` entry (generated fixtures + Common
//!   Library assets),
//! - every PNG under `corpus/incoming/` (operator-harvested real
//!   Fireworks files — never committed, scanned when present).

use crate::util::{inflate, workspace_root};
use compose::RECOGNIZED_GUIDS;
use doc::fwmap;
use doc::model::Document;
use fwpng::chunk::parse_chunks;

/// The gate: no unrecognized `EffectMoaID` in any scanned file.
pub fn run() -> Result<(), String> {
    let root = workspace_root();
    // Style libraries (`.stl`) carry effect records, not live object
    // stacks — the floor is about rendered documents, so they skip.
    let mut files: Vec<String> = crate::corpus::manifest_entries()?
        .into_iter()
        .map(|(_, rel)| rel)
        .filter(|rel| !rel.ends_with(".stl"))
        .collect();
    let incoming = root.join("corpus/incoming");
    if incoming.is_dir() {
        let mut harvested: Vec<String> = std::fs::read_dir(&incoming)
            .map_err(|e| format!("read {}: {e}", incoming.display()))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.to_ascii_lowercase().ends_with(".png"))
            .map(|name| format!("corpus/incoming/{name}"))
            .collect();
        harvested.sort();
        files.extend(harvested);
    }
    if files.is_empty() {
        return Err("effect floor: no corpus files to scan".to_owned());
    }

    let mut scanned = 0usize;
    let mut effects = 0usize;
    let mut errors = Vec::new();
    for rel in &files {
        let bytes = match std::fs::read(root.join(rel)) {
            Ok(b) => b,
            Err(err) => {
                errors.push(format!("{rel}: {err}"));
                continue;
            }
        };
        let chunks = match parse_chunks(&bytes) {
            Ok(c) => c,
            Err(err) => {
                errors.push(format!("{rel}: parse: {err}"));
                continue;
            }
        };
        if !chunks
            .iter()
            .any(|c| &c.ctype == b"mkTS" || &c.ctype == b"mkBS")
        {
            continue; // a plain PNG carries no effect stacks
        }
        match document_from(&bytes) {
            Ok(Some(document)) => {
                scanned += 1;
                for guid in document_guids(&document) {
                    effects += 1;
                    if !RECOGNIZED_GUIDS.contains(&guid.as_str()) {
                        errors.push(format!(
                            "{rel}: live effect GUID {guid} rides through unfiltered — \
                             add it to the enumeration and a kernel"
                        ));
                    }
                }
            }
            Ok(None) => {}
            Err(err) => errors.push(format!("{rel}: {err}")),
        }
    }
    if errors.is_empty() {
        eprintln!(
            "[xtask] floor: {scanned} Fireworks file(s), {effects} effect stack(s), \
             0 unrecognized GUIDs"
        );
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Reads the file's node tree into a [`Document`] — `mkTS` preferred
/// (the type tree), `mkBS` as the fallback. `Ok(None)` when the file
/// carries neither (a plain PNG).
fn document_from(bytes: &[u8]) -> Result<Option<Document>, String> {
    let chunks = parse_chunks(bytes).map_err(|e| format!("parse chunks: {e}"))?;
    for ctype in [b"mkTS", b"mkBS"] {
        let Some(chunk) = chunks.iter().find(|c| &c.ctype == ctype) else {
            continue;
        };
        let inflated = inflate(&chunk.data)?;
        let nodes = fwpng::nodetree::parse(&inflated).map_err(|e| format!("node tree: {e}"))?;
        let (document, _bag) = fwmap::read(&nodes).map_err(|e| format!("document read: {e}"))?;
        return Ok(Some(document));
    }
    Ok(None)
}

/// Every `EffectMoaID` in the document's object registry, normalized
/// like compose's `effect_id` (braces stripped, lowercased).
fn document_guids(document: &Document) -> Vec<String> {
    let mut guids = Vec::new();
    for (_, object) in document.registry.iter() {
        let effects = match object {
            doc::registry::ObjectData::Path(path) => &path.effects,
            doc::registry::ObjectData::Text(text) => &text.effects,
            doc::registry::ObjectData::Group(group) => &group.effects,
            doc::registry::ObjectData::Image(image) => &image.effects,
            doc::registry::ObjectData::Slice(_) | doc::registry::ObjectData::SymbolInstance(_) => {
                continue;
            }
        };
        for effect in effects {
            if let Some(doc::EffectValue::Text(guid)) = effect
                .properties
                .iter()
                .find(|p| p.key == "EffectMoaID")
                .map(|p| &p.value)
            {
                guids.push(
                    guid.trim()
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .to_ascii_lowercase(),
                );
            }
        }
    }
    guids
}

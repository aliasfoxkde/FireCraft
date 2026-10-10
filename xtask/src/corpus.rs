//! Corpus gate: verifies every file listed in `corpus/MANIFEST.tsv`
//! against its pinned sha256. The corpus itself is never committed
//! (Adobe-copyrighted sample files live outside the repo); the manifest
//! pins what was verified and when content drifts.
//!
//! Until M2 introduces the corpus, a missing manifest is not an error.

use crate::util::workspace_root;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Verifies `corpus/MANIFEST.tsv` (`sha256<TAB-or-space>relative-path`
/// lines, `#` comments allowed) against the files on disk.
pub fn run() -> Result<(), String> {
    let manifest = workspace_root().join("corpus").join("MANIFEST.tsv");
    if !manifest.exists() {
        return Ok(()); // corpus lands in M2
    }
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("read manifest: {e}"))?;
    let mut checked = 0usize;
    let mut errors = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let Some(want) = fields.next() else {
            continue;
        };
        let Some(rel) = fields.next() else {
            errors.push(format!("malformed manifest line: {line}"));
            continue;
        };
        match digest(&workspace_root().join(rel)) {
            Ok(got) if got == want => checked += 1,
            Ok(got) => errors.push(format!("{rel}: sha256 {got} != pinned {want}")),
            Err(err) => errors.push(format!("{rel}: {err}")),
        }
    }
    if errors.is_empty() {
        eprintln!("[xtask] corpus: {checked} file(s) verified");
        Ok(())
    } else {
        Err(format!(
            "corpus verification failed ({} of {} ok):\n{}",
            checked,
            checked + errors.len(),
            errors.join("\n")
        ))
    }
}

fn digest(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(&bytes) {
        hex.push_str(&format!("{byte:02x}"));
    }
    Ok(hex)
}

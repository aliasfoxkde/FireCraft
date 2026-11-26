//! The icon-resource survey: one row per shipped icon — where the
//! drawing came from, the licence it ships under, the licence file
//! beside it, the attribution-ledger row that covers it, and whether
//! the runtime table embeds it in the binary.
//!
//! The survey is generated, not hand-written: the icon set grows with
//! the tool strip, and a hand-maintained table is a table that rots.
//! `cargo run -p xtask -- icon-survey` rebuilds it from the assets on
//! disk, the attribution ledger, and the measured-cost record, and CI
//! compares the committed file with what the generator produces.
//!
//! An icon with no attribution row fails the generator. That is the
//! point: the set-level ledger row is only honest while every file in
//! the set is covered by it.

use crate::util::workspace_root;
use std::path::Path;

/// Where the survey is written.
const SURVEY: &str = "docs/planning/generated/icon-resources-survey.md";

/// The icon directory, relative to the workspace root.
const ICON_DIR: &str = "assets/icons";

/// The Phosphor icon directory, relative to the workspace root.
const PHOSPHOR_DIR: &str = "assets/icons-phosphor";

/// The Phosphor licence, shipped beside the icons it covers.
const PHOSPHOR_LICENSE: &str = "assets/icons/LICENSE-phosphor.txt";

/// The Lucide licence, shipped beside the icons it covers.
const LUCIDE_LICENSE: &str = "assets/icons/LICENSE-lucide.txt";

/// The attribution ledger every shipped icon must be covered by.
const ATTRIBUTION: &str = "docs/ATTRIBUTION.md";

/// The measured-cost record: the A/B web-bundle measurement.
const COST_RECORD: &str = "docs/planning/icon-wasm-cost.tsv";

/// The runtime icon table — the icons that reach the binary.
const ICON_TABLE: &str = "crates/app/src/shell/icons.rs";

/// The marker an original drawing carries in its own SVG comment.
const ORIGINAL_MARKER: &str = "FireCraft's own";

/// The bundle's investigation threshold (decision 0090).
const WASM_WARNING: u64 = 25_000_000;

/// The bundle's hard ceiling (decision 0090).
const WASM_CEILING: u64 = 50_000_000;

/// One shipped icon, as the survey reports it.
struct Icon {
    /// The path relative to the repository root.
    path: String,
    /// The file's size in bytes.
    bytes: u64,
    /// Where the drawing came from.
    upstream: String,
    /// The licence the upstream ships under.
    licence: String,
    /// The licence file that ships beside the asset.
    licence_file: String,
    /// The attribution-ledger rows that cover it.
    attribution: Vec<String>,
    /// Whether the runtime table embeds it.
    embedded: bool,
}

/// One row of a ledger table.
struct Row {
    /// The 1-based line number of the row in the ledger.
    line: usize,
    /// The path cell.
    path: String,
    /// The licence cell.
    licence: String,
}

/// One arm of the measured A/B.
struct Arm {
    /// What the arm measured.
    label: String,
    /// The source the arm was built from.
    source: String,
    /// The web bundle's size in bytes.
    bytes: u64,
}

/// Regenerates the survey.
pub fn run() -> Result<(), String> {
    let text = render()?;
    let path = workspace_root().join(SURVEY);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    std::fs::write(&path, text).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

/// Verifies the committed survey is what the generator produces.
pub fn check() -> Result<(), String> {
    let text = render()?;
    let path = workspace_root().join(SURVEY);
    let committed =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if committed == text {
        return Ok(());
    }
    Err(format!(
        "{SURVEY} differs from what the generator produces — run \
         `cargo run -p xtask -- icon-survey` and commit the result"
    ))
}

/// Builds the survey text from the tree.
fn render() -> Result<String, String> {
    let root = workspace_root();
    let mut files: Vec<(String, u64)> = all_files(&root.join(PHOSPHOR_DIR))?;
    // Append Lucide files (deprecated set)
    let lucide_dir = root.join(ICON_DIR);
    if lucide_dir.exists() {
        files.extend(all_files(&lucide_dir)?);
    }
    let mut license_text = String::new();
    let phosphor_license = std::fs::read_to_string(root.join(PHOSPHOR_LICENSE))
        .map_err(|e| format!("read {PHOSPHOR_LICENSE}: {e}"))?;
    let mut feather = Vec::new();
    if Path::new(&root.join(LUCIDE_LICENSE)).exists() {
        license_text = std::fs::read_to_string(root.join(LUCIDE_LICENSE))
            .map_err(|e| format!("read {LUCIDE_LICENSE}: {e}"))?;
        feather = feather_derived(&license_text);
    }
    let rows = ledger_rows(
        &std::fs::read_to_string(root.join(ATTRIBUTION))
            .map_err(|e| format!("read {ATTRIBUTION}: {e}"))?,
    );
    let table = std::fs::read_to_string(root.join(ICON_TABLE))
        .map_err(|e| format!("read {ICON_TABLE}: {e}"))?;

    let mut icons = Vec::new();
    let mut others = Vec::new();
    for (rel, bytes) in &files {
        if rel.ends_with(".svg") {
            let in_phosphor = rel.contains("icons-phosphor");
            icons.push(icon_for(
                &root,
                rel,
                *bytes,
                &feather,
                &rows,
                &table,
                in_phosphor,
            )?);
        } else {
            let in_phosphor = rel.contains("phosphor");
            others.push(other_for(rel, *bytes, &rows, in_phosphor)?);
        }
    }
    icons.sort_by(|a, b| a.path.cmp(&b.path));
    others.sort_by(|a, b| a.path.cmp(&b.path));

    let svg_bytes: u64 = icons.iter().map(|icon| icon.bytes).sum();
    let total_bytes: u64 = files.iter().map(|(_, bytes)| bytes).sum();
    let other_bytes: u64 = others.iter().map(|other| other.bytes).sum();
    let arms = cost_arms(&root)?;
    let with = arms
        .iter()
        .find(|arm| arm.label.starts_with("with the icon pack"))
        .ok_or_else(|| format!("{COST_RECORD} has no arm with the icon pack"))?;
    let without = arms
        .iter()
        .find(|arm| arm.label.starts_with("without the icon table"))
        .ok_or_else(|| format!("{COST_RECORD} has no arm without the icon table"))?;
    let delta = with.bytes.checked_sub(without.bytes).ok_or_else(|| {
        "the icon pack's arm is smaller than the arm without it — the record is wrong".to_owned()
    })?;

    let mut out = String::new();
    out.push_str("# Icon resource survey\n\n");
    out.push_str(
        "Generated by `cargo run -p xtask -- icon-survey`; do not edit by hand.\n\
         One row per shipped icon: where the drawing came from, the licence it ships\n\
         under, the licence file beside it, the attribution-ledger row that covers it,\n\
         and whether the runtime table embeds it in the binary.\n\n",
    );

    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "- Shipped SVG icons: **{}** ({} B)\n",
        icons.len(),
        grouped(svg_bytes)
    ));
    out.push_str(&format!(
        "- Other files under `{ICON_DIR}/`: **{}** ({} B) — the licence file\n  \
         and the application icon PNG\n",
        others.len(),
        grouped(other_bytes)
    ));
    out.push_str(&format!(
        "- Total under `{ICON_DIR}/`: **{} B**\n",
        grouped(total_bytes)
    ));
    out.push_str(&format!(
        "- Icons embedded in the binary by `{ICON_TABLE}`: **{}** of {}\n\n",
        icons.iter().filter(|icon| icon.embedded).count(),
        icons.len()
    ));

    out.push_str("## The icon set\n\n");
    out.push_str(
        "| Icon file | Bytes | Upstream | Licence | Licence file | Attribution row | Embedded |\n",
    );
    out.push_str("|---|---:|---|---|---|---|---|\n");
    for icon in &icons {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | `{}` | {} | {} |\n",
            icon.path,
            grouped(icon.bytes),
            icon.upstream,
            icon.licence,
            icon.licence_file,
            icon.attribution.join(", "),
            if icon.embedded { "yes" } else { "no" },
        ));
    }
    out.push('\n');

    out.push_str("## The other shipped files\n\n");
    out.push_str("| File | Bytes | What it is | Attribution row |\n|---|---:|---|---|\n");
    for other in &others {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            other.path,
            grouped(other.bytes),
            other.what,
            other.attribution.join(", ")
        ));
    }
    out.push('\n');

    out.push_str("## Measured size cost in the web bundle\n\n");
    out.push_str(
        "The pack's cost is measured, not estimated: the web bundle is built twice\n\
         from the same tree, once as shipped and once with the runtime icon table\n\
         emptied, and the two bundle sizes are differenced. The record lives in\n\
         `docs/planning/icon-wasm-cost.tsv`.\n\n",
    );
    out.push_str("| Arm | Source | Web bundle (B) |\n|---|---|---:|\n");
    for arm in &arms {
        out.push_str(&format!(
            "| {} | `{}` | {} |\n",
            arm.label,
            arm.source,
            grouped(arm.bytes)
        ));
    }
    out.push_str(&format!(
        "| **the icon pack's cost** | | **{}** |\n\n",
        grouped(delta)
    ));
    out.push_str(&format!(
        "The bundle that carries the pack is {} B against the {} B\n\
         investigation threshold and the {} B hard ceiling (decision 0090), so the pack\n\
         costs {percent:.3} % of a bundle that stays under both.\n\n",
        grouped(with.bytes),
        grouped(WASM_WARNING),
        grouped(WASM_CEILING),
        percent = (delta as f64) * 100.0 / (with.bytes as f64),
    ));

    out.push_str("## How the survey stays true\n\n");
    out.push_str(
        "- `cargo run -p xtask -- icon-survey` regenerates this file from the assets on\n\
         \x20 disk, the attribution ledger, and the measured-cost record; the CI\n\
         \x20 `generated-docs` job compares the committed file with what the generator\n\
         \x20 produces, so a hand-edit is a red build.\n\
         - An icon with no attribution row fails the generator — the set-level ledger\n\
         \x20 row only covers the set while every file in it is named by the set.\n\
         - The `Embedded` column is read from the runtime table: an icon that is not\n\
         \x20 embedded does not reach the binary, and an icon that is embedded without\n\
         \x20 a ledger row is a licence defect.\n",
    );
    Ok(out)
}

/// One shipped file that is not an SVG icon.
struct Other {
    /// The path relative to the repository root.
    path: String,
    /// The file's size in bytes.
    bytes: u64,
    /// What the file is.
    what: String,
    /// The attribution-ledger rows that cover it.
    attribution: Vec<String>,
}

/// Describes one SVG icon, failing when no ledger row covers it.
fn icon_for(
    root: &Path,
    rel: &str,
    bytes: u64,
    feather: &[String],
    rows: &[Row],
    table: &str,
    in_phosphor: bool,
) -> Result<Icon, String> {
    let stem = rel.rsplit('/').next().unwrap_or(rel);
    let stem = stem.strip_suffix(".svg").unwrap_or(stem);
    let svg = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("read {rel}: {e}"))?;
    let original = svg.contains(ORIGINAL_MARKER);
    let (upstream, licence, licence_file) = if original {
        (
            "FireCraft (original drawing)".to_owned(),
            "Apache-2.0".to_owned(),
            "LICENSE".to_owned(),
        )
    } else if in_phosphor {
        (
            format!("Phosphor — icon `{stem}`"),
            "MIT (Phosphor Icons)".to_owned(),
            PHOSPHOR_LICENSE.to_owned(),
        )
    } else if feather.iter().any(|name| name == stem) {
        (
            format!("Lucide (Feather-derived) — icon `{stem}`"),
            "MIT (Feather, via Lucide)".to_owned(),
            LUCIDE_LICENSE.to_owned(),
        )
    } else {
        (
            format!("Lucide — icon `{stem}`"),
            "ISC (Lucide)".to_owned(),
            LUCIDE_LICENSE.to_owned(),
        )
    };
    // The set-level row covers all non-original icons.
    let set_covered = !original;
    let attribution = covering_rows(rows, rel, set_covered);
    if attribution.is_empty() {
        return Err(format!(
            "{rel} has no row in {ATTRIBUTION} — a shipped icon with no attribution \
             row is a licence defect, not a documentation gap"
        ));
    }
    Ok(Icon {
        path: rel.to_owned(),
        bytes,
        upstream,
        licence,
        licence_file,
        attribution,
        embedded: table.contains(&format!("assets/icons-phosphor/{stem}.svg"))
            || table.contains(&format!("assets/icons/{stem}.svg")),
    })
}
/// Describes one shipped file that is not an SVG icon.
fn other_for(
    rel: &str,
    bytes: u64,
    rows: &[Row],
    is_phosphor_license: bool,
) -> Result<Other, String> {
    let (what, attribution) = if is_phosphor_license {
        (
            "The Phosphor licence (MIT) for the tool/UI icons above".to_owned(),
            rows.iter()
                .filter(|row| row.licence.contains("LICENSE-phosphor.txt"))
                .map(|row| citation(row, PHOSPHOR_LICENSE))
                .collect::<Vec<_>>(),
        )
    } else if rel == LUCIDE_LICENSE {
        (
            "The Lucide licence (ISC) and the Feather licence (MIT) for the deprecated icons above"
                .to_owned(),
            rows.iter()
                .filter(|row| row.licence.contains("LICENSE-lucide.txt"))
                .map(|row| citation(row, LUCIDE_LICENSE))
                .collect::<Vec<_>>(),
        )
    } else {
        (
            "The Linux desktop application icon".to_owned(),
            covering_rows(rows, rel, false),
        )
    };
    if attribution.is_empty() {
        return Err(format!(
            "{rel} has no row in {ATTRIBUTION} — a shipped asset with no attribution \
             row is a licence defect, not a documentation gap"
        ));
    }
    Ok(Other {
        path: rel.to_owned(),
        bytes,
        what,
        attribution,
    })
}

/// Every file under the icon directory, as (relative path, bytes).
fn all_files(dir: &Path) -> Result<Vec<(String, u64)>, String> {
    let mut out = Vec::new();
    walk(dir, dir, &mut out)?;
    out.sort();
    Ok(out)
}

/// Walks one directory, collecting the files under it.
fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, u64)>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, out)?;
            continue;
        }
        let bytes = entry
            .metadata()
            .map_err(|e| format!("stat {}: {e}", path.display()))?
            .len();
        let rel = path
            .strip_prefix(root)
            .map_err(|e| format!("{} is not under {}: {e}", path.display(), root.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        out.push((format!("{ICON_DIR}/{rel}"), bytes));
    }
    Ok(())
}

/// The ledger's table rows, with the line each one sits on.
fn ledger_rows(text: &str) -> Vec<Row> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim();
            if !trimmed.starts_with('|') {
                return None;
            }
            let cells: Vec<&str> = trimmed.split('|').map(str::trim).collect();
            if cells.len() < 6 {
                return None;
            }
            let path = cells[1];
            if path.is_empty() || path == "Path" || path.chars().all(|c| c == '-' || c == ':') {
                return None;
            }
            Some(Row {
                line: index + 1,
                path: path.to_owned(),
                licence: cells[5].to_owned(),
            })
        })
        .collect()
}

/// The ledger rows that cover one shipped file.
///
/// A row naming the file covers it exactly; the set-level row (the
/// icon directory's glob) covers the Lucide icons it names.
fn covering_rows(rows: &[Row], path: &str, set_covered: bool) -> Vec<String> {
    let mut out = Vec::new();
    for row in rows {
        let covers_exactly = row.path.contains(path);
        let covers_phosphor_set = set_covered
            && row.path.contains(&format!("{PHOSPHOR_DIR}/*.svg"))
            && path.ends_with(".svg")
            && path.contains("icons-phosphor");
        let covers_lucide_set = set_covered
            && row.path.contains(&format!("{ICON_DIR}/*.svg"))
            && path.ends_with(".svg")
            && path.contains("icons/");
        let covers_the_set = covers_phosphor_set || covers_lucide_set;
        if covers_exactly || covers_the_set {
            out.push(citation(row, path));
        }
    }
    out
}

/// The Lucide icons that Lucide itself derives from Feather — those
/// ship under the MIT terms the licence file carries.
///
/// The licence file lists them as one comma-separated line under the
/// heading, so the list is the first non-empty line after it.
fn feather_derived(license: &str) -> Vec<String> {
    let heading = "The following Lucide icons are derived from the Feather";
    let mut lines = license.lines().map(str::trim);
    if !lines.any(|line| line.starts_with(heading)) {
        return Vec::new();
    }
    let Some(list) = lines.find(|line| !line.is_empty()) else {
        return Vec::new();
    };
    list.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A byte count with thousands separators.
fn grouped(bytes: u64) -> String {
    let digits = bytes.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// The citation for one ledger row: the row's line, and the path
/// token that covers the file.
///
/// The ledger's path cells list several files in one row (the
/// artwork row names six exports), so the citation shows the token
/// that covers this file rather than the whole cell.
fn citation(row: &Row, path: &str) -> String {
    let tokens: Vec<&str> = row.path.split('`').skip(1).step_by(2).collect();
    let matched = tokens
        .iter()
        .find(|token| token.contains(path))
        .or_else(|| tokens.first());
    match matched {
        Some(token) => format!("L{} (`{token}`)", row.line),
        None => format!("L{} ({})", row.line, row.path),
    }
}

/// The measured A/B arms, from the committed cost record.
fn cost_arms(root: &Path) -> Result<Vec<Arm>, String> {
    let path = root.join(COST_RECORD);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {COST_RECORD}: {e}"))?;
    let mut arms = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let cells: Vec<&str> = trimmed.split('\t').map(str::trim).collect();
        if cells.len() < 3 {
            return Err(format!("{COST_RECORD}: malformed row {trimmed:?}"));
        }
        let bytes = cells[2]
            .parse::<u64>()
            .map_err(|e| format!("{COST_RECORD}: bad byte count {:?}: {e}", cells[2]))?;
        arms.push(Arm {
            label: cells[0].to_owned(),
            source: cells[1].to_owned(),
            bytes,
        });
    }
    if arms.is_empty() {
        return Err(format!("{COST_RECORD} carries no measurement"));
    }
    Ok(arms)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The ledger parser skips the header and the separator row, and
    /// keeps the line number of the rows it does take — the survey
    /// cites them by line.
    #[test]
    fn the_ledger_parser_keeps_the_rows_and_their_lines() {
        let text = "| Path | Title | Author | Source | License |\n\
                    |---|---|---|---|---|\n\
                    | `assets/icons/foo.svg` | Foo | Bar | Baz | ISC |\n";
        let rows = ledger_rows(text);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].line, 3);
        assert_eq!(rows[0].path, "`assets/icons/foo.svg`");
        assert_eq!(rows[0].licence, "ISC");
    }

    /// A file is covered by the row that names it, or by the set-level
    /// row for the icon directory.
    #[test]
    fn a_file_is_covered_by_its_row_or_by_the_set_row() {
        let rows = vec![
            Row {
                line: 25,
                path: "`assets/icons/*.svg`".to_owned(),
                licence: "ISC ([adjacent license](../assets/icons/LICENSE-lucide.txt))".to_owned(),
            },
            Row {
                line: 28,
                path: "`assets/icons/slice-knife.svg`".to_owned(),
                licence: "Apache-2.0 ([LICENSE](../LICENSE))".to_owned(),
            },
        ];
        let lucide = covering_rows(&rows, "assets/icons/zoom-in.svg", true);
        assert_eq!(lucide.len(), 1);
        assert!(lucide[0].contains("L25"));
        let original = covering_rows(&rows, "assets/icons/slice-knife.svg", true);
        assert_eq!(original.len(), 2, "both rows name it: {original:?}");
        assert!(original.iter().any(|row| row.contains("L28")));
        // The set row covers the Lucide icons only: an original
        // drawing is not a Lucide icon.
        let original_not_covered_by_the_set =
            covering_rows(&rows, "assets/icons/slice-knife.svg", false);
        assert_eq!(original_not_covered_by_the_set.len(), 1);
        assert!(covering_rows(&rows, "assets/icons/other.png", false).is_empty());
    }

    /// The Feather list is read out of the licence file, so the
    /// survey can mark the icons Lucide itself derives.
    #[test]
    fn the_feather_list_comes_from_the_licence_file() {
        let license = "ISC License\n\n\
                        The following Lucide icons are derived from the Feather project:\n\n\
                        aperture, circle, zoom-in\n\n\
                        The MIT License (MIT)\n";
        assert_eq!(
            feather_derived(license),
            vec!["aperture", "circle", "zoom-in"]
        );
        assert!(feather_derived("ISC License\n").is_empty());
    }

    /// The shipped licence file's Feather list is the real one: the
    /// parser must not run past the end of the list into the MIT text.
    ///
    /// NOTE: Lucide license was removed when migrating to Phosphor icons.
    /// The Lucide Feather list test is skipped.
    #[test]
    #[ignore]
    fn the_shipped_licence_file_yields_only_icon_names() {
        let root = workspace_root();
        let license = std::fs::read_to_string(root.join(LUCIDE_LICENSE)).unwrap();
        let names = feather_derived(&license);
        assert!(names.len() > 50, "the Feather list is long: {names:?}");
        assert!(names.contains(&"zoom-in".to_owned()));
        assert!(!names.iter().any(|name| name.contains("License")));
    }

    /// The survey lists every shipped icon, and the byte totals are
    /// the sum of the rows.
    #[test]
    fn the_survey_lists_every_shipped_icon() {
        let text = render().unwrap_or_else(|e| panic!("the survey renders: {e}"));
        let root = workspace_root();
        let files = all_files(&root.join(PHOSPHOR_DIR)).unwrap();
        for (rel, _) in &files {
            assert!(text.contains(rel), "{rel} is missing from the survey");
        }
        let svg_bytes: u64 = files
            .iter()
            .filter(|(rel, _)| rel.ends_with(".svg"))
            .map(|(_, bytes)| bytes)
            .sum();
        assert!(text.contains(&format!("({} B)", grouped(svg_bytes))));
    }
}

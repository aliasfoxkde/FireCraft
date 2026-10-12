//! Web-export structural golden loop (M11 task 6 stopping condition).
//!
//! For every corpus fixture carrying web-layer objects, the gate runs
//! the two template-shaped workflows (HTML+Images, CSS+Images), strips
//! each text artifact down to its *structure*, and pins the digest of
//! that normalized stream next to the fixture as `<name>.webpin`.
//!
//! Normalization is what makes the comparison CS6-meaningful: Adobe's
//! whitespace, quoting, and entity choices are noise (decision 0017),
//! so the gate compares the template-derived skeleton — tags in
//! document order, attribute *names* sorted per tag, CSS selectors
//! with property names in source order, values elided — plus the
//! emitted file-name set (auto-naming is structure: `name_rX_cY`).
//! The floor is exact (distance 0): the pins record the shape the
//! emitter committed to, and `xtask web-golden --measure` prints the
//! normalized streams for diffing when an intentional change needs a
//! re-bless.

use crate::corpus::manifest_entries;
use crate::util::{inflate, sha256_hex, workspace_root};
use fwpng::chunk::parse_chunks;
use std::path::Path;

const FIXTURE_PREFIX: &str = "corpus/fixtures/";

/// One pinned artifact: kind tag (html | css) plus the normalized
/// digest. A pin line reads `html <sha256>`.
struct PinLine {
    kind: &'static str,
    digest: String,
}

/// Gate entry point.
pub fn run() -> Result<(), String> {
    run_inner(false, false)
}

/// `xtask web-golden --bless`: re-export and rewrite every web pin.
pub fn bless() -> Result<(), String> {
    run_inner(true, false)
}

fn run_inner(write_pins: bool, measure: bool) -> Result<(), String> {
    let mut compared = 0usize;
    let mut errors = Vec::new();
    for (_, rel) in manifest_entries()? {
        if !rel.starts_with(FIXTURE_PREFIX) {
            continue;
        }
        let bytes = match std::fs::read(workspace_root().join(&rel)) {
            Ok(b) => b,
            Err(err) => {
                errors.push(format!("{rel}: {err}"));
                continue;
            }
        };
        let Some(document) = parse_document(&bytes) else {
            continue; // not a web fixture; the other gates own it
        };
        if !has_web_objects(&document) {
            continue;
        }
        let name = rel[FIXTURE_PREFIX.len()..].to_owned();

        // The two template-shaped workflows carry every structural
        // face of the exporter: the shim table + map + behavior hooks
        // (SLICES.XTT) and the absolute-positioned divs (CssLayers).
        let table = match webexport::export(
            &document,
            &webexport::WebExportOptions::default(),
            webexport::Workflow::HtmlAndImages,
        ) {
            Ok(out) => out,
            Err(err) => {
                errors.push(format!("{name}: html+images: {err}"));
                continue;
            }
        };
        let layers = match webexport::export(
            &document,
            &webexport::WebExportOptions::default(),
            webexport::Workflow::CssAndImages,
        ) {
            Ok(out) => out,
            Err(err) => {
                errors.push(format!("{name}: css+images: {err}"));
                continue;
            }
        };

        let mut pins = Vec::new();
        let mut streams = Vec::new();
        let table_html = table.html.as_deref().unwrap_or_default();
        let layers_html = layers.html.as_deref().unwrap_or_default();
        let table_css = css_file(&table);
        let layers_css = css_file(&layers);
        for (kind, text) in [
            ("html", table_html),
            ("html", layers_html),
            ("css", table_css.as_str()),
            ("css", layers_css.as_str()),
        ] {
            if text.is_empty() {
                continue;
            }
            let normalized = if kind == "html" {
                normalize_html(text)
            } else {
                normalize_css(text)
            };
            streams.push(format!("== {kind} ==\n{normalized}"));
            pins.push(PinLine {
                kind,
                digest: sha256_hex(normalized.as_bytes()),
            });
        }
        // The file-name set is structure too: it pins auto-naming
        // (`name_rX_cY` and friends) at the gate level.
        let mut names: Vec<&str> = table.files.iter().map(|f| f.name.as_str()).collect();
        names.sort_unstable();
        let nameline = format!("files {}", names.join(" "));
        streams.push(format!("== files ==\n{nameline}"));
        pins.push(PinLine {
            kind: "files",
            digest: sha256_hex(nameline.as_bytes()),
        });

        let pin = Path::new(&rel).with_file_name(format!("{name}.webpin"));
        let pin_path = workspace_root().join(&pin);
        let want = pins
            .iter()
            .map(|p| format!("{} {}\n", p.kind, p.digest))
            .collect::<String>();
        if write_pins {
            if let Err(err) = std::fs::write(&pin_path, &want) {
                errors.push(format!("{}: write pin: {err}", pin.display()));
            }
        } else {
            match std::fs::read_to_string(&pin_path) {
                Ok(pinned) if pinned == want => {}
                Ok(_) => {
                    // Distance is 0 or the fixture diverged; the
                    // measure verb prints the streams to diff.
                    errors.push(format!(
                        "{name}: web structure diverged from the pin — \
                         diff with `cargo run -p xtask -- web-golden --measure`, \
                         re-bless if intentional"
                    ));
                }
                Err(_) => errors.push(format!(
                    "{}: missing web pin — run `cargo run -p xtask -- web-golden --bless`",
                    pin.display()
                )),
            }
        }
        if measure {
            for stream in &streams {
                eprintln!("[xtask] web-golden {name}: {stream}");
            }
        }
        compared += 1;
    }

    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    if compared == 0 {
        return Err("web-golden: no fixture was eligible — the loop pins nothing".to_owned());
    }
    eprintln!("[xtask] web-golden: {compared} fixture(s) pinned at structural distance 0");
    Ok(())
}

/// `xtask web-golden --measure`: print every normalized stream so a
/// divergence can be diffed by hand before a re-bless.
pub fn measure() -> Result<(), String> {
    run_inner(false, true)
}

/// The css artifact of an export, when the workflow emitted one.
fn css_file(export: &webexport::WebExport) -> String {
    export
        .files
        .iter()
        .find(|file| file.name.ends_with(".css"))
        .map(|file| String::from_utf8_lossy(&file.bytes).into_owned())
        .unwrap_or_default()
}

/// Does the document carry web-layer objects (the fixture is a web
/// fixture iff it does)?
fn has_web_objects(document: &doc::Document) -> bool {
    document.pages.iter().any(|page| {
        page.layers
            .layers
            .iter()
            .filter(|layer| layer.has_url && layer.is_special)
            .any(|layer| layer.cells.iter().any(|cell| !cell.elements.is_empty()))
    })
}

/// Parses a fixture into a `Document` (same read path as the other
/// golden loops). `None` when the file carries no node tree.
fn parse_document(bytes: &[u8]) -> Option<doc::Document> {
    let chunks = parse_chunks(bytes).ok()?;
    let chunk = chunks.iter().find(|c| &c.ctype == b"mkTS")?;
    let inflated = inflate(&chunk.data).ok()?;
    let nodes = fwpng::nodetree::parse(&inflated).ok()?;
    let (document, _bag) = doc::fwmap::read(&nodes).ok()?;
    (!document.pages.is_empty()).then_some(document)
}

/// HTML → structure: tags in document order with sorted attribute
/// names, doctype/comments/text elided. `<style>` bodies are *not*
/// elided — they carry the CSS+Images rules — so their content is
/// re-normalized through [`normalize_css`]. `<script>` bodies stay
/// elided (the MM runtime is verbatim boilerplate, not structure).
/// `<td colspan="2" class="x">` → `td(class,colspan)`.
fn normalize_html(text: &str) -> String {
    let mut out = String::new();
    let mut style_body = String::new();
    let mut in_style = false;
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            if in_style {
                style_body.push_str(&text[i..i + 1]);
            }
            i += 1;
            continue;
        }
        if text[i..].starts_with("<!--") {
            i = text[i..].find("-->").map_or(text.len(), |at| i + at + 3);
            continue;
        }
        let Some(end) = text[i..].find('>') else {
            break;
        };
        let tag = &text[i + 1..i + end];
        i += end + 1;
        let tag = tag.trim();
        if tag.starts_with('!') || tag.starts_with('?') {
            continue; // doctype / declaration
        }
        if let Some(closing) = tag.strip_prefix('/') {
            let closing = closing.trim().to_ascii_lowercase();
            if closing == "style" {
                in_style = false;
                out.push_str(&normalize_css(&style_body));
                style_body.clear();
            }
            out.push_str(&format!("/{closing}/"));
            continue;
        }
        let mut tokens = tag.split_whitespace();
        let Some(name) = tokens.next() else { continue };
        let name = name.to_ascii_lowercase();
        if name == "style" {
            in_style = true;
        }
        let mut attrs: Vec<&str> = tokens
            .filter_map(|token| token.split('=').next())
            .map(str::trim)
            .filter(|attr| !attr.is_empty())
            .collect();
        attrs.sort_unstable();
        out.push_str(&format!("{}({})", name, attrs.join(",")));
    }
    out
}

/// CSS → structure: selectors with property names (values elided),
/// at-statements kept as written. One `sel{a,b,c}` per block; the
/// stream stays single-line (whitespace runs collapse), so an
/// artifact's normalized form diffs line-for-line.
fn normalize_css(text: &str) -> String {
    let mut out = String::new();
    for block in text.split('}') {
        let Some((head, body)) = block.split_once('{') else {
            emit_at_statements(block, &mut out);
            continue;
        };
        // A leading at-statement (`@charset "utf-8";` before the
        // first rule) shares the head with the block's selector.
        for part in head.split(';') {
            let part = part.split_whitespace().collect::<Vec<_>>().join(" ");
            if part.is_empty() {
                continue;
            }
            if let Some(statement) = part.strip_prefix('@') {
                out.push_str(&format!("@{statement};"));
                continue;
            }
            let mut props: Vec<&str> = body
                .split(';')
                .filter_map(|decl| decl.split(':').next())
                .map(str::trim)
                .filter(|prop| !prop.is_empty())
                .collect();
            props.sort_unstable();
            out.push_str(&format!("{part}{{{}}}", props.join(",")));
        }
    }
    out
}

/// The brace-less tail of a stylesheet: `@charset` / `@import`
/// statements (anything else out there is not structure).
fn emit_at_statements(text: &str, out: &mut String) {
    for statement in text.split(';') {
        let statement = statement.trim();
        if statement.starts_with('@') {
            out.push_str(&format!("{statement};"));
        }
    }
}

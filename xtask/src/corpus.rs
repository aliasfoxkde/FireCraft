//! Corpus gate: pins deterministic fixtures and verifies them.
//!
//! Two kinds of corpus entries live in `corpus/fixtures/`:
//!
//! - **Generated fixtures** (committed): built by [`gen`] with our own
//!   writers — clean-room bytes, no Adobe IP. They pin current parse
//!   behavior as regression anchors and exercise the round-trip floor.
//! - **Real Fireworks files** (never committed): dropped into
//!   `corpus/incoming/` by the operator; the manifest may pin their
//!   digests, the files stay out of the repo.
//!
//! `MANIFEST.tsv` lines: `<sha256> <relative-path> [# note]`. The gate
//! verifies digests, then runs the round-trip battery on every entry and
//! fails when the pass rate drops below [`ROUNDTRIP_FLOOR`].

use crate::util::workspace_root;
use fwpng::chunk::{is_fireworks, parse_chunks, write_chunk};
use sha2::{Digest, Sha256};
use std::io::{Read as _, Write as _};
use std::path::Path;

/// Minimum share (percent) of corpus entries that must pass the
/// round-trip battery. Rising-only: raise as fidelity grows, never lower.
pub const ROUNDTRIP_FLOOR: usize = 100;

const FIXTURE_DIR: &str = "corpus/fixtures";
const MANIFEST: &str = "corpus/MANIFEST.tsv";

/// Gate entry point: verify digests + round-trip battery.
pub fn run() -> Result<(), String> {
    verify(false)
}

/// `xtask corpus --report`: per-entry detail on stderr.
pub fn report() -> Result<(), String> {
    verify(true)
}

fn verify(verbose: bool) -> Result<(), String> {
    let manifest = workspace_root().join(MANIFEST);
    if !manifest.exists() {
        return Err(format!(
            "{MANIFEST} is missing; run `cargo run -p xtask -- corpus-gen`"
        ));
    }
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("read manifest: {e}"))?;
    let mut checked = 0usize;
    let mut passed = 0usize;
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
        let bytes = match std::fs::read(workspace_root().join(rel)) {
            Ok(b) => b,
            Err(err) => {
                errors.push(format!("{rel}: {err}"));
                continue;
            }
        };
        let got = hex(&Sha256::digest(&bytes));
        if got != want {
            errors.push(format!("{rel}: sha256 {got} != pinned {want}"));
            continue;
        }
        checked += 1;
        let name = Path::new(rel)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(rel);
        // The filename carries the detection contract: `*.fw.png` must be
        // recognized as Fireworks, plain `.png` must not.
        let checks = round_trip_checks(&bytes, rel.ends_with(".fw.png"));
        let failed: Vec<&String> = checks
            .iter()
            .filter(|(_, ok)| !ok)
            .map(|(m, _)| m)
            .collect();
        if failed.is_empty() {
            passed += 1;
            if verbose {
                eprintln!("[xtask] corpus: {name}: {} check(s) ok", checks.len());
            }
        } else {
            for msg in failed {
                errors.push(format!("{name}: {msg}"));
            }
        }
    }
    let floor_met = checked > 0 && passed * 100 >= checked * ROUNDTRIP_FLOOR;
    if errors.is_empty() && floor_met {
        eprintln!(
            "[xtask] corpus: {checked} file(s) verified, round-trip {passed}/{checked} (floor {ROUNDTRIP_FLOOR}%)"
        );
        Ok(())
    } else {
        if !floor_met {
            errors.push(format!(
                "round-trip floor not met: {passed}/{checked} below {ROUNDTRIP_FLOOR}%"
            ));
        }
        Err(errors.join("\n"))
    }
}

/// Round-trip battery for one file: what must hold for a clean
/// parse -> model -> re-emit cycle. Returns (check, passed) pairs.
fn round_trip_checks(bytes: &[u8], expect_fireworks: bool) -> Vec<(String, bool)> {
    let mut checks = Vec::new();
    let chunks = match parse_chunks(bytes) {
        Ok(c) => c,
        Err(err) => {
            checks.push((format!("parse_chunks: {err}"), false));
            return checks;
        }
    };
    let fw = is_fireworks(&chunks) == expect_fireworks;
    checks.push((format!("fireworks detection = {expect_fireworks}"), fw));

    // Every node-tree payload must canonically re-serialize to its own
    // inflated bytes.
    for ctype in [b"mkTS", b"mkBS"] {
        for chunk in chunks.iter().filter(|c| &c.ctype == ctype) {
            let label = String::from_utf8_lossy(ctype).into_owned();
            match inflate(&chunk.data) {
                Ok(inflated) => match fwpng::nodetree::parse(&inflated) {
                    Ok(nodes) => {
                        let re = fwpng::nodetree::serialize(&nodes);
                        checks.push((format!("{label} canonical re-emit"), re == inflated));
                    }
                    Err(err) => checks.push((format!("{label} parse: {err}"), false)),
                },
                Err(err) => checks.push((format!("{label} inflate: {err}"), false)),
            }
        }
    }
    // Tiles decode with exact stream size (only when tiles are present).
    if chunks.iter().any(|c| &c.ctype == b"mkBT") {
        match fwpng::parse_mkbt_chunks(&chunks) {
            Ok(tiles) => checks.push((format!("{} tile(s) decoded", tiles.len()), true)),
            Err(err) => checks.push((format!("mkBT decode: {err}"), false)),
        }
    }
    // Thumbnail payload parses.
    for chunk in chunks.iter().filter(|c| &c.ctype == b"prVW") {
        match fwpng::thumbnail::parse_prvw(&chunk.data) {
            Ok(_) => checks.push(("prVW parses".to_string(), true)),
            Err(err) => checks.push((format!("prVW parse: {err}"), false)),
        }
    }
    checks
}

fn inflate(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(data)
        .read_to_end(&mut out)
        .map_err(|e| format!("zlib inflate failed: {e}"))?;
    Ok(out)
}

/// Regenerates every fixture deterministically and rewrites the manifest
/// with fresh digests.
pub fn gen_fixtures() -> Result<(), String> {
    let dir = workspace_root().join(FIXTURE_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;

    /// One pending fixture: name, built bytes, manifest note.
    type Fixture = (&'static str, Result<Vec<u8>, String>, &'static str);
    let fixtures: [Fixture; 4] = [
        (
            "minimal.fw.png",
            build_minimal(),
            "generated: standard 8x8 RGBA + prVW + mkTS/mkBS (same tree) + mkBF + mkBT tile 0",
        ),
        (
            "tiles-split.fw.png",
            build_tiles_split(),
            "generated: mkBT tile 0 split across two chunks, tile 1 single",
        ),
        (
            "legacy-drift.fw.png",
            build_legacy_drift(),
            "generated: mkBS deliberately diverges from mkTS (comparison-helper target)",
        ),
        (
            "plain.png",
            build_plain(),
            "generated: control, no private chunks (is_fireworks must be false)",
        ),
    ];

    let mut manifest = String::from(
        "# FireCraft corpus — digests pinned by `cargo run -p xtask -- corpus-gen`.\n\
         # fixtures/ are clean-room generated (committed); incoming/ holds real\n\
         # Fireworks files, which stay uncommitted.\n",
    );
    let fixture_count = fixtures.len();
    for (name, built, note) in fixtures {
        let bytes = built?;
        std::fs::write(dir.join(name), &bytes).map_err(|e| format!("write {name}: {e}"))?;
        manifest.push_str(&format!(
            "{} {FIXTURE_DIR}/{name} # {}\n",
            hex(&Sha256::digest(&bytes)),
            note
        ));
    }
    std::fs::write(workspace_root().join(MANIFEST), manifest)
        .map_err(|e| format!("write manifest: {e}"))?;
    eprintln!(
        "[xtask] corpus-gen: {fixture_count} fixture(s) written to {FIXTURE_DIR}/, manifest updated"
    );
    Ok(())
}

/// Standard 8x8 RGBA PNG with the full private-chunk set (single-span
/// tile).
fn build_minimal() -> Result<Vec<u8>, String> {
    let standard = encode_rgba8(8, 8, |x, y| [x as u8 * 32, y as u8 * 32, 0x40, 255])?;
    insert_before_iend(
        &standard,
        &[
            (b"prVW", zlib(&prvw_payload())?),
            (b"mkBF", vec![0u8; 28]),
            (
                b"mkTS",
                zlib(b"MKBv{FRCi{1}XLCi{0}LYLv{LAYv{BKGb{0}}LYLv{LAYv{BKGb{1}}}}}")?,
            ),
            (
                b"mkBS",
                zlib(b"MKBv{FRCi{1}XLCi{0}LYLv{LAYv{BKGb{0}}LYLv{LAYv{BKGb{1}}}}}")?,
            ),
            (b"mkBT", mkbt_payload(0, false)?),
        ],
    )
}

/// mkTS + a tile-0 stream split across two consecutive mkBT chunks +
/// tile 1 in one chunk.
fn build_tiles_split() -> Result<Vec<u8>, String> {
    let standard = encode_rgba8(4, 4, |_, _| [10, 20, 30, 255])?;
    let payload = mkbt_payload(0, false)?;
    let split = MKBT_SPLIT_AT.min(payload.len());
    insert_before_iend(
        &standard,
        &[
            (b"mkTS", zlib(b"MKBv{FRCi{1}}")?),
            (b"mkBT", payload[..split].to_vec()),
            (b"mkBT", payload[split..].to_vec()),
            (b"mkBT", mkbt_payload(1, false)?),
        ],
    )
}

/// mkTS and mkBS deliberately differ (legacy drift case).
fn build_legacy_drift() -> Result<Vec<u8>, String> {
    let standard = encode_rgba8(2, 2, |_, _| [200, 100, 50, 255])?;
    insert_before_iend(
        &standard,
        &[
            (b"mkTS", zlib(b"MKBv{FRCi{1}XLCi{0}}")?),
            (b"mkBS", zlib(b"MKBv{FRCi{2}}")?),
        ],
    )
}

/// Control: plain PNG, no private chunks.
fn build_plain() -> Result<Vec<u8>, String> {
    encode_rgba8(3, 3, |x, y| {
        [255, 255, 255, if (x + y) % 2 == 0 { 255 } else { 0 }]
    })
}

/// Where the tile-0 stream is cut in `tiles-split.fw.png` (inside the
/// zlib payload, well past the 76-byte header).
const MKBT_SPLIT_AT: usize = 96;

/// Parses a standard PNG we just encoded and re-emits it with `extra`
/// private chunks inserted before IEND.
fn insert_before_iend(standard: &[u8], extra: &[(&[u8; 4], Vec<u8>)]) -> Result<Vec<u8>, String> {
    let chunks = parse_chunks(standard).map_err(|e| format!("re-parse own encoder output: {e}"))?;
    let mut file = fwpng::chunk::PNG_SIGNATURE.to_vec();
    for chunk in &chunks {
        if &chunk.ctype == b"IEND" {
            for (ctype, data) in extra {
                write_chunk(&mut file, ctype, data);
            }
        }
        write_chunk(&mut file, &chunk.ctype, &chunk.data);
    }
    Ok(file)
}

fn encode_rgba8(
    width: u32,
    height: u32,
    px: impl Fn(u32, u32) -> [u8; 4],
) -> Result<Vec<u8>, String> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&px(x, y));
        }
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("png header: {e}"))?;
        writer
            .write_image_data(&pixels)
            .map_err(|e| format!("png pixels: {e}"))?;
    }
    Ok(out)
}

/// prVW: CA FE BE EF magic + 128x128 display size + two palette entries
/// + 00 00 00 terminator + deterministic indices.
fn prvw_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&[0xCA, 0xFE, 0xBE, 0xEF]);
    payload.extend_from_slice(&128u16.to_be_bytes());
    payload.extend_from_slice(&128u16.to_be_bytes());
    payload.extend_from_slice(&[255, 0, 0]);
    payload.extend_from_slice(&[0, 255, 0]);
    payload.extend_from_slice(&[0, 0, 0]);
    for i in 0..fwpng::thumbnail::THUMB_BYTES {
        payload.push((i % 2) as u8);
    }
    payload
}

/// mkBT: FA CE CA FE magic + 24-bit index + reserved + grayscale flag +
/// 64 reserved bytes + zlib of a deterministic 64 KiB pixel span.
fn mkbt_payload(index: u32, grayscale: bool) -> Result<Vec<u8>, String> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&[0xFA, 0xCE, 0xCA, 0xFE]);
    payload.extend_from_slice(&(index << 8).to_be_bytes());
    payload.extend_from_slice(&[0, 0, 0]);
    payload.push(u8::from(grayscale));
    payload.extend_from_slice(&[0u8; 64]);
    let raw: Vec<u8> = (0..fwpng::tile::TILE_STREAM_BYTES)
        .map(|i| (i % 251) as u8)
        .collect();
    payload.extend_from_slice(&zlib(&raw)?);
    Ok(payload)
}

fn zlib(raw: &[u8]) -> Result<Vec<u8>, String> {
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(raw).map_err(|e| format!("zlib write: {e}"))?;
    enc.finish().map_err(|e| format!("zlib finish: {e}"))
}

fn hex(digest: &[u8]) -> String {
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn generated_fixtures_pass_their_own_battery() {
        for (name, built) in [
            ("minimal", build_minimal()),
            ("tiles-split", build_tiles_split()),
            ("legacy-drift", build_legacy_drift()),
        ] {
            let checks = round_trip_checks(&built.unwrap(), true);
            assert!(checks.iter().all(|(_, ok)| *ok), "{name}: {checks:?}");
        }
        // Control: detection must be false, and only the image-level
        // checks apply.
        let checks = round_trip_checks(&build_plain().unwrap(), false);
        assert!(checks.iter().all(|(_, ok)| *ok), "{checks:?}");
    }

    #[test]
    fn hex_is_lowercase_and_padded() {
        assert_eq!(hex(&[0x0a, 0xff]), "0aff");
    }
}

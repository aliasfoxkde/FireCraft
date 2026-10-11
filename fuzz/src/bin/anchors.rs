//! Deterministic bounded fuzz driver (stable toolchain).
//!
//! Runs every anchor over a fixed seed corpus mutated by a seeded xorshift
//! PRNG for a bounded number of iterations, then exits 0 — or panics, if
//! an anchor panics (which is the failure signal `xtask ci` gates on).
//! Deterministic: same binary + same args = same inputs every run, so a
//! failure reproduces exactly.
//!
//! Usage: `anchors [iterations-per-anchor] [base-seed-u64]`

use std::io::Write as _;

fn main() {
    let mut args = std::env::args().skip(1);
    let iterations: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(2048);
    let base_seed: u64 = args
        .next()
        .and_then(|a| a.parse().ok())
        .unwrap_or(0x5EED_F00D_D1CE_C0DE);

    let seeds = seeds();
    for (name, anchor, seed_inputs) in anchors(&seeds) {
        let mut rng = Rng(base_seed ^ name_hash(name));
        for seed_input in seed_inputs.iter().cycle().take(iterations) {
            let input = mutate(&mut rng, seed_input);
            anchor(&input);
        }
        eprintln!("[anchors] {name}: {iterations} iterations ok");
    }
    eprintln!("[anchors] all anchors clean");
}

/// Anchor name, entry point, and the static seeds it starts from.
type AnchorSet = Vec<(&'static str, fn(&[u8]), Vec<Vec<u8>>)>;

fn anchors(seeds: &Seeds) -> AnchorSet {
    vec![
        (
            "fwpng_chunks",
            firecraft_fuzz::anchor_fwpng_chunks,
            vec![seeds.minimal_png.clone(), seeds.empty.clone()],
        ),
        (
            "fwpng_nodetree",
            firecraft_fuzz::anchor_fwpng_nodetree,
            vec![seeds.canonical_tree.clone(), seeds.nested_tree.clone()],
        ),
        (
            "fwpng_prvw",
            firecraft_fuzz::anchor_fwpng_prvw,
            vec![seeds.prvw.clone()],
        ),
        (
            "fwpng_mkbt",
            firecraft_fuzz::anchor_fwpng_mkbt,
            vec![seeds.mkbt.clone()],
        ),
        (
            "doc_read",
            firecraft_fuzz::anchor_doc_read,
            vec![seeds.canonical_tree.clone(), seeds.nested_tree.clone()],
        ),
        (
            "vector_ops",
            firecraft_fuzz::anchor_vector_ops,
            vec![seeds.empty.clone()],
        ),
        (
            "geom_transform",
            firecraft_fuzz::anchor_geom_transform,
            vec![seeds.float_blob.clone()],
        ),
        (
            "svgimport",
            firecraft_fuzz::anchor_svgimport,
            vec![seeds.minimal_svg.clone(), seeds.minimal_svg_corrupt.clone()],
        ),
        (
            "psdimport",
            firecraft_fuzz::anchor_psdimport,
            vec![seeds.minimal_psd.clone(), seeds.minimal_psd_corrupt.clone()],
        ),
    ]
}

/// Length-prefixed section bytes.
fn section_bytes(body: &[u8]) -> Vec<u8> {
    let mut out = (body.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(body);
    out
}

/// Deterministic xorshift64*.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

/// One mutation pass: a handful of random edits (bit flip, byte set,
/// splice-duplicate, truncate) plus a chance to append hostile tails.
fn mutate(rng: &mut Rng, seed: &[u8]) -> Vec<u8> {
    let mut data = seed.to_vec();
    let edits = 1 + rng.below(8);
    for _ in 0..edits {
        match rng.below(6) {
            0 if !data.is_empty() => {
                let i = rng.below(data.len());
                data[i] ^= 1 << rng.below(8);
            }
            1 if !data.is_empty() => {
                let i = rng.below(data.len());
                data[i] = rng.next() as u8;
            }
            2 if !data.is_empty() => {
                let i = rng.below(data.len());
                let j = rng.below(data.len());
                let (a, b) = if i <= j { (i, j) } else { (j, i) };
                let slice = data[a..=b].to_vec();
                let at = rng.below(data.len() + 1);
                data.splice(at..at, slice);
            }
            3 => {
                if rng.below(4) == 0 && data.len() > 1 {
                    data.truncate(rng.below(data.len()));
                }
            }
            4 => {
                // Append a hostile tail: zeros, 0xff, or random bytes.
                let tail: Vec<u8> = match rng.below(3) {
                    0 => vec![0u8; rng.below(96)],
                    1 => vec![0xFFu8; rng.below(96)],
                    _ => (0..rng.below(96)).map(|_| rng.next() as u8).collect(),
                };
                data.extend_from_slice(&tail);
            }
            _ => {
                // Random insert of a small constant pattern.
                let pattern: &[u8] = match rng.below(4) {
                    0 => b"MKBv{LYLv{LAYv{BKGb{0}}}}",
                    1 => &[0xCA, 0xFE, 0xBE, 0xEF],
                    2 => &[0x1B; 8],
                    _ => b"\x78\x9c\x03\x00\x00\x00\x00\x01",
                };
                let at = rng.below(data.len() + 1);
                data.splice(at..at, pattern.iter().copied());
            }
        }
    }
    if data.len() > 256 * 1024 {
        data.truncate(256 * 1024);
    }
    data
}

fn name_hash(name: &str) -> u64 {
    name.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01B3)
    })
}

struct Seeds {
    empty: Vec<u8>,
    minimal_png: Vec<u8>,
    canonical_tree: Vec<u8>,
    nested_tree: Vec<u8>,
    prvw: Vec<u8>,
    mkbt: Vec<u8>,
    float_blob: Vec<u8>,
    minimal_svg: Vec<u8>,
    minimal_svg_corrupt: Vec<u8>,
    minimal_psd: Vec<u8>,
    minimal_psd_corrupt: Vec<u8>,
}

/// Seeds built in code so the driver needs no corpus files.
fn seeds() -> Seeds {
    // Minimal two-chunk stream: a tEXt-like chunk and IEND.
    let mut minimal_png = Vec::new();
    fwpng::write_chunk(&mut minimal_png, b"tEXt", b"FireCraft seed");
    fwpng::write_chunk(&mut minimal_png, b"IEND", &[]);

    let canonical_tree = b"MKBv{FRCi{1}XLCi{0}LYLv{LAYv{BKGb{0}}}}".to_vec();

    // A valid nested tree with one string node (u16 BE count + UTF-16BE
    // inside the braced payload), one hex int, one signed spelling, and
    // one float.
    let mut nested_tree = Vec::new();
    nested_tree.extend_from_slice(b"MKBv{NMs{");
    let text = "A firework";
    nested_tree.extend_from_slice(&(text.encode_utf16().count() as u16).to_be_bytes());
    for unit in text.encode_utf16() {
        nested_tree.extend_from_slice(&unit.to_be_bytes());
    }
    nested_tree.extend_from_slice(b"}SUBv{FRZi{ff}NEGi{-1a}FRZf{3.5}SUBv{BKGb{1}}}}");

    // prVW: magic + 16x16 size + 2 palette entries + 3x3 index bytes.
    let mut prvw = Vec::new();
    prvw.extend_from_slice(&0xCAFE_BEEFu32.to_be_bytes());
    prvw.extend_from_slice(&16u16.to_be_bytes());
    prvw.extend_from_slice(&16u16.to_be_bytes());
    prvw.extend_from_slice(&[0xFF, 0x00, 0x00]);
    prvw.extend_from_slice(&[0x00, 0xFF, 0x00]);
    prvw.extend_from_slice(&[0, 0, 0]);
    prvw.extend_from_slice(&[0, 1, 0, 1, 0, 1, 0, 1, 0]);

    // mkBT: magic + tile index 0 with reserved top bits + flags + 76-byte
    // header + a real zlib stream of 65536 ARGB-ish zero bytes (deep-path
    // seed: this one must parse, everything the mutator does to it should
    // at worst fail cleanly).
    let mut mkbt = Vec::new();
    mkbt.extend_from_slice(&0xFACE_CAFEu32.to_be_bytes());
    mkbt.extend_from_slice(&[0, 0, 0, 0]); // BE u32, tile 0
    mkbt.extend_from_slice(&[0, 0, 0]); // reserved
    mkbt.push(0); // grayscale flag
    mkbt.extend_from_slice(&[0u8; 64]); // reserved
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder
        .write_all(&[0u8; fwpng::tile::TILE_STREAM_BYTES])
        .expect("in-memory zlib write cannot fail");
    let zlib = encoder.finish().expect("in-memory zlib finish cannot fail");
    mkbt.extend_from_slice(&zlib);

    let mut float_blob = Vec::with_capacity(96);
    for v in [
        1.0_f64, 2.0, -0.5, 3.0, 10.0, 20.0, 0.0, 0.0, 8.0, 8.0, 64.0, 64.0,
    ] {
        float_blob.extend_from_slice(&v.to_le_bytes());
    }

    // A valid one-path SVG (deep-path seed: parses, renders, imports)
    // and a half-mutated cousin (parse must refuse cleanly).
    let minimal_svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"8\" height=\"8\">\
        <path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"#ff0000\"/></svg>"
        .to_vec();
    let minimal_svg_corrupt = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"8\" heigh\
        <pa/h d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"#ff0000\"></svg>"
        .to_vec();

    // A minimal one-layer PSD (deep-path seed): 2x2 canvas, one 1x1
    // layer with raw channels interleaved after its record (the
    // ImageMagick layout), zero composite — the layout the reader must
    // fully accept — plus a cousin cut mid-record (must refuse).
    let mut minimal_psd = Vec::new();
    minimal_psd.extend_from_slice(b"8BPS");
    minimal_psd.extend_from_slice(&1u16.to_be_bytes());
    minimal_psd.extend_from_slice(&[0; 6]);
    minimal_psd.extend_from_slice(&3u16.to_be_bytes()); // channels
    minimal_psd.extend_from_slice(&2u32.to_be_bytes()); // height
    minimal_psd.extend_from_slice(&2u32.to_be_bytes()); // width
    minimal_psd.extend_from_slice(&8u16.to_be_bytes()); // depth
    minimal_psd.extend_from_slice(&3u16.to_be_bytes()); // RGB
    let section = |out: &mut Vec<u8>, body: &[u8]| {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
    };
    section(&mut minimal_psd, &[]); // color mode data
    section(&mut minimal_psd, &[]); // image resources
    let mut record = Vec::new();
    for value in [0i32, 0, 1, 1] {
        record.extend_from_slice(&value.to_be_bytes()); // rect
    }
    record.extend_from_slice(&4u16.to_be_bytes()); // channel count
    for id in [0i16, 1, 2, -1] {
        record.extend_from_slice(&id.to_be_bytes());
        record.extend_from_slice(&3u32.to_be_bytes()); // marker + pixel
    }
    record.extend_from_slice(b"8BIMnorm"); // blend key
    record.push(255); // opacity
    record.push(0); // clipping
    record.push(0); // flags: visible
    record.push(0); // filler
    let extra_len = 4 + 4 + 4; // mask 0, ranges 0, pascal "a" padded
    record.extend_from_slice(&(extra_len as u32).to_be_bytes());
    record.extend_from_slice(&[0; 8]); // no mask, no ranges
    record.extend_from_slice(&[1, b'a', 0, 0]);
    for value in [1u8, 2, 3, 255] {
        record.extend_from_slice(&0u16.to_be_bytes()); // raw marker
        record.push(value);
    }
    let mut info = 1u16.to_be_bytes().to_vec();
    info.extend_from_slice(&record);
    section(&mut minimal_psd, &section_bytes(&info));
    // Composite: compression 0, zeroed R/G/B planes.
    let mut merged = 0u16.to_be_bytes().to_vec();
    merged.extend_from_slice(&[0u8; 12]);
    section(&mut minimal_psd, &merged);
    let minimal_psd_corrupt = minimal_psd[..50].to_vec();

    Seeds {
        empty: Vec::new(),
        minimal_png,
        canonical_tree,
        nested_tree,
        prvw,
        mkbt,
        float_blob,
        minimal_svg,
        minimal_svg_corrupt,
        minimal_psd,
        minimal_psd_corrupt,
    }
}

# Decision 0063 — WebP export (lossless VP8L)

**The first enhancement family** (the parity campaign is at its
evidence limits — Phase 9 debts empty, 3 P rows permanent or
evidence-gated): the user's standing direction is CS6 parity *plus*
enhancements over time, "such as more image support". WebP import has
existed since M8 (`io::raster` sniffs and decodes it); the export
matrix stopped at `png|jpg|bmp|tiff|wbmp`.

## Design

- **`crates/io/src/webp.rs`** — one `encode(plane)` door: lossless
  VP8L through the `image` crate's pure-Rust encoder (`image-webp`,
  already in the dependency graph behind the `webp` feature — zero
  new dependencies, wasm-safe). Lossy VP8 needs libwebp (a C
  dependency that breaks the wasm build) and stays out of scope; the
  export is the honest lossless contract — pixels in, the same
  pixels out, alpha preserved.
- **`export_raster` routes `.webp`** like every other raster door
  (render → straight-alpha plane → codec); the unknown-extension
  error names the new set. `batch_one`/`main.rs` need no change —
  their `_` arm falls through to `export_raster`.
- **Decode-side proof.** The round-trip tests decode through
  `io::raster::decode` (the independent import path) and require
  exact pixel equality — the io battery's "decode its own output
  with an independent implementation" law.

## Consequences

- No registry change (the door is the CLI/app export surface); no
  parity-table change.
- Tests: +1 io (`webp_lossless_round_trips_alpha_and_pixels` —
  RIFF/WEBP container bytes, exact gradient+alpha round-trip
  through the import decoder) and +2 app-cli (`.webp` routes
  losslessly end-to-end from a rendered document; unknown
  extensions name the updated format set).
- The README's "modern codec support (WebP, AVIF, HEIC via OS
  decoders)" ambition is now partially real: WebP in *and* out.
  AVIF/HEIC remain future work (OS-decoder bindings, wasm story
  unresolved).

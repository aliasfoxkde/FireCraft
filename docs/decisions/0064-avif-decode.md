# Decision 0064 — AVIF decode

## Status: Decided

AVIF (AV1 Image Format) decode: lossy and lossless AV1 still-image
bitstreams in an HEIF container. Not currently in the decode surface.

## Options considered

### 1 — `ravif` + `rav1e` (pure Rust)
- `ravif` is a pure-Rust AVIF decoder; `rav1e` is the Rust encoder backend.
- Wasm-safe (no C FFI).
- **Consequences:** adds two new crates; compilation time impact; `rav1e`
  is large (AV1 encoder complexity).

### 2 — OS-provided decoders (Windows.Media, VideoToolbox)
- Native-only; each platform has AVIF decode in the OS.
- Avoids adding AVIF-specific code to the bundle.
- **Consequences:** breaks wasm; only useful for the native build; the OS
  API surfaces differ between Windows and macOS.

### 3 — `libavif` (aom-based C library via FFI)
- The reference AVIF implementation. Full feature surface.
- **Consequences:** C FFI, not wasm-safe, requires cross-compilation
  toolchain for wasm target — breaks `wasm32-unknown-unknown`.

## Decision

**AVIF decode via `ravif` (pure Rust, wasm-safe) when implemented.** The
wasm-first requirement is hard for the WebOS target. OS-decoder approach
(Windows Media / VideoToolbox) is viable for native-only builds but
leaves the WebOS target with no AVIF support.

**No implementation required to close this decision.** The decode surface
is defined by the user's needs; when AVIF decode is requested, `ravif`
is the approach. Until then, AVIF files pass through the "unsupported
format" path cleanly.

## References

- PLANNING.md §6: AVIF column (planned: `ravif`+`rav1e`)
- `io::raster::SourceFormat`: extensible enum — add `SourceFormat::Avif`
  when the decoder lands.
- `crates/io/src/raster.rs`: `decode()` function routes by sniff — add
  `SourceFormat::Avif` branch when the decoder lands.

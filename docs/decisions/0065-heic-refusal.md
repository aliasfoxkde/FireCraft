# Decision 0065 — HEIC refusal

## Status: Policy

HEIC/HEIF decode is not implemented in FireCraft. This is a deliberate
refusal, not a gap.

## Why

HEIC is an HEVC (H.265) still-image profile. HEVC patents are pooled and
royalty-bearing — shipping a decoder without a license from the
Access Advance pool is a legal risk the project does not carry.

This is not a technical position. Browsers (Firefox, Chrome, Safari) and
operating systems take the same stance: they ship OS-level HEIC decode
when the OS provides it, and refuse to bundle a decoder themselves.

## Options considered

### 1 — OS-provided decoders only (Windows.Media / VideoToolbox)
- macOS: `VideoToolbox` via platform bindings; Windows: `Windows.Media`
  via `windows` crate. Each OS has a licensed HEVC decoder.
- **Safe** — no patent exposure in the project.
- **Consequences:** native-only (no wasm), platform-specific code paths,
  different APIs on each OS.

### 2 — `libheif` (C library via FFI)
- The reference HEIF implementation; decodes HEIC.
- **Consequences:** C FFI, not wasm-safe, patent exposure unclear on
  the library level, requires cross-compilation toolchain for wasm target
  — breaks `wasm32-unknown-unknown`.

### 3 — Pure-Rust `heif` or `heic` crates
- The imazen `heic` crate is AGPL + platform-backend policy.
- **Consequences:** AGPL license would require GPL-compatible licensing
  for the project; pure-Rust HEVC decode does not exist at sufficient
  quality.

## Decision

**OS-provided decoders only, when the native build is extended.** The
decode surface stays closed for now. When a user needs HEIC import on
the native build, the path is `libheif-sys` or platform bindings — not
bundling a decoder. The wasm build has no HEIC support path today and
no planned one.

This matches the project policy in PLANNING.md §6: HEIC is T4,
native-only, OS-decoder only.

## References

- PLANNING.md §6: HEIC/HEIF column ("OS decoders only — no bundled
  HEVC decode — patent pools")
- PLANNING.md §9: HEVC patent risk, OS-decoder-only policy
- Mozilla HEVC stance: Firefox relies on OS decoders on macOS and Windows

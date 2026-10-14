# Decision 0055 — mkBT tile-bank load/write (Phase 9 debt closed)

**Debt.** The `mkBT` reader existed (M7), but the bank was write-only
in the other direction: `document.tiles` was populated only by
runtime commands (`flatten`, the place-image trio, `retouch`), never
by file open — a corpus file with raster imagery would `TileBankMiss`
at render. And `save()` passed the original `mkBT` chunks through
verbatim, so tiles minted at runtime never serialized — the saved
file carried dangling `TIDi` references. A third seam: `library`
exports built `Document::default()`, dropping the bank entirely, so
a symbol member with bitmap content re-imported as a dangling grid.

## Design

- **Open joins the bank.** `app_cli::io::open` parses every `mkBT`
  chunk (split-stream reassembly through `fwpng::parse_mkbt_chunks`)
  and inserts each 128×128 ARGB tile into `document.tiles` under its
  `TIDi` (ARGB → RGBA8 at the fwpng boundary). A parse failure
  refuses the whole open — a bank the loader cannot read is a
  corrupt document, not a degraded one. Grayscale 256×256 tiles have
  no RGBA bank slot and skip with the recorded posture: a `TILv`
  that references one fails loudly at render (`TileBankMiss`), so
  unreferenced strays stay harmless.
- **Save merges, never rewrites.** The original stream's tile ids
  are computed first (`original_tile_ids`, which refuses on a
  malformed tile stream — the id set cannot be known, so neither a
  verbatim nor a merged re-emit is honest). Bank tiles already on
  disk stay verbatim; only fresh ids — the ones
  flatten/retouch/place-image minted since open — serialize, as
  `encode_mkbt` payloads written before `IEND` in bank order
  (lowest id first). Byte-exact round-trips for untouched files are
  the test-pinned law.
- **`encode_mkbt` mirrors the reader's split law.** One 128×128 RGBA8
  plane (`exactly 65536` bytes, enforced) converts to ARGB
  (`[a, r, g, b]` per pixel), deflates, and splits at the 64 KiB
  payload cap: the first payload carries the 76-byte header (magic
  `FA CE CA FE`, BE index in the top three bytes, reserved bytes,
  grayscale flag 0, 64 reserved), continuations are raw spans of the
  zlib stream — exactly the shape `parse_mkbt_chunks` reassembles.
  A high-entropy tile exceeds one payload; the split test pins it.
- **Library export carries its rasters.** `library::extract` walks
  the member closure for tile references — image grids plus the
  pattern fills of path paints (`PaintStyle.fill.pattern`) — and
  copies each referenced tile into the target bank under its source
  id, so the file's `TIDi` values match and re-import finds its
  rasters. Matte references (`TMCi`) pull nothing. A referenced id
  with no bank entry is a corrupt source: `ExtractError::MissingTile`
  refuses whole rather than land a dangling reference.
- **The gray-zone is not hidden.** `UnknownBag` tiles — `mkBT`
  chunks present in a file whose `TILv` nodes ride the bag — join
  the bank anyway; an unreferenced tile costs 64 KiB of memory and
  round-trips verbatim on save. Honest overhead beats a second
  half-parser.

## Consequences

- Registry unchanged (no new commands); parity table unchanged
  (268 C / 3 P / 0 M / 85 N).
- Tests: fwpng encoder trio (round-trip through `parse_mkbt`,
  high-entropy split reassembled through `parse_mkbt_chunks`,
  short-plane refusal); app-cli `tests/tiles.rs` (minted tiles reach
  the file, rejoin the bank with their pixels, and re-emit stably;
  tile-free byte-exactness; both tiled fixtures keep every non-`mkTS`
  chunk and their banks across open→save→open; a truncated `mkBT`
  chunk refuses at open); doc extract pair (referenced tiles carry,
  dangling reference refuses).
- Foreign tiled files are **not** byte-stable under save — `mkTS`
  re-canonicalizes by design (canonical convert posture); the pinned
  invariant is chunk-exactness outside `mkTS` plus bank equality.
- Remaining bitmap debt narrows to resample quality (bilinear) and
  out-of-bounds effect growth — both recorded in HANDOFF §4.

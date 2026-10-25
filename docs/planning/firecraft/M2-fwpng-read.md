# M2 — `fwpng`: Fireworks PNG read

> **CLOSED milestone (historical reference).** See docs/PROGRESS.md for the closure date and docs/HANDOFF.md for current work.

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)
> Format spec: PLANNING §6 · Node dictionary: `docs/research/fireworks-png-nodes.md`

**Objective:** parse any Fireworks PNG found in the wild without panicking;
extract the `prVW` thumbnail, `mkBT` tiles, and the complete `mkTS`/`mkBS`
node tree into a generic typed tree; preserve unknowns verbatim.

**Inputs:** PLANNING §6 (chunk map + encoding), vendored node dictionary,
png-analyser (MIT) as differential reference, SO 4242402 byte maps,
corpus from M1.

**Authority:** ROADMAP M2 gate. The node dictionary is observational —
where it disagrees with corpus evidence, corpus wins and the discrepancy is
recorded in `docs/decisions/`.

## Tasks (atomic)

1. **Chunk layer** — minimal PNG chunk reader/writer (len+type+data+CRC,
   streaming), tolerant of unknown/private chunks, **preserves them in
   order**; integrate `png` crate for the standard image path. Tests:
   synthetic PNG with fake private chunks round-trips byte-identically.
2. **Node-tree parser** — `MKBv{...}` grammar: FourCC + type letter
   (i/f/b/s/v), BE ints/floats/UTF-16 strings, nested containers; hostile
   input hardening (depth caps, allocation caps, truncated-stream errors —
   never panics). Zero-copy where practical. Errors carry byte offsets.
3. **Node-tree serializer** — exact-format re-emit (canonical spacing-free
   form); float formatting must round-trip CS6's decimal text (verify
   against corpus — record the exact precision rule found).
4. **`prVW` thumbnail** — `CA FE BE EF` header, palette + 128×128 indices;
   tolerant of the uninitialized-padding garbage (document which bytes are
   meaningful). Output: small RGBA bitmap.
5. **`mkBT` tiles** — `FA CE CA FE` + u24 index + grayscale flag, 64 KB
   zlib spans concatenated across chunks, 128×128 ARGB or 256×256 gray
   decode. Tests: split-stream synthesis.
6. **`mkBS` legacy copy** — parse + note (document-only, no pages);
   comparison helper mkTS vs mkBS for corpus analysis.
7. **Corpus + differential harness** — `xtask corpus` manifest (pinned
   sha256); differ script: our parse vs png-analyser JSON on samples →
   node-count/kind agreement report; set initial `ROUNDTRIP_FLOOR`.
8. **Round-trip test** — parse → serialize → byte-identical private
   payloads on ≥ `FLOOR`% of corpus; failures listed by `xtask corpus
   --report`. Fuzz: 60s+ on the chunk + nodetree parsers; zero panics.
9. **wasm check** — `fwpng` (and only std+pure deps) builds
   `wasm32-unknown-unknown`; smoke test via `wasm-bindgen-test`.

**Verification (milestone gate):** corpus 100% panic-free parse; round-trip
floor recorded and green; differential report shows agreement at recorded
floor; fuzz clean; wasm build green; any dictionary-vs-corpus corrections
documented.

**Out of scope:** typed document model (M3), rendering (M4).

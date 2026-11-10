# 0083 — Re-bless the Aegis content baseline for the parity-matrix content

Date: 2026-11-10 · Status: accepted

## Context

The dock/inspector parity-matrix work adds the CS6 Window-menu command ids
(`57650`–`57652`), the panel command ids (`43xxx`, `48xxx`, `35xxx`), the
tool-strip module, and the evidence documents that quote them. The Aegis
gate's `zip-code` rule matches any five-digit run, so that content is
reported as new findings and the gate fails.

The content-keyed baseline (decision 0074) is the remedy: it records the
number of occurrences adjudicated for each `(pattern, file, content
digest)` key. The baseline had to be re-blessed for the new content.

## Decision

The baseline was re-blessed with the **CI-pinned scanner, `aegis` 0.6.3**,
over the PR's own tree. The procedure is the one `cargo run -p xtask --
aegis --bless` performs; it was reproduced outside `cargo` because the
local build admission was closed while the GitForge runners held the
host's compilers.

Two properties make the reproduced bless equivalent to the tool's own:

1. The renderer reproduces `xtask/src/aegis.rs`'s `render_baseline`
   byte-for-byte — the header, one class-note comment block above each
   pattern's rows, and the records sorted by `(pattern, file, digest)`.
   Rendering `main`'s committed baseline from a scan of `main`'s tree
   reproduces `main`'s committed file exactly (no diff), which is the
   check that the renderer and the scanner agree with the tool.
2. The scanner is the same binary CI runs, so the content digests are the
   digests CI computes. Re-blessing with a different `aegis` build would
   have shifted the digests and failed the gate on drift rather than on
   content.

## Adjudication

Every new and changed key was read before it was admitted. The classes
and their verdicts:

| Class | What matched | Verdict |
|---|---|---|
| `zip-code` | CS6 command ids (`57650`, `43405`, `48607`, `35586`), palette ids, the `57650`/`57651`/`57652` trio in the code, its tests and its prose | False positive — command ids, not postal data |
| `pci-cardholder-data` | the word "pan" in *pan* the artboard and in *pan-zoom* | False positive — the class note already records this |
| `sql-query` | the English phrase "select … from" in the CS6 tool-strip help text and its citations | False positive — prose, and the workspace has no SQL client |
| `eval-usage` | the Rust `.eval(t)` call quoted in the session log | False positive — the scripting crate's own `eval()` |
| `phone-number` | `4294967295` (u32::MAX) quoted in the session log | False positive — a constant |
| `ssrf-localhost` | the prose naming the local GitForge API and the local preview URL | False positive — already the recorded reason for the class |

No rule class was switched off and no rule was excluded. The gate still
fails on any *further* occurrence of these literals, and on any new
literal.

## Consequences

- The PR's own tree carries a baseline that covers its content; the gate
  passes without exclusions.
- The same procedure applies to the other branches whose content trips
  the gate: re-bless with `aegis` 0.6.3 over that branch's tree, after
  reading each new key.

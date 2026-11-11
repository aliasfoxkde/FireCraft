# 0084 — Re-bless the Aegis content baseline for the slice/hotspot work

Date: 2026-11-10 · Status: accepted

## Context

This branch carries the dock/inspector parity-matrix content (it merges
that work), so the same Aegis findings the parity-matrix branch had to
adjudicate are reported here: the CS6 Window-menu command ids
(`57650`–`57652`), the panel command ids, the word "pan", and the English
phrase "select … from" in the tool-strip help text. The branch adds two
more Window-menu ids of its own (`43403`, `43406`).

## Decision

The baseline was re-blessed exactly as decision 0083 describes: over this
branch's own tree, with the **CI-pinned scanner `aegis` 0.6.3`,** using
the renderer that reproduces `xtask/src/aegis.rs`'s `render_baseline`
byte-for-byte (checked by re-rendering `main`'s committed baseline from a
scan of `main`'s tree and getting no diff).

The reproduction was verified against the failing CI run: the re-bless's
added occurrences plus its changed counts sum to **163**, and the CI gate
reported `aegis found 163 finding(s) the baseline does not cover` — the
same set, derived independently.

## Adjudication

Every new and changed key was read before it was admitted, and each is a
false positive of a class decision 0083 already records: five-digit CS6
command ids in code, comments and prose; the word "pan" in *pan* the
artboard and *pan-zoom*; "select … from" in the CS6 tool-strip help text;
the Rust `.eval(t)` call quoted in the session log; the u32::MAX
constant; and prose naming the local GitForge API and the local preview
URL. No rule class was switched off and no rule was excluded.

## Consequences

- The branch's tree carries a baseline that covers its content.
- The rows this branch adds overlap the parity-matrix branch's rows;
  merging both leaves one baseline that covers both trees.

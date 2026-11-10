# 0074 — The Aegis gate matches on content, not on line

Date: 2026-11-09 · Ticket: T27 · Status: accepted

## Decision

The Aegis gate does its own baseline matching. A finding's identity is
the **content key** `(pattern, file, content digest)`, and the
committed baseline (`.aegis-baseline.tsv`) records how many
occurrences of each key were adjudicated. The gate fails only when the
scan finds **more** occurrences of a key than the baseline records.

Aegis is invoked **without** `--baseline`; its JSON is parsed by
`xtask/src/aegis.rs`, which compares the two multisets itself.

## Why

Aegis's `--baseline` filter matches on the whole fingerprint
`pattern:file:line:digest`, so the line is part of a finding's
identity. Any prose edit that shifts a baselined finding down the
file reports it as a *new* finding, and the gate fails on noise.

Measured on the scanner itself (aegis 0.6.3, 2026-11-09):

| file:line | matched literal | fingerprint digest |
|---|---|---|
| `./a.md:1` | `zip 90210 here` | `b83c588d…9ac6c` |
| `./b.md:2` | `zip 90210 here` | `b83c588d…9ac6c` |
| `./c.md:1` | `zip 90211 here` | `7b1f167c…920e` |

The fourth fingerprint component is the digest of the **matched
literal alone** — identical across different files and different
lines, different for different literals. The content identity Aegis
already computes was therefore available all along; only the baseline
match threw it away.

The line-keyed filter was confirmed by mutation: changing one
baseline entry's digest, or its pattern component, made exactly that
finding reappear as new (exit 1). The match is on the whole
fingerprint, and no option or inline suppression changes it.

This was the third recurrence of the same class — PR #12
(`7e7f8ad`), PR #14 (`aefe5ac`), PR #10 (`1e24e1f`) and PR #11
(`62dc01d`) — and the repo's own `.aegisignore` had already recorded
the symptom: *"the baseline keys on file:line:content-digest, so every
regeneration invalidates the entry and the gate fails on noise instead
of on a real leak."*

The remedy that grew around it — re-pin the baseline after every doc
edit — is a procedure that exists only to route around a tool defect.
Per the repo's rule, the tool gets fixed and the procedure goes away.

## Consequences

- `xtask/src/aegis.rs` is a new gate module: it runs the scanner,
  parses the JSON, and compares multisets of content keys.
- The baseline is **`.aegis-baseline.tsv`**, one tab-separated record
  per key (`pattern`, `file`, `digest`, `count`). The old
  `.aegis-baseline.json` (1.8 MB, 520 findings) is deleted. The new
  file is ~430 lines and is reviewable in a diff — the old blob never
  could be, which is why a false positive could never be adjudicated
  before it was admitted.
- `cargo run -p xtask -- aegis --bless` regenerates the baseline and
  prints every added and removed key. It is the only way to admit a
  finding, and it stays a manual step: the additions must be verified
  as false positives before the rewrite is committed.
- A finding that **moved** is no longer new. A finding that was
  **deleted** is reported as baseline drift, not as a failure.
- The gate is **not** weakened. A new literal, the same literal in a
  new file, or an extra copy of a known literal still fails — the
  multiplicity is part of the key's record. Only the *line* stopped
  being part of the identity, which is what made the finding the same
  finding in the first place.
- The `log/` and generated-file exclusions in `.aegisignore` stay:
  their content churns, not just their line numbers.
- Each rule class's rows carry the **reason** those occurrences were
  adjudicated, as a comment above the group. The reason travels with
  the rows it covers, so a reviewer reading the baseline sees why a
  finding is suppressed without leaving the file. The gate's own
  source quotes the literals it explains, so it carries rows of its
  own; they are adjudicated like any other.

### Why no rule class is switched off

Decision 0070 (branch `feat/phase-21-daily-driver`) solves the same
defect with a different suppression model: it names whole rule classes
in `.aegis-pattern-state.json` and lets the scanner skip them, keeping
only a handful of per-finding entries.

This branch keeps **every rule class armed** and adjudicates each
occurrence individually. The two differ on one axis that matters: 0070
turns a rule off for the whole tree, so a genuine `path-traversal` or
`ssrf` in a later change is not scanned at all, while this branch
fails on the first new occurrence of a literal and on any new literal
of an armed class. 0070's class-level evidence (which classes cannot
fire here, and why) is preserved as the reasons written next to the
rows in this branch's baseline; the suppression itself stays narrow.

The cost is that the baseline lists every occurrence rather than a
handful of classes, and that a new numeric literal in any file needs
adjudicating before it can be baselined. That cost is visible and
bounded; the coverage that class suppression gives up is not.

### What the fix does not cover

PR #10 and PR #11 were red for a **different** reason: genuinely new
content, not a line shift. PR #10 adds `docs/img/logo.svg`, whose
numeric path data trips `australian-tfn` 23 times; PR #11 adds
`site/index.html`, `site/app.js` and `docs/SITE-DEPLOYMENT.md`, which
trip `path-traversal`, `ssrf` and `ssrf-localhost`. Those findings
survive the content-keyed gate on purpose, and each must be
adjudicated on its merits before it is baselined.

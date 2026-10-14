# FireCraft — Session Log

> One entry per working session, newest last. **Append an entry every
> session** (with [`PROGRESS.md`](PROGRESS.md)) — this file is committed;
> the detailed per-task log is `log/devlog.md` (gitignored, local only).
> Narrative arc and lessons: [`HISTORY.md`](HISTORY.md).

Format: date — what the session did — outcome (commit / gate state).

---

## 2026-10-07 — Repository created
Project inception; planning documents authored (PLANNING.md, roadmap,
format research, vendored node dictionary).

## 2026-10-10 — Bootstrap sprint: M0–M5
- M0 workspace bootstrap (xtask gates: layers/wasm/ci/panic_hunt/
  corpus/parity/scorecard; AGENTS.md; license + NOTICE).
- M1 foundations (geom depth, dirty tiles, fuzz anchors); corpus
  harness landed with a no-Wine fallback.
- M2 fwpng read path + pinned corpus.
- M3 doc model + fwmap byte-exact round-trip.
- M4 compositor + golden-image gate + bench baseline.
- M5 catalog v1 (task #42), app-cli info/convert/apply (#43), app
  shell v1 + web trunk (#44/#45), gates closed (#46).
- Infra: GitForge CI pipeline wired.
- Outcome: workspace live, all gates green.

## 2026-10-11 — Vector depth + text + imports: M6–M8 (part)
- M6 Auto Shape library (#48), freeform/reshape + align/distribute
  (#49), surfaces + gates (#50).
- M7 text + text-on-path + surfaces + gates (#51.7–9).
- M8 tasks 3–6: SVG export + resvg golden loop, SVG import (usvg),
  PSD spike + reader (decision 0006), PSD writer.

## 2026-10-12 — The close-out day: M8–M12 complete
- M8 tasks 7–8: animated GIF ↔ states; EXPv both ways + batch + export
  dialog (M8 closed).
- M9 tasks 1–8: scripting crate (sandbox, prelude), read-side DOM
  projection, mutating host API, .jsf loading, corpus run harness,
  History replay + Save Steps, CLI run-script + control channel + MCP,
  gates/fuzz/floors — **M9 closed**.
- CI battle: M9 CI died mid-compile; diagnosed via the job-log ledger —
  cgroup kill suspicion disproven; root cause was offline-cache refusal
  (stale image after lockfile move). Image re-ship rule recorded.
- M10 tasks 5–7: rich symbols (Widget.opCode), library import/export +
  Common Library, gates — **M10 closed**.
- M11 tasks 1–6: slice model/wire, web-layer commands, webexport crate
  (HTML+Images, Images Only, CSS+Images, CSS Sprites, To Files,
  states→GIF, CSS Properties), shell + CLI surfaces, gates
  (decision 0017) — **M11 closed**.
- M12 tasks 1–6: perf gate, memory ceiling (2,000-object anchor),
  `xtask dist --web` + bridge smoke, WEBOS.md, `xtask dist --native`,
  release chore (`xtask changelog`, RELEASING.md) — **M12 closed;
  roadmap M0–M12 complete.**

## 2026-10-13 — Release + M13–M15 + the parity campaign begins
- **v0.1.0 released; repo public.**
- M13 closed (interaction parity + scorecard); M14 closed (live-filter
  parity); M15 tasks 5–6 closed (gradient fills end to end, style
  model + Styles panel + .stl import).
- M15 recorded gaps closed: canvas background (#82, 77f55af), SVG
  gradient paint-server import (#83, 93e0778), path.setFillColor
  collapse (#84), pattern fills end to end (#85).
- Task #86: the **DOM verb-surface parity matrix** — the CS6 DOM guide
  swept against the registry (`dom-parity.md`), 356 verbs classed
  C/P/M/N. The remaining work is now the M families.
- Tasks #87–#89: object.group/ungroup (96aa656), page guide/grid/onion
  (32dd03a), frame management (8187375).
- Task #91: object mask family (faa738d) — parity 162C/45P/67M.

## 2026-10-13/14 — Structure families (#92, #93)
- #92 layer/page structure (decision 0021): page.setName/reorder/
  setMaster/unsetMaster/addMasterLayer + layer.reorder/setLocked;
  PageBody/Pages deltas; sharing trio deferred with the typed-model
  requirement. e830802, parity 169C/45P/60M.
  - CI leg: the app-cli symbol journal test forced symbol calls by
    name; CALLS growth 138→145 shifted seed 31's stream onto an
    all-refuse run. Probe (deleted after) proved the generator's
    deliberate junk-field/index over-draw; fix = draw-until-landed +
    deterministic seed state (535340f).
  - Lesson banked: three test failures from capture-after-mutate in
    the same batch — the capture-order rule is now standing law
    (HANDOFF §3).
- #93 shared-layer model (decision 0022): LSMv/LSEv/LSAv typed,
  reader/writer wired, synthetic round-trip gate; layer.setSharing /
  shareToPages / detachShared; Delta::SharedLayers. 25fedbc,
  parity **172C/45P/57M**, CI 7/7.

## 2026-10-14 — Documentation + handoff (this session)
- Wrote the committed doc set for future agents: HANDOFF.md (phased
  atomic plan + runbooks), PROGRESS.md (status board), HISTORY.md
  (arc + decisions + lessons), SESSIONS.md (this file).
- Cleanup: scratch scripts tidied, doc pointers added to CLAUDE.md and
  AGENTS.md.

## 2026-10-14 — Gap-family campaign day (#94–#102)
- #94 committed doc set (HANDOFF/PROGRESS/HISTORY/SESSIONS) + campaign
  plan; devlog consolidated.
- #95 canvas color + resolution setters (4b813ef) · #96 object-selection
  model + selection.* basics (cc85956) · #97 hierarchy selects +
  bitmap selectSimilar (135cb6b) · #98 selection-to-frames fans
  (decision 0026, ba60816) · #99 primitive params + autoshape redraw
  (decisions 0027/0028, 93c59b4 + 614bc64) · #100 web-object point
  append (decision 0029, 3516d89) · #100b crop/fill-edge/tween
  (decision 0030, df8fe42) · #101 flatten family via compositor bakes
  (decision 0031, 235e4d4, render_subset) · #102 knife + splitPaths
  (decision 0032, new vector::knife module).
- Parity 172C/57M → **201C/26M** across the day; registry 148 → 176.
- Infra: t5500 runner offline since 2026-10-11 (no route to host);
  fedora-docker carries the CI image; re-shipped on lockfile moves per
  the lockfile-bytes rule.
  (continued) #102 landed 3baf178, CI 7/7; #103 convertToPaths with a
  pixel-identity golden (decision 0033), 56c6f2f, CI 7/7.
  #104 document.rotate (decision 0034): page-scoped 90/180/270 with a
  full geometry re-bake — integer linear part (trig fails exactness),
  image assemble→permutate→re-tile via compose::matte_premultiplied +
  flatten::next_tile_id made shared, guides/grid swap, byte-exact
  undo. Parity **203C/45P/24M**, registry 178.

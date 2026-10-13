# M13 — Deep RE evidence & CS6 shell parity

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** close the two gaps v0.1.0 exposed. First, the binary-analysis
layer the original request named explicitly — "decompile it and use …
Ghidra" (PLANNING §1) — which M0–M12 never executed: the §5.5 passes run
against the actual CS6 binaries through REA (Ghidra headless), and their
findings land as committed spec documents. Second, the editor shell becomes
Fireworks to the eye and the hand: CS6 menu structure, the vertical Tools
panel with original-art icons and flyout groups, the Properties inspector
bottom-docked, the Pages/States/Layers/Library/History/Optimize stack, dark
chrome, the CS6 accelerator map, and per-tool cursors — all grounded in the
UI tables the RE pass extracts, not in approximation. **Gate:** each
§5.5 target lands a committed spec note with a changed-or-confirmed verdict
per known unknown; the shell implements the extracted menu/accelerator/
panel tables (structure tests pin them); `xtask ci` green; wasm build
holds the 14 MB bundle ceiling; scorecard M13 row → done with evidence.

**Inputs:** `source/Adobe Fireworks CS6/` (`Fireworks.exe`, 27 MB PE, plus
DLLs and the `Configuration/` tree) — analysis input only, never committed;
the RE toolchain standing at `/nas/Temp/tmp/firecraft-re` (Ghidra headless
+ JDK 21 + REA 6.4.0, registered as `rea mcp`); PLANNING §5.5 (the named
target list) and §8's risk row (mkTS node semantics "partially unknown …
targeted Ghidra passes on `CMkbStructureOutputter` when corpus stalls");
`docs/research/fireworks-png-nodes.md` (the observational dictionary the
RE pass checks against); the CS6 `.jsf` behavioral corpus; the shell map
(`crates/app/src/{shell,editor,canvas}.rs`: egui 0.36 panel set, `Tool`
enum, registry-only mutations, zero theming today); the user-approved icon
decision (original art, ATTRIBUTION rows in the same change).

## Design line: RE evidence is notes, not code; the shell is a product surface

The clean-room rule is unchanged and non-negotiable: the passes exist to
*understand* — pseudocode and resource tables are read, and what lands in
the repo is our own spec prose and fresh implementation. Raw Adobe-derived
dumps (decompilations, string tables, resource extracts) stay uncommitted
in scratch, exactly like the §5.4 extracts; only derived findings are
committed, and no Adobe bytes (code, icons, fonts, assets) ever enter the
tree. Where RE evidence and corpus disagree, corpus wins and the
discrepancy is recorded in `docs/decisions/` — the standing rule that kept
the format honest when the dictionary and the files disagreed.

The shell work follows the same evidence discipline as the format work
did: the menu tree, accelerator map, tool order and grouping, and panel
set come from the extracted CS6 tables (cross-checked against the `.jsf`
command tree), and structure tests pin them, so "looks like Fireworks" is
a gated claim, not a mood. The one place approximation is permanent is
iconography: the glyph set is original vector art drawn in CS6's visual
language (silhouette and weight, never traced pixels), attributed as our
own assets in `docs/ATTRIBUTION.md`.

## Tasks

1. **PNG-writer pass** — `CMkbStructureOutputter` / `CWriteDictInfo` /
   `CDbText` path: settle the mkTS known-unknowns (filter parameter
   layouts, color layouts) against the observational dictionary. Output:
   `docs/research/fw-code-notes.md` (derived spec — the writer section;
   the planning-era class names were corrected by the pass itself),
   decisions where a writer choice changes; corpus gates unchanged.
2. **Chunk-serializer pass** — confirm our writer's byte-layout choices
   per chunk family; record confirmations and deltas in the same notes.
3. **Filter/effect pipeline pass** — parameter semantics and order of
   operations for the live effect stack; correct any `filters`-crate
   deviation the evidence proves (corpus wins on conflict, recorded).
4. **UI-resource pass** — menu tree, command ids, accelerators, tool
   grouping/order, panel set from the binary's resources + the readable
   `Configuration/` tree. Output: `docs/research/fw-ui-evidence.md`
   (derived tables only) — the authority for every shell task below.
5. **Shell: chrome + layout** — theme the egui context (dark CS6-class
   palette, UI font sizing), rebuild the panel set to CS6 layout: left
   Tools panel, bottom Properties inspector, right Pages/States/Layers/
   Library/History/Optimize stack, document tabs, workspace backdrop.
6. **Shell: Tools panel + icons** — vertical tool palette per the
   extracted grouping, flyout sub-tools, original vector icon set drawn
   with the egui painter (no bitmap assets), active-tool highlight,
   per-tool cursor icons.
7. **Shell: menus + accelerators** — CS6 menu bar (File/Edit/View/Select/
   Modify/Text/Commands/Window/Help) mapped onto registry ids; the CS6
   accelerator map bound through a real shortcut table (today the hinted
   letters bind nothing); structure tests pin menu items + keymap.
8. **Interaction parity** — marquee/anchor/cursor semantics for the tool
   set, double-click and modifier behaviors the `.jsf` corpus documents,
   space-drag pan, zoom-at-pointer; `xtask ci` green with the wasm
   ceiling holding; scorecard row closes the milestone.

## Verification

- RE passes: each note names the functions inspected (addresses/symbols),
  the question asked, and the verdict per known unknown — no note without
  a verdict, no verdict without a named observation.
- Shell: `cargo test -p app` gates the new structure tests (menu items,
  keymap, tool order) on top of the existing battery; `xtask wasm` keeps
  the bundle compiling; `xtask dist --web` keeps the 14 MB ceiling and
  the bridge smoke green; `xtask perf` budgets untouched.
- Clean-room audit in the same change: `git status` shows nothing under
  `source/`, no raw decompilation text in any committed file, ATTRIBUTION
  rows for the icon set.

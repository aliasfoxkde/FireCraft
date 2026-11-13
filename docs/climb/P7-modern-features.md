# P7 — Modern features (what CS6 never had)

**Goal:** FireCraft is a better tool than CS6 was in 2013, not just a
replica. That means accessible, themeable, translatable and colour-managed,
with real typography, touch support and a scripting API that is documented.

**Exit check:** every rung below is `[x]` or `[-]` with a decision.
AUDIT §9's "Absent" list is empty except for items deferred by decision.

**Lane:** modern. Can run alongside P5/P6 after P2.

**Rule:** every new dependency must meet all of these:
- pure Rust;
- builds for `wasm32-unknown-unknown`;
- permissive licence;
- recorded in a decision with its `cargo tree` impact and its wasm-size
  delta (S8 must not rise; offset the growth in the same PR or via P8).

---

### C7.01 — Light theme and "follow system" `[ ]`
- **Needs:** C2.09 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Find the dark palette, which is set in one place (`git grep -n
     "Visuals::dark\|visuals" crates/app/src`).
  2. Put the colours in a `Theme` struct. Choose a light variant whose
     contrast meets WCAG AA for text; check with a contrast unit test on the
     theme tokens.
  3. Add View ▸ Theme ▸ Dark / Light / System, saved in preferences.
  4. On web, System follows `prefers-color-scheme`.
- **Done when:** the theme switches without a restart, and the contrast test
  passes for both themes.
- **Verify:** `cargo test -p app theme`; browser screenshots of both.
- **Moves:** modern.

### C7.02 — Screen-reader support (AccessKit) `[ ]`
- **Needs:** C2.09 · **Tags:** `[HOST:browser]` · **Size:** L
- **Steps:**
  1. The `accesskit` feature is on but unused (AUDIT §9). Give every custom
     widget a role and a label:
     - tool buttons, from their tooltip;
     - panel tabs;
     - canvas objects, exposed as a list through the Layers panel.
  2. Check `crates/engine/src/access.rs` for an existing model to reuse.
  3. Write the kittest assertions with `egui_kittest` queries by label.
- **Done when:** every toolstrip button and panel tab can be found by its
  label in a kittest test, and a smoke test with NVDA or Orca is recorded.
- **Verify:** `cargo test -p app a11y`.
- **Moves:** modern; S6 ↑.

### C7.03 — Keyboard-only operation `[ ]`
- **Needs:** C7.02 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Tab moves between the toolbar, canvas, panels and dock tabs.
  2. Arrow keys nudge the selection (CS6 has this; confirm it exists).
  3. Focus rings are visible in both themes.
  4. Esc returns to the canvas.
- **Done when:** a written script performs draw-rectangle, recolour and
  export using only the keyboard, as a shell test with key events.
- **Verify:** `cargo test -p app keyboard_only`.
- **Moves:** modern.

### C7.04 — Coalesce rapid edits into one undo step `[ ]`
- **Needs:** — · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Dragging a slider or typing in a numeric field should produce one
     history entry per gesture, not one per frame.
  2. In `crates/engine/src/history.rs`, add a coalescing key (command ID +
     target + gesture ID). The shell opens and closes a gesture around each
     drag or edit.
  3. Journal replay (`journal.rs`) must stay deterministic. Add a test.
- **Done when:** a 60-frame opacity drag leaves 1 undo entry, and the replay
  test passes.
- **Verify:** `cargo test -p engine history`.
- **Moves:** modern.

### C7.05 — Translatable UI strings `[ ]`
- **Needs:** C2.06 · **Tags:** `[HOST:linux]` · **Size:** L
- **Steps:**
  1. Write a decision choosing `fluent` or `fluent-bundle` for message
     formatting (pure Rust).
  2. Move the menu labels and panel titles into `crates/app/locales/en-US/*.ftl`.
     Code refers to them by key.
  3. A test fails if any key is missing from `en-US`.
  4. The language is chosen in preferences; the web uses
     `navigator.language`.
  5. Ship `en-US` only, plus a pseudo-locale (accented, 30 % longer) for
     layout testing.
- **Done when:** the pseudo-locale renders with no truncation in the main
  panels (screenshot), and the missing-key test passes.
- **Verify:** `cargo test -p app i18n`.
- **Moves:** modern.

### C7.06 — Colour management (ICC) `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** L
- **Steps:**
  1. Write a decision choosing `qcms` or `lcms2`-free pure-Rust moxcms
     (check its licence and wasm support).
  2. Imported PNG/JPEG/PSD with ICC profiles convert to sRGB on import;
     today the profile is ignored. Keep the original profile in the
     document if the format allows it.
  3. Export embeds an sRGB profile (iCCP / APP2), behind an option.
  4. Add a soft-proof view mode, as a stretch goal.
- **Done when:** a Display-P3 test PNG imports with sRGB values matching the
  reference conversion within ±1.
- **Verify:** `cargo test -p io icc`.
- **Moves:** modern.

### C7.07 — OpenType shaping and variable fonts `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** L
- **Steps:**
  1. Read `crates/text` to learn the current shaping path.
  2. Write a decision on `rustybuzz` shaping (plus `swash` or `skrifa` for
     outlines). It must cover ligatures, kerning and complex scripts (Arabic,
     Devanagari sample).
  3. Add variable-font axes (wght, wdth) to the Text properties.
  4. Font files used in tests must be permissively licensed (OFL), each with
     an `ATTRIBUTION.md` row.
- **Done when:**
  - "fi" ligates;
  - an Arabic sample shapes right-to-left with joined forms;
  - a wght axis change renders a different outline.
  Each has a golden test.
- **Verify:** `cargo test -p text shaping`.
- **Moves:** modern.

### C7.08 — Web fonts in the browser build `[ ]`
- **Needs:** C7.07 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. Font load opens a local font file (`<input>`), or uses
     `queryLocalFonts()` where the browser supports it, with feature
     detection.
  2. Persist loaded fonts in IndexedDB.
  3. Missing-font substitution warns the user. It must not fail silently.
- **Done when:** a loaded TTF renders, and survives a reload.
- **Verify:** a browser proof.
- **Moves:** modern.

### C7.09 — Touch and trackpad gestures `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Pinch to zoom around the gesture centre.
  2. Two-finger pan.
  3. Pen pressure from pointer events goes to the brushes, which are already
     pressure-aware.
  4. Long-press opens the context menu.
  5. Make the canvas handle sizes touch-sized when the input is touch.
- **Done when:** the zoom and pan maths have unit tests on the transform, and
  a manual tablet or phone proof is recorded.
- **Verify:** `cargo test -p app gestures`.
- **Moves:** modern.

### C7.10 — Design-token export `[ ]`
- **Needs:** C4.16 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Export the document's swatches, styles and text styles as:
     - W3C Design Tokens JSON (Community Group format);
     - CSS custom properties.
  2. Add File ▸ Export ▸ Design Tokens.
- **Done when:** a golden JSON and CSS output exist for a fixture with 3
  colours, 2 text styles and 1 gradient.
- **Verify:** `cargo test -p webexport tokens`.
- **Moves:** modern.

### C7.11 — Document the scripting API `[ ]`
- **Needs:** C5.21 · **Tags:** `[DOCS]` `[HOST:linux]` · **Size:** M
- **Steps:**
  1. Generate `docs/scripting/API.md` from the registry, which already has
     command metadata. Add `xtask scripting-docs` with a `--check` mode
     wired into `ci`.
  2. The output covers:
     - each `fw.*` and `dom.*` verb, with its parameters;
     - whether it is script-only (C5.00);
     - an example.
  3. Add a tutorial, `docs/scripting/GETTING-STARTED.md`, with 3 original
     example `.jsf` files under `examples/scripts/`.
- **Done when:** `--check` fails when a registered command has no doc entry.
- **Verify:** `cargo run -p xtask -- scripting-docs --check`.
- **Moves:** modern; S6 ↑.

### C7.12 — WebOS host file service (bridge v1 extension) `[ ]`
- **Needs:** C3.09 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Extend `docs/WEBOS.md` / `bridge-v1.schema.json` with these messages:
     - `host.open` and `host.save`, so the WebOS file manager supplies and
       receives bytes;
     - `host.recent`;
     - `host.theme`, so the WebOS theme overrides the system theme (ties to
       C7.01).
  2. Feature-detect each message through the C3.07 `hello` capabilities.
  3. Add a test host page under `web/test-host/` that drives the messages
     through Playwright.
- **Done when:** a Playwright test has the host open a file, edit it and save
  it back through the bridge.
- **Verify:** Playwright.
- **Moves:** the WebOS integration is complete.

### C7.13 — GPU renderer: research decision `[ ]`
- **Needs:** C8.07 · **Tags:** `[DOCS]` · **Size:** S
- **Steps:**
  1. Measure the CPU `compose` frame time on a 4000×3000 document with 50
     objects and filters.
  2. Write a decision on whether to move to wgpu and WebGPU, and which parts:
     the compositor only, filters, or everything. Include wasm size, browser
     support and effort phases.
  3. **No implementation in this rung.**
- **Done when:** the decision is merged with measured numbers.
- **Verify:** review.
- **Moves:** informs the post-v1 work.

### C7.14 — SVG fidelity floor `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Run `cargo run -p xtask -- svg-golden --measure`.
  2. Add 10 permissively licensed real-world SVGs (icons, illustrations),
     each with an `ATTRIBUTION.md` row.
  3. Fix the worst failure.
  4. Raise the floor.
- **Done when:** the floor is raised and the new fixtures pass.
- **Verify:** `cargo run -p xtask -- svg-golden`.
- **Moves:** modern; S6 ↑.

### C7.15 — PDF and AI import (Illustrator ≥ 9 is PDF-based) `[ ]`
- **Needs:** C4.15 · **Tags:** `[HOST:linux]` · **Size:** L
- **Steps:**
  1. Write a decision on a pure-Rust PDF reader (for example `lopdf`) plus a
     content-stream interpreter covering paths, fills, strokes and images.
     Text becomes paths or is skipped with a warning.
  2. `.ai` files with embedded PDF use the same path. Legacy PostScript AI
     is refused with a clear message.
  3. Add fuzz anchors for the parser entry points.
- **Done when:** a vector PDF page imports as editable paths, and the fuzz
  anchors run.
- **Verify:** `cargo test -p io pdf_import`.
- **Moves:** S5c −1 (AUDIT §7 gap #14).

### C7.16 — Cloud-free collaboration groundwork: document diff `[ ]`
- **Needs:** C7.04 · **Tags:** `[DOCS]` · **Size:** S
- **Steps:**
  1. Write a decision on whether the journal (`journal.rs`) can serve as an
     operation log for future collaborative editing. Cover determinism, IDs
     and conflicts.
  2. **No implementation.** This rung records the direction for after v1.
- **Done when:** the decision is merged.
- **Verify:** review.
- **Moves:** —

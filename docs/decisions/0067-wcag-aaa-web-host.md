# 0067 — WCAG 2.1 AAA audit of the web host (2026-10-25)

## Scope and honest limits

The web host is an egui canvas driven by WebAssembly. Two AAA criteria
are structurally unreachable for a canvas-rendered editor and are
recorded here as refusals, not conformance:

- **1.4.8 Visual Presentation (AAA)** — the canvas draws its own text;
  user-side text reflow/resizing (browser zoom of text alone) cannot
  apply inside the canvas. egui's own `pixels_per_point` scaling is
  the offered substitute; a preference for it is future work.
- **2.1.3 Keyboard (No Exception)** — egui routes keyboard through
  the canvas; every known interaction has a keyboard path, but "no
  exception" requires proving the absence of traps across the whole
  surface. Verified for the main paths (tools via shortcuts, menus
  via keyboard navigation); recorded as *partially verified*, not
  certified.

## What was audited and fixed

| Item | Criterion | Finding | Fix |
|---|---|---|---|
| `web/index.html` `outline: none` on canvas | 2.4.7 Focus Visible (AA, floor for AAA path) | Focus ring suppressed | `:focus-visible` 2px #7ab8ff outline |
| Canvas fallback content | 1.1.1 Non-text Content (A) | Empty canvas, no fallback | `aria-label` + `aria-describedby` + readable fallback text with a link to the desktop build |
| Meta description | best practice | Missing | Added |
| Theme text on panel fill 0x3c3c3c | 1.4.6 Contrast AAA (≥7:1) | 7.81:1 pass | — |
| Theme text on window fill 0x323232 | 1.4.6 | 9.08:1 pass | — |
| Theme text on extreme bg 0x282828 | 1.4.6 | 10.44:1 pass | — |
| Theme text on faint bg 0x444444 | 1.4.6 | **6.90:1 FAIL** | text raised 0xd9→0xdd (7.17:1), both call sites |
| autoshape panel ink 0xd9 on 0x4a | 1.4.6 | same class | raised to 0xdd with the theme |

## Accesskit

**Corrected 2026-10-29 (code audit): NOT reachable on web.** The
earlier claim ("the accessibility tree ships in the wasm bundle") was
wrong. Evidence in the pinned dependencies:

- egui 0.36 builds the AccessKit tree only when
  `Context::enable_accesskit()` has been called; eframe calls it only
  from the **native** winit path (`eframe/src/native/winit_integration.rs:180`).
- The **web** runner's event loop discards the tree updates outright:
  `eframe/src/web/app_runner.rs:394` — `accesskit_update: _,  // not
  currently implemented`.
- `accesskit_winit` (the only adapter in the tree) is winit-specific;
  there is no web/DOM adapter in eframe 0.36.

So on wasm the canvas is opaque to screen readers regardless of any
feature flag we could enable in this repo. Criteria affected:
**1.1.1 Non-text Content** (canvas fallback content in
`web/index.html` remains the only reachable accommodation) and
**4.1.2 Name, Role, Value** (unreachable inside the canvas). This is
recorded as a refusal pending an upstream eframe web-accesskit
implementation — not conformance. The `web_screen_reader` feature
(SpeechSynthesis event narration) is orthogonal and available; enabling
egui's `screen_reader` option is a separate candidate.

Full screen-reader verification on the deployed site remains open
human work and cannot change this verdict: there is no tree for a
screen reader to consume.

## Verification

Contrast ratios computed with the WCAG relative-luminance formula
(sRGB linearization), not approximated. The host-page changes are
validated by `xtask web-golden` (structural distance 0 required) and
the dist bridge smoke.

# Fireworks CS6 Parity Roadmap

This page collects the currently documented work toward Fireworks CS6 feature and UI/UX/layout parity. It separates parity work from FireCraft's modern additions. The sequence is a practical work order; individual phases may move when evidence or dependencies require it.

## Ordered work at a glance

Status below is the audited source snapshot at `5fac7a2` (2026-11-04). Check [Project Status](Project-Status) for the current deployment and validation evidence.

| Order | Work | State |
|---|---|---|
| 1 | Finish CS6 shell presentation and responsive layout (Phase 22) | Active; panel reachability and screenshot review lead |
| Alongside | Refresh the complete user-visible parity inventory | Needed before any full-parity claim; DOM parity alone does not cover UI/workflows |
| 2 | Complete daily-driver workflows (Phase 20) | Queued |
| 3 | Add modern FireCraft capabilities (Phase 21) | Queued; separate from CS6 parity |

## Cross-cutting gate: complete the parity inventory

1. **Refresh the user-visible parity matrix.** Map the CS6 menus, tools, panels, inspectors, document workflows, and important selection states to `implemented`, `partial`, `missing`, or `evidence-gated` behavior. Start this audit alongside the active shell work; finish it before claiming full parity.
2. **Keep source evidence attached.** For each gap, link the code path, research note, decision, screenshot, or behavior observation that supports the status. The existing DOM parity matrix covers scripting verbs; it is not a complete UI or workflow parity inventory.
3. **Protect the clean-room boundary.** Use documentation and behavioral observation. Do not copy Adobe code, icons, or artwork.

Until the inventory refresh is complete, this roadmap is the known backlog, not a proof that no other gaps exist. Track detailed phase criteria in [NEXT.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md).

## Then: finish the visible shell (Phase 22)

This is the current UI priority. The context menu and open-license icon pack are complete; the remaining work is:

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:f4fd3ecf7aeee82ab02450c0773aca0a8b3c825c71dd41fdae98bc0cad0612a6 -->

1. **Panel reachability and layout (22.1):** make active panels usable at normal viewport sizes, finish the documented Optimize, Common Library, CSS Properties, Find and Replace, and Slices surfaces, and verify persistence and movement.
2. **Tool strip and flyouts (22.2):** align tool order, grouping, flyout behavior, and keyboard access with the reference notes.
3. **Property inspector (22.4):** finish per-object fields and close the documented bitmap transform/resample gap (decision 0019); X/Y/W/H remains read-only. Bitmap fill/stroke editing requires a separate format decision because `IMGv` carries no fill or stroke nodes. See the [bitmap evidence](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/research/fw-ui-evidence.md#where-the-bitmap-specific-surface-lives-phase-22-ticket-224).
4. **CS6 chrome (22.5):** review menu, tabs, status area, panel headers, and the overall shell layout.
5. **Screenshot and responsive proof:** capture representative empty, vector, and bitmap states at 1280×800 and 1920×1080; record the build and replace the stale sample only after review.

<!-- /coderabbit-ai-generated:v1 -->

Detailed acceptance criteria are in [Phase 22](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md#phase-22--cs6-shell-fidelity-p0-the-looks-and-acts-like-fireworks-phase); release and gate receipts are in [PROGRESS.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PROGRESS.md).

## Next: close daily-driver feature gaps (Phase 20)

1. **In-canvas text editing (20.1)**
2. **Optimize panel and live export preview (20.2)**
3. **Animation timeline, frame timing, and onion skinning (20.3)**
4. **Mask stack editing (20.4)**
5. **Multi-document tabs (20.5)**
6. **History panel rewind (20.6)**
7. **Slice/hotspot behavior editing and HTML preview (20.7)**

## Then: add modern capabilities (Phase 21)

These items extend the discontinued product rather than reproduce a CS6 feature. They should remain clearly labeled as FireCraft additions:

1. Autosave and crash recovery (21.1)
2. HiDPI rendering and display-scale handling (21.2)
3. Command palette improvements (21.3)
4. Preferences and platform storage (21.4)
5. Accessibility re-audit and supported accesskit surfaces (21.5)
6. In-app scripting access (21.6)

## Known menu and model gaps

- 8 of 289 menu rows remain non-dispatching. The menu wiring campaign is complete; each remaining row needs its feature or evidence before it can be wired honestly.
- The DOM verb matrix has 3 partial, evidence-gated verbs and 85 entries classified as not applicable to the current API surface. See the [generated parity matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/planning/generated/dom-parity.md).
- For detailed phase criteria and dated validation receipts, use [NEXT.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md) and [PROGRESS.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PROGRESS.md).

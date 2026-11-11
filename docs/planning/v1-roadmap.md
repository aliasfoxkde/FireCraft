# FireCraft — v1 MVP roadmap

> What "v1" means, where the tree stands today, and the ordered path
> from `0.2.5` to a `1.0` that is honest about CS6 parity.
>
> Tier definitions are canonical in [`../PLANNING.md`](../PLANNING.md)
> §8; milestone gates in [`firecraft/ROADMAP.md`](firecraft/ROADMAP.md);
> the *artifact* MVP lane (which builds and signs the bits) lives in the
> release campaign docs — this file is the *feature* view and does not
> restate either.

## 1. The v1 definition

**v1 = T0 + T1 + T2 complete, T3's core web/animation/scripting
workflows complete, and a certified release matrix.** Concretely:

| Gate | Contents |
|---|---|
| **Feature gate** | Every item of tiers T0, T1, T2 done; the T3 items CS6 users hit daily (behaviors editing, CSS Properties panel, animation timeline) done; explicit scope calls recorded for the rest of T3. |
| **Parity gate** | Menu registry at **0 inert rows** (4 evidence-gated rows excepted), parity floors rising-only and met, `xtask ci` green native + wasm. |
| **Release gate** | The MVP artifact lane (Web/WASM + Linux tar/DEB/CLI + source + checksums + SBOM + verified deploy) **certified**, plus at least one executed build each for Windows and macOS, per the release campaign's platform ladder. |

Deliberately **not** in v1 (post-1.0 polish, per PLANNING §8): the T4
extras — i18n, plugin surface, HEIC OS decoders, JXL decode — and the
T3 formats with no daily-driver weight (FXG, PDF export) unless the
scope call in §4 flips them.

## 2. Where the tree stands (2026-11-11, the release train —
`integration/release-train`)

| Tier | State | Evidence |
|---|---|---|
| **T0 — MVP** | **Done** | M5/M8 gates: open→render→save `.fw.png` native + browser, primitives/pen, gradients, layers/pages/states, select/subselect/transform, undo, guides/rulers, PNG/JPEG/GIF — all landed; wasm smoke floors in `generated/parity.md`. |
| **T1 — Core parity** | **Done** (1 residual) | Auto Shapes, path ops, text-on-path, 27 blend modes, masks (engine), live filters, smart guides, symbols, 9-slice, Styles panel. Residual: **mask-stack UI** (Phase 20.4). |
| **T2 — Bitmap + formats** | **~9/10** | Selections/retouch, full filter set, PSD, SVG, TIFF/BMP/WBMP, WebP, slices/export workflows, batch, `.jsf` replay all done. Missing: **AVIF decode** (decision 0064, deliberately unimplemented). Two filter items parked behind a CS6 oracle (documented policy). |
| **T3 — Web/animation/scripting** | **~6/11** | Done: rich/animation symbols, states→GIF, CSS+Images/Sprites, HTML export, MCP. Partial: behaviors (engine yes, editing + HTML preview no), CSS Properties (workflow yes, panel no). Not started: FXG, PDF export, RPC emulation. |
| **T4 — Modern/polish** | **1/9** (expected) | Perf budgets done. WebOS bridge proven but device smoke unverified; packaging/Windows/macOS recipes executed only on rung 1. Rest is post-v1 by design. |

**Release matrix** (the shipping half of v1): Linux tar/DEB/CLI and
the WASM bundle are published; AppImage/Flatpak/Windows/macOS are
recipes never executed; nothing is signed; no artifact has reached
`CERTIFIED` (audit ladder, `docs/release/PHASE-0-AUDIT.md` §rungs).
All six releases to date were operator-published by hand.

**Engine surface.** The command registry carries **253** commands on
the release train (`registry.register(` census over
`crates/engine/src/catalog/`); `engine::catalog::ids()` is the live
authority and `xtask reachability` prints it. The number the older
docs quote (245) was `main`'s count before the train landed.

**Live phases** (19 canvas · 20 daily-driver · 21 modern · 22 shell
fidelity, `docs/NEXT.md`): 19 complete; the 20/21/22 completions sit
on the release train and burn the inert-menu count from 8 down to
**2** (`menus::tests::INERT_CEILING`). The train's open PRs (#8, #13,
#15, #41, #83–#85, #100–#106) are all ancestors of the train head, so
landing the train lands them.

## 3. The ordered path

Dependency-ordered; each row is independently gated by `xtask ci`.

1. **Land the release train** — `integration/release-train` (PR #108)
   into `integration/shell-lineage` (PR #107) into `main`. The train
   is the superset: every open PR (#8, #13, #15, #41, #83–#85,
   #100–#106) is an ancestor of its head, so landing it lands them
   all. It imports Phases 20.1/20.5/20.6, 21.1–21.6 and 22.2/22.4
   work, the inert burn-down to 2, and the release-engineering lane.
   Everything below builds on it.
2. **Close the Phase 20 remainder** — 20.2 Optimize panel (T7),
   20.3 animation timeline (T8), 20.4 mask-stack UI (T1 residual),
   20.7 slice behaviors + HTML preview (T3 residual). Order among
   these is free; 20.4 is the only *tier* blocker.
3. **Close Phase 22** — remaining panels (Optimize, Common Library,
   CSS Properties, Find-and-Replace, Slices), 22.5 chrome, 22.6 icon
   pack close-out → retires the last inert rows → **parity gate met**.
4. **T2's AVIF** — one decode branch in `io::raster::decode`
   (`ravif`/`dav1d`-class crate, wasm-safe), per decision 0064.
5. **Ratify the T3 scope call (§4)** and implement whatever flips in.
6. **Release engineering** — execute the MVP artifact lane to
   certification (source archive, SBOM, checksums, verified deploy),
   rung up AppImage + Windows + macOS from recipe to executed build,
   WebOS device smoke, then signing/attestation → first `CERTIFIED`
   artifact. *This, not features, is the long pole.*
7. **Ship `1.0.0`** — tag, release notes vs CS6, both remotes,
   Pages deploy verified against the tag hash.

## 4. Scope calls to ratify before step 5

| Item | Default for v1 | Why |
|---|---|---|
| **AVIF decode** | **In** | It is a T2 tier item; decision 0064 already scoped it to a single decode branch. |
| **FXG export** | **Out** | CS6-era interchange nobody migrates through in 2026; no milestone ticket; revisit post-1.0. |
| **PDF export** | **Out** | Deferred by M8's own ticket to T3; heavy dependency for a parity-niche format. |
| **RPC emulation (TCP 12124)** | **Out for 1.0, In for 1.x** | Desktop-only legacy automation; MCP + `.jsf` cover the scripting surface v1 users actually touch. Track as the first 1.x item. |

If any default flips, the row becomes a Phase 20-style ticket before
step 6 — the release gate does not wait on it.

## 5. Non-goals (unchanged from PLANNING §8)

Flash-era exports (SWF/AIR/Flex/MXML/Director/Device Central), `.mxp`
Extension Manager packaging, `.8bf` hosting, PDF *import*. The modern
wins over CS6 stay: WebOS embedding, MCP, autosave/crash recovery,
HiDPI, command palette.

## 6. Relationship to the other plans

- [`../PLANNING.md`](../PLANNING.md) §8 — tier contents (canonical).
- [`firecraft/ROADMAP.md`](firecraft/ROADMAP.md) — milestone gates;
  M0–M15 closed, M16/M17 superseded by the NEXT phases.
- [`../NEXT.md`](../NEXT.md) — the live backlog this roadmap draws
  steps 1–3 from.
- Release campaign docs — the *artifact* MVP lane and platform ladder;
  this roadmap's release gate is defined against them.
- [`../release/PHASE-0-AUDIT.md`](../release/PHASE-0-AUDIT.md) — the
  evidence baseline every claim above is dated against.

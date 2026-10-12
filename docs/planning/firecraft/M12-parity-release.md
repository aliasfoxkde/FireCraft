# M12 — Parity hardening & release matrix

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** FireCraft ships. The parity/scorecard machinery stops
being report-only and starts enforcing recorded budgets; performance
gets measured numbers with ceilings (cold start, pan-zoom, save,
memory); `xtask dist` assembles the release matrix — WASM-first (the
static web bundle for the user's WebOS), then the Linux native set —
with reproducible recipes for the platforms this fleet cannot build;
and the release chore (changelog, license files, version) lands.
**Gate:** `xtask ci` green with the perf gate enforcing recorded
budgets; a `xtask dist` run produces the web bundle + Linux archives
from a clean tree; the WebOS embedding guide describes a working
iframe/postMessage contract against the M5 bridge; scorecard M12 row →
done with gate evidence.

**Inputs:** the M0–M11 gates (all green) and their floors; the
`compose` bench (`benches/render.rs`, 18.7 µs compose / 40.1 µs
pipeline at 16×16, M4) as the measurement precedent; the corpus
fixtures as perf-document stand-ins; `web/` (trunk bundle,
postMessage bridge: openDocument/apply/exportPng/exportDocument, M5);
PLANNING §2 (delivery targets: WASM first, then Linux tar/AppImage/deb,
macOS universal dmg, Windows installer + portable, CLI archives),
§8 T4 row, §11.5 (releases: tagged, changelog from conventional
commits); the dual LICENSE-MIT/LICENSE-APACHE files (already in tree);
the fleet reality (Linux x86_64 + docker runners — no macOS SDK, no
Windows toolchain guarantee).

## Design line: budgets are gates, dist is a pure function of the tree

A budget that only reports is a wish. Every number M12 records —
startup, pan-zoom frame, save, peak memory, bundle size — becomes a
ceiling `xtask perf` measures and enforces, recorded with observed
values and headroom like the svg-golden tolerances, rising-only in the
parity table (a ceiling only moves down; raising one reopens the
milestone that set it). Measurement is honest about noise: warm-cache
runs, median-of-N, and the recorded host class in the report, so a
budget failure means a regression, not a co-tenant CPU spike.

The dist pipeline is the same discipline as export: a pure function
`(&tree, profile) -> artifacts`. `xtask dist` builds release binaries
and the trunk bundle from the committed tree, assembles per-target
archives with pinned layouts, and sha256-manifests everything it
writes — no hand-assembled releases. Platforms this fleet cannot build
(macOS universal, Windows installer) ship as *verified recipes*:
scripts checked into the tree with their toolchain requirements
documented, executed by whoever releases from those platforms —
the repo never carries an untested claim of a built artifact.

## Tasks

1. **Perf gate** — `xtask perf [--bless]`: measure cold open (read +
   first compose render of a corpus fixture), pan-zoom frame (view
   transform + re-render of the composed fixture at 64×48 and a
   generated stress doc), save (canonical write of the same docs), and
   peak memory (VmHWM around the open path). Median-of-N, warm cache,
   host class recorded. Budgets pinned with headroom in
   `xtask/src/perf.rs`, enforced in `ci`, floor rows in the parity
   table. **Stopping condition:** budgets recorded from observation,
   gate enforces them, a deliberately slowed path (debug compose in
   the measurement harness) fails the gate in a scratch run.
2. **Memory ceiling** — the open path's peak-RSS ceiling on a
   generated 2,000-object document (no bitmap payload): assert the
   ceiling in `xtask perf`, not just report it. **Stopping
   condition:** the ceiling holds on the fleet runners (CI log
   evidence) and the number is in the parity table.
3. **Web bundle release** — `xtask dist --web`: trunk `--release`
   build of `web/`, assembled under `dist/web/` with the bundle's
   sha256 manifest and a size budget (the wasm binary's recorded
   ceiling from task 1's numbers); the bundle opens a corpus fixture
   through the postMessage bridge in a headless smoke (the M5 bridge
   contract exercised on release bytes). **Stopping condition:**
   `dist/web/` assembles from a clean tree, manifest matches bytes,
   size under budget, bridge smoke green.
4. **WebOS embedding guide** — `docs/WEBOS.md`: the iframe/postMessage
   contract (message shapes from the M5 bridge, origin rules), the
   WebOS appinfo/shelf packaging for a static bundle, OPFS autosave
   notes, and a worked minimal host page. **Stopping condition:** the
   guide's host-page snippet, verbatim, loads the dist bundle and
   completes one open→apply→export round trip through the bridge in
   the smoke harness.
5. **Native + CLI packaging** — `xtask dist --native`: Linux x86_64
   tar.gz of the desktop shell + `firecraft-cli`, a `.deb` built from
   a pinned control file, and CLI archives; AppImage as a checked-in
   recipe (linuxdeploy presence is host-dependent). macOS universal
   dmg and Windows installer + portable: checked-in scripts +
   toolchain docs (the no-untested-claims rule), built off-fleet.
   **Stopping condition:** a clean-tree run produces tar.gz + deb +
   CLI archives with a sha256 manifest, `dpkg-deb --info` reads the
   deb, and the recipe scripts parse on their target platforms'
   documented shells.
6. **Release chore** — changelog generation from conventional commits
   (`xtask changelog`), version stamp in the workspace `Cargo.toml`s
   reviewed, license files verified against the dual-claim, release
   checklist in `docs/RELEASING.md` (the dist verb sequence + where
   artifacts land + the no-untested-claims rule). **Stopping
   condition:** `xtask changelog` output matches the git log for
   M0–M12; the checklist names every artifact `xtask dist` emits;
   scorecard M12 → done with the full gate + dist evidence.

## Non-goals (M12)

- Cross-compiling macOS/Windows *on this fleet*: no SDK borrowing, no
  unverified recipe claims — scripts + docs only.
- New user-facing features: M12 hardens and ships what M0–M11 built.
- CD/signed releases: artifacts are local; signing ceremony is the
  operator's, documented not automated.
- Installers beyond the deb on Linux (rpm/flatpak: later, demand-driven).

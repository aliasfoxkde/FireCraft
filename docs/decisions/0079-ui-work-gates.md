# 0079 — The UI-work gates measure the CS6-visible surface and pin structure, not pixels

Date: 2026-11-10 · Ticket: [#94](https://github.com/aliasfoxkde/FireCraft/issues/94) · Status: accepted

## Context

`docs/NEXT.md` asked for two gates on UI work: a **reachability gate**
(the set difference `catalog::ids()` − UI-bound commands may only
shrink) and a **panel snapshot suite** (`egui_kittest` snapshots per
panel/toolbar). Its quality-gate list also contradicted its own phase
notes: 19b.2/19b.3 were marked LANDED while the same list called the
reachability gate, the inert-drift test and the wasm budget open.

Two design questions had to be answered before any code:

1. **What does "reachable" mean?** The command palette (19.4) executes
   *every* registry command from the GUI. A metric that counts the
   palette is 100% by construction and can never fail — a gate that
   cannot fail is not a gate. The historical numbers in the docs
   ("188 commands unreachable", `comm -23 <(catalog::ids()) <(command
   names in crates/app/src)`) meant something narrower: the commands
   a user can reach through the surface CS6 actually shows — the menu
   bar, the panels, the dialogs, the canvas tools.

2. **Can the panel snapshot use `egui_kittest`?** It was checked
   before any code was written (2026-11-10):

   - It is *obtainable*. `egui_kittest` 0.36.2 is on crates.io and
     requires `egui ^0.36.2` — exactly the egui this repository's
     lockfile pins. The CI image is not offline either: it runs with
     `CARGO_NET_OFFLINE=false` (decision 0073 removed the wall; the
     baked registry cache is a fast floor, not a wall), so a crate the
     cache lacks is fetched, not refused.
   - Its *pixel* path is the blocker. The snapshot machinery sits
     behind the crate's optional `wgpu` / `egui-wgpu` features, and
     the CI image installs no graphics stack (no mesa, no Vulkan
     drivers — the image carries the toolchain, node and a headless
     chromium for the web smoke, nothing for wgpu). A wgpu adapter is
     therefore not available in the sandbox, and a pixel suite would
     fail there for want of an adapter rather than for want of a
     crate. Adding it would also move the workspace lockfile and pull
     `kittest`, `toml` and `wgpu` into every `cargo test --workspace`
     build for a gate that cannot run there yet.

## Decision

**The reachability gate measures the CS6-visible surface.** The metric
is the share of `engine::catalog::ids()` the shell crate binds: the
command-id string literals it hands the engine, collected from
`crates/app/src` with `#[cfg(test)]` code excluded. The palette is
deliberately outside the measurement — it is a power-user door, not
the surface the 19.x batches widen. The number is *derived* from the
source, so a binding that is added shows up without a second edit and
a binding that is deleted shows up as a hole.

The floor lives in a committed file, `.reachability-floor.tsv`, read
by the gate and regenerated only by `--bless` — the same shape as the
Aegis baseline (decision 0074). It is a **one-way ratchet**: a bless
may raise it and refuses to lower it. A removed binding fails the
build; a regression is fixed, not forgiven.

**The panel snapshot is pinned with the repository's own golden-file
machinery**, not with `egui_kittest` — whose pixel path needs a wgpu
adapter the CI image cannot provide, and whose structural value the
golden file already carries. The xtask crate now reads the shell's
own inventory (`app::shell::panel_inventory()`) and compares a
rendered snapshot against the committed golden
`crates/app/panels.snapshot`. The Rust `PANELS` table stays the single
authority for what the panel set *is*; the golden pins its
*structure* — id, dock side, dock order, header text, fresh-document
visibility, Window-menu togglability.

## Consequences

- The gate is meaningful: it can fail. A command removed from the
  shell's bindings drops the share below the floor and fails
  `cargo run -p xtask -- ci`.
- The measurement is derived, so it cannot drift from the code the way
  a hand-quoted count did. The measured value on `main` at
  2026-11-10 is **245 registry commands, 146 bound by the shell =
  59.59%**; 99 are unbound.
- The floor is a *share*, not a raw count: a new command that nobody
  binds also lowers it, which is the intended pressure.
- The scan is a literal scan, so a command id mentioned in a comment
  would count as bound. No id is bound that way today (all 146 sit in
  code), and such a mention raises the share, which the ratchet then
  pins — the failure mode is a floor that is slightly too generous,
  never a floor that hides a regression.
- Panel *pixels* stay unpinned. A theme or WCAG regression inside a
  panel is still reviewed as code, not as a golden diff. Closing that
  half means adding `egui_kittest` (0.36.2 matches the locked egui)
  *and* proving a software wgpu adapter in the CI image — the
  dependency is obtainable today, the renderer is not.
- The panel snapshot depends on the `app` crate, so `xtask` links
  egui. The workspace already builds that crate, and the layer gate
  allows `xtask` (L7) to depend on `app` (L5), so no tier is
  violated.
- `Cargo.lock` moves: `xtask` gains one path dependency. It is an
  in-workspace crate, so no registry crate is added and the CI image
  still resolves the build.
- The docs now say one true thing. `NEXT.md` §19b.2 records the gate
  with its measured numbers, the quality-gate list points at
  19b.2/19b.3/19b.4 as done, and the snapshot item records what was
  checked about `egui_kittest` and which half of it is still open.

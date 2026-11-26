# BonBon / Libreworks product benchmark

**Reviewed:** 2026-11-25. **Purpose:** extract product and interaction ideas for FireCraft; this is not a code or asset port plan.

Further reference: the [`bomberstudios/fireworks` command library review](fireworks-command-library-benchmark.md) examines workflow-level commands for page export, sizing, opacity, grids and tutorials.

BonBon (the README currently brands the editor Libreworks, with a Bonbon-branded build) describes itself as a native Linux image editor following the Macromedia Fireworks MX 2004 workflow. Its current repository is C++17/Qt 6 and GPL-3.0. The repository README and latest release are the primary references: [BonBon source](https://github.com/librebon/BonBon), [README](https://github.com/librebon/BonBon/blob/main/README.md), [latest release](https://github.com/librebon/BonBon/releases/latest). At review time the latest release page advertised `bonbon-0.3.37-Linux` with tar.gz, AppImage and Debian packages. These facts change over time; recheck before making compatibility or feature claims.

## Product patterns worth studying

| Pattern | BonBon evidence | FireCraft opportunity | Suggested evidence before implementation |
|---|---|---|---|
| One shortcut registry | README describes customizable tool and command bindings, built-in defaults, custom-set migration, explicit unbinds, live tooltips and Help bindings sharing the same sequences source | FireCraft already has generated shortcut sets and Help; close the gap between keymap selection and actual editable bindings, and make all UI hints derive from the active map | Test remapping a tool while canvas has focus, conflict resolution, import/reset migration, and tooltip/help/menu consistency |
| Paste placement preference | Optional paste-at-mouse-location, with Paste in Place unaffected | Consider a preference for pointer-centered paste while retaining deterministic paste-in-place behavior | Multi-object, bitmap and vector cases; pointer outside canvas; undo and cross-platform clipboard behavior |
| Complete creative asset outputs | Explicit favicon/app-icon output set with multi-size previews and a generated HTML snippet | Build on FireCraft's export/web workflows with named output recipes, preview grids, metadata and copyable integration snippets | Verify output dimensions, alpha, SVG fallback, reproducibility and overwrite prompts |
| Visible format behavior | README lists import/export formats and documents fidelity limits such as flattened Fireworks PNG imports and layered-format flattening conditions | Maintain a capability matrix by format and editability class in dialogs, Help and feature docs | Round-trip fixtures and per-format preservation/loss tests; never infer editability from extension alone |
| Consistent icon rendering | Bundles a known icon theme subset and tests that package output matches development rendering; provides a system-theme opt-in | Review FireCraft icon consistency across OS themes and native/web builds; prefer FireCraft-owned glyphs for critical controls | Screenshot matrix on Linux, Windows and WASM in light/dark themes; verify disabled/hover/focus states |
| Settings and shortcut migration | Versioned override-only shortcut sets, missing-value inheritance, explicit unbind semantics, import/export | Preserve compatibility as preferences evolve; avoid silently replacing defaults when settings are absent | Old-settings fixtures, malformed-settings handling, export/import identity and migration notes |
| Large but candid capability catalog | README documents extensive formats, tools and known limitations in the same place | Make FireCraft's feature list detailed but keep shipped, partial and planned status visually distinct | Every item links to a source, test, release, or an explicit unverified note |

## Gaps and cautions

- BonBon/Libreworks is Linux-focused and explicitly documents GUI/platform limitations. Its behavior is a useful reference, not proof that a UI pattern transfers directly to Windows or WASM.
- Its README advertises far more interchange formats and image processing than FireCraft currently claims. Prioritize by user workflows and fidelity evidence, not by matching a feature count.
- The project license is GPL-3.0. FireCraft's licensing and clean-room rules remain authoritative: use public behavior, documentation and independently created assets as references; do not copy implementation code, icons, artwork or bundled resources.
- Some capabilities have disclosed constraints (for example, bit-depth reduction, partial layered-format fidelity and flattening of unsupported structures). FireCraft should adopt that style of honest capability disclosure even where it does not adopt the feature.

## Proposed work sequence

1. Close the active experiment integration and GUI acceptance checks before adding new engine scope.
2. Audit FireCraft's shortcut flow against BonBon's registry consistency pattern; retain FireCraft's CS6 defaults and generated table.
3. Prototype pointer-centered paste as an optional preference after clipboard behavior is stable.
4. Specify export recipes and a format capability registry separately; measure expected compatibility and fixture cost before choosing formats.
5. Build an icon consistency screenshot matrix and resolve the most visible cross-platform drift.
6. Keep advanced color/HDR and format expansion behind a documented color-depth and editability plan.

## Research limitations

This is a repository-documentation comparison, not a full runtime usability review. No BonBon binary was installed or tested during this pass. The primary README is extensive and changes frequently; a versioned source snapshot should be used for any later detailed menu-by-menu comparison.

# Fireworks command library benchmark

**Reviewed:** 2026-11-25. **Reference:** [bomberstudios/fireworks](https://github.com/bomberstudios/fireworks), presented as *Orange Commands v1.7.2 for Adobe Fireworks* and support files rather than a standalone editor.

The repository README groups practical commands and explains their exact effect and usage. Its visible categories include alignment, alpha, canvas, color, effects, export and export settings, grids/guides, layers, pages, position, properties, selection, size, states and text. Examples include exact object/stroke opacity presets, math-capable size changes, guide copying between pages, page duplication/renaming/trimming, bulk page export and CSS sprite output. The README also describes installing custom keyboard sets and flags OS-level shortcut conflicts. These are behavioral/product observations from the public [repository](https://github.com/bomberstudios/fireworks) and [README](https://github.com/bomberstudios/fireworks/blob/master/README.markdown).

## Transferable ideas

- **Organize by user intent.** Browse commands as “align,” “set opacity,” “make a grid,” “export all pages,” or “change all selected text,” with clear descriptions and examples. FireCraft's registry IDs can remain stable implementation names while the palette gains human-readable names, aliases and categories.
- **Expose repeated production work as recipes.** Multi-page image export, naming rules, prefixes and output-folder choices suggest a FireCraft export-preset or batch-export workflow. Reuse existing export semantics and report per-file success/failure.
- **Make exact-value operations discoverable.** Common percent presets, fixed increments, aspect ratios and “fit to” sizing map well to FireCraft commands and palette aliases, provided each operation routes through the undoable command registry.
- **Treat keyboard configuration as part of discoverability.** Show shortcuts in menus, tooltips, tutorials and palette search from the same active binding source; document platform conflicts where relevant.
- **Explain operations in context.** The source README gives a short explanation per command. FireCraft can use descriptions and contextual tips without reproducing the original scripts.

## Clean-room and licensing note

The project is explicitly support material for Adobe Fireworks and links downloadable MXP command packages and custom keymaps. The file named `LICENSE` exists in the repository, but this review did not validate its terms or inspect package contents. Do not copy the scripts, keymap files, Adobe artwork, or implementation details into FireCraft. Use only the high-level workflow ideas above, then implement independently against FireCraft's own model and command APIs. Check the actual license and any third-party notices before any deeper reuse is considered.

## Proposed FireCraft follow-up

1. Add user-facing descriptions and aliases for the palette's highest-use commands (open/save/export, selection, vector shapes, bitmap tools, layers/pages, guides, transforms, filters).
2. Rank palette hits by exact label, word prefix, alias, then subsequence, with recents retained as a convenience rather than a substitute for search.
3. Add tutorials for batch/page export, exact sizing, opacity, guides/grids, text cleanup and reusable command recipes.
4. Design batch export around current supported formats and verify output names, overwrite behavior, cancellation, progress and partial failures.
5. Keep third-party command catalogs as research links; do not import their script files or shortcut maps.

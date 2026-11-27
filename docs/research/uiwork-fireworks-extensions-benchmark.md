# UIWORK FireworksExtensions benchmark

**Reviewed:** 2026-11-25. **Reference:** [UIWORK/FireworksExtensions](https://github.com/UIWORK/FireworksExtensions). The repository presents extensions for Adobe Fireworks, with `Commands`, `Command Panel` and `Working` directories. The GitHub page showed 35 commits and no published releases at review time. Example command names include `ButtonsExports.jsf`, `ClockwiseClone.jsf`, `GIFFrames.jsf`, `Quick Text.jsf`, `Randomize.jsf` and `Save as PSD.jsf`.

## Product insight

The examples suggest two useful extension shapes: small, direct commands for one repeatable task and persistent panels for workflows that need ongoing controls. Names such as “Quick Text” and “Save as PSD” describe user intent more clearly than internal command identifiers. Batch export and frame-oriented commands also reinforce Fireworks' production focus.

For FireCraft, this supports a palette and extension catalog organized around user tasks, with concise descriptions, aliases, category, requirements, shortcut and undo behavior. A command should say what it changes and report unavailable prerequisites. Persistent panels should be reserved for workflows where state needs to remain visible, rather than turning every action into a panel.

## Follow-up

- Add aliases and plain-language descriptions to palette entries, while retaining exact registry IDs as canonical identities.
- Consider focused commands for common production steps only where the shared engine can implement them reliably.
- Expand documented extension examples after the SDK and permission model exist.

## Limits

This is behavioral/product research, not a code-reuse proposal. The repository page did not establish a license in the reviewed material, so do not reuse its code or assets without a separate license review.

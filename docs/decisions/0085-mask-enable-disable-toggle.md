# 0085 — Modify ▸ Mask enable/disable is one toggle row

Date: 2026-11-10 · Status: accepted

## Context

CS6's Modify ▸ Mask submenu has exactly one enable/disable row, and
it is not two rows with static labels. The evidence is in the
reference install's resource DLL
(`source/Adobe Fireworks CS6/English/Resources/Fireworks Resources.dll`):

- The submenu's string block is `&Mask`, `&Reveal All`, `&Hide All`,
  `&Paste as Mask`, `&Group as Mask`, `R&eveal Selection`, `H&ide
  Selection`, `&Disable Mask`, `De&lete Mask`, `Selective &JPEG`,
  `&Save Selection as JPEG Mask`, `&Restore JPEG Mask as Selection`,
  `S&ettings...`, `Re&move JPEG Mask`, `&Lock Selection`,
  `&Flatten Selection`. There is **no** `&Enable Mask` row.
- A second, unprefixed label pool carries `Disable Mask` and — once —
  `Enable Mask`, beside the other command labels (`Auto Shape`, `Join
  Paths`, `Split Paths`, `Tint`, `Transform`, `Convert to Symbol`,
  `Break Links`, `Tween Instances`, `None`, `Crop Paths`,
  `Color Change`). `Fireworks.exe` carries `Disable Mask` but **not**
  `Enable Mask`. The label that appears only in the DLL is the one
  CS6 swaps in at runtime.
- The description-table entry for that row reads `Enable or Disable
  Mask`, with the tooltip `Toggle use of the mask for the selected
  object`.
- The keymap has one command for it: `cmd_Mask_EnableDisable`,
  `count="1"`, bound to `6 75` — Ctrl+Shift+K.

So CS6 ships one row whose visible label flips between "Enable Mask"
and "Disable Mask" with the mask's state, and one key that toggles it.

FireCraft could not express that. The menu tree is static data
(`Item.label: &'static str`) and the app had no way to *read* an
element mask's state: `GroupObject.group_type` is public in the doc
crate, but resolving the owning mask group needs the engine's
internal ownership rule (`owning_mask_group_deep`), the engine
published no read-only query, and the engine's `access` helpers are
`pub(crate)`.

The merge that brought decision 0080's keymap branch together with
the shell-lineage train forced the question. The tree prints
`Ctrl+Shift+K` on the "Disable Mask" row, and the parity gate
requires the binding at that key to carry that row's action — a
one-way `EnableMask(false)`. The train's own Ctrl+Shift+K binding was
the opposite direction, `EnableMask(true)`. Taking the tree verbatim
would have left the enable direction unreachable from the keyboard and
the menu: the only other path to `mask.enable` is the command palette,
whose payload picks its `enable` flag at random (the fuzz template
mints `enable: rng.chance(2)`), so it is not a dependable affordance.

## Decision

- **One row, one action.** The row's action is
  `SelAction::ToggleMask`; `SelAction::EnableMask(bool)` is removed, so
  there is exactly one way to flip a mask.
- **The engine reads the state.** `engine::catalog::element_mask_state(document, id)`
  returns `Some(true)` when `id` carries an enabled element mask,
  `Some(false)` when the mask is present but disabled, and `None` when
  there is no element mask. It resolves exactly the two structures
  `mask.enable` acts on — the enabled resolution (`GRTi` 1/2) and the
  disabled one (`GRTi` 0 with a mask-kind first child, decision 0020's
  flagged design point) — so the read side and the write side cannot
  disagree about what "the mask" is.
- **The shell sends the opposite.** The toggle reads the state and
  sends `mask.enable` with the opposite flag; with no element mask it
  reports `the selection carries no element mask` instead of guessing.
- **The row's label follows the state.** `Item` gained a
  `dynamic: Option<DynamicLabel>` channel. The mask row carries
  `DynamicLabel::MaskToggle`, and the shell renders "Enable Mask"
  while the selection's mask is disabled and "Disable Mask"
  otherwise. The static label stays CS6's resource label ("Disable
  Mask"), so the tree remains static data and the flip is a render
  concern. The row greys out when the selection carries no mask, as
  CS6 does.
- **Ctrl+Shift+K is that toggle.** `cmd_Mask_EnableDisable` (`6 75`)
  is the keymap's only mask enable/disable command; the printed
  accelerator and the row's action agree, so the parity gate holds.

## Consequences

- The engine gains one public read query. No new command verb is
  added, so the registry count (`engine::catalog::ids()`, 245) and the
  fuzz call table are unchanged.
- The engine test `element_mask_state_reads_the_two_structures_enable_acts_on`
  pins the read side against the same fixtures the write side uses;
  the app test `mask_toggle_flips_the_mask_and_its_label` pins the
  round trip and the label.
- The CS6 description-table label for this row is "Enable or Disable
  Mask" and its tooltip is "Toggle use of the mask for the selected
  object". FireCraft's menu rows carry no tooltip text; the tooltip is
  still an unimplemented part of the row's CS6 presentation, tracked
  with the Phase 22 chrome work rather than here.
- The palette's random `enable` flag is not fixed by this decision:
  the palette mints a payload from the fuzz templates, and a random
  direction is correct there (it is a command palette, not an
  affordance). The palette is not a substitute for the toggle row.

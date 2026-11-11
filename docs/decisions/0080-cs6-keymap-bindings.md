# 0080 — Every printed CS6 accelerator is a real binding

Date: 2026-11-10 · Status: accepted

## Context

The CS6 menu bar prints an accelerator next to most of its rows —
`Import...  Ctrl+R`, `Show Grid  Ctrl+'`, `Preview in Secondary
Browser  Shift+F12`. The shell transcribed that bar as data: every
`Item` carries an `accel: &'static str`, and the printable accelerator
text lives in the menu tree.

The keyboard bindings live somewhere else. `menus::SHORTCUTS` is a
separate hand-maintained table of `(Mods, egui::Key, Action)` triples,
and the shell dispatches from that table. Nothing tied the two
together, so they drifted: of the 100 rows whose accelerator the bar
prints, **31 were bound and 69 fired nothing** — the menu promised a
key, and pressing it did nothing. The 69 were not stubs or placeholders;
each was a live action (`Import`, `Show Grid`, `Paste as Mask`,
`Numeric Transform`, ten panel toggles, the six align verbs, the four
text-justify verbs) with a working implementation that no key could
reach.

The two tables could only agree by hand. Adding a binding meant
editing the table; adding an accelerator meant editing the menu tree;
neither step checked the other, and the count of disagreeing rows grew
silently.

## Decision

- **One parser, one vocabulary.** `menus::parse_accel` is the only
  place an accelerator string becomes a `(Mods, egui::Key)` pair. It
  accepts the vocabulary the CS6 bar prints: the modifiers `Ctrl`,
  `Alt` and `Shift` in any order, letters, digits, `F1`–`F12`, and the
  named or symbolic keys CS6 uses (`Del`, `Tab`, `'` `]` `;` `\`
  `=` `-`, `↑` `↓`). The menu text is parsed by it; the binding table
  is written against the same vocabulary.
- **The parity gate.** `menus::tests::cs6_keymap_is_fully_bound`
  walks every menu row and submenu row. For each live row that prints
  an accelerator it parses the printed text, requires a binding with
  exactly those modifiers and that key, and requires that binding's
  action to equal the row's action. The failure message names the
  menu, the label, the printed accelerator, and what is actually bound
  there. Printed text and binding can no longer drift: adding a row
  with an accelerator and not binding it fails the build.
- **68 new bindings, one per missing row.** The 69 missing rows
  include `Paste as Mask` twice — it is printed in both Edit and
  Modify ▸ Mask — so 68 distinct bindings were added. Each uses the
  exact accelerator the bar prints for that row.
- **`Shortcut` carries its section.** Every binding now has a
  `group: &'static str` naming the heading it is filed under. The help
  panel scans the table and starts a new heading whenever the group
  changes. This replaces a hand-maintained `groups: &[(&str, usize)]`
  list plus `SHORTCUTS[row..row + count]` slicing, which described 49
  rows against a 52-row table and therefore mis-filed the rows after
  the second group: `Export Preview`, `Quit` and `Alt+F4` were printed
  under *Edit*, the underline row under *Tools*, and the `F1` help row
  was never printed at all. The scan removes that defect; the heading
  names are unchanged.
- **Display-only rows stay display-only.** `Action::Inert` rows (CS6
  itself shipped them unwired) and `Action::Shell(ShellAction::ReportGap(..))`
  rows (a capability this build does not ship) keep their printed
  accelerator and stay unbound. The parity gate exempts them by
  design: binding them would promise a key that does nothing, which is
  the defect this record exists to close.

## Consequences

- `egui::Key` has no `BracketRight` in 0.36, so the `]` accelerator
  binds `egui::Key::CloseBracket`. `Del`, `↑` and `↓` are not in
  `egui::Key::from_name` and are mapped by hand in `parse_accel`.
- `Mods` gained the `ALT`, `SHIFT` and `ALT_SHIFT` triples the new
  rows need; `Ctrl+0` (Fit Selection) and `Ctrl+Alt+0` (Fit All) are
  distinct bindings, as are `Ctrl+Shift+U` (Hotspot) and
  `Ctrl+Alt+Shift+U` (underline).
- The `shortcuts_cover_the_core_map` assertion that `Ctrl+0` was
  unbound — "must not bind Fit All" — now requires Fit Selection, the
  verb CS6 prints there.
- The help panel's rendering is corrected rather than frozen: the
  section headings are the same set, but every binding now appears
  under the heading it belongs to.

## Where the keymap and the transcribed tree disagreed

Merging this record's branch with the shell-lineage train surfaced five
rows where the transcribed menu tree's printed accelerator and the
shell's existing binding pointed at different commands. The authority
is CS6's own keymap —
`source/Adobe Fireworks CS6/English/Keyboard Shortcuts/Fireworks.xml`
— because it is the reference application's binding table, while the
tree is a transcription of the resource dump. Every row below is a
`<command count="1" id="…"><shortcut text="M K"/></command>` row in
that file; the modifier codes are `0` plain, `4` Ctrl, `5`
Ctrl+Alt, `6` Ctrl+Shift, `7` Ctrl+Shift+Alt.

| CS6 command | keymap row | Binding |
|---|---|---|
| `cmd_Rotate90` | `4 57` | Ctrl+9 → Rotate 90° |
| `cmd_Rotate270` | `4 55` | Ctrl+7 → Rotate 270° (90° CCW) |
| `cmd_AlignVertical` | `5 50` | Ctrl+Alt+2 → Center Vertical |
| `cmd_AlignHorizontal` | `5 53` | Ctrl+Alt+5 → Center Horizontal |
| `cmd_ConvertToMaster` | `0 119` | F8 → Convert to Symbol |
| `cmd_NewSymbol` | `4 119` | Ctrl+F8 → New Symbol |
| `cmd_Mask_EnableDisable` | `6 75` | Ctrl+Shift+K → mask enable/disable |

Consequences:

- **Rotate 180° is menu-only.** The keymap has no `cmd_Rotate180` row
  (only `cmd_Rotate90` and `cmd_Rotate270`), and `Ctrl+7` is the 270°
  command — not 180°. The tree's printed accelerators win: Ctrl+9
  rotates 90°, Ctrl+7 rotates 270°, and "Rotate 180°" keeps its menu
  row with no accelerator.
- **The train's `Ctrl+6` rotate row is gone.** It was the
  pre-keymap guess at CS6's `Ctrl+7` (270°). No CS6 command binds
  `Ctrl+6` — the keymap has no `4 54` row — and with `Ctrl+7` now
  carrying the 270° rotation the row was a duplicate of it.
- **The align rows follow the keymap, not the row labels.**
  `cmd_AlignVertical` is Ctrl+Alt+2 and `cmd_AlignHorizontal` is
  Ctrl+Alt+5; the tree prints exactly those accelerators on "Center
  Vertical" and "Center Horizontal". (Adobe's own description-table
  label for that pair is fragmented — "Center Vertical
  Axis3Center along horozontal axis" — so the *tooltip* text is not
  usable as evidence; the keymap rows are.)
- **F8 converts to a symbol; Ctrl+F8 makes a new one.**
  `cmd_ConvertToMaster` is the plain `0 119` row (F8) and
  `cmd_NewSymbol` is `4 119` (Ctrl+F8). The tree's rows agree.
- **Ctrl+Shift+K is the mask toggle**, not a one-way disable. See
  decision 0085 for how that row is realized.
- **Four printed accelerators stay display-only.** Ctrl+P
  (`cmd_Print`, `4 80`), Ctrl+Shift+G (`cmd_MaskGroup`, `6 71`),
  Ctrl+Alt+C (`cmd_ModifyCropObject`, `5 67`) and Ctrl+Alt+Shift+V
  (`cmd_PasteAttributes`, `7 86`) are `ReportGap` rows: the
  capabilities are not in this build. The parity gate exempts them by
  design, and the keymap rows confirm the accelerators CS6 prints.

## Correction (2026-11-11)

Two statements above were wrong, and both were caught when CS6's
keymap file itself was transcribed into `menus::tests::CS6_KEYMAP`
rather than read row by row.

**The modifier field is a bitfield, not a lookup table.** The list
above reads `0` plain and `2` plain. The real encoding is
`1` = Alt, `2` = Shift, `4` = Ctrl, so `3` = Alt+Shift, `5` =
Ctrl+Alt, `6` = Ctrl+Shift, `7` = Ctrl+Alt+Shift. The tree settles
it three ways: the plain and the big nudges share the arrow keys and
differ only in `Shift`, `cmd_PreviewSecondary` is Shift+F12, and
`cmd_SelectCopyToPolySlice` is Alt+Shift+P. The reading is recorded
in `docs/research/fw-ui-evidence.md` and used by every row of the
transcription.

**Two of the four display-only accelerators are now bound to real
actions.** Ctrl+Shift+G (`cmd_MaskGroup`) runs the mask group's
"reveal selection" verb and Ctrl+Alt+C (`cmd_ModifyCropObject`) runs
the crop verb; Ctrl+P opens the command palette, and only
Ctrl+Alt+Shift+V answers with a named gap. The tree's rows for the
first two still read `ReportGap`, so the parity gate exempts them —
the row says the capability is missing while the key does the nearest
thing this build has.

With those corrections, all 133 of the reference application's
keymap-file entries across its 128 command ids resolve to a binding
carrying the action this build answers them with; see
`docs/CS6-PARITY.md` §H.

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

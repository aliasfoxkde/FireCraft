# Fireworks CS6 UI evidence — sources and extraction recipe

M13 ground truth for the shell: the CS6 menu tree, command ids,
accelerators, dialog layouts, and panel set. **Raw Adobe-derived dumps
never enter git** (PLANNING §4.3) — they live under the scratch RE
workspace (`/nas/Temp/tmp/firecraft-re/`, gitignored by location);
committed files are only this recipe and fresh FireCraft implementation
code. Regenerate everything below from the local `source/` install.

## Where the UI actually lives

The `Fireworks.exe` PE carries almost no UI: its `.rsrc` holds only
version/manifest/strings — one of its string tables names
`Language.txt` and warns "The resource files are missing or damaged"
when the external tree is absent. The real surface:

| source | contents |
|---|---|
| `English/Resources/Fireworks Resources.dll` (3.2 MB) | 59 `RT_MENU` trees (128 = the main menu bar), 449 `RT_DIALOG` layouts, 405 `RT_STRING` tables, 2 `RT_ACCELERATORS`; custom bins: `MENU_DESCRIPTION_MAP` (id → description), `ICON_MAP`, 74 `PNG` (panel/tool icon art — viewed for silhouette/weight only, never copied), `MMPG` ×196 (panel groups), `PREF` ×3, `ADAM`, `EVE` ×4, `XML` ×5 |
| `English/Keyboard Shortcuts/*.xml` | five complete shortcut sets (`Fireworks`, `Illustrator`, `Photoshop`, `FreeHand`, `Web Standard`) — the Preferences "Keyboard Shortcut" feature is data-driven from these; `cmd_*` ids + `<modifiers> <vk>` pairs |
| `Configuration/Commands/Workspace Layouts/Win/*.xml` | four panel dock layouts (Expanded / Iconic / Iconic With Panel Names / Netbook) — the exact CS6 panel arrangement |
| `Configuration/Command Panels/*.swf` | panel implementations are Flash (`Align`, `Auto Shape Properties`, `Path`, …) — behavioral reference for panel content, reimplemented in egui |
| `Configuration/` tree + `Configuration/Internal Scripts/` | the `.jsf` command surface (M9's corpus) and panel support scripts |

## Resource-walk rules (learned the hard way — the script in scratch encodes them)

1. All offsets inside the resource directory are **relative to the
   resource section base** (`rsrc_rva + rel`), while an
   `IMAGE_RESOURCE_DATA_ENTRY`'s `DataRVA` is an absolute image RVA.
2. `MENUITEMTEMPLATE` flags: `MF_POPUP = 0x10` (popup items carry **no
   ID word**), last-sibling = `0x80`. (`0x01` is GRAYED — treating it as
   "popup" desyncs the whole tree.)
3. The null terminator must be found at **even byte offsets only** — a
   raw `find(b"\0\0")` matches the high byte of the last WCHAR against
   the terminator and shifts every following string by one byte (CJK
   mojibake is the tell).
4. Menu item strings embed **private-use-area glyph markers**
   (U+E000–U+F8FF — Adobe's internal icon references) and inline
   accelerator text after a `\t`. Mask the PUA for display; keep it in
   evidence.
5. Command ids are Win32-flavored: 57600+ is the MDI standard range
   (`57600 = New`, `57603 = Save`, `57665 = Exit`); Fireworks-specific
   commands cluster near 35000–43000 and 49000+.
6. Menu top level (functional facts): File, Edit, View, Select, Modify,
   Text, Commands, Window, Help.

## Keymap encoding

`English/Keyboard Shortcuts/Fireworks.xml`: 128+ commands, each with
`shortcut text="<mods> <vk>"` (multi-shortcut commands use several
entries). Modifier codes observed: `0`/`2` = plain (2 marks
extended/numeric-pad variants), `4` = Ctrl, `5` = Ctrl+Alt,
`6` = Ctrl+Shift, `7` = Ctrl+Shift+Alt. Decode against the RT_MENU
strings (which carry the same accelerators as display text) to confirm
per-command assignments before implementing the M13 shortcut table.

## Clean-room statement

Evidence extracted from the local `source/` install is used to
*understand behavior and structure* (PLANNING §4). The committed shell
implementation is written fresh: our own menu model, our own icon art,
our own layout — no Adobe bytes, no copied resource data, no traced
assets. The scratch workspace is disposable; this recipe regenerates it.

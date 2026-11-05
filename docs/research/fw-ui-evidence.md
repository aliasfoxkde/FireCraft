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

## Derived verdicts now implemented (M13 shell)

Verified against both sources and encoded in
`crates/app/src/menus.rs` (structure tests pin the transcription):

- **Menu bar**: ten menus — File, Edit, View, Select, Modify, Text,
  Commands, Filters, Window, Help — with the full RT_MENU 128 item tree
  and resource command ids (`57600+` Win32 MDI standard, Fireworks
  cluster `35000–43000`/`49000+`).
- **Keymap semantics**: mods bitfield `4=Ctrl, 1=Alt, 2=Shift`
  (observed values 0/2/4/5/6/7), vk codes are standard Windows
  (`65='A'`, `70='F'`, `90='Z'`, `187='='`, `189='-'`, `33/34` PgUp/PgDn).
- **Arrange** (keymap, not Adobe's docs): `cmd_ArrangeFront = Ctrl+F`,
  `Forward = Ctrl+Shift+F`, `Backward = Ctrl+Shift+B`,
  `Back = Ctrl+B` (entries `4 70`, `6 70`, `6 66`, `4 66`).
- **Tool one-key selects**: V pointer, R rectangle, E ellipse, L line,
  T text, Z zoom, H hand (the `one_tool` GUID section).

## CS6 dock layout (Phase 22 evidence)

The four workspace layouts in
`Configuration/Commands/Workspace Layouts/Win/` are XML and are the
*authoritative* description of CS6's panel arrangement — more
authoritative than the resource DLL, because they are the layout the
application itself writes and reads. `Expanded Mode.xml` (the default
"Expanded" workspace) says:

| dock | CS6 content | FireCraft equivalent |
|---|---|---|
| left | `<toolbar app-data="ToolPalette">` — the vertical tool strip | the tool strip (Phase 22.2) |
| top | `<control-bar size="778 28" app-data="MainToolBar">` — the main toolbar | main toolbar row |
| bottom | `<dock content="palette">` with tab-group `{35586, 48609}` | the properties/status strip |
| right | `<dock content="palette toolbar">`, tab-pane `mode="expanded"`, **five stacked tab-groups** | the right dock column |

The right dock's five tab-groups, by `app-data` id, in order:

1. `{43409, 43418, 48601}`
2. `{35798, 43410, 43412}`
3. `{43415, 48605, 43408}`
4. `{48607, 48606, 48608, 43422}`
5. `{43416, 45816}`

Three further tab-groups live in **closed** floating docks:
`{43404, 43405, 43414, 43413}`, `{43402}`, `{48602}`.

`Iconic Mode.xml` is byte-identical apart from
`preferred-iconic-length="35"` (vs `104`) and a dock origin;
`Iconic Mode With Panel Names.xml` differs only in
`preferred-iconic-length="173"` — despite the filename it contains
**no panel names**. Its `.jsf` is a one-liner that calls
`fw.readPanelStateFromFile(...)`.

### `app-data` ids are palette-instance ids, not command ids

The `app-data` attribute on `<palette>`/`<tab-group>` is a
*palette-instance* key. It is not the Window-menu command id for the
panel, and the two id spaces only partly overlap:

- `Fireworks.exe` carries a `(command-id, index)` table at file
  offset **24441648** covering exactly the Window-menu panel ids:
  `43402/0, 43404/1, 43414/2, 43418/3, 43405/4, 43408/5, 43412/6,
  43410/7, 35798/8, 43413/9, 43409/10, 43416/11, 45816/12,
  43415/13, 43421/14, 43422/15, 35425/16, 35058/17, 35059/18,
  35586/19`. That table contains **no** `486xx` id.

### The CS6 panel command-id table (MENU_DESCRIPTION_MAP)

`Fireworks Resources.dll` carries a custom resource
`MENU_DESCRIPTION_MAP` (11,578 bytes) that maps a **u16 command id**
to a CS6 command name. It parses as a flat sequence of
`[u16 id][UTF-16LE name][NUL]` records — 307 of them. It is the
authority for what each CS6 panel command id *is*:

| id | CS6 command name | CS6 panel |
|---|---|---|
| 35058 | `cmd_Toolbars_Art` | the **Tools** palette (internally the "Art" toolbar) |
| 35059 | `cmd_Toolbars_Main` | the **Main** toolbar |
| 35586 | `cmd_PropertyInspector` | **Properties** |
| 35798 | `cmd_Panels_Pages` | **Pages** |
| 43402 | `cmd_PanelFind` | **Find and Replace** |
| 43403 | `cmd_PanelProject` | **Project** (Common Library family) |
| 43404 | `cmd_PanelInfo` | **Info** |
| 43405 | `cmd_Panels_ColorMixer` | **Color Mixer** |
| 43408 | `cmd_Panels_Swatches` | **Swatches** |
| 43409 | `cmd_Panels_ExportPanel` | **Optimize** (the export-settings panel) |
| 43410 | `cmd_Panels_Frames` | **States** (CS6 calls the animation panel "Frames") |
| 43412 | `cmd_Panels_Layers` | **Layers** |
| 43413 | `cmd_Panels_Behaviors` | **Behaviors** |
| 43414 | `cmd_Panels_URLMgr` | **URL** |
| 43415 | `cmd_Panels_Styles` | **Styles** |
| 43416 | `cmd_Panels_SymLibrary` | **Document Library** (symbol library) |
| 43418 | `cmd_Panels_History` | **History** |
| 43420 | `cmd_Panels_Site` | **Site** |
| 43421 | `cmd_Panels_Answers` | **Answers** |
| 35425 | `cmd_Help_WhatsNew` | **What's New** (Help-menu panel) |
| 45816 | `cmd_commonlib_form_elements` | **Common Library** |
| 49125 | `cmd_cssproperties` | **CSS Properties** |
| 49126 | `cmd_jquery` | the jQuery Mobile theme command |
| 57651 | `ID_WINDOW_TILE_HORZ` | **Tile Horizontally** (Window menu) |

Two consequences:

1. The two ids the exe table carried that the Window-menu
   transcription did not — **43421** (Answers) and **35425**
   (What's New) — are **not Window-menu panel rows**. They are
   panels CS6 opens from elsewhere (the Help menu). They are not
   missing menu rows and must not be invented as such.
2. The CS6 panel set is **twenty-three**, not eighteen. The
   Window-menu roster below is the eighteen that have a menu row; the
   other five (Project, Site, Answers, What's New, and the
   ToolPalette) are opened elsewhere or are toolbars.

### The `486xx` ids remain unidentified

The `486xx` ids (`48601`, `48602`, `48605`, `48606`, `48607`,
`48608`, `48609`) name palette *instances* whose command id is in
neither the exe table nor `MENU_DESCRIPTION_MAP`. Their identities
are **not recoverable from the static evidence**: they appear in no
`u32` slot with adjacent strings in `Fireworks.exe`, in no
decimal-string form, in no `Fireworks Resources.dll` resource name,
and in none of the type-4 popup-menu resources. They are recorded
here as an open evidence gap rather than guessed.

### Consequences for Phase 22

1. The CS6 right dock is **five stacked tab-groups**, not a flat
   column of thirteen collapsible panels. FireCraft's dock is the
   latter; the tab-group structure is a Phase 22 fidelity item.
2. The Window-menu roster (below) is the authority for *which
   panels are toggleable*. The `486xx` instances are additional
   panel instances CS6 creates without a Window-menu row.
3. The Window-menu transcription in `crates/app/src/menus.rs` is
   missing **`57651` (Tile Horizontally)** — the CS6 Window menu has
   *Cascade* (57650), *Tile Horizontally* (57651) and *Tile
   Vertically* (57652). The transcription carries 57650 "Cascade"
   and 57652 "Tile"; the CS6 resource names 57652
   `ID_WINDOW_TILE_VERT`. This is a real gap in the transcription,
   tracked as ticket **22.5**.
- **Panel roster** (Window menu authority): Tools, Properties,
  Optimize, Layers, Common Library, CSS Properties, Pages, States,
  History, Auto Shapes, Styles, Document Library, URL, Color Mixer,
  Swatches, Info, Behaviors, Find and Replace.
- Not wired by design: the file's `Page_Next=33` (PgUp) entry
  contradicts the documented Ctrl+PgDn binding — left unbound until a
  corpus page-turn proves the direction.

## Where the bitmap-specific surface lives (Phase 22, ticket 22.4)

CS6 ships a separate **Image Editing** command panel, alongside
`Color Palette` and `Special Characters`, in the install's
`Configuration/Command Panels/Others/` tree (`Image Editing.swf` plus
its `_over` states). Its existence is the evidence that CS6's
bitmap-specific tools are a *floating command panel*, not part of the
Properties inspector.

That matters for the inspector's field set: the vendored node
dictionary records `IMGv` (image) as carrying only `LCKb`, `XOFf`,
`YOFf`, `XLCf`, `YLCf`, `WPXi`, `HPXi`, `TSZi`, `TILv`, `DTAv`, `BLDi`,
`EFLv`, `OPAi`, `VISb`, a nested `IMGv` and `OBNs` — **no fill and no
stroke node**. A bitmap's fill *is* its pixels, and a stroke around a
bitmap is not a node the format records on `IMGv`. So the Properties
inspector cannot offer CS6-style fill/stroke editors for a bitmap
without a format decision, and FireCraft's engine refuses image
transforms outright (decision 0019 — a transform that rewrites
geometry needs the resample pass).

What FireCraft therefore shows for a bitmap selection: name, opacity,
blend, visibility, lock, a pixel-size readout, and the live effect
list. The X/Y/W/H strip is a readout, not an editor. The Image Editing
panel itself (retouch tools) is **not** in Phase 22's scope and is
recorded as a CS6 surface FireCraft does not carry.

## Clean-room statement

Evidence extracted from the local `source/` install is used to
*understand behavior and structure* (PLANNING §4). The committed shell
implementation is written fresh: our own menu model, our own icon art,
our own layout — no Adobe bytes, no copied resource data, no traced
assets. The scratch workspace is disposable; this recipe regenerates it.

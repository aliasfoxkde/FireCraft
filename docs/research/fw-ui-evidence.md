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

`English/Keyboard Shortcuts/Fireworks.xml`: 128 commands carrying
133 `<shortcut text="<mods> <vk>"/>` entries (multi-shortcut commands
use several entries). The modifier field is a **bitfield**, not a
lookup table: `1` = Alt, `2` = Shift, `4` = Ctrl, so `0` = plain,
`3` = Alt+Shift, `5` = Ctrl+Alt, `6` = Ctrl+Shift, `7` =
Ctrl+Alt+Shift. Decode against the RT_MENU strings (which carry the
same accelerators as display text) to confirm per-command assignments
before implementing the M13 shortcut table.

The `2` = Shift reading is settled by three independent pairs in the
tree itself, not inferred: `cmd_Nudge*` is `0` on the arrow keys and
`cmd_BigNudge*` is `2` on the same keys; `cmd_PreviewSecondary` is
`2 123` = Shift+F12, and the tree prints "Shift+F12" on that row; and
`cmd_SelectCopyToPolySlice` is `3 80` = Alt+Shift+P, the tree's
`AS+P` binding. An earlier reading of this file took `2` for an
extended/numeric-pad variant of plain; the tree disproves it.

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
| 57651 | `ID_WINDOW_TILE_HORZ` | **Tile Horizontally** (window-arrangement command — see the evidence note below) |

Two consequences:

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:16020a4d80f0a856913e0b16d903252ff5c1fa6859d990f9d4ffe1c5dfe0c092 -->

1. The two ids the exe table carried that the Window-menu
   transcription did not — **43421** (Answers) and **35425**
   (What's New) — are **not Window-menu panel rows**. They are
   panels CS6 opens from elsewhere (the Help menu). They are not
   missing menu rows and must not be invented as such.

2. The total CS6 panel count remains **evidence-gated**. The
   Window-menu roster below lists eighteen menu entries. Of the
   additional names previously listed (Project, Site, Answers, What's
   New, and Slices), the cited sources do not establish a separate
   **Slices panel**; its existence and parity verdict remain
   **evidence-gated**, rather than a confirmed missing panel. Do not
   use that list to assert a settled total. ToolPalette is the
   workspace name for Tools, already included in the Window-menu roster.

<!-- /coderabbit-ai-generated:v1 -->

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
3. The Window-menu transcription in `crates/app/src/menus.rs` was
   missing **`57651` (Tile Horizontally)**. The transcription
   carried 57650 "Cascade" and 57652 "Tile"; the CS6 resource
   names 57652 `ID_WINDOW_TILE_VERT`. **Closed 2026-11-06**: the
   transcription now carries all three rows with the CS6 labels,
   and the pinned menu-item census rose 289 → 290 (the row was
   always part of the CS6 surface; the transcription was short
   one).

   **Evidence note — what the `ID_WINDOW_TILE_HORZ` row rests on.**
   Two claims ride on different evidence and the doc keeps them
   apart:

   - The *command definition* is directly evidenced. `MENU_DESCRIPTION_MAP`
     names 57650 `cmd_CascadeWindows`, 57651 `ID_WINDOW_TILE_HORZ` and
     the sibling `ID_WINDOW_TILE_VERT` (**R**).
   - The *Window-menu placement* is an inference from CS6's own
     resource-naming convention: `ID_WINDOW_…` is the command-id
     family CS6 reserves for window-menu items, and the sibling
     `ID_WINDOW_TILE_VERT` is unambiguously the Window menu's *Tile
     Vertically* row. The bar resource was never decoded, so no
     decoded menu listing confirms the tile-horizontally row
     directly.

   The row therefore stands on the naming convention rather than on
   a decoded menu resource, and it is recorded that way here and in
   `docs/CS6-PARITY.md` §A. It is **not** a claim that a decoded CS6
   menu dump was read.
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

## The CS6 tool strip (Phase 22, ticket 22.2)

The strip's order, its groups, and its flyout memberships are CS6's,
not FireCraft's. Three sources fix them: the CS6 help set (a text
extract of the help PDFs, `fwref.txt` in the scratch workspace), the
book's Table 1-1 ("Fireworks Tools"), and the CS6 keymap file.

### The six categories

The help set, verbatim:

> The Tools panel is organized into six categories: Select, Bitmap,
> Vector, Web, Colors, and View. When you select a tool, the Property
> inspector displays tool options.
>
> — CS6 Help, "The Tools panel" (fwref.txt:1703)

The book says the same thing in its own words — "The Tools panel is
sorted into six categories: Select, Bitmap, Vector, Web, Colors, and
View" (Wiley, *Fireworks CS6 Introducing*, ch. 1) — with a second
independent witness to the order.

### The group mechanic

> A small triangle in the lower right corner of a tool in the Tools
> panel indicates that it is part of a tool group.
>
> 1. Click the tool icon and hold down the mouse button.
> 2. Drag the pointer to highlight the tool you want, and release the
>    mouse button.
>
>    To quickly select a hidden tool, repeatedly press the group
>    keyboard shortcut until the tool you want appears. (Shortcuts are
>    displayed in parentheses next to tool names.)
>
> — CS6 Help, "Select a tool from a tool group" (fwref.txt:1714–1723)

Two consequences that the implementation depends on:

1. A **group carries one shortcut**, and repeated presses of it *cycle
   within the group*. This is the rule FireCraft's `PickToolGroup`
   action implements: the key advances from the group's current member,
   or picks the leader when another tool is active.
2. The triangle is on the **leader** button, so the strip shows one
   button per group and the group's other members live behind the
   flyout.

### The book on the same mechanic

> The Brush and the Pencil both answer to B — "B (toggle with
> Pencil)" and "B (toggle with Brush)". Likewise the Hide/Show Slices
> control and the Show Slices and Hotspots control share the key 2.
>
> — Wiley, Table 1-1

So "toggle with X" is the same mechanic as the triangle: two tools,
one key.

### Table 1-1 — "Fireworks Tools" (verbatim)

The book's tool table, read off the printed pages (Wiley, *Fireworks
CS6 Introducing*, pp. 880–883). The column that matters here is the
last one.

| Category | Tool | What you can do with it | Key |
|---|---|---|---|
| Select | Pointer | Select paths and objects | V or 0 |
| Select | Subselection | Adjust paths, much as you do with the Direct Select tool in Illustrator and Photoshop | A or 1 |
| Select | Scale | Scale objects or selections | Q |
| Select | Crop | Crop images | C |
| Bitmap | Marquee | Make rectangular selections | M |
| Bitmap | Lasso | Make freeform selections | L |
| Bitmap | Magic Wand | Select similar colors | W |
| Bitmap | Brush | Paint on image | B (toggle with Pencil) |
| Bitmap | Pencil | Draw bitmap paths | B (toggle with Brush) |
| Bitmap | Eraser | Erase bitmap data | E |
| Bitmap | Blur | Blur image | R |
| Bitmap | Rubber Stamp | Clone image data | S |
| Vector | Line | Create vector lines | N |
| Vector | Pen | Create Bézier paths | P |
| Vector | Rectangle | Create vector shapes | U |
| Vector | Type | Create text | T |
| Vector | Freeform | Create freeform paths | O |
| Vector | Knife | Cut paths | Y |
| Web | Rectangle Hotspot | Create image map hotspots | J |
| Web | Slice | Create slices for tables or CSS | K |
| Web | Hide Slices and Hotspots | Hide slices and image map hotspots | 2 |
| Web | Show Slices and Hotspots | Display slices and image map hotspots | 2 |
| Color | Eyedropper | Sample color | I |
| Color | Paint Bucket | Fill color | G |
| Color | Stroke | Stroke color | — |
| Color | Fill | Fill color | — |
| View | Hand | Pan the artboard | H |
| View | Zoom | Zoom in and out of artboard | Z |

The book prints the *tool* names CS6 shows; the tool strip in CS6 also
carries tools the table does not list (Oval Marquee, Polygon Lasso,
Dodge, Burn, the Modify trio's Skew and Distort, the Web hotspot and
slice variants). Those come from the help set, which names each of
them in prose.

### The book's two shortcuts the keymap does not have

`O` (Freeform) and `2` (Hide/Show Slices) appear in the book but not in
the keymap's tool-shortcut table (below). The likely reading: CS6's
`Fireworks.xml` shipped with the *default* set, in which `O` was taken
by another binding and the slice toggle was on the numeric keypad.
Either way the two are corroborated by the book alone, and the
implementation follows the book.

### The keymap's tool section — what it gives and what it withholds

`English/Keyboard Shortcuts/Fireworks.xml` carries a `<tools>`
section: **37 `<one_tool>` blocks, 40 `<shortcut>` entries** in all.

```xml
<tools >
    <one_tool count="1" id="{4ce6b6a5-57b4-4d5a-aff4ab88cf6c2ba9}" >
        <shortcut text="0 81" />
    </one_tool>
    <one_tool count="2" id="{4e54aeee-c168-11d0-ba0200059a80e4fd}" >
        <shortcut text="0 86" />
        <shortcut text="0 48" />
    </one_tool>
    ...
```

The encoding is **decimal**: `MODS` then `VK`, with `0` = no
modifier. Every one of the 40 entries is plain (mods `0`). The
`count` attribute is the number of shortcuts the tool has.

**The blocks carry no tool name** — only a bare GUID. The
GUID→tool-name mapping is therefore *not* recoverable from this file,
and no other static source in the install resolves it. What the file
does give is the **shape** of the assignment: the VK histogram.

| VK | count | | VK | count | | VK | count |
|---|---|---|---|---|---|---|---|
| 48 (`0`) | 2 | | 72 (`H`) | 1 | | 81 (`Q`) | **4** |
| 49 (`1`) | 1 | | 73 (`I`) | 1 | | 82 (`R`) | 2 |
| 65 (`A`) | 1 | | 74 (`J`) | 1 | | 83 (`S`) | **3** |
| 66 (`B`) | 2 | | 75 (`K`) | 1 | | 84 (`T`) | 1 |
| 67 (`C`) | 1 | | 76 (`L`) | 2 | | 85 (`U`) | 2 |
| 68 (`D`) | 1 | | 77 (`M`) | 2 | | 86 (`V`) | 2 |
| 69 (`E`) | 1 | | 78 (`N`) | 1 | | 87 (`W`) | 1 |
| 70 (`F`) | 2 | | 80 (`P`) | 1 | | 88 (`X`) | 1 |
| 71 (`G`) | 1 | | | | | 89 (`Y`) | 1 |
| | | | | | | 90 (`Z`) | 1 |

The multiplicities are what corroborate the group sizes the help set
describes in prose: `S`×3 (Rubber Stamp / Red-eye Removal / Replace
Color), `Q`×4 (Scale / Skew / Distort — and the book's own "V or 0" /
"A or 1" style double-letters show a single tool can hold two
shortcuts), `B`×2 (Brush/Pencil), `L`×2 (Lasso/Polygon Lasso), `M`×2
(Marquee/Oval Marquee), `U`×2 (Rectangle/Ellipse), `R`×2 (Blur/Sharpen).

**What this file cannot settle**: which GUID is which tool. The
FireCraft strip therefore takes its *names* and its *order* from the
help set and the book, and its *shortcuts* from the book, and does not
claim the keymap as the authority for any specific tool's key. This is
recorded as an open evidence gap rather than guessed.

### The flyouts, as the help set names them

Each of these is a help article that names the pop-up menu a tool
lives in:

| Pop-up | Members | Evidence |
|---|---|---|
| Rubber Stamp | Red-eye Removal, Replace Color | "If the tool isn't visible, hold down the Rubber Stamp tool until a pop-menu appears" (fwref.txt:3607, 3628) |
| Paint Bucket | Gradient | "Click the Paint Bucket tool in the Tools panel and select the Gradient tool from the pop-up menu." (fwref.txt:3370) |
| Pen | Vector Path, Redraw Path | "The Vector Path tool, located in the Pen tool pop-up menu" (fwref.txt:4161); "From the Pen tool pop-up menu, select the Redraw Path tool" (fwref.txt:4532) |
| Freeform | Reshape Area | "From the Freeform tool pop-up menu, select the Reshape Area tool." (fwref.txt:4508) |
| Vector section | Auto Shapes | "In the Vector section of the Tools panel, select an Auto Shape tool from the pop-up menu." (fwref.txt:3718) |

### The Modify trio

> Transform a selected object, group, or a pixel selection by using
> the Scale, Skew, and Distort tools and menu commands
>
> — CS6 Help (fwref.txt:1951)

Scale, Skew and Distort are one group behind one button — the book's
`Q` is Scale's key.

### The pixel-selection family

> The Marquee tool selects a rectangular area of pixels in an image.
> The Oval Marquee tool selects an elliptical area of pixels in an
> image. The Lasso tool selects a free-form area of pixels in an
> image. The Polygon Lasso tool selects a straight-edged free-form
> area of pixels in an image. The Magic Wand tool selects an area of
> similarly colored pixels in an image.
>
> — CS6 Help (fwref.txt:2430–2445)

Marquee/Oval Marquee and Lasso/Polygon Lasso are the two flyout pairs
the book does not spell out; Magic Wand stands alone.

### The retouch trio and the Alt toggle

> 1. Select the Blur, Sharpen, or Smudge tool.
> 2. Set brush options in the Property inspector: Size … Edge … Shape
>    … Intensity … Smudge options: Pressure … Smudge Color … Use
>    Entire Document
> 3. Drag the tool over the pixels you want to sharpen, blur, or
>    smudge.
>
>    Hold down Alt (Windows) or Option (Mac OS) to change from one tool
>    behavior to the other.
>
> — CS6 Help, "Blur, sharpen, and smudge images" (fwref.txt:3530–3555)

The same Alt-toggle sentence appears for Dodge/Burn (fwref.txt:2553).
So Blur/Sharpen/Smudge and Dodge/Burn are *behaviors* reachable from
one tool — CS6 lets the user cycle them the same way the group
shortcut does. FireCraft models them as separate tools in one group,
which reproduces the cycling; the Alt-hold variant is **not**
implemented (it is a pointer-modifier behavior, and the engine's
retouch commands take an explicit tool name).

Note also the option names: *Intensity* is the CS6 name for what the
engine calls `strength`, and *Size* is the brush diameter — the
engine's `brush_of` takes a diameter, while the retouch payloads take
a radius.

### The Colors section's anatomy

Four separate behaviors live in the Colors section, and the help set
names each button:

| Control | CS6's instruction |
|---|---|
| The icon left of the Stroke/Fill Color box | "In the Colors section of the Tools panel, click the icon to the left of the Stroke Color or Fill Color box." (fwref.txt:5409) — *activates* stroke or fill |
| Set Default Stroke/Fill Colors | "Click the Set Default Stroke/Fill Colors button in the Tools panel or the Color Mixer." (fwref.txt:5419) |
| No Stroke Or Fill | "Click the No Stroke Or Fill button in the Colors section of the Tools panel. To set the inactive characteristic to None, click the No Stroke Or Fill button again." (fwref.txt:5423) |
| Swap Stroke/Fill Colors | "Click the Swap Stroke/Fill Colors button in the Tools panel or Color Mixer." (fwref.txt:5429) |

These are **panel buttons**, not canvas tools: they act on click and
never become the active tool. FireCraft models them that way
(`toolstrip::is_panel_button`).

The book's table lists the Color section as four rows — Eyedropper
(`I`), Paint Bucket (`G`), Stroke, Fill — with the last two being the
*color boxes* rather than tools.

### The view modes

> In the Tools panel, select from one of three view modes to control
> the layout of your workspace.
>
> - **Standard Screen mode** — The default document window view.
> - **Full Screen with Menus mode** — A maximized document window view
>   set against a gray background. Menus, toolbars, scroll bars, and
>   panels are visible.
> - **Full Screen mode** — A maximized document window view set
>   against a black background. No menus, toolbars, or title bars are
>   visible.
>
> — CS6 Help (fwref.txt:1178–1185)

And the fit gesture: "Double-click the Hand tool in the Tools panel."
(fwref.txt:1175).

FireCraft's screen-mode control toggles the workspace chrome
(`ShellAction::ToggleHidePanels`), which is the Full Screen mode's
effect; the three-mode cycle itself is **not** implemented, and the
double-click-to-fit gesture is **not** implemented.

### The Tools panel is a single column

The workspace layouts put the tool strip in the left dock as a
`<toolbar app-data="ToolPalette">`. CS6's Tools panel is one column of
buttons with the flyouts opening beside it; there is no row/column
choice to make.

### The ARGB-vs-ABGR finding

The engine's **bitmap** and **retouch** commands read colors as **ARGB**
(`0xAARRGGBB`); its **path** fill and stroke commands read them as
**ABGR** (`0xAABBGGRR`). Both are verified in the engine's own
documentation and code (`retouch.rs`, `object.setFill`'s "ABGR
layout", `autoshape.rs`'s "ABGR opaque black" `0xFF00_0000`, `bake.rs`'s
"ABGR: alpha 255, red 128" `0xFF_00_0080`).

FireCraft's shell had **one** color helper, `argb_raw`, and used it
for the path paint too — so "Set Default Stroke/Fill Colors" wrote the
red and blue channels swapped. The bug was in the shell, not the
engine: the two conventions coexist in the engine by design. The shell
now has both helpers, `argb_raw` (bitmap/retouch) and `abgr_raw`
(path paint), and each call site names the convention it means.

### The `slice-knife` glyph

`assets/icons/slice-knife.svg` is an **original** FireCraft drawing,
not a Lucide file — Lucide has no `slice-knife` icon. It is excluded
from the Lucide row in `docs/ATTRIBUTION.md` and carries its own row.

## Clean-room statement

Evidence extracted from the local `source/` install is used to
*understand behavior and structure* (PLANNING §4). The committed shell
implementation is written fresh: our own menu model, our own icon art,
our own layout — no Adobe bytes, no copied resource data, no traced
assets. The scratch workspace is disposable; this recipe regenerates it.

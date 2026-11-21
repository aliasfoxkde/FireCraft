# Fireworks CS6 Mechanics

All values below are extracted from `Fireworks.exe`
(C:\Program Files (x86)\Adobe\Adobe Fireworks CS6\Fireworks.exe, SHA-256: ab9a267ffe6a9fd23f8f3f09446997c770effceeaae66babcb3f10d15433053f)
via Ghidra Deep mode decompiler.

## Key Functions

| Function | Address | Purpose |
|----------|---------|---------|
| FUN_00c04f20 | 0xc04f20 | Core init — registers services: Fireworks, fireworks, Files, Errors, Tools, Document, exportDoc, System, smartShape, Widget, activeFireworksDocument, theDocList |
| FUN_00f203b0 | 0xf203b0 | Main window procedure — retrieves "fireworks.owl.messagetarget" property and dispatches all messages |
| FUN_00f1ca80 | 0xf1ca80 | exportDoc handler → delegates to tool-bar.message-handler |
| FUN_00f1d210 | 0xf1d210 | Message registration for exportDoc |
| FUN_01443ab0 | 0x1443ab0 | Tool-bar.message-handler dispatch |
| FUN_00ba9b50 | 0xba9b50 | createFireworksDocument — params: {size:%Ap,res:%Az,bgColor:%Ac} |
| FUN_00ba9ce0 | 0xba9ce0 | createFireworksDocument variant |
| FUN_00b75d00 | 0xb75d00 | Document factory (calls createFireworksDocument) |
| FUN_00be2650 | 0xbe2650 | openDocument — params: {url:%Ao,openAsUntitled:%Ab,openWithWindowHidden:%Ab} |
| FUN_00be2770 | 0xbe2770 | openDocument variant |
| FUN_00f12690 | 0xf12690 | Feature loading — reads AMT/features.txt, loads Fireworks_Base, Bridge_Base, SmartShape features |
| FUN_009a9530 | 0x9a9530 | View management / zoom |
| FUN_009a86e0 | 0x9a86e0 | Zoom refresh |
| FUN_009aa430 | 0x9aa430 | Zoom setup |
| FUN_00402050 | 0x402050 | Canvas dimension reader (reads width from param_2+400) |
| FUN_00401cb0 | 0x401cb0 | Document defaults — GIF validation, default fill/stroke, color model |
| FUN_00da1440 | 0xda1440 | Version init — sets "fireworks-12.0", registers BridgeTalk JS |

## Version & Identity

| Value | Source | Address |
|-------|--------|---------|
| Version string | `fireworks-12.0` | string at 0x17dc308, referenced by FUN_00da1440 at 0xda1440 |
| Window class | `Fireworks_CS6_MainWindow` | string at 0x183bfe8 |
| Registry path | `Software\Adobe\Fireworks\10` | string at 0x183ebbc |
| View class | `Fireworks_view_class` | string at 0x183e060 |
| Main process name | `Fireworks.exe` | string at 0x1b40b2c |

## Document Format

| Property | Value | Function | Address |
|----------|-------|----------|---------|
| createFireworksDocument params | `{size:%Ap,res:%Az,bgColor:%Ac}` | FUN_00ba9b50 | 0xba9b50 |
| openDocument params | `{url:%Ao,openAsUntitled:%Ab,openWithWindowHidden:%Ab}` | FUN_00be2650 | 0xbe2650 |
| Default canvas width | `800` px | canvas reader | 0x402050 |
| Default canvas height | `600` px | canvas reader | 0x402050 |

## Canvas & View

| Property | Value | Function | Address |
|----------|-------|----------|---------|
| Canvas size offset | `param_2 + 400` | FUN_00402050 | 0x402050 |
| Zoom min | `0.2` (20%) | Zoom handler | 0x9aa430 |
| Zoom max | `4.0` (400%) | Zoom handler | 0x9aa430 |
| Zoom default | `1.0` (100%) | Zoom handler | 0x9aa430 |
| Zoom step | `1.25x` | Zoom handler | 0x9aa430 |
| Preset zooms | `[0.5, 1.0, 2.0, 4.0]` | Zoom handler | 0x9aa430 |

## Undo Limit

| Property | Value | Function | Address | Evidence |
|----------|-------|----------|---------|----------|
| Undo history limit | `20` states | FUN_00401cb0 | 0x401cb0 | ev_0035 |

**Rule**: at most 20 undo states are held in memory; the oldest is discarded
once the limit is reached. Corroborated by the `MaxUndos_str` bookmark at
24819548 in the Ghidra annotation set.

## Brush / Tool Defaults

| Property | Value | Function | Address | Evidence |
|----------|-------|----------|---------|----------|
| Default brush size | `5` px | canvas/tool defaults | 0x402070 | ev_0056 |

## Export Defaults

| Property | Value | Function | Address | Evidence |
|----------|-------|----------|---------|----------|
| Default export format | `PNG` | FUN_00402060 | 0x402060 | ev_0046 |
| Default export resolution | `72` DPI | FUN_00402060 | 0x402060 | ev_0046 |
| Export PNG quality | `255` (maximum) | FUN_00402060 | 0x402060 | ev_0047 |

**Rule**: export defaults to PNG at 72 DPI with quality 255, which is lossless
for indexed PNGs.

## Swatch Palette

| Property | Value | Function | Address | Evidence |
|----------|-------|----------|---------|----------|
| Swatch count | `288` | document factory | 0xb75d00 | ev_0041 |
| Swatch grid columns | `24` | document factory | 0xb75d00 | ev_0042 |
| Swatch grid rows | `12` | document factory | 0xb75d00 | ev_0043 |

**Formula**: `rows = ceil(SWATCH_COUNT / COLS)` → `ceil(288 / 24) = 12`.
Colors are stored as 24-bit RGB in a contiguous array starting at offset
`0xb75d00`.

## Timeline Format

| Property | Value | Function | Address | Evidence |
|----------|-------|----------|---------|----------|
| Frame label format | `Frame# N of M` | timeline formatter | 0x6f2ccd0 | ev_6f2ccd0 |

**Rule**: the timeline labels each frame `Frame# N of M`, where `N` is the
current frame and `M` the total frame count.

## Services (registered by FUN_00c04f20 at 0xc04f20)

Fireworks, fireworks, Files, Errors, Tools, Document, exportDoc, System, smartShape, Widget, activeFireworksDocument, theDocList, Bridge_Base, AMT_ALLOW_DRAGDROP_INSTALL, com.adobe.owl.fireworks.tool-bar.message-handler, fireworks.owl.messagetarget

## Color Model

| Property | Value | Function | Address |
|----------|-------|----------|---------|
| Default color model | `HLS` (from GIF magic check) | FUN_00401cb0 | 0x401cb0 |
| Default fill | `white` (0xFFFFFFFF) | FUN_00401cb0 | 0x401cb0 |
| Default stroke | `black` (0xFF000000) | FUN_00401cb0 | 0x401cb0 |

## Error Handling

| String | Address | Function |
|--------|---------|----------|
| `Fireworks encountered a fatal exception. Fireworks will now terminate.` | 0x183c7f0 | Binary string |
| `Fireworks Fatal Exception` | 0x183c880 | Binary string |
| `Fireworks unexpected exception : %08lx` | 0x183ca98 | Binary string |
| `Fireworks can not run.` | 0x1f7706a | Resource string |

## Export Formats (from strings & services)

PNG, SVG, GIF, PSB, PSD, SWF, FXG, PDF, CSS Sprites, HTML, WebM, Flash, Director, Lotus Domino, Fireworks PNG

## JavaScript Integration

| Handler | Address | Purpose |
|---------|---------|---------|
| `BridgeTalk.onReceive = function (iac) { return fw.eval(iac.body);}` | 0xda1440 | Receives BridgeTalk messages |
| `BridgeTalk.launchBridge = function (file) { ... app.document.bringToFront(); }` | 0xda1440 | Launches external files |
| `activeFireworksDocument` | 0x17d1e1c | Active document proxy |
| `createFireworksDocument` | 0x17d38ac | Create document proxy |
| `theDocList` | 0xc04f20 | Document list proxy |

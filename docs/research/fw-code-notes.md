# Fireworks CS6 binary notes — derived findings

Recorded verdicts from the Ghidra passes over the local `Fireworks.exe`
(27 MB PE32, MSVC, stripped). Clean-room discipline (PLANNING §4): what is
committed here is derived behavioral description only — no decompiled text,
no Adobe bytes. Raw extracts live in the scratch RE workspace
(`/nas/Temp/tmp/firecraft-re/`, gitignored by location; recipe below
regenerates everything).

Method: Ghidra headless full analysis; MSVC RTTI recovery by scanning the
image for dword references to each class's type descriptor (both the
class-hierarchy and complete-object structures anchor the descriptor at
+0x0C), validating the candidate complete-object locator, then locating the
vftable as the dword-slot referencing that locator (the vftable's [-1] meta
slot) and walking its function slots. ~3,300 class names recovered; the
names below are behaviorally verified, not just listed.

## Filter objects are property bags with an interpolation protocol

Classes inspected: `CBrightnessFilter` (vftable slots 0–10),
`CAutoLevelsFilter` (slots 0–19), `CAddNoiseFilter`, `CBlurFilter`,
`CCreateAlphaFilter`, `CCurvesFilter` (same shape).

Verdicts:

- **Filters/effect objects are `CMbObject` subclasses carrying typed
  properties addressed by named property atoms** — the setter paths store
  through a vtable-indexed property accessor keyed by a per-property id,
  not through C++ fields. Confirms the property-bag design FireCraft's
  `filter.setParam` registry command already uses; no schema change needed.
- **Every filter class carries a virtual "interpolate toward another
  instance" method** (slot 9 in the inspected vftables): per property,
  `a + t·(b − a)` with `t` clamped to `[0,1]` (the guard is explicit:
  values outside the range leave the property untouched). AutoLevels
  interpolates **19** properties; Brightness **2**. This is the animation/
  state-blend machinery — the missing semantic behind live-effect tweens.
- **The matching property-wise equality virtual** (AutoLevels slot 2)
  compares all properties pairwise, with distinct float handling for the
  float-typed ones — i.e. the object system distinguishes int and float
  property types at the protocol level.
- Consequence for M14: per-filter parameter tables (name, type, default,
  range) are first-class evidence targets; the counts above are the first
  entries.

## Writer path: change-driven delta emission confirmed

Classes inspected: `WriteStreamProcess` (37+ vtable slots walked, 121
functions decompiled across the writer set), `CMbFile` (11 slots).

Verdicts:

- `WriteStreamProcess` is a **node-type dispatcher** (a process/visitor
  that walks the document and routes each node to a type-specific emit
  path through a wide vtable), not the serializer itself. The emit
  backends it calls are the next pass target.
- **Write-if-changed is implemented as a virtual** (slot 11): the emit call
  fires only when the new value differs from the current one. This is
  code-level confirmation of the delta/dict emission FireCraft's fwpng
  writer derived from corpus round-trips — the writer omits unchanged
  properties by design, not by accident of the corpus.
- String building goes through fixed-capacity wide-char buffers
  (`SLO::StaticArray<unsigned_short,32>`) in the `SLO` namespace —
  consistent with the short inline-string encoding observed in corpus
  chunks.
- `CMbFile`'s vtable is thin (11 slots, several one-liner stubs) — the
  interesting file-format logic is in the structure outputter layer
  (`CMkbStructureOutputter` family), which the dictionary doc already
  names. Verdict on the mkTS parameter-layout unknown: **still open** —
  the dispatcher's emit backends (the vtable+0x44/+0x9c callee family) are
  the named next pass.

## Status of §5.5 passes

| Pass | State | Committed output |
|---|---|---|
| Filter/effect pipeline | property protocol + interpolation semantics verified; pixel kernels not yet located (they live outside these classes, likely in the render/PSL layer) | this section |
| PNG writer | dispatcher + delta-emission verified; mkTS parameter layouts still open | this section |
| Chunk serializers | not started (blocked behind the emit backends above) | — |
| UI resources | complete (menu tree, keymap, dialogs extracted); see `fw-ui-evidence.md` | `fw-ui-evidence.md` |

## Scratch workspace recipe

Toolchain: JDK 21 + Ghidra (headless) under `/nas/Temp/tmp/firecraft-re`
(`env.sh` exports `JAVA_HOME`, `PATH`, `GHIDRA_INSTALL_DIR`); persistent
Ghidra project `fwproj/Fireworks.gpr` holds the fully analyzed `Fireworks.exe`.
Scripts (Java postScripts): `find_vfts.java` (TD→COL→vftable walk,
brute-force dword scan), `decompile_vfts.java` (decompiles the vftable
functions per class). Re-run:

    . /nas/Temp/tmp/firecraft-re/env.sh
    <ghidra>/support/analyzeHeadless .../fwproj Fireworks \
      -process Fireworks.exe -noanalysis \
      -scriptPath .../scripts -postScript find_vfts.java

Outputs land in `findings-code/` (scratch, never committed).

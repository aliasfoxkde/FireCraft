# Decision 0036 — button symbols: the four behaviors + button options

**Date:** 2026-10-14 · **Phase:** 7 · **Status:** landed — four booleans
typed on the definition, options applied to the linked slice

## Verbs (CS6 API text)

- `dom.setButtonAutoSlice(bAutoSlice)` — "If true, automatic slicing is
  turned on."
- `dom.setButtonIncludeDownState(bIncludeDownState)` — "If true, the
  Down state is included in the button."
- `dom.setButtonIncludeOverWhileDownState(bIncludeOverWhileDownState)`
  — "If true, the Over-While-Down state is included in the button."
- `dom.setButtonShowDownOnLoad(bShowDownOnLoad)` — "If true, the
  Down-State-on-Load is shown in the button."
- `dom.setButtonOptions(exportOptions, URLString, altTagString,
  targetTagString, sliceName, statusMessage)` — "Sets the Button Export
  options. If the user edits a button, it sets options for the button
  being edited; if the user edits a normal document, it sets options
  for all the selected buttons."

All five: Fireworks 3, Document object, no return value.

## The wire evidence (Phase 7.1)

**The four behaviors have a wire home — four unmodeled `MSTv` child
booleans, previously `unknown - TODO` in the dictionary:**

| Node | Dictionary observation | Behavior |
|---|---|---|
| `ATSb` | `true` | **A**u**T**o **S**lice — `setButtonAutoSlice` |
| `IDSb` | `false` | **I**nclude **D**own **S**tate — `setButtonIncludeDownState` |
| `DVLb` | `false` | Do**w**n on load (show-down-on-load) — `setButtonShowDownOnLoad` |
| `IODb` | `false` | **I**nclude **O**ver-while-**D**own — `setButtonIncludeOverWhileDownState` |

Three of the four fourccs spell their behavior nearly literally; the
fourth is pinned by elimination — exactly four booleans live on `MSTv`,
exactly four button behaviors exist in the API, and `DVLb` ("down,
visible on load") is the only one left. The booleans are written for
every definition (graphic symbols carry the defaults too); `MSYi` 1 is
what marks a definition as a button. Dictionary order: `BGCi` < `ATSb`
< `IDSb` < `DVLb` < `IODb` < `MSYi`? < `NGSv`?.

The button states themselves are the definition's frames: `FRCi` counts
Up/Over/Down/Over-While-Down, and the state booleans say which trailing
states exist — "include the Down state" is a statement about frame
count, already modeled.

**`setButtonOptions` lives on the button's slice, not the definition.**
The `MSTv` child list has no URL/alt/export nodes; what it does have is
the link: `URLv` (slice) carries `MSNi`? — "Symbol Unique Number" — so
a button's auto-slice points back at its definition. `setButtonOptions`
therefore mutates the slice(s) whose `MSNi` matches the definition's
`MSNi`: `A2Ts` (URL → `SliceObject::link`), `ALTs` (alt),
`OBNs` (sliceName), `EXPv` (exportOptions). The reader previously left
the slice's `MSNi` in the bag ("MSNi and anything else: preserved");
it is now typed as `SliceObject::symbol_number`.

**No `.jsf` usage.** A sweep of the CS6 `Configuration/` tree finds
zero uses of any of the five verbs (the only `setButton*` hits are
jQuery mobile inside `HTML Code/`, not Fireworks automation). The API
text plus the dictionary are the authority.

## Design

- **Model:** `SymbolDefinition` grows four `Option<bool>` fields
  (`auto_slice`, `include_down_state`, `show_down_on_load`,
  `include_over_while_down`) — presence-tracked like `NGSv`: absent on
  the wire → `None` → omitted on write, so definitions without the
  nodes round-trip byte-exactly and the observed `true` on `ATSb`
  survives verbatim. The four booleans mean the same thing on a graphic
  definition (CS6 writes the defaults); `None` simply records that the
  file did not carry them.
- **Slice link:** `SliceObject::symbol_number: Option<i64>` — typed
  from the previously-bagged `MSNi`; `None` omitted on write.
- **Commands** (engine `symbol.rs`, `SetSymbolType` shape —
  `definition_position` lookup, `Delta::Library` capture):
  `symbol.setButtonAutoSlice`, `symbol.setButtonIncludeDownState`,
  `symbol.setButtonIncludeOverWhileDownState`,
  `symbol.setButtonShowDownOnLoad` take `{symbol, value}` and set the
  definition boolean. `symbol.setButtonOptions` takes
  `{symbol, link?, alt?, slice_name?, export?}` and applies each
  present field to every slice whose `symbol_number` matches;
  `None` = leave untouched (CS6 passes every argument; the payload
  carries only what changes, the house payload economy). No linked
  slice → `MissingTarget` (CS6's auto-slice invariant guarantees one).

## Deviations (recorded, not hidden)

1. **`targetTagString` and `statusMessage` are not carried.** The
   dictionary has no node for either (no fourcc reads as an HTML target
   or browser status line on `URLv`); they are CS6 export-time
   furniture without a wire home here. Recorded as unmodeled — the
   command payload omits them rather than inventing nodes.
2. **No auto-slice synthesis.** `setButtonAutoSlice(true)` records the
   flag; the engine does not generate the slice at export (buttons
   without a real `URLv` stay slice-less). Feeds the recorded
   auto-slice debt in the webexport path.
3. **State booleans do not resize `FRCi`.** CS6's Button Editor adds/
   removes state frames when the boxes toggle; here the frame count is
   independent data (a definition can carry a frame with no state
   flag). Editing `FRCi` stays `symbol`-level, not a side effect.
4. **Per-definition, not per-selection.** CS6 applies to "the button
   being edited" or "all selected buttons" (UI-session addressing);
   the commands address a definition by `MSNi` (the engine's addressing
   rule everywhere since `symbol.setType`).

## Consequences

- The four dictionary `unknown - TODO` entries are now typed; the
  vendored dictionary stays observational (no edit) — the mapping lives
  here and in the model docs.
- **No corpus oracle carries the booleans or the slice `MSNi` link**
  (zero fixtures), so per the standing rule the gate is the synthetic
  write→read→byte-equal round-trip with a wire-order check
  (`button_booleans_and_slice_link_round_trip`); a fixture arriving
  later supersedes it and the mapping is re-verified against it.
- Parity: the five rows flip M→C (**208 C / 45 P / 15 M / 88 N** at
  this batch's close); registry 178 → 183.
- Slice `MSNi` is now typed: round-trip floors rise with the new field,
  and any third-party slice-symbol link survives verbatim instead of
  riding the bag.
- Auto-slice synthesis at export remains open debt (webexport).

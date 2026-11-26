# Fireworks JavaScript engine errata compatibility review

**Reviewed:** 2026-11-25. **Reference:** [fwextensions/fwjs-errata](https://github.com/fwextensions/fwjs-errata), a community compendium of observed bugs and undocumented behavior in Adobe Fireworks' JavaScript extension host.

The compendium records both language-engine behavior and host DOM bugs. Examples include Fireworks-native `FwArray`/`FwDict` values not behaving like standard arrays/objects; `in` and `hasOwnProperty()` inconsistencies; `customData` aliasing after duplicate and limited deletion; `pngText` string/length limits; short `Files.readLine()` limits; nonstandard regex and `Array.sort()` behavior; URI helper crashes; frame arguments ignored by some DOM calls; gradient handles not moving with grouped elements; and inconsistent symbol-script invocation counts. It also documents useful old-engine capabilities such as `toSource()`, getters/setters and filesystem helpers.

## What FireCraft should do

FireCraft's scripting host is an independent compatibility implementation. Treat this repository as behavioral test input, not an authority to copy code or defects. For each observed behavior:

1. Decide whether it is a script-compatibility expectation or a bug that FireCraft should correct.
2. Add a minimal clean-room regression fixture for the script-visible behavior, where it is in scope.
3. Preserve safe legacy expectations (for example, array-like `fw.selection` and legacy field/property shapes) behind a documented adapter when feasible.
4. Prefer correct, typed, deterministic FireCraft behavior over intentionally reproducing data corruption, wrong-frame mutation, crashes or silent no-ops.
5. Document intentional differences and provide diagnostics when a script depends on an unsupported quirk.

## Compatibility matrix to build

| Observed errata | FireCraft relevance | Proposed check |
|---|---|---|
| Native arrays/dictionaries differ from standard JS objects | Host adapter shapes are user-visible to `.jsf` scripts | Script fixtures exercise selection arrays, document arrays, custom-data maps, iteration and property checks |
| `customData` object sharing after duplicate; `delete` ineffective | Document metadata and duplicate semantics can corrupt user expectations | Verify clone independence, save/reopen identity and removal semantics; FireCraft should avoid shared mutable metadata |
| Document `pngText` truncates and coerces values | FireCraft's native format has unknown-node preservation and its own metadata contract | Test bounded, versioned extension metadata and exact preservation without reproducing legacy truncation |
| `Files.readLine()` truncates long lines | Host file I/O affects scripts and `.fwsteps` | Long-line and large-file fixtures with explicit caps/errors |
| Assignment-in-condition, regex, sort return, URI functions differ | JavaScript language compatibility | Curated interpreter conformance fixtures; do not emulate crashes, and clearly specify any supported legacy shim |
| Frame indexes ignored or current-frame layer objects leaked | Multi-state/frame scripts can mutate the wrong content | Every frame-targeting verb must prove active-frame routing and reject mismatched/stale frame references |
| Element lock/visible indexing reversed | Silent target mismatch | Round-trip selection-to-object mapping tests for first, middle and last element |
| `moveSelectionTo` leaves grouped gradient handles behind | Canvas transform correctness | Grouped-gradient movement fixture compares shape and handle transforms |
| Symbol scripts execute multiple times during UI operations | Performance and side-effect safety | Define invocation count/event semantics in FireCraft; never repeat side-effecting callbacks without a contract |
| `Files` and `File` APIs differ | Script portability and permissions | Capability tests per platform and a clear, sandboxed API contract |

## Clean-room boundary

The reference discusses examining/decompiling Adobe's interpreter source. That is outside FireCraft's clean-room rules and is not part of this plan. Use only public behavioral observations and independent tests. Do not copy source, reverse-engineer the implementation or deliberately reproduce crash/data-loss bugs.

## Planned sequence

1. Cross-reference the current `.jsf` corpus with this matrix and existing scripting tickets.
2. File regression fixtures only where a current FireCraft behavior is absent or unverified.
3. Define the intended compatibility envelope and differences in `docs/USAGE.md` and the scripting docs.
4. Add a script diagnostics panel/Automation Console only after specifying bounded output, capabilities, cancellation and error locations; see [`fwconsole-benchmark.md`](fwconsole-benchmark.md).

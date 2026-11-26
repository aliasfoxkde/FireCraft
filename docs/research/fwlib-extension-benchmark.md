# fwlib extension-library benchmark

**Reviewed:** 2026-11-25. **Reference:** [fwextensions/fwlib](https://github.com/fwextensions/fwlib), described as AMD modules for Adobe Fireworks extension developers. The repository page identifies an MIT license; this note captures concepts only and does not propose porting the code.

## Useful design lessons

### Persisted document data needs a supported container

`fwlib/DomStorage` exists because Fireworks' `dom.pngText` property is limited to short strings. The library serializes JSON, splits it into chunks and restores it from the document, so extension-owned metadata travels with the document rather than becoming a missing sidecar.

FireCraft already preserves unknown Fireworks PNG nodes/chunks and owns its document model. The transferable idea is a documented, namespaced metadata extension point with explicit versioning, size limits, migration rules and unknown-field preservation. Do not reproduce Fireworks' storage workaround or assume its 1023-character behavior applies to FireCraft.

### Hide awkward layer traversal behind a domain API

The `layers` module wraps Fireworks' parent-layer traversal and above/below navigation. FireCraft should provide user-facing and extension-facing helpers at the document/engine boundary for ancestor lookup, sibling ordering and moving selected objects/layers, while maintaining a single source of truth and typed errors.

### Make plugin work reliable across sessions

The `prefs` module stores persistent extension settings as JSON in preference files. FireCraft's own preferences and future extension settings should use a versioned schema, bounded data, stable namespacing, import/export and explicit migration rather than scripts writing ad hoc files.

### Give extensions reusable infrastructure

The repository includes file helpers, richer font inspection and a module-loading arrangement for Fireworks' JavaScript runtime. FireCraft's scripting and MCP foundations could evolve into a supported extension SDK: documented file dialogs, format metadata, font catalog access, command registration, undo transactions, preferences and typed document access, with platform capability declarations.

## Applicability and cautions

- `fwlib` is an extension-support library, not a modern editor. Its purpose is to improve extension authoring and work around limitations of Adobe's scripting environment.
- FireCraft has its own Rust engine, sandboxed scripting and command registry; do not carry over AMD/RequireJS, legacy APIs, or assumptions about Adobe object types.
- License metadata is MIT on the repository page. Still treat the source as reference-only under FireCraft's clean-room rule, and review license/attribution before any direct reuse.

## Proposed FireCraft follow-up

1. Specify the document metadata extension contract and test unknown-version/oversize behavior.
2. Audit layer hierarchy helpers for stable API seams and consistent undo semantics.
3. Define versioned extension preferences and file-access capabilities.
4. Draft an extension SDK surface that reuses command-registry semantics across desktop, web, CLI and MCP.
5. Prototype a documented helper package only after a plugin threat model and platform capability model are accepted.

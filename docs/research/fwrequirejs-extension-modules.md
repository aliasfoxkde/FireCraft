# fwrequirejs extension-module benchmark

**Reviewed:** 2026-11-25. **Reference:** [fwextensions/fwrequirejs](https://github.com/fwextensions/fwrequirejs). Its README describes a RequireJS/AMD loader adapted to Adobe Fireworks' extension environment, including separate contexts so extensions can use different module versions without sharing one loader namespace.

## Transferable lesson

Extensions benefit from explicit module ownership, dependency declarations and version boundaries, especially when extension load order or a shared JavaScript global can otherwise cause collisions. FireCraft should keep its own stable extension API with declared dependencies, namespaced preferences, capability limits and version negotiation. Isolation should be enforced by FireCraft's runtime rather than relying on every extension author to avoid global names.

## FireCraft application

1. Inventory the current `.jsf` and `.fwsteps` loading paths, scripting runtime, extension manager and command registration boundaries.
2. Define a versioned module manifest and dependency contract only after the extension SDK's capability model is specified.
3. Ensure two extensions can coexist when they depend on different helper versions; fail with actionable diagnostics for missing or incompatible dependencies.
4. Keep filesystem, network, document mutation and process permissions explicit and user-visible.

## Limits

This is a legacy loader for an old host. Do not port RequireJS or copy implementation code. Use the architectural idea of explicit, isolated module contexts only if the current Rust/JS runtime needs it. FireCraft's clean-room rule remains in force; no Adobe implementation or assets are used.

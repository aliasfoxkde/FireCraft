# T31 — Measured modular delivery and extension packages

**Status:** Planned — current build already applies Rust release LTO, a web-specific size optimization, Binaryen optimization, gzip staging and an optional CDN URL; no verified per-feature lazy-loading or stable third-party extension package contract exists yet.

**Objective:** Keep FireCraft within its WebOS/browser delivery constraints while making optional features and extensions independently loadable across WASM and native platforms, without breaking document compatibility, offline editing or the shared engine.

**Why this task exists:** Splitting a binary into files does not reduce the initial download when all files are fetched at startup. Native Rust dynamic libraries also do not provide a stable cross-toolchain ABI by default. Decide module boundaries from measured first-load cost, feature-use frequency, platform limits and lifecycle needs.

## Existing evidence to keep current

- Workspace release profile uses thin LTO. The web release profile and Binaryen flags are configured in the web distribution path; verify the exact current commands before changing them.
- `xtask dist --web` has a raw WASM investigation warning and hard ceiling, and stages gzip representations for WASM, JavaScript and HTML. The serving host must serve them with correct content-encoding and MIME headers.
- `WASM_CDN_URL` can move the WASM object off the Pages bundle host. Record both the Pages per-object constraint and the CDN response behavior; compressed sidecars do not help if the deployed host ignores them.
- Raw WASM bytes, compressed transfer bytes, all startup resources and lazy feature resources are separate metrics. Track total installed native size separately from per-file size.
- Rust identifiers are not delivered as source-level variables in WASM. Identifier renaming/obfuscation is not a meaningful size optimization. Strip debug/name data where appropriate, minify generated JavaScript, and measure Brotli/gzip; preserve source maps or symbol artifacts privately where needed for diagnostics.

## Work sequence

### T31.1 — baseline and budgets

- Build the same named commit reproducibly and report raw WASM, gzip and Brotli sizes, generated JS, HTML, initial total transfer, deferred-resource transfer, startup-to-interactive time and peak memory.
- Measure top feature weights and usage paths; keep core document open/save, editing, undo and recovery eager.
- Reconcile `xtask/src/dist.rs`, deployment configuration, `docs/decisions/0090-wasm-bundle-size-policy.md`, and current historical measurements. Do not lower an established parity floor without an accepted decision.
- Add a generated report or deterministic command output and attach a known-good artifact receipt.

**Acceptance:** A repeatable report includes commit/toolchain, artifact digests, raw and compressed bytes, and load measurements. A local test server and production-like host return correct `Content-Type` and `Content-Encoding` for each representation.

### T31.2 — optimize before splitting

- Inspect release profile, target features, panic strategy, dead-code elimination, LTO, stripping and Binaryen flags; compare each candidate against the same baseline.
- Minify generated JS as part of the build only if trunk does not already do so; do not minify Rust source or claim identifier obfuscation is a size optimization.
- Validate CDN/off-origin WASM loading, caching, integrity/versioning, CSP/CORS and offline/fallback behavior.
- Prefer Brotli or platform-native dynamic compression when supported; retain gzip where compatibility or static hosting requires it.

**Acceptance:** Keep only changes that reduce measured transfer or startup cost without violating crash diagnostics, reproducibility or browser support. Record before/after measurements.

### T31.3 — true lazy modules

- Prototype one high-weight optional feature using a versioned module boundary and deferred fetch/initialization. Candidate selection follows T31.1 measurements; do not choose by intuition alone.
- Verify the module is absent from initial network requests, loads on demand, reports progress/errors, caches by content digest and fails without taking down core editing.
- Check `wasm-bindgen` split-linked-module and bundler behavior against the actual Trunk/WebOS embedding path; linked-module splitting alone does not guarantee lazy loading.
- Preserve common document commands and serialization semantics in shared engine crates; keep UI shell or optional codecs out of core unless required.

**Acceptance:** Browser network evidence proves the feature is not fetched at startup; cold and warm load latency, total bytes, offline failure behavior and WebOS compatibility are recorded. The existing bridge/browser smoke and size gate pass.

### T31.4 — cross-platform extension contract

- Define a versioned manifest: package ID/version, supported FireCraft API range, capabilities, entry point, resource limits, permissions, dependencies, signature/provenance and uninstall/rollback data.
- Prefer a sandboxed WASM extension API shared by native and browser hosts, with explicit host imports for commands, panels, document access, storage, dialogs and network. Require user-visible grants for sensitive capabilities.
- Keep native-only integrations behind narrow host adapters. Use process/RPC isolation where an extension needs operating-system access; do not expose an unstable Rust ABI through arbitrary DLL/dylib loading.
- Specify discovery, installation, update, disable, crash isolation, compatibility checks, telemetry defaults and offline operation before enabling third-party packages.
- Document trusted first-party packages separately from community packages and make package review/signing expectations clear.

**Acceptance:** A sample extension is a real package, installs and uninstalls on Windows, Linux, macOS and WASM/WebOS hosts that claim support, cannot access undeclared capabilities, and is tested against incompatible API versions and malformed packages. No fake data or placeholder extension is used.

## Decisions and constraints

- Do not split the core engine merely to make the file smaller. Preserve the crate layering and one document authority.
- Do not promise dynamic taskbar/plugin/UI capabilities where the host lacks them; report support per host.
- Do not call minification “obfuscation.” FireCraft is open source; code concealment is not a product requirement. Shipping debug symbols separately is a debugging/distribution choice.
- Cloudflare Pages per-object limits, CDN behavior, cache headers and deployment products can change. Recheck current limits and serving semantics at implementation time.

## References

- [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html): optimization levels, LTO, stripping and debug settings.
- [wasm-bindgen guide](https://rustwasm.github.io/docs/wasm-bindgen/): module generation and split linked modules; confirm behavior against the current Trunk bundler.
- [MDN: HTTP content negotiation](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Content_negotiation): content encoding and response headers.
- [WebAssembly component model](https://component-model.bytecodealliance.org/): interface-oriented interoperability research; adoption remains an explicit architecture decision.

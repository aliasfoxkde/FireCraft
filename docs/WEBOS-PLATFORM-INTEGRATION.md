# FireCraft — WebOS Platform Integration Roadmap

> **Status:** long-term integration requirements; not a current release blocker and not a claim that native WebOS services exist.
> **Reviewed:** 2026-11-10.
> **Host platform plan:** [ArtCraft-WebOS shared platform architecture PR #36](https://github.com/aliasfoxkde/ArtCraft-WebOS/pull/36).
> **Current embedding contract:** [WEBOS.md](WEBOS.md).
> **Production work order:** [PRODUCTION-READINESS-PLAN.md](PRODUCTION-READINESS-PLAN.md).

## 1. Product role

FireCraft is not merely a browser editor that happens to render inside an iframe. It is intended to be a first-class creative application in ArtCraft WebOS and, eventually, a reusable part of a larger productivity operating environment.

FireCraft must preserve two equally important modes:

- **Standalone:** browser/PWA or desktop users can use FireCraft directly, without WebOS present.
- **Platform-integrated:** WebOS can launch FireCraft, persist its documents, discover its commands, invoke authorized operations, expose it to CLI/MCP/AI workflows, and later map the same app semantics onto native host services.

The WebOS environment is intended to provide a shared SDK, reusable UI components, common domain libraries, app/game package management, a CLI/MCP/AI control plane, and eventually browser-to-native/ISO/device delivery. FireCraft should consume those platform contracts rather than reimplement each service.

The companion WebOS architecture plan defines the host-side direction. This document defines what FireCraft must preserve and expose.

## 2. Current integration foundation

The current FireCraft implementation already provides:

- A WASM browser build with a postMessage bridge.
- Document open, registry command application, PNG export, and document export/save requests.
- A single engine command registry used by the desktop GUI and native CLI/MCP surfaces.
- A native CLI with JSON control-channel and MCP transports.
- A scripting engine whose queued mutations drain through the same command registry.
- A WebOS package descriptor and worked host page used by the release-byte bridge smoke.
- A WebOS app registration and host-side autosave/restore integration in the ArtCraft-WebOS repository.

This is a useful foundation, but it is not yet full WebOS CLI/MCP integration. The current browser bridge is a small host-control surface; the browser cannot expose a TCP listener like the native CLI. The WebOS architecture audit records the resulting Craft MCP gap.

## 3. Required architectural boundary

Keep these layers distinct:

1. **Document/domain core:** document model, parsers/writers, rendering, codecs, commands, validation, and scripting semantics.
2. **Command/capability catalog:** stable command IDs, input/output schemas, read/write classification, permissions, cancellation, progress, and typed errors.
3. **Application adapters:** desktop UI, browser UI, native CLI, native MCP, WebOS postMessage host bridge, and future native OS adapter.
4. **Platform services:** storage, file picker, clipboard, notifications, network, audio/GPU/input, and process/device capabilities supplied by WebOS or the native host.
5. **Transport/policy:** origin checks, app identity, user/session grants, request IDs, rate/size limits, approval, and audit.

All document mutations must continue through the same engine/registry semantics. Do not create a second set of WebOS-only commands that can diverge from the GUI/CLI/MCP behavior.

## 4. Future WebOS command and agent contract

The FireCraft bridge should evolve compatibly from the existing message contract. A future versioned handshake should make the following discoverable:

- Protocol/API version and compatible version range.
- FireCraft app ID, application version, build/source identity, and runtime capabilities.
- Available operations and command schemas, including argument types, defaults, validation, and result/error shape.
- Read-only versus mutating operations, destructive/privileged classifications, and requested capabilities.
- Current document state (open/dirty/name/type), without leaking document contents unless authorized.
- Health/readiness, progress, cancellation, and structured diagnostics.
- Correlation/request IDs and bounded payload/file sizes.

Potential host capabilities include app-private file access, open/save/export dialogs, autosave/recovery, clipboard, notifications, and agent invocation. They must be granted by the host, scoped to FireCraft's app identity and the active user/session, and revocable.

The WebOS host should act as the browser-side broker for external CLI/MCP/AI clients. It can route an authorized typed tool call to the running FireCraft iframe over the browser bridge and return the result to the client. A browser app must not pretend it can open arbitrary TCP listeners. Native local MCP can remain a separate transport that calls the same core command service.

## 5. Security requirements before expanding agent control

The current FireCraft documentation says the host must validate message origins because the bundle may send replies with a wildcard target origin. The WebOS integration currently checks the iframe source window for messages, while the future bridge contract must require both source-window identity and exact expected origin on every request/reply path.

Before exposing general command or MCP access:

- Introduce a protocol version and request ID for every request/reply pair.
- Validate message source, exact origin, app ID, protocol version, message type, and payload schema.
- Reply only to the exact expected origin; migrate away from wildcard replies when a negotiated host origin is available.
- Bind each request to an authorized app/user/session and an explicit capability set.
- Distinguish read-only operations from document mutations, file writes/deletes, network access, installs, and other high-impact actions.
- Require host/user approval for destructive, irreversible, externally visible, or privileged operations.
- Enforce payload/file-size caps, timeouts, cancellation, concurrency limits, and rate limits.
- Treat document text, imported files, web content, and app output as untrusted data for AI-agent purposes; do not let prompt injection grant tools or capabilities.
- Record auditable action metadata without retaining document contents or secrets by default.
- Add hostile-origin, sibling-frame, malformed-message, replay, cross-session, oversized-payload, and revoked-capability tests.

Do not expose arbitrary JavaScript eval as a shortcut to AI control. The supported surface should be typed tools over the same command catalog.

## 6. Shared libraries and component strategy

FireCraft should be designed for reuse with sibling Craft apps without turning the app into a monolith or prematurely extracting unstable abstractions.

Candidate shared domain libraries include:

- Color and geometry primitives.
- Document/scene models and portable serialization contracts where formats genuinely overlap.
- Raster/vector operations, codecs, compositing, and filters.
- File and asset import/export adapters.
- Typed command schemas, validation, errors, progress, and MCP mapping.
- Scripting host primitives only where language/runtime semantics are shared and security boundaries match.

The WebOS host should own platform UI primitives and app-host services. FireCraft should own editor-specific canvas tools, inspectors, panels, and document semantics. Reuse the host's design tokens, dialogs, notifications, menus, file APIs, and command palette through a stable SDK rather than duplicating them in every app.

### Important size and runtime distinction

A shared Cargo crate reduces duplicated source and maintenance, but when multiple Craft projects compile separate WASM binaries, the crate's machine code is normally linked into each app. That does not automatically reduce the aggregate download or installed size.

To reduce cross-app overhead, evaluate:
- shared host services instead of each app embedding the same platform behavior;
- versioned shared TypeScript/ES modules for browser-hosted common code where origin/CORS/CSP rules permit;
- shared WASM components/modules or a host-provided runtime service where import/ABI/isolation support is proven;
- common app design-system components and lazy entry chunks;
- package dependency deduplication for the future native distribution.

Measure per-app and aggregate cold/warm downloads, compressed bytes, cache hit behavior, startup, resident memory, and compatibility. Cross-origin iframe apps do not share JavaScript memory merely because they use the same library source. Avoid extracting a shared library without at least two real consumers, an owner, versioning, contract tests, license review, and measurable benefit.

## 7. Native host integration

When WebOS has a native host, FireCraft should continue using its same document and command core while platform adapters supply:

- native file access, file picker, atomic save/replace, and recovery paths;
- clipboard, drag/drop, notifications, and app/window lifecycle;
- GPU/canvas, audio, input, gamepad, and display scaling;
- process, network, credential, and device services only through explicit grants.

The installed WebOS host may use a Linux base and Btrfs for system/user storage, snapshots, rollback, and recovery. Those are host operating-system responsibilities. FireCraft should consume a portable filesystem/storage API and must not depend directly on Btrfs, a bootloader, or raw block devices. A browser build cannot implement or validate an ISO installer or native ROM.

## 8. Integration test matrix

The cross-repository contract needs automated tests for:

1. Standalone FireCraft opens and edits a document without a WebOS host.
2. WebOS launches FireCraft and restores its app-private autosave/document state.
3. The host performs open, command apply, PNG export, and document save over the release WASM bytes.
4. A command discovered from FireCraft's schema produces the same document result through GUI, native CLI, native MCP, and WebOS-hosted invocation.
5. The host exposes FireCraft tools to an authorized MCP/AI client and returns typed results, errors, progress, and cancellation.
6. Unauthorized origins, sibling frames, expired sessions, missing capabilities, malformed/replayed requests, and oversized payloads are rejected.
7. Save, autosave, recovery, export, and file permissions are correct under both browser storage and future native storage adapters.
8. Shared library/API version mismatch fails clearly and does not corrupt documents.
9. Native adapter behavior is conformance-tested against the browser implementation before the native target is declared supported.

## 9. Sequencing

This is deliberately not a blocker for the current FireCraft browser release. Keep the following sequence:

- **Now:** preserve the current single-registry semantics; keep the basic bridge smoke green; retain native CLI/MCP/scripting behavior; measure the web build and prune only proven target-irrelevant dependencies.
- **Next:** define a versioned command schema and bridge handshake; add host identity/origin validation and conformance fixtures; coordinate with the WebOS SDK owner.
- **Later:** implement full WebOS CLI/MCP routing and AI approval/audit; adopt shared domain libraries only after cross-app consumers exist; integrate shared design-system and platform-service components.
- **Native phase:** implement host adapters and validate FireCraft inside the native WebOS runtime; do not add Btrfs or installer concerns to FireCraft's document core.

Acceptance requires cross-repository tests and a versioned compatibility policy. A green standalone browser smoke alone does not prove complete WebOS CLI/MCP integration.

## 10. Reuse the Craft foundation and expose a stable FireCraft SDK

FireCraft should take advantage of the shared work already underway in the wider Craft ecosystem without surrendering ownership of its engine, product behavior, release process, or roadmap.

### Existing upstream work to evaluate

- [storytold/craft-libs](https://github.com/storytold/craft-libs) is an established Rust workspace of small shared libraries under a permissive MIT OR Apache-2.0 license. It already contains relevant candidates: craft-atomic, craft-cms, craft-settings, craft-platform, craft-dialogs, craft-keymap, craft-ui, craft-ai-core, craft-web-api, and craft-web-host.
- The web API and host crates are particularly relevant prior art for a common app-control channel and parent-page bridge. Compare their request/response, discovery, origin validation, versioning, and permission semantics with FireCraft's current bridge before considering a staged migration.
- craft-ai-core is a shared data model and contract, not a complete agent executor or an automatic provider integration. Keep execution adapters, user consent, host policy, and FireCraft command invocation explicit.
- The [shared-library candidate process](https://github.com/storytold/craft-libs/issues/2) deliberately distinguishes extraction, consolidation, API standardization, and component reuse. It also requires multiple consumers, a meaningful correctness/performance/maintenance benefit, and preservation of app-specific behavior.

Do not adopt a crate solely because its name looks relevant. For each candidate, record its API fit, Rust/egui version compatibility, dependency and feature footprint on native and wasm32-unknown-unknown, license and asset notices, target support, tests, maintenance activity, and measured effect on FireCraft's build and runtime. Keep FireCraft's document model, Fireworks-compatibility rules, editing policy, and UX in FireCraft. Shared crates should own mechanisms and neutral contracts, not force every application to behave identically.

### SDK layers and public contract

The first SDK should be a thin, stable facade over the existing FireCraft engine and command registry—not a rewrite of the engine and not a universal application framework.

1. **Domain/API layer:** stable component identity, version, capability discovery, document open/inspect/mutate/export operations, typed command metadata and results, errors, progress, cancellation, and compatibility negotiation.
2. **Language-neutral contract:** versioned JSON Schema (or a reviewed equivalent such as WIT for component interfaces) for commands, manifests, events, capabilities, and errors. Treat the schema as the contract; do not make WebOS consume private Rust structs.
3. **Runtime adapters:** native Rust API, CLI, MCP transport, browser WASM/JavaScript bridge, and WebOS host adapter. Each adapter translates to the same authoritative command service.
4. **Developer distribution:** Cargo dependencies pinned to tested revisions initially; WASM plus its host client; API docs, examples, integration tests, licenses/attribution, and a machine-readable compatibility manifest. Add crates.io publication, a C ABI, or other language bindings only after a real downstream consumer and a support owner justify their maintenance burden.

The SDK must be independently usable by a third-party host. WebOS is an important consumer and conformance target, not a hidden runtime dependency. Maintain a standalone FireCraft release and test path even when SDK packages are published alongside it.

### Shared engineering and release practices

Use [craft-common-actions](https://github.com/storytold/craft-common-actions) as a source of proven CI/CD patterns and reusable workflows: shared quality gates, packaging lint, cross-platform build matrices, package signing, checksums, and release aggregation. Evaluate the existing workflow's Storytold-specific Git dependency checks, font/i18n inputs, packaging conventions, and job matrix before enabling it in FireCraft. Keep FireCraft-specific browser bridge tests, document-format corpus tests, and deployment verification mandatory.

The candidate fork at [artcraft-candidates/FireCraft](https://github.com/artcraft-candidates/FireCraft) is the appropriate first compatibility environment for shared workflows and cross-app crate testing. The canonical repository remains [aliasfoxkde/FireCraft](https://github.com/aliasfoxkde/FireCraft). Reusable improvements can be contributed upstream; once verified, workflow adoption and product changes return through reviewed PRs to the canonical repo.


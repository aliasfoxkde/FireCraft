# 0071 — The WASM bundle stays on Cloudflare Pages; the 21.5 MB figure is our budget, not a platform limit

- **Date:** 2026-11-06
- **Status:** Accepted
- **Supersedes:** nothing
- **Relates to:** `xtask/src/dist.rs` (the size ceiling and the web
  distribution recipe), `docs/HANDOFF.md` (release runbook), decision
  0070 (the gate that runs before a release)

## Context

The release recipe pins a WASM bundle ceiling:

```rust
// xtask/src/dist.rs
const WASM_SIZE_CEILING: u64 = 21_500_000;
```

`enforce_wasm_budget` fails the build when the bundle exceeds it. On
2026-11-06 that number was asked about directly: is 21.5 MB a limit of
WASM 3.0, of GitHub, or of Cloudflare Pages — and could the bundle just
be loaded from GitHub instead?

The number is **FireCraft's own budget**. It is not a property of the
WebAssembly standard, of WASM 3.0 or of any host: it is a tripwire the
project sets for itself, deliberately re-pinned each time the bundle
grows (the release notes for v0.2.5 record a re-pin). What matters for
a release is therefore the *external* limits, which were measured rather
than recalled.

Measured facts:

- The shipped bundle is `dist/web/firecraft-web-25ef8a220b81e769_bg.wasm`
  — **21,271,259 B raw, 7,376,731 B gzipped** (the gzip figure
  measured on that artifact).
- Cloudflare Pages enforces a **25 MiB (26,214,400 B) per-asset
  limit**. The bundle is therefore ~4.94 MB under the only binding
  external cap, and ~18.8 MB under it once gzipped on the wire.
- GitHub's raw file host serves `.wasm` with the MIME type
  `application/octet-stream`, not `application/wasm`. This was
  confirmed by listing a real `.wasm` path through the GitHub API
  (`gh api repos/rustwasm/wasm-bindgen/git/trees/main?recursive=1`)
  and reading the response headers. `WebAssembly.instantiateStreaming`
  rejects a response whose MIME type is not `application/wasm`, so a
  loader that uses it throws a `TypeError` against a GitHub raw URL and
  must fall back to fetching the bytes and handing an `ArrayBuffer` to
  `instantiate` directly.
  GitHub raw serves with a wildcard CORS policy, so the cross-origin
  fetch itself is not the problem.
- A GitHub raw URL points at a ref: `refs/tags/v0.2.5/...` is mutable
  in principle and caches indefinitely in practice, so a redeploy of the
  same tag can be served stale from a browser cache. Cloudflare Pages
  serves a deployment-scoped, content-addressed URL with immutable
  caching.

## Decision

1. **The bundle stays on Cloudflare Pages.** It is the host the
   release recipe already publishes to, it serves the correct MIME type
   for streaming instantiation, and its URLs are immutable.

2. **The 21.5 MB ceiling stays as a budget, not a limit.** It is
   FireCraft's own regression tripwire: a bundle that grows past it
   means the code got heavier and someone should look. It is re-pinned
   deliberately, and the re-pin is recorded in the release notes.

3. **If the bundle ever outgrows 25 MiB, the move is Cloudflare R2 (or
   equivalent object storage) behind a Worker that serves
   `application/wasm` with immutable, content-addressed URLs** — not
   GitHub raw. The MIME type is the disqualifying detail for GitHub
   raw, and object storage gives the same immutable-URL property that
   Pages provides today.

4. **Loading the WASM from GitHub is a fallback, not an improvement.**
   It is workable — the fetch succeeds and the MIME type can be worked
   around — but it trades away streaming instantiation and immutable
   caching for nothing that is needed before the 25 MiB cap is in
   reach.

## Consequences

- No release-recipe change is required today: the bundle is under the
  external cap with ~4.94 MB of headroom, and the gzip figure means
  the wire cost is ~7.4 MB.
- The ceiling stays in the gate, and a bundle approaching 25 MiB is
  visible as a budget trip before it becomes a Pages upload failure.
- The MIME-type constraint is written down rather than discovered: a
  future host change has to serve `application/wasm`, and a loader
  that streams must not be pointed at a host that does not.

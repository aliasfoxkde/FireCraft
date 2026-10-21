# Embedding FireCraft in WebOS

The web bundle is the primary release target (PLANNING §2): the same
app shell the desktop build runs, compiled for wasm32 and driven from
a host page over a postMessage bridge (M5). This document is the
embedding contract — and it is load-bearing: the worked host page
below is extracted **verbatim** by `xtask dist --web` and used as the
bridge smoke, so the page taught here is the page proven against the
release bytes on every dist run.

## Getting the bundle

    cargo run -p xtask -- dist --web

assembles `dist/web/`: the trunk `--release` bundle (`index.html`,
one `.js`, one `.wasm`), a `MANIFEST.sha256` over every file
(`sha256sum -c MANIFEST.sha256` verifies it), the wasm size budget
enforced, and the bridge smoke green. Serve the directory over
HTTP(S) — `application/wasm` is required for
`WebAssembly.instantiateStreaming`; `file://` will not work.

## The iframe contract

Embed `index.html` in an iframe. It brings its own
`#firecraft-canvas` (eframe owns sizing — give the iframe real
width/height) and registers a `message` listener once the shell is
up.

**Inbound** (host → bundle, via `iframe.contentWindow.postMessage`):

| message | meaning |
|---|---|
| `{ type: "openDocument", bytes: ArrayBuffer }` | load a `.fw.png` through the standard read path |
| `{ type: "apply", name: string, payload?: object }` | run one registry command (the same ids the CLI and MCP server speak) |
| `{ type: "exportPng" }` | render the current page/frame, reply with PNG bytes |
| `{ type: "exportDocument" }` | canonical save, reply with `.fw.png` bytes |
| `{ type: "ping" }` | liveness check, replies `ready` |
| a bare `ArrayBuffer` | shorthand for `openDocument` |

**Outbound** (bundle → host, via `parent.postMessage`): every reply
carries `source: "firecraft"` — filter on it, or you will hear your
own messages —

| reply | fields |
|---|---|
| `{ source, type: "ready", ok: true }` | the shell is up (also the `ping` answer) |
| `{ source, type: "opened", ok, error? }` | openDocument outcome |
| `{ source, type: "applied", ok, error? }` | apply outcome |
| `{ source, type: "png", bytes: ArrayBuffer }` | rendered pixels |
| `{ source, type: "document", bytes: ArrayBuffer }` | saved document bytes |

**Origin rules.** The bundle posts its replies to `"*"` — a WebOS may
mount it on any origin — so the *host* must validate
`event.origin` against the origin it loaded the bundle from and
ignore everything else. The bundle, in turn, ignores messages whose
`source` is `"firecraft"` and anything without a `type` it knows.
Commands through `apply` are registry ids only: there is no side
door; the bridge replays through the same journal the GUI and CLI use.

## Worked host page

This is the canonical host page. `__FIXTURE_B64__` is a stand-in for
a base64 `.fw.png` payload (the dist smoke inlines the composed corpus
fixture there); a real host fetches or composes its own bytes. The
markers below are the contract with `xtask dist --web` — keep them
intact when editing this page, or the smoke has nothing to prove.

<!-- begin: smoke host page -->
```html
<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>FireCraft host</title></head>
<body style="margin:0;background:#111">
<iframe id="firecraft" src="/index.html" style="width:100vw;height:100vh;border:0"></iframe>
<script>
"use strict";
const frame = document.getElementById("firecraft");
// The tab title doubles as the status line: a hosting shell (and the
// dist smoke) can watch for "green" without touching the page.
const fail = (why) => { document.title = "FireCraft host: FAIL " + why; };
const post = (message) => frame.contentWindow.postMessage(message, "*");
function fixtureBytes() {
    const bin = atob("__FIXTURE_B64__");
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return bytes;
}
let pngBytes = 0;
window.addEventListener("message", (event) => {
    const data = event.data;
    if (!data || data.source !== "firecraft") return;
    if (data.type === "ready") {
        post({ type: "openDocument", bytes: fixtureBytes().buffer });
    } else if (data.type === "opened") {
        if (!data.ok) return fail("openDocument: " + (data.error || "unknown"));
        post({
            type: "apply", name: "object.createPath",
            payload: { kind: "rect", x: 1, y: 1, width: 4, height: 4, page: 0, layer: 0, cell: 0 },
        });
    } else if (data.type === "applied") {
        if (!data.ok) return fail("apply: " + (data.error || "unknown"));
        post({ type: "exportPng" });
    } else if (data.type === "png") {
        if (!data.bytes || data.bytes.byteLength === 0) return fail("exportPng: empty");
        pngBytes = data.bytes.byteLength;
        post({ type: "exportDocument" });
    } else if (data.type === "document") {
        if (!data.bytes || data.bytes.byteLength === 0) return fail("exportDocument: empty");
        document.title = "FireCraft host: green (png " + pngBytes + " B, doc " +
            data.bytes.byteLength + " B)";
    }
});
</script></body></html>
```
<!-- end: smoke host page -->

## Autosave (host-owned OPFS)

The bundle is deliberately stateless between loads: browser file
writes download, they do not persist (there is no path-addressable
file system in the browser). Persistence belongs to the host, which
is exactly what the bridge is for — keep the last document in the
Origin Private File System and replay it on launch:

```js
const root = await navigator.storage.getDirectory();
// save: after any "applied" reply, debounce, then
const handle = await root.getFileHandle("autosave.fw.png", { create: true });
const saved = await postForBytes(frame.contentWindow, { type: "exportDocument" });
await handle.createWritable().then((w) => w.write(saved).then(() => w.close()));
// launch: before openDocument, restore
const file = await root.getFileHandle("autosave.fw.png").catch(() => null);
if (file) post({ type: "openDocument", bytes: await (await file.getFile()).arrayBuffer() });
```

(`postForBytes` is the host's request/reply correlation: tag each
request, resolve on the matching `source: "firecraft"` reply.)

## WebOS packaging

A static bundle on a static origin — the shelf entry is metadata, not
code. Sketch of the app manifest (`appinfo.json`, fields per the
target platform's packaging spec; icons/backgrounds are
operator-supplied art — see `docs/ATTRIBUTION.md` before adding any
asset that is not your own):

```json
{
  "id": "dev.firecraft.web",
  "version": "@VERSION@",
  "vendor": "FireCraft",
  "type": "web",
  "main": "index.html",
  "title": "FireCraft",
  "icon": "assets/icon.png",
  "largeIcon": "assets/largeIcon.png",
  "bgImage": "assets/background.png"
}
```

The `@VERSION@` placeholder is replaced at build time by `xtask dist
--web` (line 165 of `xtask/src/dist.rs`): it reads
`packaging/webos/appinfo.json`, substitutes `@VERSION@` with the workspace
version, and writes the stamped copy to `dist/web/appinfo.json`. The
committed file in `packaging/webos/` always carries `@VERSION@`.

The shipped layout is what `dist/web/` contains; the host page above
(`host.html` alongside the bundle) is the `main` entry when the WebOS
shell should drive the editor instead of showing it bare. Everything
ships as static files — no server-side component exists.

## What's proven (2026-10-20)

The following have been verified:
- **`dist --web` builds and stages correctly.** The bundle contains
  `index.html`, one `.js` bootstrapper, one `.wasm` binary, a
  `MANIFEST.sha256` over every file, and the stamped `appinfo.json`.
- **Bridge smoke passes.** Headless Chromium loads the bundle via the
  smoke host page, runs the full four-hop sequence (ping → ready →
  openDocument → apply → exportPng → exportDocument), and reports green
  in the page title. Verified at every `dist --web` run.
- **Production deployment.** `dist/web/` is deployed to
  <https://firecraft-web.pages.dev> via `wrangler pages deploy`.
  The served `MANIFEST.sha256` hashes match `dist/web/` for the
  current build. Verified: SHA-256 of the live wasm matches the zip
  artifact from the v0.2.2 release.
- **`appinfo.json` version stamping.** At build time, `@VERSION@` in
  `packaging/webos/appinfo.json` is replaced with the workspace version.
  The staged copy in `dist/web/appinfo.json` is stamped. Verified by
  `cat dist/web/appinfo.json` showing `"version": "0.2.2"` after a
  v0.2.2 build.

## What's not proven (gap)

The bundle has **never been installed on an actual WebOS device.** The
WebOS packaging is a dry-run: the artifact is correct, the smoke passes
in Chromium, and Cloudflare Pages serves it correctly — but no
operator has installed `appinfo.json` + the wasm bundle onto a WebOS
device and verified the app launches and responds to the bridge API.

**Blocker:** No WebOS device is in scope for this project. The gap
cannot be closed without either:
1. An operator with WebOS hardware who can install and test the bundle, or
2. A WebOS emulator that can run the app in CI.

The recipe (packaging/webos/appinfo.json + host.html) is complete. The
hardware proof is the only remaining step.

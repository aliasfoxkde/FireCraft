# P3 — Web production (the primary release target)

**Goal.** https://firecraft-web.pages.dev serves a build that CI produced from a
tagged commit and that anyone can trace. The WebOS bridge is versioned and
origin-safe. The app installs as a PWA and works offline. Open and Save use
real files, and paste accepts OS images.

**Exit check:**
- S9 = yes.
- The bridge v1 contract is documented and smoke-tested.
- Lighthouse reports the app installable.
- An autosave of a 20 MB document survives a reload.

**Lane:** web. Starts after P0.

**Every rung here touches `web/` or the shell.** The browser proof in
HANDOFF §6.2 is mandatory before the PR, and `xtask dist --web` must stay
green, because it runs the bridge smoke on the release bytes
(`xtask/src/dist.rs::smoke_bridge`).

**Key files:**
- `web/src/main.rs` (bridge, autosave, file input)
- `web/src/platform.rs`
- `web/index.html`
- `packaging/webos/{appinfo.json,host.html}`
- `xtask/src/dist.rs` (stamps and smoke)
- `docs/WEBOS.md` (contract)

---

### C3.01 — Serve `version.json` (build provenance) `[ ]`
- **Needs:** C0.08 · **Tags:** `[HOST:linux]` `[HOST:browser]` · **Size:** S
- **Steps:**
  1. In `xtask/src/dist.rs::web()`, write `dist/web/version.json` containing
     `{"version", "commit", "dirty", "builtAt", "wasmSha256"}`:
     - `commit`: `git rev-parse HEAD`;
     - `dirty`: `git status --porcelain` is non-empty;
     - `wasmSha256`: the digest the manifest already computes.
  2. Show `version` + short `commit` in Help ▸ About on web.
  3. Add a smoke assertion that `version.json` parses and that its digest
     matches the staged wasm.
- **Done when:** `dist --web` emits `version.json`, and the About dialog shows
  the commit.
- **Verify:** `cargo run -p xtask -- dist --web && cat dist/web/version.json`.
- **Moves:** prerequisite for S9.

### C3.02 — Make missing assets return 404 instead of index.html `[ ]`
- **Needs:** C3.01 · **Tags:** `[HOST:browser]` · **Size:** XS
- **Steps:**
  1. On Pages today, `/assets/icon.png` and `/manifest.webmanifest` return
     the SPA HTML with status 200, which hides broken references.
  2. Add `dist/web/_redirects` (or `404.html`) so that only `/` and
     `/index.html` serve the app, and unknown paths return 404. Cloudflare
     Pages semantics: a top-level `404.html` disables the SPA fallback.
  3. Generate it from `dist.rs`.
- **Done when:** `curl -sI <preview>/nope.png` returns 404.
- **Verify:** the curl above against a preview deploy (C3.03), or
  `wrangler pages dev dist/web`.
- **Moves:** surfaces broken assets.

### C3.03 — CI web deploy job (preview on PR, production on tag) `[ ]`
- **Needs:** C3.01, C0.11 (secrets) · **Tags:** `[MAINTAINER]` (secrets) ·
  **Size:** M
- **Steps:**
  1. Add `.github/workflows/deploy-web.yml`:
     - **on `pull_request`:** build `xtask dist --web`, then run
       `wrangler pages deploy dist/web --project-name firecraft-web
       --branch pr-<n>` (a preview). Comment the preview URL on the PR.
     - **on `push: tags: v*`:** same build from the tag SHA, deployed to the
       production branch.
     - **on `workflow_dispatch`:** manual production deploy of a given tag.
  2. Pin the actions by SHA (decision 0099). Run
     `scripts/validate_workflow_pins.py`.
  3. After the deploy, `curl` `version.json` and fail the job if its
     `commit` ≠ `github.sha`.
  4. Delete the commented-out deploy step at `release.yml:382` and point its
     comment to the new workflow.
- **Done when:** a PR gets a working preview URL whose `version.json` names
  the PR head.
- **Verify:** the PR comment, plus `curl <preview>/version.json`.
- **Moves:** S9 tooling.

### C3.04 — Production redeploy from a tag (closes #30 / T18) `[ ]`
- **Needs:** C3.03 · **Tags:** `[MAINTAINER]` (approves the production
  deploy) · **Size:** XS
- **Steps:**
  1. The maintainer picks a tag: the next patch, e.g. `v0.2.6`, cut through
     the RELEASING.md recipe, or a re-run on `v0.2.5`.
  2. Run the `deploy-web.yml` workflow_dispatch for it.
  3. Record in `docs/TASKS.md` and on #30:
     - the tag, commit and run ID;
     - the `version.json` content;
     - the live `wasmSha256`.
- **Done when:** the live `version.json` matches a tag commit.
- **Verify:** `curl -s https://firecraft-web.pages.dev/version.json`.
- **Moves:** **S9 no → yes.**

### C3.05 — Bridge: reply only to the sender's origin `[ ]`
- **Needs:** C0.02 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. `web/src/main.rs:42` posts replies to `"*"`. Capture `event.origin` of
     each inbound `message` event and reply with
     `post_message(&value, &origin)`.
  2. Unsolicited events (`ready`) go to the origin of the first accepted
     message. Before that, they go nowhere, or to the origin listed in
     C3.06's allowlist.
  3. Update `packaging/webos/host.html` if it depends on `"*"`.
- **Done when:** no `"*"` target remains, and the bridge smoke passes.
- **Verify:** `git grep -n '"\*"' web/src`; `cargo run -p xtask -- dist --web`.
- **Moves:** security.

### C3.06 — Bridge: inbound origin allowlist `[ ]`
- **Needs:** C3.05 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. Accept messages only from origins in an allowlist.
  2. Read the list at boot from, in order:
     - (a) a `?hostOrigin=` query parameter, which must match
       `document.referrer`'s origin;
     - (b) a `hostOrigins` array in `version.json` or `appinfo.json`.
  3. Default: the same origin, plus the WebOS origins named in
     `docs/WEBOS.md`.
  4. Ignore and `console.warn` everything else.
  5. Extend the bridge smoke with a message from a foreign origin, which must
     be ignored.
- **Done when:** the smoke proves a foreign origin gets no reply and causes no
  state change.
- **Verify:** `dist --web` smoke.
- **Moves:** security; S6 ↑.

### C3.07 — Bridge v1 protocol: version, request IDs, capabilities `[ ]`
- **Needs:** C3.06 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Every message is `{ "fc": 1, "id": <string|null>, "type": …, … }`.
  2. Every reply echoes `id`.
  3. Add `hello` → `{type:"hello", fc:1, version, commands:[…registry ids…],
     formats:{import:[…], export:[…]}}`.
  4. Keep the legacy unversioned messages working, and log a deprecation
     warning when one is used.
  5. Unknown `type` → reply `{ok:false, error:"unknown-type"}`.
- **Done when:** the smoke exercises `hello`, `apply` with an id, and an
  unknown type.
- **Verify:** `dist --web` smoke.
- **Moves:** S6 ↑.

### C3.08 — Bridge: payload caps `[ ]`
- **Needs:** C3.07 · **Tags:** `[HOST:browser]` · **Size:** XS
- **Steps:**
  1. Reject `openDocument` bytes above a cap (e.g. 256 MiB) with
     `{ok:false, error:"too-large"}`, following AGENTS.md "Numbers are
     hostile".
  2. Make the cap a named constant beside the other caps.
  3. Add a fuzz anchor for the message parser in `fuzz/src/lib.rs`, if the
     parse is a separable function. If it isn't, extract it first.
- **Done when:** oversized messages are refused, with a test.
- **Verify:** `cargo test -p web` (or the crate's name); `cargo run --bin anchors`
  inside `fuzz/`.
- **Moves:** S6 ↑.

### C3.09 — Document bridge v1 `[ ]`
- **Needs:** C3.08 · **Tags:** `[DOCS]` · **Size:** S
- **Steps:**
  1. Rewrite the bridge section of `docs/WEBOS.md` as a contract:
     - message table (type, fields, reply, errors);
     - the origin rules;
     - the caps;
     - the deprecation of the legacy forms.
  2. Add a JSON Schema at `packaging/webos/bridge-v1.schema.json`.
  3. Add a 20-line host example to the doc.
- **Done when:** the doc matches the code.
- **Verify:** `host.html` uses only documented messages.
- **Moves:** —

### C3.10 — Ship the appinfo icons `[ ]`
- **Needs:** C0.05 · **Tags:** `[HOST:browser]` · **Size:** XS
- **Steps:**
  1. `packaging/webos/appinfo.json` references `assets/icon.png` and
     `largeIcon.png`, and neither is in `dist/web`.
  2. Render them from the approved flame master in `assets/` at the sizes
     `docs/WEBOS.md` requires.
  3. Copy them in `dist.rs`.
  4. Add a row to `ATTRIBUTION.md`; they are FireCraft-original.
- **Done when:** both URLs return `image/png` on a preview deploy.
- **Verify:** `curl -sI <preview>/assets/icon.png`.
- **Moves:** —

### C3.11 — Loading screen with progress `[ ]`
- **Needs:** C3.01 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. The page shows nothing while about 7 MB of compressed wasm streams.
  2. In `web/index.html`, add a lightweight splash: logo, a progress bar
     driven by a `fetch` reader over the wasm's `content-length`, and an
     error state with a retry button.
  3. Keep it CSP-safe (no inline eval).
  4. Hide the splash on eframe's first frame.
- **Done when:** a throttled "Slow 4G" load in Playwright shows the progress
  bar advancing.
- **Verify:** a Playwright screenshot at about 30 % load, attached to the PR.
- **Moves:** UX.

### C3.12 — Web app manifest (installable) `[ ]`
- **Needs:** C3.02, C3.10 · **Tags:** `[HOST:browser]` · **Size:** XS
- **Steps:**
  1. Add `manifest.webmanifest` with name, icons (192/512), `display:
     standalone`, `theme_color` matching the dark CS6 theme, and `start_url`.
  2. Add the `<link rel=manifest>` tag.
  3. Add `file_handlers` for `.fw.png`/`.png` (Chromium supports launching
     the app with files).
- **Done when:** Chrome DevTools ▸ Application ▸ Manifest shows no errors.
- **Verify:** `npx lighthouse <preview> --only-categories=pwa` (or the
  installability audit) in the PR.
- **Moves:** —

### C3.13 — Service worker: offline launch `[ ]`
- **Needs:** C3.12 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. Add `sw.js`, generated by `dist.rs` with the hashed asset list taken
     from the manifest.
  2. Precache on install, serve cache-first for hashed assets and
     network-first for `index.html` and `version.json`.
  3. Bump the cache name with the version.
  4. Don't intercept bridge `postMessage` traffic, since it isn't HTTP.
- **Done when:** the Playwright test passes: load the app, go offline,
  reload, and the app boots.
- **Verify:** the Playwright test, added to the web smoke if practical.
- **Moves:** —

### C3.14 — Move autosave from localStorage to IndexedDB `[ ]`
- **Needs:** C0.02 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. `web/src/main.rs` (lines 193–260) mirrors the session into
     `localStorage` as base64. That quota is about 5 MB, so large documents
     silently stop autosaving.
  2. Store the raw bytes in IndexedDB instead, through `web-sys` / `idb`
     bindings that are wasm-safe with no C.
  3. Migrate any existing localStorage entry once, then delete it.
  4. Surface a status-bar warning if a write fails.
- **Done when:** a 20 MB document survives a reload.
- **Verify:** a Playwright test that creates a large bitmap, reloads, and
  asserts that the restore offer appears and restores.
- **Moves:** reliability.

### C3.15 — File System Access API for Open and Save `[ ]`
- **Needs:** C3.14 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. When `window.showOpenFilePicker` exists, use it for File ▸ Open and
     keep the `FileSystemFileHandle` on the document tab.
  2. File ▸ Save writes back through the handle with no dialog.
  3. Save As uses `showSaveFilePicker`.
  4. Keep the current `<input type=file>` and Blob-download fallback for
     Firefox and Safari.
  5. Persist handles in IndexedDB; Open Recent on web uses them in C6.13.
- **Done when:** on Chromium, Ctrl+S overwrites the opened file without a
  dialog, and on Firefox the fallback still works.
- **Verify:** Playwright on Chromium (with `--enable-features` if needed)
  plus a manual Firefox check recorded in the PR.
- **Moves:** S5 (save-workflow row).

### C3.16 — OS clipboard image paste and copy `[ ]`
- **Needs:** C0.02 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. **Web:**
     - On the `paste` DOM event, read `image/png` from `clipboardData.items`.
       Decode it through the existing PNG import, then place it as a bitmap
       through the existing place-image command (decision 0037).
     - Edit ▸ Copy also writes a PNG of the selection with
       `navigator.clipboard.write`.
  2. **Desktop:** if eframe/egui exposes image clipboard data, use it.
     Otherwise record it as desktop follow-up in a decision. Do not add a
     C dependency.
- **Done when:** pasting a screenshot from the OS creates a bitmap object.
- **Verify:** a Playwright test using `page.evaluate` to dispatch a synthetic
  paste carrying a PNG blob.
- **Moves:** S5 (clipboard row).

### C3.17 — Cross-browser smoke `[ ]`
- **Needs:** C3.03 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. Run the existing web smoke against the preview URL under Playwright
     with `chromium`, `firefox` and `webkit`, in `deploy-web.yml`.
  2. Record the failures as issues. Do not skip a browser to make it green.
- **Done when:** the job runs all three browsers and its results are visible.
- **Verify:** the workflow run.
- **Moves:** S10 (web artifact smoke).

### C3.18 — Drag a file onto the tab strip to open it `[ ]`
- **Needs:** C3.15 · **Tags:** `[HOST:browser]` · **Size:** XS
- **Steps:**
  1. Confirm that drag-and-drop opens a document in a **new tab**. Today it
     may replace or place.
  2. Match CS6: dropping on the canvas places the file; dropping on the tab
     strip or an empty workspace opens it.
- **Done when:** both behaviours are tested.
- **Verify:** Playwright drop events.
- **Moves:** S5.

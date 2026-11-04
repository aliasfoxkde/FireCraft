# T18 — Ship v0.2.5 (release + WebOS deploy)

**Objective:** cut the v0.2.5 release end to end: stamps → gate → dist
for every target → GitHub release assets → GitForge tag → Cloudflare
Pages deploy of the web bundle (the user's WebOS review surface).

**Inputs:** `docs/RELEASING.md` (the release runbook); the 8 version
stamps (memory: firecraft-release-doors); `xtask dist --native/--web`;
`wrangler pages deploy firecraft-web`; GitForge tags only.

**Authority:** repo release policy (`RELEASING.md`); release
immutability per global scaffolding rules.

**Stopping condition:** GitHub release carries all dist artifacts (see
T23 for the breadth bar), web bundle live at
https://firecraft-web.pages.dev serving the new build, tag pushed to
GitForge, receipts recorded (SHA, job ids, deploy confirmation).

**Verification:** smoke the live site (R/E shape-tool drag + export
diff per the headless-input-probe memory), release page lists assets,
`xtask ci` green at the release SHA.

**Status:** blocked on T23 (asset breadth) — do not cut the release
until the dist matrix matches it.

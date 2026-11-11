# Project Status

## Current snapshot

The last audited source snapshot is `main` at `5fac7a2` (2026-11-04); the latest release tag is `v0.2.5`. The detailed [Progress page](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PROGRESS.md) is the authority for current counts and validation receipts.

The source snapshot has a recent optimized web distribution whose manifest verifies. The public Pages deployment is a **third** bundle: on 2026-11-11 its trunk asset name (`2274ac8c2925b61d`) and wasm digest (`f4f0a3f0…`) matched neither the published v0.2.5 release asset (`f9dbe2f14305b2d9` / `a8a4fa4f…`) nor a rebuild of the v0.2.5 tag (`103c62d3366aef17` / `5934702f…`), so the live build corresponds to no identifiable release and its source revision is unknown. The served `index.html` differs from the build's own only by the substituted asset names and the Cloudflare analytics beacon — that difference is explained. Do not infer the live build from the source version alone: every commit since the v0.2.5 stamp reports version `0.2.5`. The measurement is in [T18](../tasks/T18-release.md#deployment-receipt--2026-11-11).

## Where work stands

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:53e1babaf030bba0ca85e43bacab55a245bc04312570811385e3712883f14109 -->

- M0–M15 and the DOM verb-surface campaign are complete.
- The 19.6 menu-wiring campaign is complete; 2 of 296 rows are inert and eight additional rows remain feature- or evidence-gated.
- Phase 22 shell fidelity is the active UI front. Phase 20 daily-driver features and Phase 21 modern additions are queued.
- T22 recorded a replacement 1280×800 vector screenshot and empty, vector, and bitmap captures at both 1280×800 and 1920×1080. See the [build and capture receipts](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/tasks/T22-screenshot-ui.md#receipts--2026-11-04) for the tested artifact and limits.

<!-- /coderabbit-ai-generated:v1 -->

## Live sources

- [Detailed roadmap](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md)
- [Build and deployment evidence](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PROGRESS.md)
- [Release notes](https://github.com/aliasfoxkde/FireCraft/tree/main/docs/releases)

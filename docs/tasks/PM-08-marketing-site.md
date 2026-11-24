# PM-08 — Public marketing site: Phase 0 audit and deploy repair

> **Parent workstream:** SITE-01 (POST-MVP-ROADMAP §5.20; detail in
> [MARKETING-SITE-ROADMAP.md](../MARKETING-SITE-ROADMAP.md)).
> **Priority:** P1 — the public site is the product's front door, and the tracked
> deploy path does not currently pass its own validation.
> **Status:** `IN PROGRESS` — Phase 0 only. Editor feature work stays behind Gate 0;
> this track touches no Rust code.
> **Owner:** unassigned. **Created:** 2026-11-24.
> **Dependencies:** Gate 0 confirms no other agent owns `site/`; Cloudflare account
> access for project topology. No editor-code dependency.

## 1. User problem

A visitor has no verified public product home. The repository contains a credible
static one-pager, but its deployment workflow validates a logo file that is not in
the tree, and the default Pages hostname does not resolve, so there is no production
receipt and no verified public URL. Product claims, screenshots, and downloads have
also never been checked against a build.

## 2. Current implementation and evidence

- `site/index.html` (254 lines, 10,410 B): one page with anchor navigation to
  `#features`, `#workflow`, `#download`, `#project`; skip link; responsive nav
  toggle; meta description and Open Graph tags.
- `site/styles.css` (10,569 B) includes a `prefers-reduced-motion` block.
- `site/app.js` (3,195 B): fetches `https://api.github.com/repos/aliasfoxkde/FireCraft/releases/latest`,
  builds rows with `textContent` (no HTML injection), filters uploaded assets, and
  falls back to the canonical Releases page on any error. It does **not** validate
  `browser_download_url` against an allowlist.
- Assets: `firecraft-mark.png`, `favicon.png`, `app-preview-dark.svg`. There is no
  `firecraft-mark.svg`.
- `wrangler.toml`: `name = "firecraft-site"`, `pages_build_output_dir = "./site"`.
- `.github/workflows/deploy-site.yml`: validates files and greps, then
  `wrangler pages deploy site --project-name=firecraft-site --branch=main`; needs
  `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`.
- **Defect found and fixed in this change:** the workflow required
  `site/assets/firecraft-mark.svg`, which does not exist, so the validate step
  failed before deploy. It now checks the tracked `.png`.
- **Defect found and fixed in this change:** `docs/SITE-DEPLOYMENT.md` described the
  logo as a self-contained SVG; corrected to the PNG.
- `https://firecraft-site.pages.dev/` does not resolve from this host (DNS
  `ENOTFOUND`), so no production deployment is verified. Actual project mode,
  production branch, domain, and deployed SHA are **UNVERIFIED**.
- The page loads Google Fonts from `fonts.googleapis.com` and uses a
  `raw.githubusercontent.com` screenshot and Open Graph image.
- No `sitemap.xml`, `robots.txt`, custom `404.html`, or `_headers` exists.
- Screenshot provenance: `docs/img/app-main.png` is referenced but not
  build-stamped in the page.

## 3. Scope

In scope: the Phase 0 audit — site file/claim inventory, feature-claim evidence
matrix, asset provenance and license inventory, Pages topology verification,
deploy-path validation, local browser/accessibility/performance baseline, and
deduplication of the SITE-01A–I child slices against existing issues and tickets.

**Out of scope:** production deployment, DNS, or domain changes (owner-gated);
framework/static-site-generator migration; editor/Rust changes; new analytics or
forms; building the multi-page site before Phase 0 exits; creating all nine child
tickets before the audit.

## 4. Files and surfaces likely affected

`site/index.html`, `site/styles.css`, `site/app.js`, `site/assets/`,
`wrangler.toml`, `.github/workflows/deploy-site.yml`, `docs/SITE-DEPLOYMENT.md`,
`docs/MARKETING-SITE-ROADMAP.md`, and — only after production verification —
`docs/HANDOFF.md` and public links.

## 5. Decisions required

- Actual Pages deployment mode (Git integration vs direct upload), production
  branch, project name, and domain, read from the dashboard rather than inferred.
- Whether the external Google Fonts request is accepted; default is self-hosted or
  system typography.
- Whether the single-page anchor architecture stays for launch or gains real routes
  for Features, Gallery, Downloads, Getting started, and Community.
- Production hostname and explicit owner approval for launch.

## 6. Implementation steps

1. Confirm no other worktree, branch, PR, or agent owns `site/`; record what is
   found. Stop if ownership is contested.
2. Write `docs/SITE-BASELINE-AUDIT.md`: file inventory with hashes, every visible
   claim, asset provenance/licenses, external requests, and the gap list.
3. Verify the real `firecraft-site` project (mode, branch, domain, output, latest
   deployment SHA). Mark each unknown UNVERIFIED with the exact question.
4. Keep the deploy-validation fix; run the validation block locally.
5. Build the claim matrix: each public claim maps to source, tests, a release asset,
   or current docs — otherwise it is removed or downgraded to Available/Preview/
   In development/Planned.
6. Serve `site/` and capture the browser baseline at 360, 768, and 1280 px:
   console/network, keyboard-only path, mobile nav, reduced motion, 200% text zoom,
   release API blocked/fallback.
7. Run axe (or equivalent) and Lighthouse; record tool versions and scores.
8. Deduplicate the SITE-01A–I slices against open issues and existing tickets, then
   propose only the slices the audit proves are needed.

## 7. Acceptance criteria

- `docs/SITE-BASELINE-AUDIT.md` exists with the claim matrix, asset provenance,
  deployment topology, and a prioritized gap list.
- The workflow's validation block passes locally; a deploy either succeeds to a
  preview with a receipt or is blocked by a named, missing account fact.
- Browser, accessibility, and performance baselines are recorded with commit,
  viewport, browser, and tool versions.
- No duplicate site or framework is introduced; no secret, tracker, or unapproved
  analytics appears in `site/`.
- `firecraft-web` (editor, `dist/web`) and `firecraft-site` (site, `site/`) remain
  separate in config and deployment.
- No production or "live" claim is made without a verified URL and deployment SHA.

## 8. Test plan

- Simulate the workflow validation: file-presence tests plus the three greps.
- Serve `site/` over a local static server; Playwright at the three viewports;
  check links, metadata, release fallback with the API blocked, console errors, and
  failed requests.
- axe scan plus manual keyboard/focus/reduced-motion/text-zoom checks.
- Lighthouse mobile run; record Performance, Accessibility, Best Practices, SEO.
- Grep `site/` for tokens, credentials, analytics hosts, and third-party scripts.
- No Rust tests are required while the change stays inside `site/` and docs.

## 9. Manual verification

Walk the four journeys (prospective user, former Fireworks user, designer/developer,
contributor) from a clean browser profile. Confirm the mobile menu and skip link,
that every download row comes from published release assets or the fallback, and
that screenshots match the current build. Compare the rendered page with the claim
matrix row by row.

## 10. Performance, security, portability

Targets: mobile Lighthouse Performance ≥ 90; Accessibility, Best Practices, and SEO
≥ 95 on a recorded run; LCP ≤ 2.5 s, INP ≤ 200 ms, CLS ≤ 0.1 where measurable.
Keep static output, no backend, no secrets in browser assets, HTTPS-only links,
safe DOM construction, and a restrictive policy for scripts, styles, and framing
that is compatible with the chosen static architecture. Avoid render-blocking third-party fonts.

## 11. Risks and rollback

- **Wrong Pages project:** deploying `site/` over `firecraft-web` would replace the
  editor. Mitigation: separate outputs and project names; verify before any deploy.
- **Stale or unsupported claims:** mitigated by the claim matrix and status labels.
- **External dependency/privacy drift:** Google Fonts and raw GitHub images are
  third-party requests; self-host or record the decision.
- **Dashboard drift:** if project state cannot be read, report the blocker instead
  of inferring it from `wrangler.toml`.
- Rollback for any site deployment is the previous Cloudflare Pages deployment;
  no editor artifact is involved.

## 12. Stopping condition and closure evidence

Stop if a competing owner holds `site/`, if the Pages project/domain/mode cannot be
verified, or if production deployment needs account access the agent does not have.
Close Phase 0 with the audit document, the passing validation block, recorded
browser/accessibility/performance results, and either a preview deployment receipt
or a blocked report naming the exact missing account fact. Do not create the
SITE-01A–I child tickets until this audit exits.

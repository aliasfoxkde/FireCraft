# FireCraft Marketing Website Roadmap

> **Document type:** implementation roadmap and acceptance specification for the public product website.
> **Status:** planned enhancement to the existing static site; not a claim that the public site is complete or deployed.
> **Reviewed:** 2026-11-26.
> **Parent workstream:** SITE-01.
> **Companion:** [Post-MVP Agent Handoff](POST-MVP-AGENT-HANDOFF.md).
> **Hosting target:** Cloudflare Pages, in the existing FireCraft repository.

## 1. Mission and intended outcome

Build a compelling, credible public home for FireCraft that helps a first-time visitor understand the product, see real editor workflows, try or download the current release, evaluate its maturity, and contribute.

The website is part of the product—not a disposable promotional page. It should communicate why FireCraft exists, what works today, what is still being built, how to install it, how user documents are handled, and how the open-source community can participate.

The objective is a high-quality static website that is fast, accessible, discoverable, privacy-conscious, maintainable, and inexpensive to deploy. It should be deployed through Cloudflare Pages from the same repository while remaining independent of the editor's WebAssembly build and deployment.

## 2. Existing repository state to preserve

The remote roadmap branch inspected on 2026-11-23 already contains:

- A static site in the repository's site directory: index.html, styles.css, app.js, and an assets directory.
- A one-page design with responsive navigation, product positioning, feature cards, screenshots, project links, and a release panel.
- JavaScript that requests the latest GitHub Release metadata and builds download links, with a fallback to the canonical Releases page.
- A root wrangler.toml containing the Pages project name firecraft-site and a Pages output directory of ./site.
- An existing separate editor browser deployment described in docs/HANDOFF.md: project firecraft-web, built from dist/web, currently deployed manually through Wrangler.

This is a remote branch snapshot, not a statement about the developer's current local working tree, actual Pages dashboard state, custom domain, or latest production deployment. The agent must re-inspect the local source and Cloudflare configuration before making changes.

**Do not create a second marketing site. Do not replace the existing site with a framework template. Do not repoint firecraft-web to the marketing site or deploy site/ to the editor project.** Preserve the existing approved brand asset and single logo treatment.

## 3. Product position and messaging

The core proposition should be specific:

**FireCraft is a free, open-source creative editor that brings vector and bitmap work together in a modern, editable graphics workflow inspired by the best ideas of classic Fireworks.**

The message should communicate:
- One editable workspace for vector and bitmap content.
- A Fireworks-inspired workflow with a modern Rust foundation.
- Native and browser/WASM delivery, with availability stated accurately.
- An editable Fireworks PNG document workflow and clear format-fidelity limits.
- Production-oriented export and automation capabilities where proven.
- Open-source licensing, visible development, and community participation.

Do not describe FireCraft as a finished Photoshop replacement, a complete Fireworks CS6 clone, a Figma replacement, or a professional painting suite until those claims are supported by end-to-end evidence.

Every public product statement must be classified internally as one of:
- **Available:** present in a published build, with a verified workflow.
- **Preview:** usable but experimental or subject to documented limitations.
- **In development:** actively being implemented, not represented as released.
- **Planned:** roadmap intent only.

The public site should not imply that all platforms, formats, brushes, AI features, color workflows, or compatibility targets are available merely because they appear in a roadmap or build recipe. Platform downloads must come from published release assets, not from a planned-platform table.

## 4. Audience and primary journeys

Design and test the website for four audiences:

1. **Prospective users:** understand the product, inspect real screenshots, try the browser build if currently reliable, and download a supported build.
2. **Former Fireworks users:** understand the project lineage, familiar workflows, document compatibility, and what has changed.
3. **Designers and developers:** evaluate vector/bitmap features, output workflows, automation, file formats, and limitations.
4. **Contributors:** find the source, license, contribution guide, issue templates, roadmap, build instructions, and project health.

Primary journeys must be obvious within the first screen:
- Try the current browser build, when verified.
- Download the latest supported release.
- See the actual application.
- Read what works and what is still in progress.
- Explore source code or contribute.

## 5. Information architecture

Build a small, coherent site. Prefer a few useful pages over a large collection of thin marketing pages.

### 5.1 Required pages

| Route or page | Required content | Acceptance |
|---|---|---|
| Home | Clear product promise, strong real product visual, core differentiators, feature summary, current availability, primary CTAs, project/community trust | A new visitor can explain what FireCraft is and find Try, Download, Features, and Source without searching. |
| Features | Grouped editor capabilities with Available/Preview/In development labels and links to evidence/docs | Every claim is sourced to current tests, feature docs, screenshots, or a published release. |
| Screenshots / Gallery | Current, authentic editor captures with meaningful captions, build/date context, responsive display, descriptive alt text | No mockup is presented as a real application screenshot; outdated screenshots are labeled or replaced. |
| Downloads | Latest published release, asset list grouped by verified platform/artifact type, checksums/manifest, install notes, browser-build link | Download destinations are taken from current release metadata or canonical release pages. Unsupported platforms are not advertised as available. |
| Getting started | First launch, create/open/save, import/export, browser build caveats, links to full usage docs | Instructions are reproducible from a clean environment and match the current release. |
| Extensions | Official/community program, researched workflow categories, submission and safety plan, and current implementation status | The page links to the detailed ecosystem plan and clearly states that the catalog, package intake, and one-click install are not yet operational. |
| Project / Community | License, repository, contribution guide, issue reporting, roadmap, release history, attribution | Visitors can identify the project governance and participate without a hidden signup requirement. |

### 5.2 Recommended follow-on pages

Add only when the content is real and maintainable:
- Changelog / release history, generated from the canonical release notes.
- Documentation landing page that links into the existing docs or a separately approved docs publishing system.
- Privacy and data-handling statement based on actual application and website behavior.
- Accessibility statement that records the audited site scope and any known limitations.
- Format compatibility reference generated from or linked to the repository's capability matrix.
- An operational extension catalog and author submission portal, after the extension API, package verification, review process, moderation, and rollback gates in [`research/fireworks-extensions.md`](research/fireworks-extensions.md) are implemented.

Avoid adding a blog or CMS until there is an owner and a durable publishing workflow. Avoid duplicating the entire repository documentation set on the marketing site.

## 6. Visual and interaction direction

The target is a polished creative-software product site: confident, visually memorable, editorially clean, and centered on the actual application.

- Retain the approved flame identity and existing brand asset provenance. Do not create a replacement logo or alternate mark without owner approval.
- Use high-quality screenshots of real, current editor builds. Show meaningful artwork and actual panels/tools, not empty canvases or invented UI.
- Build a visual system around the approved brand palette, readable typography, consistent spacing, clear hierarchy, restrained gradients, and purposeful contrast.
- Use responsive layouts that work on wide monitors, laptops, tablets, and phones. Mobile is a first-class website viewport even when the editor is not a phone-first product.
- Provide visible focus states, skip navigation, semantic landmarks, accessible labels, keyboard-operable menus, and descriptive link text.
- Respect prefers-reduced-motion. Do not rely on animation, hover, color, or icon shape alone to communicate state.
- Provide useful image captions, responsive image sizes, width/height attributes, and modern formats with fallbacks when justified.
- Keep navigation and CTAs consistent. Avoid modal popups, newsletter capture, fake urgency, fabricated user counts, fake testimonials, or unsubstantiated performance comparisons.

The current site has Google Fonts requests. Audit privacy, availability, and performance; prefer local/system typography or a properly licensed, self-hosted subset unless the owner explicitly accepts the external request and documents it.

## 7. Technical architecture

### 7.1 Keep the existing static architecture unless evidence requires a change

Begin by auditing the current HTML/CSS/JavaScript implementation. Improve it incrementally. A framework or static-site generator is allowed only if the agent proves a concrete benefit that outweighs dependency, build, maintenance, and deployment complexity.

Default constraints:
- Static output suitable for Cloudflare Pages.
- No server-side rendering requirement.
- No account, database, application backend, or Worker requirement for the core marketing experience.
- No coupling to the editor's Rust workspace, WASM build, or runtime.
- No heavy client framework solely to render static copy.
- Keep browser JavaScript small, progressively enhanced, and fail-safe.
- Do not add runtime dependencies without checking licensing, supply chain, security, and long-term maintenance.

If multi-page routes are introduced, ensure direct navigation and refresh work on Pages without fragile client-side routing. Keep links useful when JavaScript is disabled.

### 7.2 Release metadata and downloads

The existing page reads GitHub's public latest-release endpoint. Audit this implementation and preserve the useful dynamic behavior while hardening it:

- Treat GitHub response fields as untrusted input.
- Permit only expected GitHub release download URL hosts and HTTPS.
- Build DOM nodes safely; do not inject API-provided HTML.
- Handle rate limits, network failures, malformed responses, draft/prerelease policy, no assets, and missing platform assets.
- Clearly distinguish stable releases from prereleases.
- Group assets from actual uploaded release metadata and link to checksums/manifests.
- Show a reliable fallback link to GitHub Releases.
- Do not invent platform-specific downloads, file sizes, version numbers, or support claims.
- Consider generating a small release manifest at build time only if its source, freshness, and fallback behavior are unambiguous.

Never place a GitHub token, Cloudflare credential, signing key, or other secret in browser code or a public environment variable.

### 7.3 Screenshots and product visuals

- Use only approved brand artwork and images the project is allowed to redistribute.
- Capture editor screenshots from identified builds and real workflows.
- Store website-optimized copies in the site asset tree when that makes the site self-contained; record source capture, source SHA/build, optimization, and license/provenance.
- Avoid requiring the live site to fetch core screenshots from GitHub's raw-content CDN.
- Do not use generated mockups as product screenshots. Concept art must be labeled as concept art.
- Optimize images for quality and size; do not distort, recolor, or create alternate logo assets.

### 7.4 SEO and discoverability

Implement and verify:
- Unique page titles and descriptions.
- Canonical URLs only after the production hostname is confirmed.
- Open Graph and social preview metadata with an appropriate project-owned image.
- sitemap.xml and robots.txt appropriate to the final production host.
- Correct heading hierarchy, semantic HTML, descriptive image alternatives, and meaningful internal links.
- Clean page URLs and a useful not-found response.
- Structured metadata only for facts that are actually verified; no fabricated ratings, prices, awards, download counts, or supported platforms.

## 8. Cloudflare Pages deployment architecture

### 8.1 Two distinct Pages products

Keep these deployments independent:

| Purpose | Existing project name in repository docs/config | Source/output | Rule |
|---|---|---|---|
| Public marketing website | firecraft-site (root wrangler.toml currently points at ./site) | site/ | Public product information and marketing assets only. |
| Browser editor | firecraft-web (documented editor deploy) | dist/web | Actual WebAssembly editor bundle and host files. |

The existing repository configuration and actual Cloudflare dashboard may have drifted. Verify the real Pages project names, connected repository, deployment mode, production branch, output directory, domain mappings, and latest deployed commit before operating either project.

Never deploy one output directory to the other project's name. Never change the editor project's production branch, custom domain, output directory, or deployment mode as part of marketing-site work.

### 8.2 Git integration versus direct upload

Inspect how firecraft-site is currently deployed before changing its workflow. Do not assume that a Pages project created for direct upload can be converted to Git integration or vice versa without consequences. Cloudflare documents limitations on switching deployment modes.

If the existing project is Git-integrated:
- Use a root directory/build configuration scoped to site/.
- Configure build-watch include/exclude paths so unrelated Rust/editor changes do not trigger unnecessary marketing builds.
- Include any true shared source dependencies in the watch scope.
- Use preview deployments for pull requests and keep main as production only after confirmation.
- Surface deployment checks in the GitHub review process.

If the existing project uses direct upload:
- Preserve that mode until an explicit migration decision is made.
- Use the repository's current Wrangler configuration after verifying its schema and installed Wrangler version.
- Build or stage only the static site output, then deploy to firecraft-site.
- Do not add credentials to repository files or commit generated deployment bundles.

The current root Wrangler configuration names firecraft-site and uses ./site as the Pages output directory. Confirm this is still correct before relying on it.

### 8.3 Preview and production release gates

1. Run the site-specific build/validation locally.
2. Run browser checks against the exact built site.
3. Deploy a preview using the existing supported Pages workflow.
4. Inspect preview on desktop, tablet, and phone widths; validate links, downloads, images, console, metadata, and accessibility.
5. Obtain explicit owner approval before production deployment or changing a production domain.
6. Deploy only the verified site output to firecraft-site.
7. Record source commit, Wrangler version, Pages project, deployment ID/URL, production URL, smoke results, and rollback target.
8. Verify the deployed site independently from the source tree.

Do not claim that the marketing site is publicly launched until the production URL and deployed commit have been verified.

## 9. Privacy, security, licensing, and trust

- Do not add third-party analytics, advertising pixels, session replay, fingerprinting, newsletter forms, or user tracking by default.
- Cloudflare Web Analytics may be considered only after the owner approves it and the privacy disclosure accurately describes the deployed configuration.
- Avoid third-party font, video, image, and script requests where practical. Document any required external service and its data flow.
- The website does not need to collect user data to publish downloads or link to GitHub.
- Use HTTPS links, secure external-link attributes, safe DOM APIs, and a restrictive Content Security Policy compatible with the chosen static architecture.
- Keep security headers and cache rules appropriate for static content; do not cache mutable release metadata indefinitely.
- Use Apache-2.0 source and approved brand asset terms accurately. Keep the attribution ledger current for all third-party fonts, icons, screenshots, templates, and images.
- Review all copied product text and claims. Do not reproduce Krita, Photopea, Pixlr, Adobe, or other product branding, assets, proprietary UI, or copy.
- Publish a privacy/data-handling statement that distinguishes website requests, outbound GitHub navigation/API metadata requests, and in-app document processing. Do not make broader privacy promises than source behavior and deployment configuration support.

## 10. Performance and accessibility targets

Targets for the public site:
- Mobile Lighthouse Performance: 90 or higher on a recorded representative run.
- Lighthouse Accessibility, Best Practices, and SEO: 95 or higher, with any exception documented and justified.
- Core Web Vitals target: LCP at or below 2.5 seconds, INP at or below 200 ms, and CLS at or below 0.1 at the 75th percentile when real-user data becomes available.
- Avoid large JavaScript bundles, layout shifts, oversized hero media, render-blocking third-party font requests, and unnecessary network calls.
- WCAG 2.2 AA as the target for the website interface, assessed with automated tests plus manual keyboard and screen-reader checks. Do not claim conformance based solely on a Lighthouse score.
- Test contrast, focus visibility, text scaling, reduced motion, mobile navigation, touch target sizing, image alternatives, and error/fallback states.

Record test URLs, commit, browser, viewport, tool versions, and results. A passing automated score does not replace manual review.

## 11. Work plan and sequencing

### Phase 0 — Reconnaissance and ownership

- Follow AGENTS.md and the post-MVP handoff preflight.
- Inspect the live local tree, branch, worktrees, local-only changes, PRs, and existing site work.
- Compare site/, root wrangler.toml, README site links, docs/HANDOFF.md, current Pages dashboard state, and the production site.
- Identify which screenshots are current and which product claims are substantiated.
- Inventory all assets and their licenses.
- Confirm whether firecraft-site exists, how it deploys, its production branch, domain, and current deployment SHA.
- Produce a site baseline audit. Do not start a competing website rewrite while an existing site branch or local implementation is active.

**Exit:** current site baseline, known deployment topology, content/claim audit, and deduplicated task list.

### Phase 1 — Positioning and content

- Write the one-sentence product promise and concise supporting copy.
- Define the audience journeys and page hierarchy.
- Build a claim matrix linking each public claim to current source/test/release evidence.
- Select real screenshots from current builds and plan updates for obsolete captures.
- Decide which platform/download rows can be shown from current releases.
- Draft getting-started, privacy, and community content using existing project authorities.

**Exit:** owner-reviewable content and visual direction with no unsupported capability claims.

### Phase 2 — Site implementation

- Improve the existing homepage and reusable components.
- Add or refine the required pages without duplicating repository documentation.
- Improve mobile/tablet layouts and navigation.
- Implement accessible gallery and screenshot presentation.
- Harden dynamic release listing and its fallback.
- Add metadata, sitemap, robots, not-found handling, caching, and security headers.
- Keep the site independent of the editor WASM bundle.

**Exit:** complete static build, functional links, stable release metadata behavior, and responsive pages.

### Phase 3 — Verification

- Validate HTML, links, metadata, sitemap, robots, asset references, and output directory.
- Run unit tests for release metadata parsing and URL validation.
- Run Playwright on desktop/tablet/mobile viewports.
- Test keyboard-only navigation, focus, reduced motion, zoom/text scaling, and an assistive-technology smoke workflow.
- Run axe or an equivalent automated accessibility scan and record manual follow-up.
- Run Lighthouse and record the test environment/results.
- Check the browser console and network failures.
- Confirm no secrets, unintended analytics, broken download links, oversized assets, or third-party licensing omissions.

**Exit:** reproducible quality evidence tied to the exact commit.

### Phase 4 — Preview and public launch

- Deploy a preview to the existing firecraft-site project using its verified deployment mode.
- Conduct visual/product/technical review.
- Confirm the production hostname and DNS/TLS configuration with the owner.
- Obtain explicit approval for production deployment.
- Deploy, verify the production URL, and record the deployment receipt.
- Update README and docs with the actual public URL only after verification.
- Add the marketing-site release/deploy procedure to the operational handoff without changing the editor firecraft-web recipe.

**Exit:** verified public website with a documented rollback path and current release links.

### Phase 5 — Ongoing maintenance

- Update screenshots and feature claims as editor releases mature.
- Validate download metadata against each published release.
- Keep release notes and supported platforms accurate.
- Re-run site tests on meaningful changes.
- Track broken links, outdated content, page performance, and accessibility regressions.
- Keep public claims synchronized with FEATURES.md, CS6-PARITY.md, release manifests, and current task status.

## 12. Required task decomposition

Create one parent task, SITE-01, with child tasks small enough to review. The agent must first search existing issues and tickets and reuse IDs where appropriate.

Suggested child slices:
1. SITE-01A — existing site and Cloudflare deployment audit.
2. SITE-01B — product messaging and feature-claim evidence matrix.
3. SITE-01C — real screenshot and brand asset refresh.
4. SITE-01D — responsive homepage and navigation.
5. SITE-01E — features, gallery, downloads, and getting-started pages.
6. SITE-01F — release metadata hardening and fallback tests.
7. SITE-01G — SEO, sitemap, not-found, headers, caching, and privacy review.
8. SITE-01H — accessibility, responsive browser tests, and performance budgets.
9. SITE-01I — Pages preview, production approval, deployment receipt, and runbook.

Do not create all child tickets before Phase 0 confirms the existing implementation and issue inventory. Combine or omit slices that already exist.

## 13. Acceptance criteria

The marketing website is ready for public launch only when:

- The existing site has been improved rather than duplicated.
- Product positioning is clear and consistent across pages.
- All feature, format, platform, and performance claims map to verified evidence.
- Screenshots are authentic, current, captioned, accessible, and licensed.
- All required journeys work with keyboard, touch, and small viewports.
- Downloads use published release assets and handle failures safely.
- GitHub API outage/rate limiting degrades to a working canonical releases link.
- All pages work on direct navigation and refresh.
- HTML, metadata, links, assets, sitemap, robots, and not-found behavior are validated.
- Accessibility and performance targets are measured; exceptions are documented.
- No secret, tracker, unapproved analytics, unlicensed asset, or misleading claim is introduced.
- The site deploys independently to firecraft-site without changing firecraft-web.
- Preview and production deploy receipts identify exact source commits and output.
- README, docs index, operational handoff, and project release materials link to the verified live site.

## 14. Success metrics

Prefer measurable user outcomes:
- Visitor-to-browser-build or visitor-to-download click-through, if measured through privacy-approved aggregate analytics.
- Successful download asset resolution and low failure rate.
- Time to find current release, supported-platform status, and getting-started instructions.
- Contribution guide and issue-template navigation.
- Page performance, accessibility regressions, broken links, and outdated capability statements.
- Website maintenance effort per editor release.

Do not add visitor analytics merely to populate a metrics view. Deployment/build evidence and public repository activity are sufficient for the initial launch.

## 15. Stop conditions

Stop and report when:
- A local branch or active agent already owns the site files.
- The actual Pages project name, deployment mode, production branch, or domain cannot be verified.
- A change risks redirecting the existing editor deployment.
- A public product claim cannot be tied to a verified current build.
- A screenshot or third-party resource has unclear redistribution rights.
- A proposed framework adds substantial runtime/build complexity without a clear gain.
- A requested analytics, form, or remote API feature requires unapproved data collection or privileged secrets.
- Production deployment would require changing account configuration or DNS without explicit owner authorization.

The public site must be accurate before it is impressive, and impressive without becoming a second application to maintain.


## 16. Cloudflare Pages reference documentation

Use current official documentation during deployment work; account configuration and Wrangler behavior can change.

- [Pages monorepos](https://developers.cloudflare.com/pages/configuration/monorepos/) — separate projects, root directories, and project limits.
- [Pages build configuration](https://developers.cloudflare.com/pages/configuration/build-configuration/) — root directory, build command, and output directory.
- [Pages build watch paths](https://developers.cloudflare.com/pages/configuration/build-watch-paths/) — reduce unrelated builds for multiple projects in one repository.
- [Pages Git integration](https://developers.cloudflare.com/pages/configuration/git-integration/) — branch deployments and preview behavior.
- [Pages GitHub integration](https://developers.cloudflare.com/pages/configuration/git-integration/github-integration/) — repository integration and pull-request previews.
- [Deploy static HTML to Pages](https://developers.cloudflare.com/pages/framework-guides/deploy-anything/) — static output and direct deployment workflow.

Do not treat this documentation as evidence that FireCraft's account is already configured. The live account and project settings must be inspected independently.

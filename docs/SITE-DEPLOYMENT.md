# FireCraft marketing site

The public marketing site lives in `site/` and is intentionally independent of the
Rust/WASM editor bundle in `web/`. It is a static, dependency-free HTML/CSS/JS site
with no build step: Cloudflare Pages serves the directory as-is.

## Structure

| Path | Purpose |
|---|---|
| `site/index.html` | Home: product promise, real capture, capability summary, honest status, release panel |
| `site/features.html` | Grouped capability reference with Available / Preview / In development labels |
| `site/gallery.html` | Captures with build provenance, plus one clearly labelled concept artwork |
| `site/formats.html` | Format direction matrix, document-fidelity rules, platform availability |
| `site/downloads.html` | Live release assets, checksum and install notes, source build |
| `site/docs.html` | Quickstart, CLI, automation surfaces, documentation map |
| `site/project.html` | Mission, clean-room policy, roadmap, governance, licensing, contributing |
| `site/404.html` | Custom not-found page |
| `site/_headers` | Response headers: content security policy, nosniff, referrer policy, frame denial, permissions policy |
| `site/robots.txt` | Crawler policy |
| `site/assets/` | Brand mark (SVG), two real captures, one labelled concept artwork, provenance notes |

There is no external font, analytics tag, or third-party script. The only network
call the site makes is the public GitHub release API, which populates the release
panel; it is validated against an HTTPS host allowlist and falls back to the
canonical release page.

## Local preview

From the repository root, serve `site/` as the document root:

    python3 -m http.server 8080 --directory site

Then open the address the server prints. Pages are linked relatively, so every
route works on a plain static server without rewriting.

## Cloudflare Pages deployment

`.github/workflows/deploy-site.yml` deploys `site/` to the Pages project
`firecraft-site` when a change under `site/**` or `wrangler.toml` lands on `main`,
or on manual dispatch. `wrangler.toml` names the project and the output directory;
keep both in step with this document.

Required GitHub Actions repository secrets:

- `CLOUDFLARE_API_TOKEN`: a scoped Cloudflare API token allowed to deploy the target
  Pages project.
- `CLOUDFLARE_ACCOUNT_ID`: the Cloudflare account that owns the project.

Before deploying, the workflow validates that every page exists, that the headers
and asset files are present, that the accessibility and content anchors the pages
depend on are in place, and that no third-party font or analytics host appears in
`site/`. Create the `firecraft-site` project first, or change the project name
consistently in `wrangler.toml`, this document, and the workflow.

A successful workflow run and a reachable production URL are both required before
the site is described as published. As of 2026-11-23 the project topology
(mode, production branch, domain, deployed SHA) is **unverified** and
`firecraft-site.pages.dev` does not resolve from the development host.

## Proof required before push

`AGENTS.md` requires a browser proof for any `web/` or shell change, and this site
falls under the same rule. Serve `site/`, load each route at mobile and desktop
widths, confirm the mobile menu and skip link, block the release API and confirm the
fallback, and record axe and Lighthouse results. Do not push a site change on the
strength of the validation block alone.

## Content accuracy rules

- Keep the published release and download assets sourced from GitHub Releases.
- Do not advertise Windows, macOS, Android, AppImage, Flatpak, or RPM as available
  until corresponding artifacts are published and verified.
- Keep roadmap capabilities distinguished from implemented functionality with the
  four state labels; never list a planned platform as a download.
- Use real captures for product evidence and label concept artwork as concept art.
- Preserve the Adobe trademark and non-affiliation notice in the footer.
- Record new asset sources in `site/assets/README.md` before using them.

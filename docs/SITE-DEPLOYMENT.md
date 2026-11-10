# FireCraft marketing site

The public-facing marketing site lives in site/ and is intentionally independent of the Rust/WASM editor bundle in web/. It is a static, dependency-free HTML/CSS/JavaScript site; the latest-release panel reads the canonical GitHub Releases API and falls back to the release page if the API is unavailable.

## Local preview

From the repository root, run a static HTTP server with site/ as its document root:

    python3 -m http.server 8080 --directory site

Then open http://localhost:8080. The screenshot links to the tracked application screenshot at docs/img/app-main.png on the default branch. The logo is a self-contained SVG in site/assets/.

## Cloudflare Pages deployment

The repository workflow .github/workflows/deploy-site.yml deploys the site/ directory to the Cloudflare Pages project firecraft-site when changes land on main, or on manual dispatch.

Configure these GitHub Actions repository secrets before enabling deployment:

- CLOUDFLARE_API_TOKEN: a scoped Cloudflare API token with permission to deploy to the target Pages project.
- CLOUDFLARE_ACCOUNT_ID: the Cloudflare account identifier that owns the project.

Create the Pages project named firecraft-site first, or change the project name consistently in wrangler.toml, this document, and the workflow. A successful deployment workflow and a reachable production URL are required before announcing the site as published.

## Content accuracy rules

- Keep the currently published release and download assets sourced from GitHub Releases.
- Do not advertise Windows, macOS, Android, AppImage, Flatpak, or RPM as available until corresponding artifacts are published and verified.
- Keep roadmap capabilities clearly distinguished from implemented functionality.
- Use the repository's own application screenshot; do not use mock editor UI as product evidence.
- Preserve the Adobe trademark and non-affiliation notice in the footer.

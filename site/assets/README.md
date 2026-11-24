# Site assets — sources and provenance

All artwork in this directory is original project artwork or a capture of the
project's own editor. No Adobe code, artwork, icon, or screenshot is present.

| File | What it is | Source |
|---|---|---|
| `firecraft-mark.svg` | The approved flame mark, single path with the brand gradient. | Extracted unchanged from the `#flame` path in `app-preview-dark.svg`. Used for the header, footer, and favicon. |
| `app-main.png` | Real editor capture, 1280×800. | `docs/img/app-main.png` in the repository (web bundle built 2026-11-04). |
| `mask-stack.png` | Real editor capture, 1280×800. | `docs/img/mask-stack-1280x800.png` in the repository. |
| `app-preview-dark.svg` | Vector concept artwork of the dark workspace. Labelled as a mock-up wherever it is shown. | Repository `docs/img/app-preview-dark.svg`. |

## Why the PNG brand files were replaced

`firecraft-mark.png` and `favicon.png` previously contained the same 15,000-byte
blob as `docs/img/logo.png` and `assets/branding/firecraft-approved-master.png`,
and that blob is not a PNG (no PNG signature). Browsers therefore showed a broken
image wherever the mark appeared.

The site now ships the mark as SVG, rendered from the approved vector path, so it
stays sharp at every size and needs no raster fallback. The broken raster files
were removed from this directory.

The same invalid blob is still used by the README and the branding master. That is
a repository asset problem separate from the website and should be fixed at the
source rather than patched here.

## Rebuilding

```sh
# The mark is a single path plus its linear gradient, copied from the preview SVG.
# To regenerate the PNG copies (optional; the site does not need them):
convert -background none -density 300 assets/firecraft-mark.svg -resize 512x512 mark.png
```

## Rules

- Do not add a second logo treatment or recolor the mark.
- Do not add concept art without labelling it as concept art on the page.
- Do not add a capture without its build/date provenance.
- No third-party font, script, analytics, or image host is used by this site.

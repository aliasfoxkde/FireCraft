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

Every brand raster in the repository — `firecraft-mark.png`, `favicon.png`,
`docs/img/logo.png`, and `assets/branding/firecraft-approved-master.png` — held
the same 15,000-byte blob with no PNG signature, so browsers showed a broken
image wherever the mark appeared.

The site ships the mark as SVG, rendered from the approved vector path, so it
stays sharp at every size and needs no raster fallback. The repository rasters are
now real rasterizations of that same path: the branding master at 512 px, the
README logo and hicolor icon at 256 px, and the web favicon at 64 px. They were
produced by printing the SVG at 512 px with headless Chrome and converting through
`pdftocairo -png -transp`, which keeps both the gradient and the transparent
background. ImageMagick's built-in SVG renderer is not usable here: it flattens
the gradient to solid black.

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

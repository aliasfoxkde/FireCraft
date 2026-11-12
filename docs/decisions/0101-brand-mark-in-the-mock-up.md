# 0101 — The dark-mode mock-up shows the brand mark itself, and its placement transform is adjudicated

Date: 2026-11-12 · Issue [#131](https://github.com/aliasfoxkde/FireCraft/issues/131) · Status: accepted

## Context

The project's identity is the vector flame (`docs/img/logo.svg`, one path with
the ember-red-to-orange gradient). The fox was retired with it, but three
published places still carried the old motif: the README's dark-mode image,
the landing mock-up's artwork, and the attribution row that named "a
geometric fox illustration under edit". The mock-up is the graphic a
first-time visitor sees, so the retired motif was the project's face.

The README's dark-mode image was a *screen capture* whose canvas held the
old logo object, so its alt text described what the pixels actually
contained. That text could not simply be reworded: the capture had to be
replaced, and no renderer on this host draws the SVG correctly — the
internal SVG renderer ignores `objectBoundingBox` gradients and
multi-transform chains, so an exported PNG of the new artwork would be
wrong. A wrong PNG is worse than no PNG.

Drawing the mark into the mock-up raised two questions that had to be
settled before the artwork was right.

## Decision

- **The mock-up shows the mark the logo draws it.** The flame is placed with
  the canonical path from `docs/img/logo.svg` and the logo's own gradient,
  so the two cannot drift. It is one path and one gradient, as the logo is.
  A first attempt added a second, scaled copy of the path as a gold "inner
  core"; a geometric check of that construction — the path flattened to a
  polygon, the scaled copy's vertices tested against the original — showed
  the copy leaves the flame along **37% of its outline**, with excursions of
  up to 64 path units, because the flame's tail and tendrils are not
  star-shaped about the mark's centre. The core was removed rather than
  shipped as a defect.
- **The selection chrome is fitted to the mark, not to the old artwork.**
  The path's outline is flattened, its bounds mapped through the artwork's
  transform, and the selection rectangle, its eight handles, the six path
  nodes, the layer names, the status bar and the properties panel all
  describe the object that is actually on the artboard.
- **The SVG is the single source.** `docs/img/app-preview-dark.png`,
  `site/assets/app-preview-dark.png` and the retired capture
  `docs/img/app-dark.png` are removed rather than regenerated; the site's
  copy is the same file. Browsers render the SVG directly. The README shows
  the mock-up with a caption that says what it is — original artwork, not a
  screen capture — because a capture's alt text has to describe its pixels,
  and the capture that used to be there held the retired mark.
- **The placement transform is adjudicated, not dodged.** The mock-up places
  the mark with `transform="translate(403.79 193.46) scale(0.31214)"`. The
  scale factor's digits read as a five-digit run, and Aegis's `zip-code`
  rule matches any five-digit run, so the gate reports it as new. Two
  occurrences — one per copy of the graphic — are false positives of a
  class the baseline already carries, and the rule class's reason in
  `xtask/src/aegis.rs` now names this source so a reviewer can read what
  the rows cover. No rule class was switched off and no rule was excluded.

The Aegis re-bless was reproduced outside `cargo`, as decisions 0083 and
0084 did, because the local build admission was closed while the GitForge
runners held the host's compilers. The renderer reproduces
`xtask/src/aegis.rs`'s `render_baseline` byte-for-byte: rendering `main`'s
committed baseline from a scan of `main`'s tree reproduces `main`'s
committed file exactly, with no diff. The scanner is the CI-pinned
`aegis` 0.6.3, so the content digests are the digests CI computes. The
rewrite was iterated to a fixed point: the second pass found no change.

## Consequences

- The published artwork carries the flame, and only the flame. The SVG is
  the artifact; there is no raster copy to drift from it.
- The flame's placement is derived from the mark's own outline rather than
  drawn by eye, so the selection chrome, the layer list and the properties
  panel describe the object on the artboard.
- The Aegis baseline carries two new `zip-code` rows (the placement
  transform's scale factor, in each copy of the graphic) and drops the four
  `australian-tfn` rows the retired fox's bezier paths had contributed. The
  `australian-tfn` reason also returns to the wording the gate's source
  holds: the committed baseline had drifted from it when the brand study
  illustration was added, and regenerating from the source restores it.
- The mock-up is still a mock-up. It is original artwork of the shell, not a
  screenshot of a build, and the README now says so.

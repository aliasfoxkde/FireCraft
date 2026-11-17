# 0125 — The Optimize panel is bound to the record the export path reads

Date: 2026-11-16 · Tickets: T7, issue #23 (Phase 20.2) · Status: accepted

## Context

Issue #23 asks the Optimize panel to be bound to the document's
`ExportOptions` — "not to a page-level setting" — on the stated ground
that "the document's `ExportOptions` is what the export path actually
reads".

The export path does not read the document's record first. The
resolution is `io::expv::for_page`:

```rust
pub fn for_page(document: &doc::Document, page_index: usize) -> Option<&ExportOptions> {
    let page = document.pages.get(page_index);
    page.and_then(|page| page.export.as_ref())
        .or(document.export.as_ref())
}
```

The page's `EXPv` comes first; the document's is the fallback. The CLI
export resolves through it — `app_cli::ops::export_raster` takes
`::io::expv::for_page(document, 0)` and hands that record to
`jpeg_options`, `png_options` or `gif_options` — and so does every
other export path in the tree.

The panel writes the current page's record, through the command
`page.setExport` (decision 0095's field set). So the panel and the
export already read the same record: the panel's page record when the
page carries one, the document's otherwise. The panel's preview
resolves the same way (`io::expv::for_page(document, page)`).

Binding the panel to the document's record instead would *break* the
binding the issue asks for. `for_page` prefers the page's record, so a
document-level edit would be ignored by the export whenever the page
carries one — the panel would show a value the export ignores, which
is the exact failure the issue was opened to prevent.

## Decision

- **The panel stays bound to the page's `EXPv`,** because that is the
  record `io::expv::for_page` — the export path's own resolution —
  reads for that page. The panel and the export cannot diverge.
- **Issue #23's requirement to bind to the document's `ExportOptions`
  is superseded** by the resolution code. The issue's premise about
  what the export path reads is not what the code does.
- **The binding is proved by a regression, not by inspection.** The
  test `the_preview_encodes_what_the_export_encodes` changes the
  quality through the panel's own command path, encodes the page
  through the panel's preview, and encodes the same document through
  `app_cli::ops::export_raster`: the two byte strings are equal, and a
  second quality change makes the exported bytes differ. A panel
  reading a different record would fail the equality; a preview
  reading a different configuration than the export would fail it too.

## Consequences

- Issue #23's acceptance item — "the preview is bound to the same
  `ExportOptions` the export path reads — verified by the regression,
  not by inspection alone" — is met by that test.
- The panel's preview and the exported file are the same bytes for the
  same document and settings, which is the property the issue's
  "both halves" criterion asks for.
- The document-level `ExportOptions` remains reachable as the fallback
  for pages that record nothing; the panel shows the resolved value
  for the page it is on, so a document-level default appears in the
  panel unchanged and editing it writes the page's record — which is
  how a page override is made.
- The pixel form of the issue's `egui_kittest` snapshot stays open: the
  CI image proves no software adapter, so a pixel comparison cannot run
  there (decision 0087). The receipt delivered here is the byte-level
  one, which is the half that decides whether the preview is the
  export.

# 0105 — The Pages panel renames and deletes pages

Date: 2026-11-12 · Issue: [#139](https://github.com/aliasfoxkde/FireCraft/issues/139) ·
Status: accepted

## Context

The Pages panel listed the document's pages and nothing else: a row
per page, a click to make it current. The engine has carried
`page.add`, `page.delete` and `page.setName` from the start, and the
shell's menu could add a page — but the panel could neither rename nor
delete one. A multi-page Fireworks document therefore accumulated
unnamed pages that could not be removed except by editing the file
outside the application.

The gap is a surface gap, not a capability gap: the reachability gate
measured `page.delete` and `page.setName` as carried by the engine and
unbound by the shell.

## Decision

- **The Pages panel carries a rename and a delete per row**, in the
  same shape as the Layers panel's rename: a "✎" button opens an
  inline name field (Enter applies), and a "×" button deletes the
  page.
- **The delete goes through `page.delete`**, so the engine's refusal
  stands: the last page of a document cannot be deleted, and the
  refusal reaches the status line.
- **The current page index follows the shorter list.** Deleting the
  page the user is on moves them to the last remaining page rather
  than leaving the index past the end.
- **Undo restores the page.** The command's delta carries the whole
  page, so a mistaken delete is one undo away — the same bargain the
  Layers panel's delete makes.

## Evidence

- `page.delete` refuses to remove the last page (`cannot delete the
  last page`), and `page.setName` writes the page's name field.
- The Layers panel already renames and deletes layers this way, with
  no confirmation dialog: the panel is the deliberate surface, and the
  engine's refusal plus undo carry the edge cases.
- The Pages panel's row is a snapshot of the document's page list, so
  a rename or a delete is visible in the next frame.

## Consequences

- Two more engine commands are reachable from the shell's designed
  surface: `page.setName` and `page.delete`.
- The page rename follows the document, so a saved file carries the
  names the user gave its pages.
- The shell's `rename_page` field mirrors `rename_layer`: one inline
  rename open at a time.

See also decision [0104](0104-the-shells-selection-verbs-go-through-the-engine.md)
— the same week's other step in closing the measured surface gap.

# T20 — 20.1 In-canvas text editing

**Objective:** double-click a text object edits it in place: an egui
TextEdit overlay at the text frame, writing through `text.setText`.

**Inputs:** `text.createText`/`text.setText` commands; the shell's
canvas overlay machinery (transform handles from 19.1 show the pattern);
`TextStyle` for caret styling; CS6 in-place editing behavior (frame
grows with content, Esc/commit semantics).

**Authority:** CS6 observed editing behavior.

**Stopping condition:** in-place edit round-trips (create → edit →
re-render shows new glyphs), Esc cancels, click-away commits; kittest
focus-regression tests land with it; wasm ceiling holds.

**Verification:** `cargo test -p app` incl. the new kittest tests, full
gate before commit.

**LANDED 2026-11-06** (`shell::begin_text_edit` / `text_edit_overlay`):
entry is a Text-tool click on a text object or a Pointer double-click
(both CS6 gestures); the overlay is a frameless `TextEdit` pinned to
the frame at current zoom; Esc cancels, focus loss or a tool switch
commits (once — an unchanged buffer writes no journal row); re-opening
mid-edit keeps the buffer. Session tests
`text_edit_session_buffers_and_commits` / `text_edit_cancel_and_lost_object`
cover buffer, commit, cancel, and the vanished-object path;
`text_edit_focus_paths` covers the tool-switch commit and the Esc
no-write path. The kittest focus-regression tests are folded into the
repo-wide egui_kittest snapshot gate (NEXT.md "Quality gates for UI
work", gate 1) — the crate carries no kittest harness yet, so the
focus paths are verified at the session level; that gate is the
tracking point for UI-level focus coverage.


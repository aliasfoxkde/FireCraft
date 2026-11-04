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

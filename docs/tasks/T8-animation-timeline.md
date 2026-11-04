# T8 — 20.3 Animation timeline

**Objective:** the frames strip with per-frame timing, onion skinning,
and tween instance playback (`symbol.tween`).

**Inputs:** the page/frame model (states), `symbol.tween` command, the
shell's frame navigation UI, CS6 States/Animation panel behavior.

**Authority:** CS6 observed behavior; the engine's frame/animation
command surface (already parity-complete).

**Stopping condition:** strip navigates states, timing edits persist in
the model, onion skin renders (toggleable), tween instances animate;
tests cover state transitions; full gate green.

**Verification:** `cargo test -p app`, manual canvas check (rendered
frames), full gate before commit.

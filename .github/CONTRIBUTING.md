# Contributing to FireCraft

Thank you for your interest in improving FireCraft — a clean-room,
open-source design and image editor in pure Rust, with a first-class
WASM build.

## Ground rules

- **Clean-room policy.** Contributions must be written from public
  specs, observed behavior, and corpus evidence — never from
  decompiled or copied proprietary code or assets (see
  `docs/PLANNING.md` for the full method). Do not paste output from
  disassemblers into code, comments, tests, or docs. Do not submit
  fonts, icons, or imagery extracted from the reference application.
- **Licensing.** The project is licensed under Apache-2.0. By
  contributing you agree your work is released under it, unless you
  state otherwise in the contribution.
- **Assets.** Any non-code asset (font, image, icon set) must be an
  original work or permissively licensed, and must add a row to
  `docs/ATTRIBUTION.md` in the same change.

## Workflow

1. Fork / branch from `main`.
2. Make your change. The architecture is a layered crate workspace —
   see `docs/PLANNING.md` §7 and `CLAUDE.md` for the layout.
3. Run the full gate before pushing:

   ```
   cargo run -p xtask -- ci
   ```

   That runs fmt, clippy (`-D warnings`), the test suite, layer
   enforcement, and the wasm build. All of them must pass; the
   repository carries rising-only test floors, so new behavior needs
   tests.
4. Commit with a conventional subject (`feat:`, `fix:`, `docs:`,
   `refactor:`, `test:`, `chore:`), imperative mood, ≤50 characters.
5. Open a pull request with a clear description of the behavior
   change and how you verified it.

## Code style

- No `unwrap()` or `panic!` in non-test code; return `Result`.
- `unsafe_code = "forbid"` — do not relax it.
- Never break the wasm build.
- User-visible changes should update `docs/CHANGELOG.md`.

## Where to discuss

- Bugs and features: GitHub Issues.
- Questions and design talk: GitHub Discussions.

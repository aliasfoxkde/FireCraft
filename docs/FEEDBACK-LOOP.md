# FireCraft Controlled Feedback-to-Release Loop

## 1. Diagnostic Payload (`firecraft-diagnostic/v1`)

In-app feedback collects only minimal, user-consented diagnostic fields:
- `schema`: `"firecraft-diagnostic/v1"`
- `app_version`: workspace version + git short SHA
- `platform`: OS / arch / target (`native` or `wasm32`)
- `summary`: user-entered description
- `repro_steps`: optional user-entered steps

All free-text fields pass through a PII/secret scrubber (redacting email addresses, local home paths, and token-like strings) before previewing or opening a prefilled GitHub Issue URL.

## 2. Zero-Token Submission

- **Primary (Zero-Token):** Opens a prefilled `https://github.com/aliasfoxkde/FireCraft/issues/new?...` URL in the system browser (or `window.open` on WASM).
- **Offline:** Copies the scrubbed diagnostic JSON/Markdown to the clipboard or saves it locally.

## 3. Triage & Release Cadence

- **P0 (Crash / Data Loss / Security):** Immediate hotfix branch, regression test, and patch release.
- **P1 (Broken Core Workflow):** Targeted for the next minor/patch milestone.
- **P2/P3 (Polish / Enhancement):** Triaged into `docs/TASKS.md` and milestone backlog.

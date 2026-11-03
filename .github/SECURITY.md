# Security Policy

## Supported versions

Security fixes are made against the latest tagged release and `main`.
Older releases are supported best-effort only.

## Reporting a vulnerability

Please do **not** open a public issue for a suspected vulnerability.

Use GitHub's private vulnerability reporting on this repository
(**Security → Report a vulnerability**), which notifies the
maintainers privately. Include:

- the affected version or commit,
- the environment (desktop OS/browser for the WASM build),
- a description or proof-of-concept of the issue,
- the impact you believe it enables.

You will receive an acknowledgment within a few days and a follow-up
with the assessment. Once a fix ships, we will credit the reporter in
the release notes unless anonymity is requested.

## Scope notes

FireCraft processes untrusted document files (Fireworks PNG and
raster formats) and runs as a WASM application in the browser.
Areas of particular interest include parsers for untrusted input and
the web bridge boundary. Findings in vendored dependencies should
also be reported here if they are reachable from FireCraft.

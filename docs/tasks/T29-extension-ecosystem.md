# T29 — Build the FireCraft extension ecosystem

**Objective:** deliver a trustworthy, discoverable extension ecosystem with a
public official/community catalog and author submission path, followed by
in-app install/update. The current site is informational only; no package
submission, installable catalog, or one-click installation is live.

**Inputs:**
- [Historical Fireworks workflow and licensing research](../research/fireworks-extensions.md).
- [PLUG-01 / PLUG-02 architecture requirements](../POST-MVP-ROADMAP.md#514-plugin-ecosystem-and-extensibility).
- [Marketing site page requirements](../MARKETING-SITE-ROADMAP.md#51-required-pages).
- Existing command registry, document transaction/undo paths, MCP APIs, release
  signing and provenance work.

**Authority:** implement against FireCraft's versioned interfaces only. Do not
copy legacy Fireworks code, packages, screenshots, or assets without explicit
file-level rights. A public repository or a roundup entry is not a license.

## Work sequence

1. **Catalog and author contract:** define the typed API and compatibility
   policy; version the package manifest; specify artifact kinds, entry points,
   dependencies, capabilities, source revision, license, publisher, and
   immutable release metadata. Build a schema validator, SDK examples, API
   reference, and compatibility fixtures.
2. **Safe local manager:** implement per-user staging, archive/path/size checks,
   integrity and provenance validation, explicit capability review, activation,
   disable/uninstall, offline install, update, rollback, safe startup, and
   structured diagnostics. A download must not implicitly execute code.
3. **Catalog service and submissions:** publish official and community entries
   with task-based search, artifact kind, supported app/API and platforms,
   scope/selection requirements, shortcut/menu placement, expected outputs,
   license, publisher, release history, capabilities, and precise validation
   labels. Add authenticated publisher submissions and visible review status.
4. **Review and response operations:** automate schema, archive, dependency,
   license, source/revision, compatibility, and integrity checks; separately
   record human review. Define rejection/appeal, security report, takedown,
   advisory, owner transfer, abandonment, and official-promotion procedures.
5. **Marketing launch:** replace the informational-only status only after the
   catalog, moderation, support ownership, privacy terms, and security response
   are operational. Do not populate the page with placeholder entries or
   fabricated ratings/review states.
6. **In-app discovery:** add browse/search, stage, explicit enable, update, and
   rollback flows only after the above gates. Capability expansion requires
   renewed consent; local/offline extensions remain usable without an account.

## Acceptance evidence

- API contract and package schema have compatibility tests and published
  version/deprecation rules.
- Malformed, path-traversing, oversized, tampered, incompatible, interrupted,
  permission-escalating, and rollback cases are covered by tests.
- A real sample community package adds a command and panel without modifying
  FireCraft core; it uses approved mutation APIs and document undo.
- Official/community ownership, license, source, API/platform compatibility,
  capabilities, and each actually completed validation check are visible.
- Submission, review, security incident, delisting/advisory, and promotion
  operations are exercised end to end before opening public submissions.
- Website and in-app claims match the live service state; no simulated catalog
  or invented publisher/review data is used.

**Current state (2026-11-26):** historical research and an informational
Extensions page are published in the repository. The page links users to an
existing GitHub issue form for use-case ideas. API/SDK, installer, catalog
service, package intake, moderation operations, and in-app marketplace are not
implemented.

**Stopping condition:** the public catalog/submission launch and in-app
installer each have independent acceptance receipts. Do not mark T29 complete
when only the roadmap or marketing page exists.

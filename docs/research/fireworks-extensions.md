# Fireworks extensions: workflow research and marketplace design

**Reviewed:** 2026-11-26

**Purpose:** extract product and ecosystem lessons from historical Fireworks extension catalogs and source mirrors for FireCraft's planned Extension Library. This is a desk study, not a security audit, license opinion, or claim that any legacy package works in FireCraft.

## Executive synthesis

The strongest pattern across these references is that extensions remove repeated work close to the current selection or document: batch page/export work, precise geometry changes, responsive resize behavior, reusable grids, text and color conversion, linked asset refresh, path operations, and custom tools. They come in several interaction forms—one-shot commands, keyboard shortcut sets, persistent panels, editable auto-shapes, and developer utilities. A useful FireCraft library should index all of these by the job a person is trying to do and expose commands through the same search and command registry as built-in actions.

The history also exposes failure modes to design out. Fireworks installation depended on legacy MXP/Extension Manager workflows, sometimes privileged installation, copied shortcut files, and restarts. Source mirrors may be incomplete, stale, or messy; `.JSF` commands could be paired with `.FLA`/`.SWF` panels; catalog descriptions are not compatibility tests. FireCraft should adopt the workflow ideas while using its own versioned API, modern package metadata, per-user verified installation, capability controls, rollback, and a clear distinction between official and community software.

The updated Speckyboy index and Smashing Magazine's categorized guide sharpen the catalog requirements: group by the work users are trying to finish, and make an entry actionable with selection/document scope, output, shortcut or menu location, supported version, and installation state. Examples span grids and presets, responsive prototypes, page/state exports, SVG and sprite generation, named-asset export, smart-resize anchors, and editable tools. These are workflow references only; they do not establish current FireCraft support or grant reuse rights.

## What the sources show

| Source | Observed material | FireCraft insight | Limit |
|---|---|---|---|
| [Speckyboy: 15 Free Adobe Fireworks Extensions & Commands](https://speckyboy.com/adobe-fireworks-extensions-commands-for-web-designers/) | The 2024-updated roundup covers responsive prototype export, modular grids, document demos, slice-to-code, page/state/sprite/style/SVG export, guides, mockup tables, CSS cleanup, favicons, placeholders, text, glyphs, and command collections. | Organize the catalog around tasks and outcomes: layout, prototype, web export, vectors, text, and asset preparation. Surface concrete before/after examples and expected outputs. | A roundup is a discovery source, not proof that linked downloads remain available, maintained, compatible, safe, or reusable. Several entries describe older Fireworks-era behavior. |
| [Smashing Magazine: Useful Adobe Fireworks Resources — Extensions (Part 1)](https://www.smashingmagazine.com/2014/07/useful-adobe-fireworks-resources-extensions-part1/) | The article groups commands, command panels, and auto-shapes. Examples include responsive smart resize with per-object anchors, multi-border controls, data-driven tables, keyboard resizing that preserves text proportions, linked image refresh, grids/presets, generated web assets, path commands, text/color helpers, textures, and prototype frameworks. | Treat extensions as first-class workflow tools, not only scripts. Preserve object relationships during transforms; make repetitive export/settings changes batchable; use direct property/control-point interaction for objects with meaningful controls. | The article describes Fireworks CS6 and historical packaging/runtime assumptions. Do not present feature descriptions as a FireCraft parity commitment. |
| [Aaron Beall's Fireworks Guru extension catalog](https://fireworks.abeall.com/extensions/) | Its taxonomy separates commands, panels, and Auto Shapes & Tools. The catalog groups path combination/alteration/point-selection tools and also lists glyph, layer, export, objects, patterns, text, brush, document, filters, web, grouping, masks, modify, rename, and grid extensions. Its 2011 page describes an auto-updater and Extension Manager/MXP install flow that could require administrator launch on Vista/Windows 7. | Make extension kind, supported actions, affected document scope, and update channel explicit metadata. Design safe per-user updates instead of requiring an administrator or restarting the editor. | Page is last updated in 2011; its install/runtime instructions are obsolete examples, not recommendations. Downloads need independent provenance and compatibility checks. |
| [John Dunning's Fireworks catalog](https://www.johndunning.com/fireworks/) | Productivity and developer tools include page/layer batching, page-size adjustments, shared export settings, responsive export, Dynamic Symbols, asset generation, text variables, SVG import, a developer console, spacing controls, keyboard resizing, grid insertion, and auto-shape helpers. | Target document-level and cross-page jobs that otherwise require repeated manual steps. Distinguish quick commands from stateful panels; document scope, selection behavior, presets, and output locations. Search should cover installed commands and context targets. | A free download/catalog listing does not grant a redistribution license. Current availability and each extension's maintenance state were not verified. The historical developer console is not a safe model for unrestricted FireCraft code execution. |
| [Matt Stow's Firewoiks extension index](https://www.mattstow.com/fireworks.html) | Examples include CSS normalization/export helpers, fluid-width calculations that account for strokes/groups, exact corner radii after scaling, and color-to-clipboard commands. Entries include version history describing bug fixes and compatibility adjustments. | Put unit-aware geometry/color operations close to the selection; account for strokes, groups, and transforms rather than reporting only raw primitive dimensions. Show compatibility and change history, and test generated CSS/serialized values. | Material is dated (2012–2013) and tied to legacy browser/CSS behavior. It is inspiration for workflow and regression coverage, not current web-output guidance. |
| [bomberstudios' Orange Commands](https://bomberstudios.github.io/fireworks/) | A documented collection separates many small commands into areas such as align/distribute, alpha, canvas, color, effects, export, grids, guides, layers, and text. The page gives command descriptions and shortcuts, and lists separate CS3–CS6 packages. | Catalog commands consistently with name, category, context/selection requirements, shortcut, concise result, and version support. Bundled narrow tools can be easier to learn than a monolithic panel. | It targets Adobe Fireworks, with old platform-specific shortcut installation paths. The web page's “open source” label does not establish the license for every file; verify the repository's actual notices before reuse. |
| [senocular's Fireworks extension source mirror](https://github.com/senocular/fireworks-extensions) | Its README explicitly warns that source may be older than published extensions, incomplete (including panel graphics), lost, and messy. It distinguishes `.MXI` installer metadata, `.JSF` JavaScript commands, and `.FLA`/`.SWF` Flash panel UI. GitHub repository metadata currently has no license identifier. | Require a complete manifest and package inventory; validate entry points, resources, host/API versions, and hashes rather than trusting a catalog label. Test the installed artifact, not just source. Make missing resources and unsupported API calls visible. | Historical source/installer files are not suitable inputs to FireCraft. Lack of a repository license identifier means no blanket reuse permission; each file needs provenance and licensing review. |
| [WaveF/Fireworks repository](https://github.com/WaveF/Fireworks) | The repository describes itself as Fireworks extensions and artwork and has top-level directories for Command Panels, Commands, Concepts, Extension Packs, JSML, and Styles. GitHub metadata currently has no license identifier. | A useful catalog needs more than executable plugins: list templates, style/resource packs, examples, and learning material as separate artifact kinds with clear install semantics. Provide API concepts and package docs alongside runnable samples. | Public hosting does not establish license or suitability for reuse. “Concepts” and “artworks” should not be treated as production-compatible packages. |
| [bomberstudios' Nathan Pitman extension mirror](https://github.com/bomberstudios/nathan-pitman-fireworks-extensions) | The README calls it a source mirror. The repository includes small utilities such as AutoSave, Calculator, Duplicate/Offset/Rotate, FillerText, InsertBrowserCursor, MainToolbar, and NewFromTemplate. GitHub metadata currently has no license identifier. | Small, focused utilities can deliver real value; support narrowly scoped commands and templates, not only large extensions. Provide install/enable/disable and provenance information even for a one-command package. | The mirror itself says the extensions were released elsewhere. Its contents may not be complete or current, and no repository-level license metadata is present. Do not copy code or assets without file-level rights review. |

## Workflow categories to use in the FireCraft catalog

Start with user goals rather than legacy extension authors or historical software versions:

1. **Arrange and transform:** align/distribute, smart resize, spacing, duplicate/offset/rotate, grid insertion, guides, and geometry helpers.
2. **Document and page operations:** create/rename/resize pages, distribute layers, batch apply export settings, generate demos, templates, and multi-page transformations.
3. **Web and asset export:** export named objects, slices, sprites, pages/states, SVG, CSS/style tokens, favicons, responsive previews, and format batches.
4. **Vector and path tools:** combine/deform paths, edit/select points, convert strokes, create auto-shapes, and expose adjustable per-object parameters.
5. **Text, typography, and color:** placeholder text, case conversion, text attributes, glyph insertion, variables, exact color clipboard formats, and style generation.
6. **Assets and resources:** linked-image refresh, textures, patterns, styles, brushes, templates, and reusable libraries. Cataloging does not imply that the app can execute each asset kind.
7. **Developer and inspection tools:** command history, API examples, document/selection inspection, output diagnostics, and package validators. Production extensions must not inherit an unrestricted console merely because a legacy developer tool had one.

An extension can belong to more than one category. Catalog metadata should capture artifact kind (`command`, `panel`, `tool/auto-shape`, `resource pack`, `template`, `developer tool`), workflow tags, app version range, extension API version, platform support, license, publisher, source, release history, screenshots, and requested capabilities.

## Public Extension Library and community marketplace

The marketing site should have a durable **Extensions** page (for example, `/extensions`) linked from product navigation and the README. It should explain the extension model and show official and community catalogs without implying equivalent support. The page can exist before in-app installation; it must not pretend one-click installation works until the signed package flow is implemented.

The current site page is an informational program page, not a live marketplace: it accepts extension ideas through the project's existing issue form, but it does not accept package submissions, list installable extensions, or provide one-click installation. Keep that distinction visible until a real catalog and submission service pass their launch gates. When the catalog is operational, provide separate official and community browse views, task/category filters, compatibility and capability facets, and a publisher submission path with review-status tracking. Do not use a mock catalog or fabricated review badges to make the page look populated.

Historical catalogs also suggest a practical listing format: title, publisher, concise task description, extension kind, supported app/API versions, platform, selection/document scope, shortcut or menu location, expected output, screenshots, install/update instructions, release history, and license. Search should match both user phrasing ("export selected layers as assets") and implementation terms ("batch export", "sprite", "SVG").

### Official vs community status

- **Official** means FireCraft maintainers own the package and compatibility promise, publish immutable signed releases from the canonical repository, and test it against supported app/API versions. Official status is earned through maintainer adoption and release gates; it is not a popularity badge.
- **Community** means an independently maintained package with a named publisher and source repository. Display license, app/API compatibility, release date, source, permissions, and review status. Do not imply that community packages are reviewed unless an actual review record exists.
- **Review labels** should distinguish automated package validation, API compatibility checks, human code/security review, and official maintenance. “Validated” must name exactly which checks ran and for which version.
- A maintainer may propose promoting a community package to official status. Promotion requires rights to the code/assets, an explicit ownership and support transfer, stable API compatibility, tests and release automation, security/privacy review, issue handling, and a new official signed build. Preserve original authorship and attribution.

### Submission and moderation lifecycle

1. The author submits a manifest, immutable release artifact, source URL/revision, license, screenshots, changelog, compatibility range, contact/maintainer details, and a declared capability list.
2. Automated intake checks schema, hashes/signatures, archive paths, malware/vulnerability signals, dependency/license metadata, forbidden secrets, compatibility tests, and that the claimed artifact matches its source revision.
3. Human review checks behavior, permissions, privacy, accessibility of UI, update/ownership policy, and whether product/Adobe trademarks or assets imply endorsement.
4. Accepted releases enter the community catalog with their exact review state and immutable version. A takedown/security-report path can delist distribution without rewriting history; users can see advisories and affected versions.
5. Updates repeat validation. A package cannot silently broaden its declared capabilities; capability changes require renewed user consent. Maintain owner transfer and abandoned-package procedures.

### Installation and security requirements

- Use a versioned manifest and package format; reject path traversal, ambiguous entry points, unbounded decompression, untrusted symlinks, mismatched hashes, and unsigned/untrusted release metadata according to the selected trust policy.
- Install to a per-user staging area atomically. Verify before activation, retain the prior version, support rollback, and keep a safe mode that starts without third-party code.
- Show source/publisher, official/community status, license, version/API compatibility, capabilities, data/network behavior, dependencies, and last validation before installation and before first enable.
- “One click” may fetch and stage a package. Enabling executable code is a separate explicit action; never auto-enable a new package or silently accept a permission increase.
- Route mutations through versioned FireCraft commands and document transactions so they validate inputs, respect undo, and report structured failures. Keep read-only access separate from mutations.
- Prefer a restricted runtime and capability-based file/network/process access. Do not promise that a process boundary or scripting API is a sandbox until it has been threat-modeled and tested. A terminal, arbitrary file access, network, and secrets require distinct capabilities and explicit consent.
- Make local install, offline packages, disable/uninstall, API compatibility warnings, update rollback, and safe startup work without requiring an account. Catalog sign-in or telemetry must not become a condition of using locally installed extensions.

## Architecture and delivery sequence

1. **API contract:** inventory the command/MCP/document model surface; define typed, versioned extension commands and read-only document queries. Add compatibility tests and one example package. No arbitrary legacy DOM or Flash/SWF compatibility promise.
2. **Local manager:** install/enable/disable/update/uninstall, per-user staging, hashes/signatures, permission review, safe mode, logs, offline install, pinning, and rollback. Test malformed, incompatible, interrupted, and permission-escalation packages.
3. **Author SDK:** manifest schema, validator CLI, examples, compatibility harness, API docs, migration policy, and stable release process.
4. **Public catalog:** marketing `/extensions` page with separate official/community views; provenance and status are visible. Begin with a manually curated catalog and no executable one-click install until the local manager is safe.
5. **Community submissions:** automated and human review queue, policy/license checks, vulnerability reporting, publisher/ownership management, immutable versions, removal/advisory handling, and promotion-to-official criteria.
6. **In-app one-click lifecycle:** integrate catalog search and stage install/update in the extension manager; show capabilities before install and first enable; require fresh consent for capability escalation; allow rollback and safe mode from Help/Settings.

These are planned milestones; no FireCraft extension marketplace or third-party execution sandbox is implemented by this research document.

## Licensing and compatibility guardrails

- Do not copy Fireworks `.JSF`, `.MXI`, `.FLA`, `.SWF`, artwork, icons, screenshots, or downloaded extension bundles into FireCraft. Recreate product behavior independently against FireCraft's own typed APIs.
- “Free,” “open source” in a roundup, a public GitHub repository, or a mirror is not a substitute for a file-level license. The GitHub API reported no repository license identifier for the three mirrors above at the time of review; other files may carry their own notices, which still require inspection.
- Adobe Fireworks/Fireworks CS6 are product trademarks. Describe FireCraft as Fireworks-inspired/clean-room and avoid implying Adobe affiliation, file/API compatibility, or endorsement.
- Legacy installers, menu paths, OS versions, and Flash panels are historical evidence. Do not run untrusted extension code or install MXP/JSF/SWF packages into FireCraft. Compatibility is per FireCraft API release and platform, tested on the actual packaged extension.
- Treat availability and source contents as time-sensitive. Recheck upstream pages, revisions, licenses, and download integrity before any future publication or formal endorsement.

## References

- [Speckyboy — 15 Free Adobe Fireworks Extensions & Commands for Web Designers](https://speckyboy.com/adobe-fireworks-extensions-commands-for-web-designers/)
- [Smashing Magazine — Useful Adobe Fireworks Resources: Extensions (Part 1)](https://www.smashingmagazine.com/2014/07/useful-adobe-fireworks-resources-extensions-part1/)
- [Aaron Beall — Fireworks Guru extensions](https://fireworks.abeall.com/extensions/)
- [John Dunning — Fireworks extensions](https://www.johndunning.com/fireworks/)
- [Matt Stow — Firewoiks extensions and resources](https://www.mattstow.com/fireworks.html)
- [bomberstudios — Orange Commands](https://bomberstudios.github.io/fireworks/)
- [senocular — Fireworks extension sources](https://github.com/senocular/fireworks-extensions) and [README caveats/file types](https://raw.githubusercontent.com/senocular/fireworks-extensions/master/README.md)
- [WaveF — Fireworks extensions and artworks](https://github.com/WaveF/Fireworks)
- [bomberstudios — Nathan Pitman extension mirror](https://github.com/bomberstudios/nathan-pitman-fireworks-extensions)
- [Smashing Magazine — introduction to Fireworks extension development](https://www.smashingmagazine.com/2014/06/introduction-to-developing-fireworks-extensions-theyre-just-javascript/)
- [Fireworks-specific `fwlib` helper library](https://github.com/fwextensions/fwlib) and [developer console](https://github.com/fwextensions/fwconsole).

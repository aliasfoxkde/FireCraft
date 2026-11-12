# T27-35 — Audit `build-windows.ps1` and the Inno Setup configuration

> **RP:** RP-02 · **Priority:** P1 · **Status:** IMPLEMENTED (audited 2026-11-12)
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-04

**Objective:** read the Windows build script and the Inno Setup configuration line by
line and record their environment assumptions, product identity, shortcuts and version
metadata — the fleet cannot run them, so a written audit is the only inspection
available.

**Inputs:** `packaging/build-windows.ps1`; `packaging/windows/firecraft.iss`;
`docs/TOOLCHAINS.md`; the version stamps.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.2 (Windows requirements, including
the "Audit Windows build script environment assumptions" task named in the campaign).

**Acceptance criteria:** the script's environment assumptions are listed with their
risks; the installer's product identity, publisher, version, install scope and shortcut
set are documented; every value that must match a version stamp is identified.

**Verification:** `read packaging/build-windows.ps1 packaging/windows/firecraft.iss` —
plus, in this change, the fixes the audit produced are visible in the diff of the two
recipe files.

**Evidence:** [decision 0101](../../decisions/0101-off-fleet-packaging-recipe-audit.md)
records the audit and its consequences; the two recipe files carry the fixes inline.

**Blockers:** no Windows host. The build rows (T27-36/37/38) remain open.

**Stopping condition:** the audit is committed with a per-assumption risk list. **Met
2026-11-12** — the audit is below, and the three defects it found are fixed in the
recipes.

---

## The audit

### Environment assumptions and their risks

| # | Assumption the recipe makes | How it is met | Risk if unmet | What would catch it |
|---|---|---|---|---|
| A1 | A Windows host with PowerShell 5.1 or later | `#Requires -Version 5.1`; the script uses no PS7-only syntax | The recipe does not run at all | The operator's first invocation |
| A2 | The MSVC toolchain (`rustup target add x86_64-pc-windows-msvc` plus VS Build Tools for `link.exe`) | `rustup target add $Target` adds the target; `cargo build --release --locked -p app -p app-cli --target $Target` links with MSVC | The build fails at link time with an unreadable `link.exe` error | The build step itself |
| A3 | Network access for the crate registry | `--locked` pins the lockfile, but the crates still have to be fetched unless the cargo cache is warm | The build fails on a fetch; a cold cache on a fresh Windows host downloads the whole dependency graph | The build step itself |
| A4 | The Rust toolchain is at least the workspace `rust-version` (1.95) | Nothing in the recipe checks it; `rustup` installs whatever the operator has | A stale toolchain produces a confusing compile error rather than a clear one | The build step, indirectly |
| A5 | The MSVC runtime (`vcruntime140.dll` and friends) is present on the target machine | **Not handled.** The binaries are built against the MSVC CRT dynamically | On a clean Windows VM the app fails to start with a missing-DLL error, which reads as a broken build rather than a missing redistributable | The clean-VM smoke (T27-38) — the row's acceptance names "paths with spaces and non-ASCII characters" but nothing about the runtime |
| A6 | `ISCC.exe` (Inno Setup 6) on `PATH` | `Get-Command ISCC.exe -ErrorAction SilentlyContinue`; the script **skips** the installer when it is absent and says so | A Windows run without Inno Setup ships a ZIP and no installer, and the log line is easy to miss | The operator's own reading of the log; the artifact matrix records the installer as recipe-only |
| A7 | PowerShell's `Compress-Archive` and `Get-FileHash` behave as the script assumes | Both are used for the ZIP and the digest | A ZIP with an unexpected layout, or a digest that does not match the published file | The T27-36 inspection |
| A8 | The staged tree is disposable | `Remove-Item -Recurse -Force $Stage` before staging | Stale files from an earlier run survive into the ZIP | The T27-36 inspection |

### Product identity

| Field | Value | Source |
|---|---|---|
| `AppId` | `{7E6B4F4E-8E1F-4B0A-9A7C-4D2C1F0AF1CE}` | committed constant — the upgrade key, so it must never change between versions |
| `AppName` | `FireCraft` | the project name |
| `AppVersion` / `AppVerName` | the workspace version / `FireCraft <version>` | `[workspace.package] version` in `Cargo.toml` |
| `AppPublisher` | `FireCraft contributors` | **added by this audit** — matches the copyright holder in `NOTICE` |
| `AppPublisherURL` | `https://github.com/aliasfoxkde/FireCraft` | **added by this audit** — the repository URL from `Cargo.toml` |
| Install scope | per-machine (`{autopf}`), administrative install | `DefaultDirName`; Inno's default `PrivilegesRequired` is `admin`, which is what `{autopf}` requires |
| Install directory | `{autopf}\FireCraft` → `C:\Program Files\FireCraft` | committed constant |
| Start-menu group | `FireCraft` | `DefaultGroupName` |
| Output file | `firecraft-<version>-windows-x86_64-setup.exe` | the version stamp plus the platform triple |
| Uninstall display icon | `{app}\bin\firecraft.exe` | the desktop binary |
| `ArchitecturesInstallIn64BitMode` | `x64compatible` | the artifact is x86_64 only |

The installer ships no icon of its own (`SetupIconFile` is unset) because the repository
has no `.ico` — the brand refresh (issue #131) owns the artwork, and a `.ico` derived
from today's mark would be stale the moment it lands. The gap is recorded here and in
T27-43 rather than papered over with the wrong icon.

### Version metadata coupling

One version — `Cargo.toml`'s `[workspace.package] version` (currently `0.2.5`) — reaches
four places, and every one of them is substituted at build time rather than typed:

1. the ZIP file name, `firecraft-<version>-windows-x86_64.zip`;
2. the installer file name, `firecraft-<version>-windows-x86_64-setup.exe`;
3. `AppVersion` and `AppVerName` in the installer;
4. `OutputBaseFilename` in the installer, which is what `Move-Item` later renames.

`Get-WorkspaceVersion` reads it out of `Cargo.toml` and **throws** when the key is
missing, so a manifest without a version fails the recipe rather than producing a
`-windows-` artifact with an empty version. The installer path uses the same value
through the `__APPVERSION__` stand-in.

### Shortcut set

| Shortcut | Target | Notes |
|---|---|---|
| `{group}\FireCraft` | `{app}\bin\firecraft.exe` | the desktop application |
| `{group}\FireCraft CLI help` | `{cmd}` with `/k ""{app}\bin\firecraft-cli.exe"" --help` | opens a console window and prints the CLI help — a convenience, not a feature; the CLI has no `--gui` mode |

`[Run]` adds a post-install "Launch FireCraft" entry, flagged `nowait postinstall
skipifsilent` — it does not block a silent install.

## Defects the audit found, and the fixes

**D1 — the installer could not have compiled.** The recipe stages a substituted copy of
`firecraft.iss` into `dist\windows-stage\`, but the committed `LicenseFile=..\..\LICENSE`
is resolved against the *script's* directory. In the repository that path is the root
`LICENSE` and it exists; next to the staged script it is `dist\LICENSE`, which nothing
creates. The compiler would have failed on a missing license file — on the only path the
recipe actually uses. **Fixed:** the license is staged beside the script and the path is
substituted as `__LICENSEFILE__`, resolved to an absolute path so no directory
assumption survives. Compiling the committed `.iss` directly now fails closed on the
unsubstituted stand-in instead of producing a mislabelled installer.

**D2 — the distribution shipped without its `NOTICE`.** Apache-2.0 §4(d) requires a
distribution to carry the attribution notices when the work has a `NOTICE` file, and this
repository's `NOTICE` carries the Adobe trademark disclaimer — the disclaimer that says
FireCraft is not affiliated with Adobe. The recipe staged `LICENSE` only. **Fixed:** the
Windows tree and the macOS bundle both stage `NOTICE` next to `LICENSE`.

**D3 — the substitution could have mangled a path.** The recipe substituted the version
with `-replace`, whose replacement syntax reads `$` as a capture-group reference. The
version is numeric today, so the substitution was safe; the license path, which is not
numeric and can contain `$` in a user profile name, would not have been. **Fixed:** both
stand-ins are substituted through a `MatchEvaluator`, which treats the replacement as
literal text.

## What this audit does not close

- **No Windows build exists.** The recipes are still existence-checked only; T27-36
  (MSVC build and ZIP), T27-37 (installer) and T27-38 (clean-VM smoke) are open, and
  they are the rows that turn these assumptions into evidence.
- **The MSVC runtime assumption (A5) is unresolved.** It has two honest answers, both
  for the operator who runs T27-36: install the VC++ 2015-2022 redistributable on the
  smoke VM, or build the binaries with `-C target-feature=+crt-static` so they carry the
  runtime themselves. The second is a one-line change to the recipe's `cargo build`, and
  the recipe does not make it silently.
- **The shortcut set is not verified.** The `{cmd}` shortcut's quoting is the part most
  likely to be wrong, and only a real installer build (T27-37) can show it.

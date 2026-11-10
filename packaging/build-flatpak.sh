#!/usr/bin/env bash
# FireCraft Flatpak build recipe (T27 RP-01).
#
# RECIPE, not a gate: like the AppImage/Windows/macOS recipes, this
# needs the Flatpak SDK/runtime on the build host, which the release
# fleet does not carry. The manifest itself (dev.firecraft.FireCraft.yml)
# is YAML parse-checked by `xtask dist --native` so a structurally
# broken manifest fails the gate.
#
# Toolchain: flatpak + flatpak-builder, org.freedesktop.Sdk//24.08 and
# the corresponding .Sdk extension. Run from the repository root.
set -euo pipefail

readonly MANIFEST="packaging/flatpak/dev.firecraft.FireCraft.yml"
readonly REPO="dist/flatpak-repo"
readonly STAGE="dist/flatpak-stage"
readonly APP_ID="dev.firecraft.FireCraft"

version() {
    # The workspace version is the single source of truth.
    sed -n '/^\[workspace\.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p;q;}' Cargo.toml
}

readonly VERSION="$(version)"
readonly OUT="firecraft-${VERSION}-linux-x86_64.flatpak"

echo "==> flatpak-builder -> local repo ${REPO}"
rm -rf "${REPO}" "${STAGE}"
# --share=network: the cargo build inside the build sandbox fetches the
# dependency graph, and the sandbox has no network by default — without
# this the build dies at the first fetch. (A published flatpak should
# vendor the registry instead; this recipe builds the candidate, and
# the published artifact's provenance is the SBOM/attestation path.)
flatpak-builder --share=network --repo="${REPO}" --force-clean "${STAGE}" "${MANIFEST}"

echo "==> bundle -> ${OUT}"
flatpak build-bundle "${REPO}" "${OUT}" "${APP_ID}"

# The test installation. `flatpak run` has no --repo option: an
# application has to be installed into an installation before it can
# be run. The previous recipe passed --repo to `flatpak run`, which
# flatpak rejects ("Unknown option --repo"), so the launch step never
# ran — the recipe's only runtime evidence was a command that could
# not execute.
echo "==> install the bundle into the user installation"
flatpak install --user --bundle "${OUT}" --noninteractive
flatpak list --user --columns=application,version | grep -q "^${APP_ID}" ||
    echo "WARNING: ${APP_ID} is not listed after the install"

echo "==> launch: flatpak run --command=firecraft-cli ${APP_ID} --help"
flatpak run --command=firecraft-cli "${APP_ID}" --help

echo "==> removal"
flatpak uninstall --user --noninteractive "${APP_ID}"
if flatpak list --user --columns=application | grep -q "^${APP_ID}"; then
    echo "ERROR: ${APP_ID} is still installed after the removal" >&2
    exit 1
fi

echo "==> ${OUT}: $(sha256sum "${OUT}")"

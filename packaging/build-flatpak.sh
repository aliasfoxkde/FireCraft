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

version() {
    sed -n '/^\[workspace\.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p;q;}' Cargo.toml
}

readonly VERSION="$(version)"
readonly OUT="firecraft-${VERSION}-linux-x86_64.flatpak"

echo "==> flatpak-builder -> local repo ${REPO}"
rm -rf "${REPO}" "${STAGE}"
flatpak-builder --repo="${REPO}" --force-clean "${STAGE}" "${MANIFEST}"

echo "==> sandbox smoke: run --command=firecraft --version"
flatpak run --command=firecraft --repo="${REPO}" dev.firecraft.FireCraft --version

echo "==> bundle -> ${OUT}"
flatpak build-bundle "${REPO}" "${OUT}" dev.firecraft.FireCraft

echo "==> ${OUT}: $(sha256sum "${OUT}")"

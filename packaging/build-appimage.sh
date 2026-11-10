#!/usr/bin/env bash
# FireCraft AppImage recipe (M12).
#
# This is a RECIPE, not a gate: `xtask dist --native` produces the
# tar.gz/deb set on the release fleet; the AppImage is assembled by
# whoever releases, on a host where linuxdeploy is present or
# downloadable (its AppImage needs FUSE or --appimage-extract-and-run).
# The repo carries the recipe — never an untested claim of a built
# artifact.
#
# Toolchain (packaging/TOOLCHAINS.md): rustup stable, GNU tar,
# linuxdeploy-x86_64.AppImage (pin your download; see TOOLCHAINS.md).
# Run from the repository root.
set -euo pipefail

readonly LINUXDEPLOY="${LINUXDEPLOY:-linuxdeploy-x86_64.AppImage}"
readonly STAGE="dist/appimage-stage"

version() {
    # The workspace version is the single source of truth.
    sed -n '/^\[workspace\.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p;q;}' Cargo.toml
}

readonly VERSION="$(version)"
readonly OUT="firecraft-${VERSION}-linux-x86_64.AppImage"

echo "==> preflight"
# The desktop entry and the icon are a pair: linuxdeploy resolves
# `Icon=` against the icon theme in the AppDir, and an entry whose
# icon is missing produces an image with no icon rather than an error.
# Checking both here turns a silent packaging defect into a failed
# recipe.
readonly DESKTOP_SOURCE="packaging/desktop/firecraft.desktop"
readonly ICON_SOURCE="assets/icons/hicolor/256x256/apps/firecraft.png"
[ -f "${DESKTOP_SOURCE}" ] || { echo "missing ${DESKTOP_SOURCE}" >&2; exit 1; }
[ -f "${ICON_SOURCE}" ] || { echo "missing ${ICON_SOURCE}" >&2; exit 1; }
grep -q '^Icon=' "${DESKTOP_SOURCE}" ||
    { echo "${DESKTOP_SOURCE} has no Icon= entry" >&2; exit 1; }

echo "==> building release binaries"
cargo build --release --locked -p app -p app-cli

echo "==> staging AppDir"
rm -rf "${STAGE}"
mkdir -p "${STAGE}/usr/bin" \
         "${STAGE}/usr/share/applications" \
         "${STAGE}/usr/share/doc/firecraft"
install -m 755 target/release/firecraft     "${STAGE}/usr/bin/firecraft"
install -m 755 target/release/firecraft-cli "${STAGE}/usr/bin/firecraft-cli"
install -m 644 "${DESKTOP_SOURCE}" \
    "${STAGE}/usr/share/applications/firecraft.desktop"
{
    echo "FireCraft ${VERSION} is dual-licensed MIT OR Apache-2.0."
    echo
    cat LICENSE
} > "${STAGE}/usr/share/doc/firecraft/copyright"
mkdir -p "${STAGE}/usr/share/icons/hicolor/256x256/apps"
install -m 644 "${ICON_SOURCE}" \
    "${STAGE}/usr/share/icons/hicolor/256x256/apps/firecraft.png"

# Icon ships in-tree at assets/icons/hicolor/256x256/apps/firecraft.png —
# an original work drawn for this repo (see docs/ATTRIBUTION.md), not a
# copy from the reference install. linuxdeploy hard-requires an Icon=
# entry in the desktop file (continuous build, 2026-11-06: exits with
# "ERROR: Icon entry missing" and no output image), so the icon is part
# of the recipe contract, not optional decoration.

echo "==> linuxdeploy -> ${OUT}"
cd dist/appimage-stage
"${LINUXDEPLOY}" \
    --appdir . \
    --desktop-file usr/share/applications/firecraft.desktop \
    --output appimage
mv -f FireCraft_"${VERSION}"-x86_64.AppImage "../${OUT}"
cd ../..

echo "==> ${OUT}: $(sha256sum "${OUT}")"

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

echo "==> building release binaries"
cargo build --release --locked -p app -p app-cli

echo "==> staging AppDir"
rm -rf "${STAGE}"
mkdir -p "${STAGE}/usr/bin" \
         "${STAGE}/usr/share/applications" \
         "${STAGE}/usr/share/doc/firecraft"
install -m 755 target/release/firecraft     "${STAGE}/usr/bin/firecraft"
install -m 755 target/release/firecraft-cli "${STAGE}/usr/bin/firecraft-cli"
install -m 644 packaging/appimage/firecraft.desktop \
    "${STAGE}/usr/share/applications/firecraft.desktop"
{
    echo "FireCraft ${VERSION} is dual-licensed MIT OR Apache-2.0."
    echo
    cat LICENSE-MIT
    echo
    cat LICENSE-APACHE
} > "${STAGE}/usr/share/doc/firecraft/copyright"

# No icon ships in-tree (asset rule: docs/ATTRIBUTION.md) — linuxdeploy
# warns and proceeds; drop an icon into usr/share/icons/hicolor/.../apps/
# before this step if the release carries operator-supplied art.

echo "==> linuxdeploy -> ${OUT}"
cd dist/appimage-stage
"${LINUXDEPLOY}" \
    --appdir . \
    --desktop-file usr/share/applications/firecraft.desktop \
    --output appimage
mv -f FireCraft_"${VERSION}"-x86_64.AppImage "../${OUT}"
cd ../..

echo "==> ${OUT}: $(sha256sum "${OUT}")"

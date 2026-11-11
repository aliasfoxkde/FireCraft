#!/usr/bin/env bash
# FireCraft Flatpak build recipe (T27 RP-01).
#
# RECIPE, not a gate: like the AppImage/Windows/macOS recipes, this
# needs the Flatpak SDK/runtime on the build host, which the release
# fleet does not carry. The manifest itself (dev.firecraft.FireCraft.yml)
# is YAML parse-checked by `xtask dist --native` so a structurally
# broken manifest fails the gate.
#
# Toolchain: flatpak + flatpak-builder, org.freedesktop.Sdk//26.08 and
# the org.freedesktop.Sdk.Extension.rust-stable extension. Run from the
# repository root.
set -euo pipefail

readonly MANIFEST="packaging/flatpak/dev.firecraft.FireCraft.yml"
readonly REPO="dist/flatpak-repo"
readonly STAGE="dist/flatpak-stage"
readonly STATE="dist/flatpak-state"
readonly APP_ID="dev.firecraft.FireCraft"
readonly FLATHUB="https://dl.flathub.org/repo/flathub.flatpakrepo"

version() {
    # The workspace version is the single source of truth.
    #
    # The range form (`sed -n '/^\[workspace\.package\]/,/^\[/p'`) stops
    # at the first line of the range — the `[workspace.package]` header
    # itself — and quits before the `version =` line is ever printed, so
    # it yields an empty string and every artifact name below collapses to
    # `firecraft--linux-x86_64.flatpak`. Awk keeps scanning until it
    # sees the version key inside the section.
    awk '
      /^\[workspace\.package\]/ { in_pkg = 1; next }
      /^\[/ { in_pkg = 0 }
      in_pkg && /^version[ \t]*=/ { gsub(/[" ]/, "", $3); print $3; exit }
    ' Cargo.toml
}

readonly VERSION="$(version)"
readonly OUT="firecraft-${VERSION}-linux-x86_64.flatpak"

echo "==> flatpak-builder -> local repo ${REPO}"
rm -rf "${REPO}" "${STAGE}" "${STATE}"
# `--install-deps-from=flathub` names a remote that must already exist
# in the installation, or the builder aborts with "Remote flathub not
# found"; add it if this host has not seen it before.
flatpak remote-add --user --if-not-exists flathub "${FLATHUB}"
# flatpak-builder has NO --share option: its own option table
# (src/builder-main.c) rejects it with "Unknown option --share", so
# the previous recipe aborted before doing any work. The network grant
# for the cargo build inside the build sandbox is a *build-arg* and is
# declared in the manifest instead — see the module's
# `build-options.build-args: [--share=network]` in
# packaging/flatpak/dev.firecraft.FireCraft.yml. `flatpak build` (which
# flatpak-builder invokes) does accept `--share=network`, and it is
# what lifts the sandbox's default `--unshare-net`.
#
# --install-deps-from=flathub pulls the org.freedesktop.Sdk//26.08
# runtime and the rust-stable SDK extension the manifest names.
# --disable-rofiles-fuse: the rofiles-fuse helper is unavailable on
# hosts without FUSE, and the recipe must run unprivileged.
flatpak-builder --user \
    --install-deps-from=flathub \
    --disable-rofiles-fuse \
    --default-branch=stable \
    --force-clean \
    --repo="${REPO}" \
    --state-dir="${STATE}" \
    "${STAGE}" "${MANIFEST}"

echo "==> bundle -> ${OUT}"
# `flatpak build-bundle` takes the branch as its fourth argument and
# defaults to `master`; the builder was told to publish `stable`, so
# without the explicit branch the bundle resolves a ref that is not in
# the repo. --runtime-repo records where `flatpak install` can fetch
# the runtime from.
flatpak build-bundle --runtime-repo="${FLATHUB}" \
    "${REPO}" "${OUT}" "${APP_ID}" stable

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

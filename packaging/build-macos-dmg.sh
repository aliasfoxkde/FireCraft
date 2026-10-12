#!/usr/bin/env bash
# FireCraft macOS universal dmg recipe (M12).
#
# OFF-FLEET recipe: this host class has no macOS SDK and no Apple
# toolchain, so the repo never builds or ships a dmg — the operator
# runs this on macOS (Xcode command-line tools required) and owns the
# artifact. See packaging/TOOLCHAINS.md for the no-untested-claims
# rule. Run from the repository root.
set -euo pipefail

readonly TARGETS="aarch64-apple-darwin x86_64-apple-darwin"
readonly STAGE="dist/macos-stage"
readonly APP="FireCraft.app"

version() {
    sed -n '/^\[workspace\.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p;q;}' Cargo.toml
}

readonly VERSION="$(version)"
readonly OUT="firecraft-${VERSION}-macos-universal.dmg"

echo "==> rustup targets"
rustup target add ${TARGETS}

echo "==> building release binaries (both slices)"
cargo build --release --locked -p app -p app-cli --target aarch64-apple-darwin
cargo build --release --locked -p app -p app-cli --target x86_64-apple-darwin

echo "==> lipo -> universal"
rm -rf "${STAGE}"
mkdir -p "${STAGE}/${APP}/Contents/MacOS" \
         "${STAGE}/${APP}/Contents/Resources/licenses"
lipo -create \
    target/aarch64-apple-darwin/release/firecraft \
    target/x86_64-apple-darwin/release/firecraft \
    -output "${STAGE}/${APP}/Contents/MacOS/firecraft"
lipo -create \
    target/aarch64-apple-darwin/release/firecraft-cli \
    target/x86_64-apple-darwin/release/firecraft-cli \
    -output "${STAGE}/${APP}/Contents/MacOS/firecraft-cli"

# Bundle metadata: bundle id mirrors the WebOS packaging id
# (docs/WEBOS.md); version from the workspace manifest.
cat > "${STAGE}/${APP}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>CFBundleName</key><string>FireCraft</string>
    <key>CFBundleDisplayName</key><string>FireCraft</string>
    <key>CFBundleIdentifier</key><string>dev.firecraft.firecraft</string>
    <key>CFBundleVersion</key><string>${VERSION}</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundleExecutable</key><string>firecraft</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST

install -m 644 LICENSE-MIT \
    "${STAGE}/${APP}/Contents/Resources/licenses/LICENSE-MIT"
install -m 644 LICENSE-APACHE \
    "${STAGE}/${APP}/Contents/Resources/licenses/LICENSE-APACHE"

echo "==> codesign (ad-hoc; swap -s - for your identity to distribute)"
codesign --force --deep -s - "${STAGE}/${APP}"

echo "==> hdiutil -> ${OUT}"
hdiutil create -volname "FireCraft ${VERSION}" \
    -srcfolder "${STAGE}" -format UDZO -ov "${OUT}"

echo "==> ${OUT}: $(shasum -a 256 "${OUT}")"


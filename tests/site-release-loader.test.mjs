import test from "node:test";
import assert from "node:assert/strict";
import { isTrustedDownloadUrl, categorizeAsset, selectSafeAssets } from "../site/app.js";

test("isTrustedDownloadUrl allows only canonical GitHub release downloads", () => {
  assert.equal(isTrustedDownloadUrl("https://github.com/aliasfoxkde/FireCraft/releases/download/v1.0.0/firecraft.deb"), true);
  assert.equal(isTrustedDownloadUrl("https://evil.example.com/firecraft.exe"), false);
  assert.equal(isTrustedDownloadUrl("javascript:alert(1)"), false);
});

test("categorizeAsset classifies all 1.0 platforms", () => {
  assert.equal(categorizeAsset("firecraft-1.0.0-windows-x64-setup.exe"), "Windows");
  assert.equal(categorizeAsset("firecraft-1.0.0-macos-universal.dmg"), "macOS");
  assert.equal(categorizeAsset("firecraft-1.0.0-linux-x86_64.AppImage"), "Linux desktop");
  assert.equal(categorizeAsset("firecraft-web-1.0.0.zip"), "Web / WASM");
  assert.equal(categorizeAsset("RELEASE_SHA256.txt"), "Checksums & integrity");
});

test("selectSafeAssets filters untrusted URLs and non-uploaded assets", () => {
  const safe = selectSafeAssets({
    assets: [
      { name: "ok.deb", state: "uploaded", browser_download_url: "https://github.com/aliasfoxkde/FireCraft/releases/download/v1.0.0/ok.deb" },
      { name: "bad.exe", state: "uploaded", browser_download_url: "https://evil.example.com/bad.exe" },
      { name: "draft.zip", state: "starter", browser_download_url: "https://github.com/aliasfoxkde/FireCraft/releases/download/v1.0.0/draft.zip" },
    ],
  });
  assert.equal(safe.length, 1);
  assert.equal(safe[0].name, "ok.deb");
});

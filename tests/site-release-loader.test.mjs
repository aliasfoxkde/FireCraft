import test from "node:test";
import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import { isTrustedDownloadUrl, categorizeAsset, selectSafeAssets, nextTheme } from "../site/app.js";

test("isTrustedDownloadUrl allows only canonical GitHub release downloads", () => {
  assert.equal(isTrustedDownloadUrl("https://github.com/aliasfoxkde/FireCraft-CraftAnything-Experiment/releases/download/v0.3.5/firecraft_0.3.5_amd64.deb"), true);
  assert.equal(isTrustedDownloadUrl("https://github.com/aliasfoxkde/FireCraft/releases/download/v0.3.5/firecraft_0.3.5_amd64.deb"), false);
  assert.equal(isTrustedDownloadUrl("https://evil.example.com/firecraft.exe"), false);
  assert.equal(isTrustedDownloadUrl("javascript:alert(1)"), false);
});

test("categorizeAsset classifies published v0.3.5 release assets", () => {
  assert.equal(categorizeAsset("firecraft-v0.3.5-windows-x86_64.zip"), "Windows");
  assert.equal(categorizeAsset("firecraft-v0.3.5-macos-universal.dmg"), "macOS");
  assert.equal(categorizeAsset("firecraft-v0.3.5-linux-x86_64.AppImage"), "Linux desktop");
  assert.equal(categorizeAsset("firecraft-v0.3.5-wasm32-web.zip"), "Web / WASM");
  assert.equal(categorizeAsset("RELEASE_SHA256.txt"), "Checksums & integrity");
});

test("selectSafeAssets filters untrusted URLs and non-uploaded assets", () => {
  const safe = selectSafeAssets({
    assets: [
      { name: "ok.deb", state: "uploaded", browser_download_url: "https://github.com/aliasfoxkde/FireCraft-CraftAnything-Experiment/releases/download/v0.3.5/ok.deb" },
      { name: "bad.exe", state: "uploaded", browser_download_url: "https://evil.example.com/bad.exe" },
      { name: "draft.zip", state: "starter", browser_download_url: "https://github.com/aliasfoxkde/FireCraft-CraftAnything-Experiment/releases/download/v0.3.5/draft.zip" },
    ],
  });
  assert.equal(safe.length, 1);
  assert.equal(safe[0].name, "ok.deb");
});

test("marketing page points to the canonical product, demos, and docs", async () => {
  const html = await readFile(new URL("../site/index.html", import.meta.url), "utf8");
  const app = await readFile(new URL("../site/app.js", import.meta.url), "utf8");
  assert.match(html, /FireCraft - Open Source Raster and Vector Graphics Editor/);
  assert.match(html, /FireCraft-CraftAnything-Experiment\/releases\/latest/);
  assert.match(html, /wasm-review-v0-3-5\.firecraft-web\.pages\.dev/);
  assert.match(html, /artcraft-webos\.pages\.dev/);
  assert.match(html, /FireCraft-CraftAnything-Experiment\/wiki/);
  assert.match(html, /extensions\.html/);
  assert.match(app, /api\.github\.com\/repos\/aliasfoxkde\/FireCraft-CraftAnything-Experiment\/releases\/latest/);
  assert.doesNotMatch(app, /aliasfoxkde\/FireCraft\/releases/);
});

test("extension catalogue describes current status without claiming an installed marketplace", async () => {
  const html = await readFile(new URL("../site/extensions.html", import.meta.url), "utf8");
  assert.match(html, /community extensions/i);
  assert.match(html, /marketplace is planned/i);
  assert.match(html, /FireCraft-CraftAnything-Experiment\/issues\/new\/choose/);
  assert.match(html, /docs\/research\/fireworks-extensions\.md/);
  assert.doesNotMatch(html, /\bInstall now\b/i);
});

test("theme toggle cycles through system, dark, and light modes", () => {
  assert.equal(nextTheme("system"), "dark");
  assert.equal(nextTheme("dark"), "light");
  assert.equal(nextTheme("light"), "system");
  assert.match(nextTheme("unknown"), /system/);
});

test("landing page exposes one accessible theme toggle and cursor-responsive hero", async () => {
  const html = await readFile(new URL("../site/index.html", import.meta.url), "utf8");
  const js = await readFile(new URL("../site/app.js", import.meta.url), "utf8");
  assert.match(html, /data-theme-toggle/);
  assert.match(js, /prefers-color-scheme/);
  assert.match(js, /pointermove/);
});

test("published pages reference existing local assets and sitemap routes", async () => {
  for (const page of ["index.html", "extensions.html"]) {
    const pageUrl = new URL(`../site/${page}`, import.meta.url);
    const html = await readFile(pageUrl, "utf8");
    for (const [, reference] of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
      if (/^(?:#|https?:|mailto:|data:|javascript:)/i.test(reference)) continue;
      const target = new URL(reference, pageUrl);
      await access(target);
    }
  }
  const sitemap = await readFile(new URL("../site/sitemap.xml", import.meta.url), "utf8");
  assert.match(sitemap, /firecraft-site\.pages\.dev\//);
  assert.match(sitemap, /firecraft-site\.pages\.dev\/extensions\.html/);
  assert.doesNotMatch(sitemap, /features\.html|downloads\.html|gallery\.html/);
});

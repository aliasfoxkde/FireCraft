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
  const robots = await readFile(new URL("../site/robots.txt", import.meta.url), "utf8");
  assert.match(sitemap, /marketing-review\.firecraft-web\.pages\.dev\//);
  assert.match(sitemap, /marketing-review\.firecraft-web\.pages\.dev\/extensions\.html/);
  assert.match(robots, /Sitemap: https:\/\/marketing-review\.firecraft-web\.pages\.dev\/sitemap\.xml/);
  assert.doesNotMatch(sitemap, /features\.html|downloads\.html|gallery\.html/);
});

test("main-branch deployment publishes a review preview without replacing the WebAssembly app", async () => {
  const workflow = await readFile(new URL("../.github/workflows/deploy-site.yml", import.meta.url), "utf8");
  const target = workflow.match(/command:\s*pages deploy site --project-name=([\w-]+) --branch=([\w-]+)/);
  assert.ok(target, "the Pages deploy target must be explicit");
  assert.deepEqual(target.slice(1), ["firecraft-web", "marketing-review"]);
});

function contrastRatio(foreground, background) {
  const luminance = (color) => {
    const channels = color.match(/[0-9a-f]{2}/gi).map((value) => parseInt(value, 16) / 255);
    const linear = channels.map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
    return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
  };
  const values = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
  return (values[0] + 0.05) / (values[1] + 0.05);
}

test("mobile extension navigation controls an existing nav and Light mode text is readable", async () => {
  const html = await readFile(new URL("../site/extensions.html", import.meta.url), "utf8");
  const styles = await readFile(new URL("../site/styles.css", import.meta.url), "utf8");
  const extensionStyles = await readFile(new URL("../site/extensions.css", import.meta.url), "utf8");
  const navId = html.match(/<nav[^>]*id="([^"]+)"/);
  assert.equal(navId?.[1], "primary-nav");
  assert.match(html, /aria-controls="primary-nav"/);

  const lightButton = styles.match(/html\[data-color-scheme="light"\] \.button-quiet\{color:(#[0-9a-f]{6})/i);
  const lightReference = extensionStyles.match(/html\[data-color-scheme="light"\] \.ext-reference\{color:(#[0-9a-f]{6})/i);
  assert.ok(lightButton, "quiet buttons need an explicit Light-mode text color");
  assert.ok(lightReference, "the dark reference panel needs explicit Light-mode text color");
  assert.ok(contrastRatio(lightButton[1], "#f6f3ee") >= 4.5);
  assert.ok(contrastRatio(lightReference[1], "#191b22") >= 4.5);
});

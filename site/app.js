const ALLOWED_DOWNLOAD_PREFIX = "https://github.com/aliasfoxkde/FireCraft/releases/download/";
const FALLBACK_URL = "https://github.com/aliasfoxkde/FireCraft/releases/latest";
const RELEASE_ENDPOINT = "https://api.github.com/repos/aliasfoxkde/FireCraft/releases/latest";

export function isTrustedDownloadUrl(url) {
  return typeof url === "string" && url.startsWith(ALLOWED_DOWNLOAD_PREFIX);
}

export function categorizeAsset(name) {
  const n = String(name || "").toLowerCase();
  if (n.endsWith(".exe") || n.endsWith(".msi") || n.includes("windows")) return "Windows";
  if (n.endsWith(".dmg") || n.includes("macos")) return "macOS";
  if (n.endsWith(".appimage") || n.endsWith(".deb") || n.endsWith(".flatpak") || n.endsWith(".rpm") || (n.includes("linux") && !n.includes("cli"))) return "Linux desktop";
  if (n.includes("cli")) return "Command-line tools";
  if (n.includes("firecraft-web") || n.includes("wasm")) return "Web / WASM";
  if (n.includes("source")) return "Source archive";
  if (n.includes("manifest") || n.includes("sha256") || n.includes("sbom") || n.includes("provenance") || n.endsWith(".asc")) return "Checksums & integrity";
  return "Release asset";
}

export function selectSafeAssets(release) {
  if (!release || !Array.isArray(release.assets)) return [];
  return release.assets.filter(
    (a) => a && a.state === "uploaded" && typeof a.name === "string" && isTrustedDownloadUrl(a.browser_download_url)
  );
}

function initSite() {
  if (typeof document === "undefined") return;
  const navToggle = document.querySelector(".nav-toggle");
  const nav = document.querySelector(".nav");
  navToggle?.addEventListener("click", () => {
    const open = navToggle.getAttribute("aria-expanded") !== "true";
    navToggle.setAttribute("aria-expanded", String(open));
    nav?.classList.toggle("open", open);
  });
  nav?.querySelectorAll("a").forEach((a) =>
    a.addEventListener("click", () => {
      nav.classList.remove("open");
      navToggle?.setAttribute("aria-expanded", "false");
    })
  );
  const yearNode = document.querySelector("#year");
  if (yearNode) yearNode.textContent = String(new Date().getFullYear());

  const releaseNode = document.querySelector("#release-version");
  const dateNode = document.querySelector("#release-date");
  const assetsNode = document.querySelector("#download-assets");
  if (!releaseNode || !dateNode || !assetsNode) return;

  async function loadRelease() {
    try {
      const response = await fetch(RELEASE_ENDPOINT, { headers: { Accept: "application/vnd.github+json" } });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const release = await response.json();
      const label = release.name || release.tag_name || "Latest release";
      releaseNode.textContent = release.prerelease ? `${label} (Prerelease)` : label;
      dateNode.textContent = release.published_at
        ? new Date(release.published_at).toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" })
        : "Published GitHub release";
      const safeAssets = selectSafeAssets(release);
      if (!safeAssets.length) throw new Error("No verified assets");
      assetsNode.replaceChildren();
      for (const asset of safeAssets) {
        const row = document.createElement("a");
        row.className = "asset-row";
        row.href = asset.browser_download_url;
        row.rel = "noreferrer";
        const span = document.createElement("span");
        const title = document.createElement("b");
        title.textContent = `${categorizeAsset(asset.name)} · ${asset.name}`;
        const meta = document.createElement("small");
        meta.textContent = `${asset.size ? (asset.size / 1048576).toFixed(1) + " MB · " : ""}${asset.download_count || 0} downloads`;
        span.append(title, meta);
        const action = document.createElement("strong");
        action.textContent = "Download ↗";
        row.append(span, action);
        assetsNode.append(row);
      }
    } catch {
      releaseNode.textContent = "Published release downloads";
      dateNode.textContent = "Open GitHub to see the latest verified assets.";
      assetsNode.replaceChildren();
      const row = document.createElement("a");
      row.className = "asset-row";
      row.href = FALLBACK_URL;
      const span = document.createElement("span");
      const title = document.createElement("b");
      title.textContent = "Browse GitHub releases";
      const meta = document.createElement("small");
      meta.textContent = "Release API unavailable — use the canonical release page";
      span.append(title, meta);
      const action = document.createElement("strong");
      action.textContent = "View releases ↗";
      row.append(span, action);
      assetsNode.append(row);
    }
  }
  loadRelease();
}

initSite();

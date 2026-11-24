/* FireCraft website — progressive enhancement.
   No framework, no tracker, no third-party request except the public GitHub
   release API (used only to list real published assets). Every feature is
   optional: the pages read correctly with JavaScript disabled. */

(() => {
  "use strict";

  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  /* ---------- navigation ---------- */

  const toggle = document.querySelector(".nav-toggle");
  const nav = document.querySelector("#primary-nav");

  if (toggle && nav) {
    const setOpen = (open) => {
      toggle.setAttribute("aria-expanded", String(open));
      nav.classList.toggle("open", open);
    };

    toggle.addEventListener("click", () => {
      setOpen(toggle.getAttribute("aria-expanded") !== "true");
    });

    nav.addEventListener("click", (event) => {
      if (event.target.closest("a")) setOpen(false);
    });

    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && toggle.getAttribute("aria-expanded") === "true") {
        setOpen(false);
        toggle.focus();
      }
    });
  }

  /* Mark the current page in the navigation. */
  const path = window.location.pathname.replace(/\/index\.html$/, "/");
  document.querySelectorAll(".nav a[href]").forEach((link) => {
    const href = link.getAttribute("href");
    if (!href || href.startsWith("http") || href.startsWith("#")) return;
    const target = new URL(href, window.location.origin).pathname.replace(/\/index\.html$/, "/");
    if (target === path) link.setAttribute("aria-current", "page");
  });

  /* ---------- footer year ---------- */

  const year = document.querySelector("#year");
  if (year) year.textContent = String(new Date().getFullYear());

  /* ---------- reveal on scroll ---------- */

  const revealables = document.querySelectorAll(".reveal");
  if (revealables.length && !reducedMotion && "IntersectionObserver" in window) {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          entry.target.classList.add("is-visible");
          observer.unobserve(entry.target);
        }
      },
      { rootMargin: "0px 0px -8% 0px", threshold: 0.08 },
    );
    revealables.forEach((node) => observer.observe(node));
  } else {
    revealables.forEach((node) => node.classList.add("is-visible"));
  }

  /* ---------- copy buttons ---------- */

  document.querySelectorAll("[data-copy]").forEach((button) => {
    button.addEventListener("click", async () => {
      const selector = button.getAttribute("data-copy");
      const source = selector ? document.querySelector(selector) : null;
      if (!source) return;

      const text = source.textContent.replace(/\s+$/u, "");
      try {
        await navigator.clipboard.writeText(text);
      } catch {
        /* Clipboard API can be blocked; fall back to a selection. */
        const range = document.createRange();
        range.selectNodeContents(source);
        const selection = window.getSelection();
        selection?.removeAllRanges();
        selection?.addRange(range);
        return;
      }
      button.dataset.copied = "true";
      const label = button.textContent;
      button.textContent = "Copied";
      window.setTimeout(() => {
        button.dataset.copied = "false";
        button.textContent = label;
      }, 1600);
    });
  });

  /* ---------- release panel ---------- */

  const RELEASE_API = "https://api.github.com/repos/aliasfoxkde/FireCraft/releases/latest";
  const RELEASES_PAGE = "https://github.com/aliasfoxkde/FireCraft/releases/latest";

  /* Only these hosts may appear as download destinations. */
  const DOWNLOAD_HOSTS = new Set([
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
  ]);

  const panel = document.querySelector("[data-release]");
  if (!panel || !("fetch" in window)) return;

  const versionNode = panel.querySelector("[data-release-version]");
  const dateNode = panel.querySelector("[data-release-date]");
  const statusNode = panel.querySelector("[data-release-status]");
  const assetsNode = panel.querySelector("[data-release-assets]");

  const safeDownloadUrl = (raw) => {
    if (typeof raw !== "string") return null;
    try {
      const url = new URL(raw);
      if (url.protocol !== "https:") return null;
      if (!DOWNLOAD_HOSTS.has(url.hostname)) return null;
      return url.href;
    } catch {
      return null;
    }
  };

  const assetLabel = (name) => {
    const value = name.toLowerCase();
    if (value.endsWith(".deb")) return "Debian package";
    if (value.endsWith(".tar.gz") || value.endsWith(".tgz")) return "Archive";
    if (value.endsWith(".zip")) return "Archive";
    if (value.endsWith(".wasm")) return "WebAssembly";
    if (value.endsWith(".sha256") || value.includes("checksum")) return "Checksum";
    if (value.includes("manifest")) return "Manifest";
    if (value.includes("cli")) return "Command line";
    if (value.includes("web")) return "Web bundle";
    if (value.includes("linux")) return "Linux build";
    return "Release asset";
  };

  const formatSize = (bytes) => {
    if (typeof bytes !== "number" || !Number.isFinite(bytes) || bytes <= 0) return null;
    const megabytes = bytes / 1048576;
    if (megabytes >= 1) return `${megabytes.toFixed(1)} MB`;
    return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  };

  const renderFallback = (message) => {
    if (versionNode) versionNode.textContent = "Published release downloads";
    if (dateNode) dateNode.textContent = message;
    if (statusNode) {
      statusNode.dataset.state = "error";
      statusNode.textContent = "Release page available";
    }
    if (!assetsNode) return;
    assetsNode.replaceChildren();
    const row = document.createElement("a");
    row.className = "asset";
    row.href = RELEASES_PAGE;
    row.rel = "noreferrer";

    const label = document.createElement("span");
    const title = document.createElement("b");
    title.textContent = "Browse every published asset";
    const meta = document.createElement("small");
    meta.textContent = "Canonical GitHub Releases page";
    label.append(title, meta);

    const tag = document.createElement("span");
    tag.className = "tag";
    tag.textContent = "GitHub ↗";
    row.append(label, tag);
    assetsNode.append(row);
  };

  const renderRelease = (release) => {
    const tag = typeof release.tag_name === "string" ? release.tag_name : "";
    const title = typeof release.name === "string" && release.name.trim() ? release.name : tag;

    if (versionNode) versionNode.textContent = title || "Latest published release";
    if (dateNode) {
      const published = typeof release.published_at === "string" ? new Date(release.published_at) : null;
      const valid = published && !Number.isNaN(published.valueOf());
      dateNode.textContent = valid
        ? `Published ${published.toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" })}`
        : "Published on GitHub";
    }
    if (statusNode) {
      statusNode.dataset.state = "ok";
      statusNode.textContent = release.prerelease ? "Pre-release" : "Stable release";
    }

    const assets = Array.isArray(release.assets) ? release.assets : [];
    const usable = assets
      .map((asset) => {
        const url = safeDownloadUrl(asset?.browser_download_url);
        const name = typeof asset?.name === "string" ? asset.name : "";
        if (!url || !name) return null;
        return { url, name, size: asset.size, downloads: asset.download_count };
      })
      .filter(Boolean);

    if (!assetsNode) return;
    assetsNode.replaceChildren();

    if (usable.length === 0) {
      renderFallback("This release has no downloadable assets yet.");
      return;
    }

    /* Show the checksums and manifests first, then platform artifacts. */
    const ordered = usable.sort((a, b) => {
      const rank = (asset) => (/sha256|manifest/i.test(asset.name) ? 0 : 1);
      return rank(a) - rank(b) || a.name.localeCompare(b.name);
    });

    for (const asset of ordered.slice(0, 8)) {
      const row = document.createElement("a");
      row.className = "asset";
      row.href = asset.url;
      row.rel = "noreferrer";

      const label = document.createElement("span");
      const name = document.createElement("b");
      name.textContent = asset.name;
      const meta = document.createElement("small");
      const parts = [assetLabel(asset.name)];
      const size = formatSize(asset.size);
      if (size) parts.push(size);
      if (typeof asset.downloads === "number" && asset.downloads > 0) {
        parts.push(`${asset.downloads.toLocaleString()} downloads`);
      }
      meta.textContent = parts.join(" · ");
      label.append(name, meta);

      const tag = document.createElement("span");
      tag.className = "tag";
      tag.textContent = "Download ↗";
      row.append(label, tag);
      assetsNode.append(row);
    }
  };

  const loadRelease = async () => {
    try {
      const response = await fetch(RELEASE_API, {
        headers: { Accept: "application/vnd.github+json" },
        referrerPolicy: "no-referrer",
      });
      if (!response.ok) throw new Error(`Release lookup returned ${response.status}`);
      const release = await response.json();
      if (!release || typeof release !== "object") throw new Error("Malformed release payload");
      renderRelease(release);
    } catch {
      renderFallback("The release service is unavailable right now.");
    }
  };

  loadRelease();
})();

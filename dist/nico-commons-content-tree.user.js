// ==UserScript==
// @name         Nico Commons ContentTree - Copy TSV
// @namespace    https://ngtkana.local/
// @version      1.0.0
// @description  Nico Commons content tree children -> filter utaite covers -> copy TSV
// @match        https://commons.nicovideo.jp/works/*/tree/children*
// @grant        none
// @downloadURL  https://raw.githubusercontent.com/ngtkana/tampermonkey-scripts/main/dist/nico-commons-content-tree.user.js
// @updateURL    https://raw.githubusercontent.com/ngtkana/tampermonkey-scripts/main/dist/nico-commons-content-tree.user.js
// ==/UserScript==

(() => {
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __esm = (fn, res) => function __init() {
    return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
  };
  var __commonJS = (cb, mod) => function __require() {
    return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
  };

  // src/nico-commons-content-tree/core.js
  function sleep(ms) {
    return new Promise((r) => setTimeout(r, ms));
  }
  function looksLikeUtaMita(title, titleKeywords = TITLE_KEYWORDS_DEFAULT) {
    return titleKeywords.some((k) => String(title ?? "").includes(k));
  }
  function normalizeTitle(title) {
    return String(title ?? "").trim();
  }
  function isVideoSm(globalId) {
    return typeof globalId === "string" && /^sm\d+$/.test(globalId);
  }
  function buildChildrenApiUrl(rootId, { offset = 0, limit = DEFAULT_LIMIT } = {}) {
    return `https://public-api.commons.nicovideo.jp/v1/tree/${encodeURIComponent(rootId)}/relatives/children?_offset=${offset}&_limit=${limit}&with_meta=1&_sort=-id&only_mine=0`;
  }
  function buildAccountUsersApiUrl(userIds) {
    const qs = userIds.map((id) => `userIds=${encodeURIComponent(id)}`).join("&");
    return `https://account.nicovideo.jp/api/public/v1/users.json?${qs}`;
  }
  async function fetchJson(fetchImpl, url) {
    const res = await fetchImpl(url, { credentials: "omit" });
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${url}`);
    return await res.json();
  }
  async function fetchAllChildren(fetchImpl, rootId, { limit = DEFAULT_LIMIT, delayMs = 80, onProgress } = {}) {
    let offset = 0;
    let total = Infinity;
    const all = [];
    if (typeof onProgress === "function") {
      onProgress({ phase: "children", fetched: 0, total: null, offset: 0, lastBatchSize: 0 });
    }
    while (offset < total) {
      const url = buildChildrenApiUrl(rootId, { offset, limit });
      const j = await fetchJson(fetchImpl, url);
      const children = j?.data?.children;
      const contents = children?.contents ?? [];
      total = Number(children?.total ?? contents.length);
      all.push(...contents);
      offset += contents.length;
      if (typeof onProgress === "function") {
        const totalFinite = Number.isFinite(total) ? total : null;
        onProgress({
          phase: "children",
          fetched: all.length,
          total: totalFinite,
          offset,
          lastBatchSize: contents.length
        });
      }
      if (contents.length === 0) break;
      await sleep(delayMs);
    }
    return all;
  }
  function parseAccountUsersResponse(json) {
    const users = json?.data?.users ?? json?.users ?? json?.data ?? [];
    const map = /* @__PURE__ */ new Map();
    if (!Array.isArray(users)) return map;
    for (const u of users) {
      const id = u?.id ?? u?.userId ?? u?.user_id;
      const name = u?.nickname ?? u?.nickName ?? u?.name ?? u?.userName ?? u?.username;
      if (id != null && name) map.set(Number(id), String(name));
    }
    return map;
  }
  async function fetchUserMap(fetchImpl, userIds, { chunkSize = 80, delayMs = 60, onProgress } = {}) {
    const uniq = Array.from(new Set(userIds.map(Number).filter((x) => Number.isFinite(x))));
    const map = /* @__PURE__ */ new Map();
    const total = uniq.length;
    if (typeof onProgress === "function") onProgress({ phase: "users", done: 0, total, chunkSize });
    for (let i = 0; i < uniq.length; i += chunkSize) {
      const chunk = uniq.slice(i, i + chunkSize);
      const url = buildAccountUsersApiUrl(chunk);
      const j = await fetchJson(fetchImpl, url);
      const partial = parseAccountUsersResponse(j);
      for (const [k, v] of partial.entries()) map.set(k, v);
      if (typeof onProgress === "function") {
        const done = Math.min(i + chunk.length, total);
        onProgress({ phase: "users", done, total, chunkSize });
      }
      await sleep(delayMs);
    }
    return map;
  }
  function extractCandidates(children, { titleKeywords = TITLE_KEYWORDS_DEFAULT } = {}) {
    return children.map((c) => {
      const title = normalizeTitle(c?.title ?? "");
      const userId = Number(c?.userId);
      const url = c?.watchURL ?? "";
      const id = c?.globalId ?? "";
      if (!isVideoSm(id)) return null;
      if (!title) return null;
      if (!looksLikeUtaMita(title, titleKeywords)) return null;
      if (!url) return null;
      return { title, userId, url };
    }).filter(Boolean);
  }
  function buildTsv(candidates, userMap) {
    const lines = candidates.map((x) => {
      const owner = userMap.get(x.userId) ?? String(x.userId ?? "");
      return [x.title, owner, x.url].join("	");
    });
    return lines.join("\n");
  }
  var TITLE_KEYWORDS_DEFAULT, DEFAULT_LIMIT;
  var init_core = __esm({
    "src/nico-commons-content-tree/core.js"() {
      TITLE_KEYWORDS_DEFAULT = ["\u6B4C\u3063\u3066", "\u6B4C\u308F\u305B\u3066\u3044\u305F\u3060\u304D", "\u6B4C\u3044\u307E\u3057\u305F"];
      DEFAULT_LIMIT = 50;
    }
  });

  // src/nico-commons-content-tree/index.js
  var require_index = __commonJS({
    "src/nico-commons-content-tree/index.js"() {
      init_core();
      var TITLE_KEYWORDS = ["\u6B4C\u3063\u3066", "\u6B4C\u308F\u305B\u3066\u3044\u305F\u3060\u304D", "\u6B4C\u3044\u307E\u3057\u305F"];
      function pickRootId() {
        const m = location.pathname.match(/\/works\/([^/]+)\/tree\/children/);
        return m?.[1] ?? null;
      }
      async function copyToClipboard(text) {
        try {
          await navigator.clipboard.writeText(text);
          return true;
        } catch {
          const ta = document.createElement("textarea");
          ta.value = text;
          ta.style.position = "fixed";
          ta.style.top = "-9999px";
          document.body.appendChild(ta);
          ta.focus();
          ta.select();
          const ok = document.execCommand("copy");
          ta.remove();
          return ok;
        }
      }
      function findInsertPoint() {
        return document.querySelector("section.p-treeWorks .l-outsideHeading") ?? document.querySelector("section.p-treeWorks");
      }
      function makeUi() {
        const wrap = document.createElement("span");
        wrap.dataset.ngtkanaCopytsv = "wrap";
        wrap.style.display = "inline-flex";
        wrap.style.alignItems = "center";
        wrap.style.gap = "8px";
        wrap.style.marginLeft = "10px";
        const btn = document.createElement("button");
        btn.type = "button";
        btn.textContent = "Copy";
        btn.title = "Copy TSV to clipboard";
        btn.setAttribute("aria-label", "Copy TSV to clipboard");
        btn.dataset.ngtkanaCopytsv = "btn";
        btn.style.padding = "2px 8px";
        btn.style.borderRadius = "999px";
        btn.style.border = "1px solid rgba(0, 0, 0, 0.25)";
        btn.style.background = "white";
        btn.style.cursor = "pointer";
        btn.style.fontSize = "12px";
        btn.style.fontWeight = "700";
        const badge = document.createElement("span");
        badge.dataset.ngtkanaCopytsv = "badge";
        badge.textContent = "Candidates: ?";
        badge.style.padding = "2px 8px";
        badge.style.borderRadius = "999px";
        badge.style.border = "1px solid rgba(0, 0, 0, 0.12)";
        badge.style.background = "rgba(0, 0, 0, 0.04)";
        badge.style.fontSize = "12px";
        const status = document.createElement("span");
        status.dataset.ngtkanaCopytsv = "status";
        status.textContent = "";
        status.style.padding = "2px 6px";
        status.style.borderRadius = "999px";
        status.style.border = "1px solid rgba(0, 0, 0, 0.08)";
        status.style.background = "rgba(0, 0, 0, 0.02)";
        status.style.fontSize = "12px";
        status.style.color = "rgba(0, 0, 0, 0.7)";
        status.style.display = "none";
        wrap.appendChild(btn);
        wrap.appendChild(badge);
        wrap.appendChild(status);
        return { wrap, btn, badge, status };
      }
      function fmtProgress(current, total) {
        if (!Number.isFinite(total)) return `${current}/?`;
        return `${current}/${total}`;
      }
      async function run() {
        const rootId = pickRootId();
        if (!rootId) return;
        if (document.querySelector('[data-ngtkana-copytsv="wrap"]')) return;
        const insertPoint = findInsertPoint();
        if (!insertPoint) return;
        const { wrap, btn, badge, status } = makeUi();
        insertPoint.appendChild(wrap);
        btn.addEventListener("click", async () => {
          btn.disabled = true;
          btn.textContent = "Copying...";
          badge.textContent = "Starting...";
          status.style.display = "none";
          status.textContent = "";
          try {
            const children = await fetchAllChildren(fetch, rootId, {
              onProgress: ({ phase, fetched, total }) => {
                if (phase !== "children") return;
                badge.textContent = `Children: ${fmtProgress(fetched, total)}`;
              }
            });
            const candidates = extractCandidates(children, { titleKeywords: TITLE_KEYWORDS });
            badge.textContent = `Candidates: ${candidates.length}`;
            const userMap = await fetchUserMap(
              fetch,
              candidates.map((x) => x.userId),
              {
                onProgress: ({ phase, done, total }) => {
                  if (phase !== "users") return;
                  badge.textContent = `Users: ${fmtProgress(done, total)}`;
                }
              }
            );
            const tsv = buildTsv(candidates, userMap);
            await copyToClipboard(tsv);
            badge.textContent = `Candidates: ${candidates.length}`;
            btn.textContent = "Copied";
            status.style.display = "inline-block";
            status.textContent = `Copied ${candidates.length} lines`;
            status.style.borderColor = "rgba(0, 128, 0, 0.25)";
            status.style.background = "rgba(0, 128, 0, 0.06)";
            status.style.color = "rgba(0, 100, 0, 0.9)";
            await sleep(700);
          } catch (e) {
            console.error(e);
            btn.textContent = "Error";
            badge.textContent = "Candidates: ?";
            status.style.display = "inline-block";
            status.textContent = `Failed: ${e?.message ?? e}`;
            status.style.borderColor = "rgba(200, 0, 0, 0.25)";
            status.style.background = "rgba(200, 0, 0, 0.06)";
            status.style.color = "rgba(160, 0, 0, 0.9)";
            alert(`Copy TSV failed: ${e?.message ?? e}`);
          } finally {
            btn.disabled = false;
            btn.textContent = "Copy";
          }
        });
      }
      var tries = 0;
      var timer = setInterval(() => {
        tries++;
        run().catch(console.error);
        if (document.querySelector('[data-ngtkana-copytsv="wrap"]') || tries > 40) {
          clearInterval(timer);
        }
      }, 300);
    }
  });
  require_index();
})();

// ==UserScript==
// @name         Nico Watch - Children TSV
// @namespace    https://ngtkana.local/
// @version      2.0.0
// @description  On nicovideo watch pages, fetch all child contents from the Commons tree API, filter likely utaite covers, then copy/download TSV.
// @match        https://www.nicovideo.jp/watch/*
// @grant        none
// ==/UserScript==

(() => {
  "use strict";

  const STORAGE_KEY_UI_OPEN = "ngtkana.children_tsv.ui_open";

  const TITLE_KEYWORDS = [
    "歌って",
    "歌わせていただき",
    "歌いました",
  ];

  const DEFAULT_CHILDREN_LIMIT = 100;
  const DEFAULT_CHILDREN_DELAY_MS = 80;
  const DEFAULT_USERS_CHUNK_SIZE = 80;
  const DEFAULT_USERS_DELAY_MS = 60;

  function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  function pickRootIdFromWatch() {
    const m = location.pathname.match(/^\/watch\/(sm\d+)$/);
    return m?.[1] ?? null;
  }

  function normalizeTitle(title) {
    return String(title ?? "").trim();
  }

  function looksLikeUtaMita(title, titleKeywords = TITLE_KEYWORDS) {
    const s = String(title ?? "");
    return titleKeywords.some((k) => s.includes(k));
  }

  function isVideoSm(globalId) {
    return typeof globalId === "string" && /^sm\d+$/.test(globalId);
  }

  function buildChildrenApiUrl(rootId, { offset = 0, limit = DEFAULT_CHILDREN_LIMIT } = {}) {
    return `https://public-api.commons.nicovideo.jp/v1/tree/${encodeURIComponent(rootId)}/relatives/children?_offset=${offset}&_limit=${limit}&with_meta=1&_sort=-id&only_mine=0`;
  }

  function buildAccountUsersApiUrl(userIds) {
    const qs = userIds.map((id) => `userIds=${encodeURIComponent(id)}`).join("&");
    return `https://account.nicovideo.jp/api/public/v1/users.json?${qs}`;
  }

  async function fetchJson(url) {
    const res = await fetch(url, {
      credentials: "omit",
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${url}`);
    }
    return await res.json();
  }

  async function fetchAllChildren(rootId, {
    limit = DEFAULT_CHILDREN_LIMIT,
    delayMs = DEFAULT_CHILDREN_DELAY_MS,
    onProgress,
  } = {}) {
    let offset = 0;
    let total = Infinity;
    const all = [];

    if (typeof onProgress === "function") {
      onProgress({
        phase: "children",
        fetched: 0,
        total: null,
        offset: 0,
        lastBatchSize: 0,
      });
    }

    while (offset < total) {
      const url = buildChildrenApiUrl(rootId, { offset, limit });
      const json = await fetchJson(url);
      const children = json?.data?.children;
      const contents = children?.contents ?? [];

      total = Number(children?.total ?? contents.length);

      all.push(...contents);
      offset += contents.length;

      if (typeof onProgress === "function") {
        onProgress({
          phase: "children",
          fetched: all.length,
          total: Number.isFinite(total) ? total : null,
          offset,
          lastBatchSize: contents.length,
        });
      }

      if (contents.length === 0) {
        break;
      }

      await sleep(delayMs);
    }

    return all;
  }

  function parseAccountUsersResponse(json) {
    const users = json?.data?.users ?? json?.users ?? json?.data ?? [];
    const map = new Map();

    if (!Array.isArray(users)) return map;

    for (const user of users) {
      const id = user?.id ?? user?.userId ?? user?.user_id;
      const name =
        user?.nickname ??
        user?.nickName ??
        user?.name ??
        user?.userName ??
        user?.username;

      if (id != null && name) {
        map.set(Number(id), String(name));
      }
    }

    return map;
  }

  async function fetchUserMap(userIds, {
    chunkSize = DEFAULT_USERS_CHUNK_SIZE,
    delayMs = DEFAULT_USERS_DELAY_MS,
    onProgress,
  } = {}) {
    const uniq = Array.from(
      new Set(
        userIds
          .map(Number)
          .filter((x) => Number.isFinite(x))
      )
    );

    const map = new Map();
    const total = uniq.length;

    if (typeof onProgress === "function") {
      onProgress({
        phase: "users",
        done: 0,
        total,
        chunkSize,
      });
    }

    for (let i = 0; i < uniq.length; i += chunkSize) {
      const chunk = uniq.slice(i, i + chunkSize);
      const url = buildAccountUsersApiUrl(chunk);
      const json = await fetchJson(url);
      const partial = parseAccountUsersResponse(json);

      for (const [k, v] of partial.entries()) {
        map.set(k, v);
      }

      if (typeof onProgress === "function") {
        onProgress({
          phase: "users",
          done: Math.min(i + chunk.length, total),
          total,
          chunkSize,
        });
      }

      await sleep(delayMs);
    }

    return map;
  }

  function extractCandidates(children, { titleKeywords = TITLE_KEYWORDS } = {}) {
    return children
      .map((c) => {
        const title = normalizeTitle(c?.title ?? "");
        const userId = Number(c?.userId);
        const url = c?.watchURL ?? "";
        const id = c?.globalId ?? "";

        if (!isVideoSm(id)) return null;
        if (!title) return null;
        if (!looksLikeUtaMita(title, titleKeywords)) return null;
        if (!url) return null;

        return { title, userId, url };
      })
      .filter(Boolean);
  }

  function buildTsv(candidates, userMap) {
    const header = ["タイトル", "投稿者", "URL"].join("\t");
    const lines = candidates.map((x) => {
      const owner = userMap.get(x.userId) ?? String(x.userId ?? "");
      return [x.title, owner, x.url].join("\t");
    });
    return [header, ...lines].join("\n");
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
      ta.style.left = "-9999px";
      document.body.appendChild(ta);
      ta.focus();
      ta.select();
      const ok = document.execCommand("copy");
      ta.remove();
      return ok;
    }
  }

  function downloadText(filename, text, mime = "text/tab-separated-values;charset=utf-8") {
    const blob = new Blob([text], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  function fmtProgress(current, total) {
    if (!Number.isFinite(total)) return `${current}/?`;
    return `${current}/${total}`;
  }

  function fmtStatusLine(s) {
    if (!s) return "";
    if (s.phase === "children") {
      return `Children: ${fmtProgress(s.fetched, s.total)} (+${s.lastBatchSize})`;
    }
    if (s.phase === "users") {
      return `Users: ${fmtProgress(s.done, s.total)}`;
    }
    return "";
  }

  function isDarkTheme() {
    try {
      return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
    } catch {
      return false;
    }
  }

  function makeButtonFactory({ dark }) {
    return function makeButton(label) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.textContent = label;

      const border = dark ? "rgba(255,255,255,0.14)" : "rgba(0,0,0,0.12)";
      const bg = dark ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.04)";
      const bgHover = dark ? "rgba(255,255,255,0.14)" : "rgba(0,0,0,0.07)";
      const fg = dark ? "#fff" : "inherit";

      Object.assign(btn.style, {
        appearance: "none",
        border: `1px solid ${border}`,
        background: bg,
        color: fg,
        padding: "8px 10px",
        borderRadius: "10px",
        cursor: "pointer",
        fontSize: "12px",
        fontWeight: "700",
        lineHeight: "1.2",
      });
      btn.addEventListener("mouseenter", () => {
        btn.style.background = bgHover;
      });
      btn.addEventListener("mouseleave", () => {
        btn.style.background = bg;
      });
      return btn;
    };
  }

  function findEmbeddedMount() {
    // 「この動画の親作品・子作品」セクション内に埋め込む。
    // SPAで遅延描画されるので、テキストとリンクの両方で安定的に特定する。
    const section = Array.from(document.querySelectorAll("section")).find((sec) => {
      const h1 = sec.querySelector("h1");
      return h1 && String(h1.textContent || "").includes("この動画の親作品・子作品");
    });
    if (!section) return null;

    const commonsLink = section.querySelector('a[href*="commons.nicovideo.jp/works/"]');
    if (!commonsLink) return null;

    const headerRow = commonsLink.parentElement;
    if (!headerRow) return null;

    return { section, commonsLink, headerRow };
  }

  function makeEmbeddedUi(mount) {
    const dark = isDarkTheme();
    const makeButton = makeButtonFactory({ dark });

    const { section, commonsLink, headerRow } = mount;

    // 右側（commonsLinkの場所）をまとめるラッパーに差し替え
    const rightWrap = document.createElement("div");
    rightWrap.dataset.ngtkanaChildrenTsv = "headerRight";
    Object.assign(rightWrap.style, {
      display: "flex",
      alignItems: "center",
      gap: "8px",
      flexWrap: "wrap",
    });
    headerRow.insertBefore(rightWrap, commonsLink);
    rightWrap.appendChild(commonsLink);

    const toggleBtn = makeButton("Children TSV");
    Object.assign(toggleBtn.style, {
      borderRadius: "999px",
      padding: "6px 10px",
      fontSize: "12px",
    });

    const miniStatus = document.createElement("span");
    miniStatus.dataset.ngtkanaChildrenTsv = "miniStatus";
    miniStatus.textContent = "";
    Object.assign(miniStatus.style, {
      fontSize: "12px",
      opacity: "0.7",
      lineHeight: "1.2",
      whiteSpace: "nowrap",
    });

    rightWrap.appendChild(toggleBtn);
    rightWrap.appendChild(miniStatus);

    const wrap = document.createElement("div");
    wrap.dataset.ngtkanaChildrenTsv = "wrap";
    Object.assign(wrap.style, {
      marginTop: "12px",
      display: "flex",
      flexDirection: "column",
      gap: "8px",
      width: "100%",
      maxWidth: "100%",
      padding: "10px",
      borderRadius: "10px",
      border: `1px solid ${dark ? "rgba(255,255,255,0.14)" : "rgba(0,0,0,0.12)"}`,
      background: dark ? "rgba(0, 0, 0, 0.28)" : "rgba(0, 0, 0, 0.03)",
      color: "inherit",
      fontFamily: "system-ui, sans-serif",
    });

    const status = document.createElement("div");
    status.dataset.ngtkanaChildrenTsv = "status";
    status.textContent = "Ready";
    Object.assign(status.style, {
      fontSize: "12px",
      lineHeight: "1.4",
      padding: "8px 10px",
      borderRadius: "8px",
      background: dark ? "rgba(255,255,255,0.06)" : "rgba(0,0,0,0.04)",
      whiteSpace: "pre-wrap",
      wordBreak: "break-word",
    });

    const progressOuter = document.createElement("div");
    Object.assign(progressOuter.style, {
      width: "100%",
      height: "8px",
      borderRadius: "999px",
      overflow: "hidden",
      background: dark ? "rgba(255,255,255,0.10)" : "rgba(0,0,0,0.08)",
    });

    const progressInner = document.createElement("div");
    progressInner.dataset.ngtkanaChildrenTsv = "progress";
    Object.assign(progressInner.style, {
      width: "0%",
      height: "100%",
      background: "linear-gradient(90deg, #4ea1ff 0%, #67e8f9 100%)",
      transition: "width 160ms ease",
    });
    progressOuter.appendChild(progressInner);

    const row = document.createElement("div");
    Object.assign(row.style, {
      display: "flex",
      gap: "8px",
      flexWrap: "wrap",
    });

    const copyBtn = makeButton("Copy TSV");
    const downloadBtn = makeButton("Download TSV");
    const closeBtn = makeButton("折りたたむ");
    Object.assign(closeBtn.style, {
      marginLeft: "auto",
    });

    row.appendChild(copyBtn);
    row.appendChild(downloadBtn);
    row.appendChild(closeBtn);

    wrap.appendChild(status);
    wrap.appendChild(progressOuter);
    wrap.appendChild(row);

    // セクション内に差し込み
    section.appendChild(wrap);

    const getOpen = () => {
      try {
        return localStorage.getItem(STORAGE_KEY_UI_OPEN) === "1";
      } catch {
        return false;
      }
    };

    const setOpen = (open) => {
      wrap.style.display = open ? "flex" : "none";
      try {
        localStorage.setItem(STORAGE_KEY_UI_OPEN, open ? "1" : "0");
      } catch {
        // ignore
      }
    };

    const initialOpen = getOpen();
    setOpen(initialOpen);

    toggleBtn.addEventListener("click", () => {
      const next = wrap.style.display === "none";
      setOpen(next);
    });
    closeBtn.addEventListener("click", () => {
      setOpen(false);
    });

    return {
      mode: "embedded",
      wrap,
      status,
      miniStatus,
      progressInner,
      copyBtn,
      downloadBtn,
      closeBtn,
      setOpen,
    };
  }

  function setUiBusy(ui, busy) {
    ui.copyBtn.disabled = busy;
    ui.downloadBtn.disabled = busy;
    ui.copyBtn.style.opacity = busy ? "0.6" : "1";
    ui.downloadBtn.style.opacity = busy ? "0.6" : "1";
    ui.copyBtn.style.cursor = busy ? "default" : "pointer";
    ui.downloadBtn.style.cursor = busy ? "default" : "pointer";
  }

  function updateProgressBar(ui, phaseState) {
    let ratio = 0;

    if (phaseState?.phase === "children" && Number.isFinite(phaseState.total) && phaseState.total > 0) {
      ratio = Math.max(0, Math.min(1, phaseState.fetched / phaseState.total));
    } else if (phaseState?.phase === "users" && Number.isFinite(phaseState.total) && phaseState.total > 0) {
      ratio = Math.max(0, Math.min(1, phaseState.done / phaseState.total));
    }

    ui.progressInner.style.width = `${ratio * 100}%`;
  }

  async function buildTsvForRootId(rootId, ui) {
    let lastPhaseState = null;

    const setPhase = (s) => {
      lastPhaseState = s;
      const line = fmtStatusLine(s);
      ui.status.textContent = line;
      if (ui.miniStatus) ui.miniStatus.textContent = line;
      updateProgressBar(ui, s);
    };

    setPhase({
      phase: "children",
      fetched: 0,
      total: null,
      offset: 0,
      lastBatchSize: 0,
    });

    const children = await fetchAllChildren(rootId, {
      limit: DEFAULT_CHILDREN_LIMIT,
      delayMs: DEFAULT_CHILDREN_DELAY_MS,
      onProgress: setPhase,
    });

    const candidates = extractCandidates(children, {
      titleKeywords: TITLE_KEYWORDS,
    });

    ui.status.textContent =
      `Children done: ${children.length}\n` +
      `Candidates: ${candidates.length}`;
    if (ui.miniStatus) ui.miniStatus.textContent = `Candidates: ${candidates.length}`;

    updateProgressBar(ui, {
      phase: "children",
      fetched: Number(children.length),
      total: Number(children.length || 1),
    });

    const userMap = await fetchUserMap(
      candidates.map((x) => x.userId),
      {
        chunkSize: DEFAULT_USERS_CHUNK_SIZE,
        delayMs: DEFAULT_USERS_DELAY_MS,
        onProgress: setPhase,
      }
    );

    const tsv = buildTsv(candidates, userMap);
    const filename = `${rootId}_children.tsv`;

    return {
      tsv,
      filename,
      childrenCount: children.length,
      candidatesCount: candidates.length,
      userCount: userMap.size,
      lastPhaseState,
    };
  }

  async function runAction(kind, ui) {
    const rootId = pickRootIdFromWatch();
    if (!rootId) {
      ui.status.textContent = "watch/sm... のページでのみ動作します。";
      return;
    }

    setUiBusy(ui, true);
    ui.progressInner.style.width = "0%";
    ui.status.textContent = `Starting: ${rootId}`;
    if (ui.miniStatus) ui.miniStatus.textContent = "Starting...";

    try {
      const result = await buildTsvForRootId(rootId, ui);

      if (kind === "copy") {
        await copyToClipboard(result.tsv);
        ui.status.textContent =
          `Copied: ${result.candidatesCount} lines\n` +
          `children=${result.childrenCount}, users=${result.userCount}`;
        if (ui.miniStatus) ui.miniStatus.textContent = `Copied: ${result.candidatesCount}`;
      } else if (kind === "download") {
        downloadText(result.filename, result.tsv);
        ui.status.textContent =
          `Downloaded: ${result.filename}\n` +
          `children=${result.childrenCount}, users=${result.userCount}, candidates=${result.candidatesCount}`;
        if (ui.miniStatus) ui.miniStatus.textContent = `Downloaded: ${result.filename}`;
      }

      ui.progressInner.style.width = "100%";
    } catch (e) {
      console.error(e);
      ui.status.textContent = `Failed: ${e?.message ?? e}`;
      if (ui.miniStatus) ui.miniStatus.textContent = "Failed";
      ui.progressInner.style.width = "0%";
      alert(`Children TSV failed: ${e?.message ?? e}`);
    } finally {
      setUiBusy(ui, false);
    }
  }

  function boot() {
    if (document.querySelector('[data-ngtkana-children-tsv="wrap"]')) return;

    const rootId = pickRootIdFromWatch();
    if (!rootId) return;

    const mount = findEmbeddedMount();
    if (!mount) return;

    const ui = makeEmbeddedUi(mount);
    ui.wrap.dataset.ngtkanaChildrenTsv = "wrap";
    ui.status.textContent = `Ready: ${rootId}`;
    if (ui.miniStatus) ui.miniStatus.textContent = `Ready: ${rootId}`;

    ui.copyBtn.addEventListener("click", () => {
      runAction("copy", ui);
    });

    ui.downloadBtn.addEventListener("click", () => {
      runAction("download", ui);
    });
  }

  let tries = 0;
  const timer = setInterval(() => {
    tries += 1;
    try {
      boot();
    } catch (e) {
      console.error(e);
    }
    // 埋め込み先が遅延描画されることがあるので、しばらく待つ。
    // フローティングへのフォールバックは行わない。
    if (document.querySelector('[data-ngtkana-children-tsv="wrap"]') || tries > 400) {
      clearInterval(timer);
      if (!document.querySelector('[data-ngtkana-children-tsv="wrap"]')) {
        console.warn("[Children TSV] mount point not found; UI was not inserted.");
      }
    }
  }, 300);
})();
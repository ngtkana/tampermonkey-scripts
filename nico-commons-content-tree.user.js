// ==UserScript==
// @name         Nico Watch - Children TSV
// @namespace    https://ngtkana.local/
// @version      3.1.0
// @description  On nicovideo watch pages, fetch all child contents from the Commons tree API, filter likely utaite covers, then copy/download TSV.
// @match        https://www.nicovideo.jp/watch/*
// @grant        GM_xmlhttpRequest
// @downloadURL  https://raw.githubusercontent.com/ngtkana/tampermonkey-scripts/main/nico-commons-content-tree.user.js
// @updateURL    https://raw.githubusercontent.com/ngtkana/tampermonkey-scripts/main/nico-commons-content-tree.user.js
// ==/UserScript==

(() => {
  "use strict";

  const STORAGE_KEY_UI_OPEN = "ngtkana.children_tsv.ui_open";

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
    // NFKC 正規化 + 小文字化（ML モデル用）
    return String(title ?? "").normalize("NFKC").toLowerCase();
  }

  // ============ ML Model Inference ============

  let MODEL = null; // グローバル MODEL（非同期で読み込み）

  function loadModel() {
    if (MODEL) return; // 既に読み込まれている

    // GM_xmlhttpRequest で model.js を fetch（プルーニング版）
    GM_xmlhttpRequest({
      method: "GET",
      url: "https://raw.githubusercontent.com/ngtkana/tampermonkey-scripts/main/nico-commons-ngram/annotate/model.js",
      onload: (response) => {
        try {
          if (response.status === 200) {
            // model.js は "const MODEL = {...};" という形式なので、eval で実行
            eval(response.responseText);
            console.log("[ML] Model loaded successfully");
            console.log(`[ML Model] vocab size: ${Object.keys(MODEL.vocab).length}, weights length: ${MODEL.weights.length}`);
          } else {
            throw new Error(`HTTP ${response.status}`);
          }
        } catch (err) {
          console.error("[ML] Failed to load model:", err.message);
          MODEL = { bias: 0, n_min: 3, n_max: 5, vocab: {}, weights: [] };
        }
      },
      onerror: (err) => {
        console.error("[ML] Failed to fetch model:", err);
        MODEL = { bias: 0, n_min: 3, n_max: 5, vocab: {}, weights: [] };
      },
    });
  }

  function extractNgrams(text, nMin, nMax) {
    const ngrams = new Set();
    for (let n = nMin; n <= nMax; n++) {
      for (let i = 0; i + n <= text.length; i++) {
        ngrams.add(text.substring(i, i + n));
      }
    }
    return ngrams;
  }

  function vectorize(text, vocab, nMin, nMax) {
    const ngrams = extractNgrams(text, nMin, nMax);
    const features = [];
    for (const gram of ngrams) {
      const idx = vocab[gram];
      if (idx !== undefined) {
        features.push([idx, 1.0]); // 1.0 = binary feature
      }
    }
    return features;
  }

  function sigmoid(x) {
    return 1.0 / (1.0 + Math.exp(-x));
  }

  function predictWithModel(title, model) {
    const features = vectorize(title, model.vocab, model.n_min, model.n_max);
    let score = model.bias;
    for (const [idx, weight] of features) {
      // weights 配列の長さを超えていないか確認
      if (idx >= 0 && idx < model.weights.length) {
        score += model.weights[idx] * weight;
      }
    }
    return sigmoid(score);
  }

  function classifyWithModel(title) {
    // モデルが読み込まれていない場合はスキップ
    if (!MODEL || !MODEL.vocab || Object.keys(MODEL.vocab).length === 0) {
      return false; // デフォルトは「カバーではない」と判定
    }

    const features = vectorize(title, MODEL.vocab, MODEL.n_min, MODEL.n_max);
    const prob = predictWithModel(title, MODEL);

    // debug: console に出力（browser console で確認可能）
    if (prob >= 0.3) {
      console.log(`[ML] title="${title.substring(0, 30)}" features=${features.length} prob=${prob.toFixed(4)}`);
    }

    return prob >= 0.5; // 閾値 0.5
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

  function extractCandidates(children) {
    return children
      .map((c) => {
        const title = normalizeTitle(c?.title ?? "");
        const userId = Number(c?.userId);
        const url = c?.watchURL ?? "";
        if (c?.contentKind !== "video") return null;
        if (!title) return null;
        if (!classifyWithModel(title)) return null;
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
      return `子作品取得中: ${fmtProgress(s.fetched, s.total)} 件 (+${s.lastBatchSize})`;
    }
    if (s.phase === "users") {
      return `投稿者取得中: ${fmtProgress(s.done, s.total)} 件`;
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
    commonsLink.insertAdjacentElement("afterend", rightWrap);

    const toggleBtn = makeButton("▶ カバーTSV");
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

    const copyBtn = makeButton("コピー");
    const downloadBtn = makeButton("ダウンロード");

    row.appendChild(copyBtn);
    row.appendChild(downloadBtn);

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
      toggleBtn.textContent = open ? "▲ カバーTSV" : "▶ カバーTSV";
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

    return {
      mode: "embedded",
      wrap,
      status,
      miniStatus,
      progressInner,
      copyBtn,
      downloadBtn,
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

    const candidates = extractCandidates(children);

    ui.status.textContent =
      `子作品: ${children.length} 件\n` +
      `候補: ${candidates.length} 件`;
    if (ui.miniStatus) ui.miniStatus.textContent = `候補: ${candidates.length} 件`;

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
    ui.status.textContent = `取得開始: ${rootId}`;
    if (ui.miniStatus) ui.miniStatus.textContent = "取得中...";

    try {
      const result = await buildTsvForRootId(rootId, ui);

      if (kind === "copy") {
        await copyToClipboard(result.tsv);
        ui.status.textContent =
          `コピー完了: ${result.candidatesCount} 件\n` +
          `子作品: ${result.childrenCount} 件、投稿者: ${result.userCount} 件`;
        if (ui.miniStatus) ui.miniStatus.textContent = `コピー完了: ${result.candidatesCount} 件`;
      } else if (kind === "download") {
        downloadText(result.filename, result.tsv);
        ui.status.textContent =
          `ダウンロード完了: ${result.filename}\n` +
          `子作品: ${result.childrenCount} 件、投稿者: ${result.userCount} 件、候補: ${result.candidatesCount} 件`;
        if (ui.miniStatus) ui.miniStatus.textContent = `ダウンロード完了`;
      }

      ui.progressInner.style.width = "100%";
    } catch (e) {
      console.error(e);
      ui.status.textContent = `エラー: ${e?.message ?? e}`;
      if (ui.miniStatus) ui.miniStatus.textContent = "エラー";
      ui.progressInner.style.width = "0%";
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
    ui.status.textContent = `準備完了: ${rootId}`;
    if (ui.miniStatus) ui.miniStatus.textContent = "";

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

  // 起動時にモデルを読み込み（非同期）
  loadModel();
})();

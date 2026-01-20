// ==UserScript==
// @name         Nico Commons ContentTree - Copy TSV
// @namespace    https://ngtkana.local/
// @version      0.1.0
// @description  Nico Commons content tree children -> filter utaite covers -> copy TSV
// @match        https://commons.nicovideo.jp/works/*/tree/children*
// @grant        none
// ==/UserScript==

(() => {
  "use strict";

  const TITLE_KEYWORDS = ["歌って", "歌わせていただき", "歌いました"];
  const LIMIT = 50; // ちょい大きめで通信回数減らす

  function sleep(ms) {
    return new Promise((r) => setTimeout(r, ms));
  }

  function pickRootId() {
    const m = location.pathname.match(/\/works\/([^/]+)\/tree\/children/);
    return m?.[1] ?? null; // sm**** / nc**** / lv****
  }

  function looksLikeUtaMita(title) {
    return TITLE_KEYWORDS.some((k) => title.includes(k));
  }

  function normalizeTitle(title) {
    // 長いのは「仕様」じゃなくて投稿者が長く付けてるだけ。
    // でも「見た目だけ短くしたい」ならここでカットできる。
    return (title ?? "").trim();
  }

  async function fetchJson(url) {
    const res = await fetch(url, { credentials: "omit" });
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${url}`);
    return await res.json();
  }

  async function fetchAllChildren(rootId) {
    let offset = 0;
    let total = Infinity;
    const all = [];

    while (offset < total) {
      const url =
        `https://public-api.commons.nicovideo.jp/v1/tree/${encodeURIComponent(rootId)}/relatives/children` +
        `?_offset=${offset}&_limit=${LIMIT}&with_meta=1&_sort=-id&only_mine=0`;

      const j = await fetchJson(url);

      const children = j?.data?.children;
      const contents = children?.contents ?? [];
      total = Number(children?.total ?? contents.length);

      all.push(...contents);
      offset += contents.length;

      // もしAPIが0件返してきたら無限ループ回避
      if (contents.length === 0) break;

      // サーバーに優しく
      await sleep(80);
    }

    return all;
  }

  async function fetchUserMap(userIds) {
    // account API は userIds=... を並べる形式
    // 念のためチャンク
    const uniq = Array.from(new Set(userIds.filter((x) => Number.isFinite(x))));
    const map = new Map();

    const chunkSize = 80;
    for (let i = 0; i < uniq.length; i += chunkSize) {
      const chunk = uniq.slice(i, i + chunkSize);
      const qs = chunk.map((id) => `userIds=${encodeURIComponent(id)}`).join("&");
      const url = `https://account.nicovideo.jp/api/public/v1/users.json?${qs}`;

      const j = await fetchJson(url);

      // 返り値の形は環境で微妙に違う可能性があるので、雑に拾う
      const users = j?.data?.users ?? j?.users ?? j?.data ?? [];

      if (Array.isArray(users)) {
        for (const u of users) {
          const id = u?.id ?? u?.userId ?? u?.user_id;
          const name = u?.nickname ?? u?.nickName ?? u?.name ?? u?.userName ?? u?.username;
          if (id != null && name) map.set(Number(id), String(name));
        }
      }
      await sleep(60);
    }
    return map;
  }

  async function copyToClipboard(text) {
    // clipboard API がダメな時用にフォールバック
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
    // 「親子登録する」ボタンのある見出しエリアを狙う
    // body > section.p-treeWorks ... div.l-outsideHeading
    return (
      document.querySelector("section.p-treeWorks .l-outsideHeading") ??
      document.querySelector("section.p-treeWorks")
    );
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
    btn.textContent = "(Copy TSV)";
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
    badge.textContent = "歌みた候補: ?";
    badge.style.padding = "2px 8px";
    badge.style.borderRadius = "999px";
    badge.style.border = "1px solid rgba(0, 0, 0, 0.12)";
    badge.style.background = "rgba(0, 0, 0, 0.04)";
    badge.style.fontSize = "12px";

    wrap.appendChild(btn);
    wrap.appendChild(badge);
    return { wrap, btn, badge };
  }

  async function run() {
    const rootId = pickRootId();
    if (!rootId) return;

    // 二重挿入防止
    if (document.querySelector('[data-ngtkana-copytsv="wrap"]')) return;

    const insertPoint = findInsertPoint();
    if (!insertPoint) return;

    const { wrap, btn, badge } = makeUi();
    insertPoint.appendChild(wrap);

    btn.addEventListener("click", async () => {
      btn.disabled = true;
      btn.textContent = "(Loading...)";
      badge.textContent = "歌みた候補: …";

      try {
        const children = await fetchAllChildren(rootId);

        // 作品タイトル（元作品名）はページから取る：あなたの希望どおり「元の私の仕様」に寄せる
        // （元作品列が欲しいならTSVの先頭に足すだけ）
        const parentTitle =
          document
            .querySelector(
              "section.p-treeWorks h1, section.p-treeWorks .p-treeWorksHeading__title, section.p-treeWorks .c-heading"
            )
            ?.textContent?.trim() ?? "";

        // まず歌みた候補だけ
        const candidates = children
          .map((c) => {
            const title = normalizeTitle(c?.title ?? "");
            const userId = Number(c?.userId);
            const url = c?.watchURL ?? "";
            // 動画(sm)っぽいものだけ残す（生放送 lv / コモンズ nc を落とす）
            const id = c?.globalId ?? "";
            const isVideoSm = typeof id === "string" && /^sm\d+$/.test(id);
            if (!isVideoSm) return null;
            if (!title) return null;
            if (!looksLikeUtaMita(title)) return null;
            if (!url) return null;
            return { parentTitle, title, userId, url };
          })
          .filter(Boolean);

        badge.textContent = `歌みた候補:${candidates.length}`;

        // userId → 表示名
        const userMap = await fetchUserMap(candidates.map((x) => x.userId));

        // 出力は「あなたの元の仕様」に合わせて 3列：タイトル/投稿者/URL
        // ヘッダ無しが元と同じ（必要ならここでヘッダ付ける）
        const lines = candidates.map((x) => {
          const owner = userMap.get(x.userId) ?? String(x.userId ?? "");
          return [x.title, owner, x.url].join("\t");
        });

        const tsv = lines.join("\n");
        await copyToClipboard(tsv);

        btn.textContent = "(Copied!)";
        await sleep(700);
      } catch (e) {
        console.error(e);
        btn.textContent = "(Error)";
        badge.textContent = "歌みた候補: ?";
        alert(`Copy TSV 失敗: ${e?.message ?? e}`);
      } finally {
        btn.disabled = false;
        btn.textContent = "(Copy TSV)";
      }
    });
  }

  // SPAっぽく後からDOMが来るので少し待って何回か試す
  let tries = 0;
  const timer = setInterval(() => {
    tries++;
    run().catch(console.error);
    if (document.querySelector('[data-ngtkana-copytsv="wrap"]') || tries > 40) {
      clearInterval(timer);
    }
  }, 300);
})();

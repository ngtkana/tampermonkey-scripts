import { fetchAllChildren, fetchUserMap, extractCandidates, buildTsv, sleep } from "./core.js";

const TITLE_KEYWORDS = ["歌って", "歌わせていただき", "歌いました"];

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
      const children = await fetchAllChildren(fetch, rootId);
      const candidates = extractCandidates(children, { titleKeywords: TITLE_KEYWORDS });
      badge.textContent = `歌みた候補:${candidates.length}`;

      const userMap = await fetchUserMap(
        fetch,
        candidates.map((x) => x.userId)
      );
      const tsv = buildTsv(candidates, userMap);
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

let tries = 0;
const timer = setInterval(() => {
  tries++;
  run().catch(console.error);
  if (document.querySelector('[data-ngtkana-copytsv="wrap"]') || tries > 40) {
    clearInterval(timer);
  }
}, 300);

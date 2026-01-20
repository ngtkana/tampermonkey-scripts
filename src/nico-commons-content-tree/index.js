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
        },
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
          },
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

let tries = 0;
const timer = setInterval(() => {
  tries++;
  run().catch(console.error);
  if (document.querySelector('[data-ngtkana-copytsv="wrap"]') || tries > 40) {
    clearInterval(timer);
  }
}, 300);

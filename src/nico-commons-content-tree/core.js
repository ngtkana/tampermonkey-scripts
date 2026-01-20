export const TITLE_KEYWORDS_DEFAULT = ["歌って", "歌わせていただき", "歌いました"];
export const DEFAULT_LIMIT = 50;

export function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

export function looksLikeUtaMita(title, titleKeywords = TITLE_KEYWORDS_DEFAULT) {
  return titleKeywords.some((k) => String(title ?? "").includes(k));
}

export function normalizeTitle(title) {
  return String(title ?? "").trim();
}

export function isVideoSm(globalId) {
  return typeof globalId === "string" && /^sm\d+$/.test(globalId);
}

export function buildChildrenApiUrl(rootId, { offset = 0, limit = DEFAULT_LIMIT } = {}) {
  return (
    `https://public-api.commons.nicovideo.jp/v1/tree/${encodeURIComponent(rootId)}/relatives/children` +
    `?_offset=${offset}&_limit=${limit}&with_meta=1&_sort=-id&only_mine=0`
  );
}

export function buildAccountUsersApiUrl(userIds) {
  const qs = userIds.map((id) => `userIds=${encodeURIComponent(id)}`).join("&");
  return `https://account.nicovideo.jp/api/public/v1/users.json?${qs}`;
}

export async function fetchJson(fetchImpl, url) {
  const res = await fetchImpl(url, { credentials: "omit" });
  if (!res.ok) throw new Error(`HTTP ${res.status}: ${url}`);
  return await res.json();
}

export async function fetchAllChildren(
  fetchImpl,
  rootId,
  { limit = DEFAULT_LIMIT, delayMs = 80, onProgress } = {}
) {
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
        lastBatchSize: contents.length,
      });
    }

    if (contents.length === 0) break;
    await sleep(delayMs);
  }

  return all;
}

export function parseAccountUsersResponse(json) {
  const users = json?.data?.users ?? json?.users ?? json?.data ?? [];
  const map = new Map();
  if (!Array.isArray(users)) return map;

  for (const u of users) {
    const id = u?.id ?? u?.userId ?? u?.user_id;
    const name = u?.nickname ?? u?.nickName ?? u?.name ?? u?.userName ?? u?.username;
    if (id != null && name) map.set(Number(id), String(name));
  }
  return map;
}

export async function fetchUserMap(
  fetchImpl,
  userIds,
  { chunkSize = 80, delayMs = 60, onProgress } = {}
) {
  const uniq = Array.from(new Set(userIds.map(Number).filter((x) => Number.isFinite(x))));
  const map = new Map();

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

export function extractCandidates(children, { titleKeywords = TITLE_KEYWORDS_DEFAULT } = {}) {
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

export function buildTsv(candidates, userMap) {
  const lines = candidates.map((x) => {
    const owner = userMap.get(x.userId) ?? String(x.userId ?? "");
    return [x.title, owner, x.url].join("\t");
  });
  return lines.join("\n");
}

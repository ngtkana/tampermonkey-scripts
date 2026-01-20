"use strict";

const TITLE_KEYWORDS_DEFAULT = ["歌って", "歌わせていただき", "歌いました"];
const DEFAULT_LIMIT = 50;

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
  return (
    `https://public-api.commons.nicovideo.jp/v1/tree/${encodeURIComponent(rootId)}/relatives/children` +
    `?_offset=${offset}&_limit=${limit}&with_meta=1&_sort=-id&only_mine=0`
  );
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

async function fetchAllChildren(fetchImpl, rootId, { limit = DEFAULT_LIMIT, delayMs = 80 } = {}) {
  let offset = 0;
  let total = Infinity;
  const all = [];

  while (offset < total) {
    const url = buildChildrenApiUrl(rootId, { offset, limit });
    const j = await fetchJson(fetchImpl, url);

    const children = j?.data?.children;
    const contents = children?.contents ?? [];
    total = Number(children?.total ?? contents.length);

    all.push(...contents);
    offset += contents.length;

    if (contents.length === 0) break;
    await sleep(delayMs);
  }

  return all;
}

function parseAccountUsersResponse(json) {
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

async function fetchUserMap(fetchImpl, userIds, { chunkSize = 80, delayMs = 60 } = {}) {
  const uniq = Array.from(new Set(userIds.map(Number).filter((x) => Number.isFinite(x))));
  const map = new Map();

  for (let i = 0; i < uniq.length; i += chunkSize) {
    const chunk = uniq.slice(i, i + chunkSize);
    const url = buildAccountUsersApiUrl(chunk);
    const j = await fetchJson(fetchImpl, url);
    const partial = parseAccountUsersResponse(j);
    for (const [k, v] of partial.entries()) map.set(k, v);
    await sleep(delayMs);
  }

  return map;
}

function extractCandidates(children, { titleKeywords = TITLE_KEYWORDS_DEFAULT } = {}) {
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
  const lines = candidates.map((x) => {
    const owner = userMap.get(x.userId) ?? String(x.userId ?? "");
    return [x.title, owner, x.url].join("\t");
  });
  return lines.join("\n");
}

module.exports = {
  TITLE_KEYWORDS_DEFAULT,
  DEFAULT_LIMIT,
  sleep,
  looksLikeUtaMita,
  normalizeTitle,
  isVideoSm,
  buildChildrenApiUrl,
  buildAccountUsersApiUrl,
  fetchJson,
  fetchAllChildren,
  parseAccountUsersResponse,
  fetchUserMap,
  extractCandidates,
  buildTsv,
};

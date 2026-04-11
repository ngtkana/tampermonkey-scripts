#!/usr/bin/env python3
"""
Step 2: child_counts.json の上位曲について子作品を取得して children.json に保存。
        各曲最大 MAX_PER_SONG 件、合計上位 TOP_N 曲を対象とする。
"""

import json
import time
import urllib.request
from pathlib import Path

CHILD_COUNTS = Path(__file__).parent / "child_counts.json"
OUTPUT = Path(__file__).parent / "children.json"

TOP_N = 10
MAX_PER_SONG = 200
BATCH = 100
DELAY_S = 0.3


def fetch_batch(sm_id: str, offset: int, limit: int) -> dict:
    url = (
        f"https://public-api.commons.nicovideo.jp/v1/tree/{sm_id}"
        f"/relatives/children?_offset={offset}&_limit={limit}"
        f"&with_meta=1&_sort=-id&only_mine=0"
    )
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=10) as resp:
        return json.loads(resp.read())


def fetch_children(sm_id: str, max_count: int) -> list[dict]:
    results = []
    offset = 0
    total = None

    while offset < max_count:
        limit = min(BATCH, max_count - offset)
        data = fetch_batch(sm_id, offset, limit)
        children = data["data"]["children"]

        if total is None:
            total = children["total"]

        contents = children["contents"]
        if not contents:
            break

        for c in contents:
            if c.get("contentKind") != "video":
                continue
            title = str(c.get("title") or "")
            title = title.split("\u202a")[0]           # YouTube @mention 埋め込み制御文字を除去
            title = title.replace("\u2028", " ").replace("\u2029", " ").strip()
            video_id = c.get("contentId") or c.get("id") or ""
            if title:
                results.append({"video_id": video_id, "title": title, "source": sm_id})

        offset += len(contents)
        if offset >= (total or 0):
            break

        time.sleep(DELAY_S)

    return results


def main():
    counts = json.loads(CHILD_COUNTS.read_text(encoding="utf-8"))
    top = counts[:TOP_N]

    all_children = []
    for i, entry in enumerate(top, 1):
        sm_id = entry["id"]
        print(f"[{i}/{len(top)}] {sm_id} ({entry['child_count']} 件) → 最大{MAX_PER_SONG}件取得")
        try:
            children = fetch_children(sm_id, MAX_PER_SONG)
            print(f"  取得: {len(children)} 件")
            all_children.extend(children)
        except Exception as e:
            print(f"  ERROR: {e}")
        time.sleep(DELAY_S)

    OUTPUT.write_text(json.dumps(all_children, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"\n合計 {len(all_children)} 件 → {OUTPUT}")


if __name__ == "__main__":
    main()

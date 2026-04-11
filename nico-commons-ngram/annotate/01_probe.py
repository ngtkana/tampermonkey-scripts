#!/usr/bin/env python3
"""
Step 1: ニコニコランキングページから sm ID を抽出し、commons API で子作品数をプローブして
        child_counts.json に保存する。
"""

import re
import json
import time
import urllib.request
from pathlib import Path

RANKING_URL = "https://www.nicovideo.jp/ranking/genre/dshv5do5"
OUTPUT = Path(__file__).parent / "child_counts.json"
DELAY_S = 0.3

def fetch_ranking_html() -> str:
    req = urllib.request.Request(RANKING_URL, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=10) as resp:
        return resp.read().decode("utf-8")

def extract_sm_ids(html: str) -> list[str]:
    ids = sorted(set(re.findall(r'\bsm\d+\b', html)))
    return ids

def fetch_child_count(sm_id: str) -> int | None:
    url = (
        f"https://public-api.commons.nicovideo.jp/v1/tree/{sm_id}"
        f"/relatives/children?_offset=0&_limit=1&with_meta=1&_sort=-id&only_mine=0"
    )
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read())
        return data["data"]["children"]["total"]
    except Exception as e:
        print(f"  ERROR {sm_id}: {e}")
        return None

def main():
    print(f"Fetching {RANKING_URL} ...")
    html = fetch_ranking_html()
    ids = extract_sm_ids(html)
    print(f"Found {len(ids)} unique sm IDs")

    results = []
    for i, sm_id in enumerate(ids, 1):
        count = fetch_child_count(sm_id)
        print(f"[{i:3}/{len(ids)}] {sm_id}: {count}")
        if count is not None:
            results.append({"id": sm_id, "child_count": count})
        time.sleep(DELAY_S)

    results.sort(key=lambda x: x["child_count"], reverse=True)

    OUTPUT.parent.mkdir(exist_ok=True)
    OUTPUT.write_text(json.dumps(results, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"\nSaved {len(results)} entries to {OUTPUT}")
    print("\nTop 20:")
    for r in results[:20]:
        print(f"  {r['id']}: {r['child_count']}")

if __name__ == "__main__":
    main()

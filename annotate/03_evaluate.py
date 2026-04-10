#!/usr/bin/env python3
"""
Step 3: dataset.json を使って現在のキーワードリストの
        Precision / Recall / F1 を計算し、改善候補キーワードを提案する。
"""

import json
from pathlib import Path

DATASET = Path(__file__).parent / "dataset.json"

POSITIVE_KEYWORDS = [
    "歌って", "うたって", "唄って",
    "歌った", "うたった", "唄った",
    "歌いました", "うたいました", "唄いました",
    "歌わせていただき", "うたわせていただき",
    "歌いますた", "歌いなおし",
    "弾き語り",
    "カバー", "cover",
]

NEGATIVE_KEYWORDS = [
    # 非カバー動画
    "まとめ", "音源", "講座", "配布", "メドレー", "予告", "人力",
    # 合成音声系ソフトウェア・規格
    "utau", "vocaloid", "ボカロ", "neutrino", "synthesizerv", "synthv",
    "voiceroid", "ボイスロイド", "a.i.voice", "合成音声", "nnsvs", "voicevox",
]


def looks_like_utamita(title: str, pos: list[str], neg: list[str]) -> bool:
    sl = title.lower()
    if any(k.lower() in sl for k in neg):
        return False
    return any(k.lower() in sl for k in pos)


def f1(tp, fp, fn):
    p = tp / (tp + fp) if (tp + fp) else 0.0
    r = tp / (tp + fn) if (tp + fn) else 0.0
    f = 2 * p * r / (p + r) if (p + r) else 0.0
    return p, r, f


def main():
    dataset = json.loads(DATASET.read_text(encoding="utf-8"))

    # 評価
    tp = fp = fn = tn = 0
    false_neg = []
    false_pos = []

    for row in dataset:
        title, gold = row["title"], row["label"]
        pred = 1 if looks_like_utamita(title, POSITIVE_KEYWORDS, NEGATIVE_KEYWORDS) else 0

        if gold == 1 and pred == 1:
            tp += 1
        elif gold == 0 and pred == 1:
            fp += 1
            false_pos.append(title)
        elif gold == 1 and pred == 0:
            fn += 1
            false_neg.append(title)
        else:
            tn += 1

    p, r, f = f1(tp, fp, fn)
    total = tp + fp + fn + tn
    print(f"=== 現在のキーワードリスト評価 ===")
    print(f"  サンプル数 : {total}  (陽性={tp+fn}, 陰性={tn+fp})")
    print(f"  TP={tp}  FP={fp}  FN={fn}  TN={tn}")
    print(f"  Precision : {p:.3f}")
    print(f"  Recall    : {r:.3f}")
    print(f"  F1        : {f:.3f}")

    print(f"\n=== FN ({fn}件) サンプル ===")
    for title in false_neg[:20]:
        print(f"  {title}")

    print(f"\n=== FP ({fp}件) サンプル ===")
    for title in false_pos[:20]:
        print(f"  {title}")

    print("\n=== FN タイトル中の候補キーワード出現数 ===")
    for kw in ["歌コレ", "歌枠", "歌ってみました", "cover", "covered", "sing", "sang",
               "vocal", "弾き語り", "合唱", "生歌", "歌練習", "練習", "歌う"]:
        cnt = sum(1 for t in false_neg if kw.lower() in t.lower())
        if cnt:
            print(f"  {kw!r:20s}: {cnt}")

    print("\n=== FP タイトル中の NEGATIVE 候補キーワード出現数 ===")
    for kw in ["remix", "リミックス", "mmd", "踊ってみた", "叩いてみた", "弾いてみた",
               "ランキング", "マッシュアップ", "解説", "実況", "synthv", "voicevox",
               "手描き", "人力", "組曲", "メドレー", "ゲーム"]:
        cnt = sum(1 for t in false_pos if kw.lower() in t.lower())
        if cnt:
            print(f"  {kw!r:20s}: {cnt}")


if __name__ == "__main__":
    main()

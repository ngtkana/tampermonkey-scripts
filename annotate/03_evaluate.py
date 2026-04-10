#!/usr/bin/env python3
"""
Step 3: dataset.json を使って現在のキーワードリストの
        Precision / Recall / F1 を計算し、report.md に結果を出力する。
"""

import json
from datetime import datetime
from pathlib import Path

DATASET = Path(__file__).parent / "dataset.json"
REPORT  = Path(__file__).parent / "report.md"

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

    # stdout
    print(f"Precision={p:.3f}  Recall={r:.3f}  F1={f:.3f}  "
          f"(TP={tp} FP={fp} FN={fn} TN={tn})")
    print(f"→ {REPORT}")

    # report.md
    lines = [
        f"# キーワードリスト評価レポート",
        f"",
        f"生成日時: {datetime.now().strftime('%Y-%m-%d %H:%M')}",
        f"",
        f"## スコア",
        f"",
        f"| 指標 | 値 |",
        f"|------|----|",
        f"| サンプル数 | {total} (陽性={tp+fn}, 陰性={tn+fp}) |",
        f"| TP | {tp} |",
        f"| FP | {fp} |",
        f"| FN | {fn} |",
        f"| TN | {tn} |",
        f"| Precision | {p:.3f} |",
        f"| Recall    | {r:.3f} |",
        f"| **F1**    | **{f:.3f}** |",
        f"",
        f"## キーワードリスト",
        f"",
        f"**POSITIVE** ({len(POSITIVE_KEYWORDS)}件)",
        f"",
        "```",
        "\n".join(POSITIVE_KEYWORDS),
        "```",
        f"",
        f"**NEGATIVE** ({len(NEGATIVE_KEYWORDS)}件)",
        f"",
        "```",
        "\n".join(NEGATIVE_KEYWORDS),
        "```",
        f"",
        f"## False Negative ({fn}件) — 取りこぼし",
        f"",
    ]
    for t in false_neg:
        lines.append(f"- {t}")

    lines += [
        f"",
        f"## False Positive ({fp}件) — 誤検出",
        f"",
    ]
    for t in false_pos:
        lines.append(f"- {t}")

    lines += [
        f"",
        f"## FN タイトル中の POSITIVE 候補キーワード",
        f"",
    ]
    for kw in ["歌コレ", "歌枠", "歌ってみました", "cover", "covered",
               "vocal", "弾き語り", "合唱", "生歌", "歌練習", "歌う"]:
        cnt = sum(1 for t in false_neg if kw.lower() in t.lower())
        if cnt:
            lines.append(f"- `{kw}`: {cnt}件")

    lines += [
        f"",
        f"## FP タイトル中の NEGATIVE 候補キーワード",
        f"",
    ]
    for kw in ["remix", "リミックス", "mmd", "踊ってみた", "叩いてみた", "弾いてみた",
               "ランキング", "マッシュアップ", "cevio", "voisona", "巡回", "手描き"]:
        cnt = sum(1 for t in false_pos if kw.lower() in t.lower())
        if cnt:
            lines.append(f"- `{kw}`: {cnt}件")

    REPORT.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()

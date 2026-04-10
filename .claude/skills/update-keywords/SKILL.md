---
name: update-keywords
description: キーワードリストを更新して評価するスキル。nico-commons-content-tree.user.js の POSITIVE_KEYWORDS / NEGATIVE_KEYWORDS を変更するとき、または `/update-keywords` と呼ばれたときに必ず使う。両ファイルの同期と評価の自動実行を担う。
---

# キーワード更新ワークフロー

`POSITIVE_KEYWORDS` / `NEGATIVE_KEYWORDS` を変更するときは **必ず2ファイルを同時に更新** し、評価を再実行する。

## 対象ファイル（常に同期させること）

1. **本体スクリプト**
   `/home/kana/repos/tampermonkey-scripts/nico-commons-content-tree.user.js`
   → `POSITIVE_KEYWORDS`（17行目付近）と `NEGATIVE_KEYWORDS`（25行目付近）

2. **評価スクリプト**
   `/home/kana/repos/tampermonkey-scripts/annotate/03_evaluate.py`
   → `POSITIVE_KEYWORDS`（12行目付近）と `NEGATIVE_KEYWORDS`（22行目付近）

## 手順

1. 両ファイルのキーワードリストを **同じ内容** に更新する
2. 評価スクリプトを実行する:
   ```
   cd /home/kana/repos/tampermonkey-scripts && python3 annotate/03_evaluate.py
   ```
3. F1スコアと report.md の内容をユーザーに伝える

## チェックリスト

- [ ] `nico-commons-content-tree.user.js` のリストを更新した
- [ ] `annotate/03_evaluate.py` のリストを**同じ内容に**更新した
- [ ] `python3 annotate/03_evaluate.py` を実行してスコアを確認した

## 注意

- どちらか片方だけ更新すると評価と本番が乖離する
- 追加前に副作用（既存TPがFNに変わらないか）を確認するとよい

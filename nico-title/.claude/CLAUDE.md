# CLAUDE.md - nico-title プロジェクト開発ガイド

## 基本方針

* **Rust ファースト**: すべての処理は Rust で実装
* **Python は使わない**: JSON 解析なども Rust コマンドで実行
* **毎回のコンパイル確認**: `cargo build --release` で型安全性を確認
* **lint 適用**: 編集のたびに rust-analyzer, rustfmt, clippy を適用

---

## プロジェクト構成

```
src/
├── main.rs          # CLI entry point (clap-based)
├── bio.rs           # BIO タグ変換（HTML エンティティ処理含む）
├── crf.rs           # CRF モデル（Forward-Backward, Viterbi）
├── features.rs      # 特徴量抽出（文字型分類）
├── analyze_results.rs # 分析コマンド群
├── extract.rs       # ルールベース抽出器
├── annotate.rs      # LLM annotation（Claude API）
├── compare.rs       # CRF vs ルールベース比較
└── download.rs      # NicoNico API ダウンロード

data/
├── nico_api_result.tsv           # API レスポンス（タイトル一覧）
├── nico_api_annotations.jsonl    # LLM 抽出結果（extracted_title）
├── nico_bio_tags.jsonl           # BIO タグ付きデータ（tokens）
├── crf_model.json                # 訓練済み CRF モデル
├── analysis_results.jsonl        # 全 1574 件の分析結果
├── analysis_mismatches.jsonl     # 不一致 770 件の詳細
└── suspicious_annotations.jsonl  # 怪しい annotation 358 件
```

---

## ワークフロー

### 1. データの準備

```bash
# LLM が extracted_title を抽出（Claude API 使用）
cargo run -- annotate --count 200

# extracted_title から BIO タグを生成
cargo run -- bio-convert --input data/nico_api_annotations.jsonl \
                         --output data/nico_bio_tags.jsonl

# （重要）BIO 変換の正確性を確認
cargo run -- check-bio-conversion \
    --bio-file data/nico_bio_tags.jsonl \
    --mismatches-file data/analysis_mismatches.jsonl
```

### 2. CRF モデル訓練

```bash
# CRF を訓練（デフォルト: lr=0.01, lambda=0.001, epochs=20）
cargo run -- crf-learn --learning-rate 0.01 --lambda 0.001 --epochs 20 \
                       --input data/nico_bio_tags.jsonl \
                       --output data/crf_model.json

# モデルを評価（テストセット 20% でスパン単位の F1 を計算）
cargo run -- crf-eval --input data/nico_bio_tags.jsonl \
                      --model data/crf_model.json \
                      --test-ratio 0.2
```

### 3. 分析と改善

```bash
# 全データを分析（LLM と CRF の予測を比較）
cargo run -- analyze-results --input data/nico_bio_tags.jsonl \
                             --model data/crf_model.json \
                             --output data/analysis_results.jsonl

# 不一致パターンを見やすく表示
cargo run -- show-mismatches --input data/analysis_mismatches.jsonl --count 5

# 怪しい annotation を検出（Gold が O で始まるケース）
cargo run -- find-suspicious --input data/analysis_mismatches.jsonl \
                             --output data/suspicious_annotations.jsonl
```

---

## 重要な実装ポイント

### CRF 学習（crf.rs）

**log-space 計算**: 数値アンダーフローを防ぐため、すべての確率計算を log 空間で実施

```rust
fn log_sum_exp(a: f64, b: f64) -> f64 {
    let max_val = a.max(b);
    let min_val = a.min(b);
    max_val + (1.0 + (min_val - max_val).exp()).ln()
}
```

**勾配クリッピング**: max_norm=5.0 で学習を安定化

**最適ハイパーパラメータ**: lr=0.008, lambda=0.03, epochs=20
- F1 スコア: 78.9% (ルールベース 78.1% を +0.9 pts 上回る)
- Precision: 83.1%, Recall: 75.2%

### 特徴量抽出（features.rs）

**文字型分類**: 10 種類（kanji, hiragana, katakana, ascii_letter, 等）

**特徴量**:
- 現在・前後の文字型（context）
- セパレータ（/, ／, feat., Vo. など）との距離
- 相対位置（先頭・末尾・中央）

---

## トラブルシューティング

### BIO 変換で見つからないケース

**原因**: HTML エンティティが未処理
**確認**: `cargo run -- check-bio-conversion`
**修正**: normalize_text() にエンティティを追加

### CRF 学習が発散

**原因**: learning_rate が大きすぎる（0.05 は不安定）
**解決**: 0.01 以下に設定、gradient clipping max_norm を確認

### 予測がすべて O（検出なし）

**原因**: transition[I][I] が負すぎる
**確認**: モデルの transition 行列を確認（data/crf_model.json）
**修正**: I→I 遷移スコアを大きくする

---

## 分析コマンド早見表

| コマンド | 目的 | 出力 |
|---------|------|------|
| `analyze-results` | 全データ分析 | analysis_results.jsonl (1574件) |
| `show-mismatches` | 不一致パターン表示 | 3パターン × N件の例 |
| `find-suspicious` | 怪しい annotation 検出 | suspicious_annotations.jsonl |
| `check-bio-conversion` | BIO 変換の正確性 | 一致/不一致/見つからない の統計 |
| `crf-eval` | モデル評価 | Precision/Recall/F1 (span-level) |

---

## 注意点

* **データは JSONL 形式**: JSON Lines（改行区切り）、jq で検査
* **テキスト正規化は必須**: annotation → BIO 変換の前に必ず実施
* **test_ratio は固定**: train/test 分割は決定的（seed 固定）
* **Python スクリプトは使わない**: すべて Rust コマンドで実行

# CLAUDE.md

## 基本方針

* **Rust ファースト**: すべての処理は Rust で実装
* **毎回のコンパイル確認**: `cargo build --release` で型安全性を確認
* **Don't touch git**: git 操作は人間が行います

---

## ワークフロー

### データ準備

```bash
cargo run -- annotate --count 200    # LLM で曲名を抽出（Claude API）
cargo run -- bio-convert             # BIO タグを生成
```

### 学習・評価

```bash
cargo run -- crf-learn --learning-rate 0.008 --lambda 0.03 --epochs 20
cargo run -- crf-eval --test-ratio 0.2     # F1: 78.9%（ルールベース比 +0.8pt）
```

### 分析

```bash
cargo run -- analyze-results         # 全件分析
cargo run -- show-mismatches         # 不一致確認
```

---

## プロジェクト構成

```
src/
├── main.rs           # CLI エントリポイント
├── crf.rs            # CRF モデル（Viterbi・SGD 学習）
├── features.rs       # 文字型特徴量（10 種類）
├── bio.rs            # BIO タグ変換
├── analyze_results.rs # 分析・評価コマンド
├── extract.rs        # ルールベース抽出
├── annotate.rs       # Claude API アノテーション
└── download.rs       # NicoNico API ダウンロード

data/
├── nico_api_result.tsv        # 生データ（原本）
├── nico_api_annotations.jsonl # LLM アノテーション
├── nico_bio_tags.jsonl        # BIO 学習データ
└── crf_model.json             # 訓練済みモデル
```

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## Project Overview

This repository contains:
1. **nico-commons-ngram** — Rust ML project for Japanese song cover video classification
   - Char n-gram features + Logistic Regression with L2 regularization
   - Commands: learn, predict, tune, cross-val, export
2. **Tampermonkey scripts** — Browser automation for Nico Nico Douga
   - nico-commons-content-tree.user.js — filter utaite covers
   - sheer-reservelog-to-gcal-url.user.js — calendar integration

---

## nico-commons-ngram / Rust Development

### Quick Start

```bash
cd nico-commons-ngram

# Build (debug)
cargo build

# Build (optimized)
cargo build --release

# Run tests
cargo test

# Run specific command
cargo run -- learn          # Train model on dataset.json
cargo run -- predict "【歌ってみた】千本桜"
cargo run -- tune           # Grid search hyperparameters
cargo run -- cross-val -k 5  # 5-fold cross-validation
cargo run -- export         # Export pruned model as JavaScript
```

### Architecture

**Key Modules:**
- `src/main.rs` — CLI entry point (clap-based)
- `src/model.rs` — `Model` struct, `Metrics` (precision/recall/F1), `HyperParams`
- `src/learn.rs` — training pipeline, `learn()`, `predict()`, `tune()`, `cross_validate()`, `export()`
- `src/ngram.rs` — Unicode char n-gram extraction (n=3..5)

**Data Flow:**
```
annotate/dataset.json (1753 labeled titles)
  ↓ [load + normalize (NFKC) + shuffle]
  ├─→ train (1402 samples, 80%)
  │    ├─→ vocab build (53K n-grams)
  │    └─→ SGD learn
  │
  └─→ test (351 samples, 20%) → evaluate metrics
  
  ↓ [save trained weights]
  
annotate/model.json (2.6 MB, full model)
  ↓ [export --threshold 0.05]
  ↓
annotate/model.js (298 KB, pruned for Tampermonkey)
```

**Model Serialization:**
- **model.json**: Full model (vocab + weights + bias + idf)
- **model.js**: Pruned export (const MODEL = {...};) for JS embedding

### Important Concepts

**Metrics:**
- F1 score is the primary metric (precision/recall balance)
- Early stopping on validation F1 (patience=10)
- L2 regularization (lambda=1e-4) prevents overfitting

**Feature Extraction:**
- Normalize with NFKC (Japanese-safe)
- Extract char n-grams: "歌ってみた" → ["歌って", "ってみ", "てみた", ...]
- Binary features (1.0) or TF-IDF weighted (optional)

**Hyperparameters:**
```rust
HyperParams {
    n_min: 3, n_max: 5,     // n-gram range
    learning_rate: 0.1,
    lambda: 1e-4,           // L2 coefficient
    epochs: 50,
    early_stop_patience: 10,
    use_tfidf: false,       // Binary by default
}
```

### Development Notes

- Use `rust-analyzer` (configured via `~/.claude/keybindings.json` if needed)
- All JSON operations: use **jq** for inspection (e.g., `jq '.vocab | length' model.json`)
- For testing: `cargo test` runs unit tests in model.rs, ngram.rs
- Warnings about unused fields/functions are expected (they are used by Tampermonkey)

---

## Comparison Workflow (ML vs Rule-Based)

When comparing NN model to rule-based classifier:
- Rule-based keywords are in `nico-commons-content-tree.user.js` (POSITIVE_KEYWORDS, NEGATIVE_KEYWORDS)
- Use `cargo run -- compare` to evaluate both on test set
- Output includes confusion matrix, precision/recall/F1 for both approaches

---

## Tampermonkey Scripts

- **nico-commons-content-tree.user.js**: Rule-based filtering (string matching)
- **sheer-reservelog-to-gcal-url.user.js**: Calendar event creation
- When embedding ML model: update script to load `const MODEL` from model.js

---

## Common Tasks

### Train a new model
```bash
cargo run -- learn
# Outputs: annotate/model.json
```

### Evaluate with new threshold
```bash
cargo run -- export --threshold 0.1 --output model_strict.js
```

### Compare models on test set
```bash
cargo run -- compare
```

### Run 5-fold cross-validation
```bash
cargo run -- cross-val -k 5
```

---

## Dependencies

- **clap**: CLI argument parsing
- **serde**: JSON serialization
- **unicode-normalization**: NFKC normalization (Japanese)
- **rand**: Data shuffling

---

## Edition Note

`Cargo.toml` specifies `edition = "2024"` (future edition), which may cause warnings. If compatibility issues arise, downgrade to `edition = "2021"`.


# 気をつけて

* JSONの中身を軽く確認したいときはjqを使って
* Rustはrust-analyzerを使いながらコーディングして
* コードを追加・編集するたび、適切な単位にモジュール・ファイルを分割して
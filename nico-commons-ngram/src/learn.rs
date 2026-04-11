use crate::ngram;
use crate::model::{HyperParams, Model};
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;
use rand::seq::SliceRandom;

const DATASET_PATH: &str = "annotate/dataset.json";
const MODEL_OUTPUT: &str = "annotate/model.json";

#[derive(serde::Deserialize)]
struct Sample {
    title: String,
    label: i32,
}

pub fn learn() -> Result<(), Box<dyn std::error::Error>> {
    let params = HyperParams::default();

    // 1. データセット読み込み
    let samples = load_dataset(DATASET_PATH)?;
    eprintln!("[INFO] loaded {} samples", samples.len());

    // 2. 前処理（正規化 + 小文字化）
    let mut processed: Vec<(String, f64)> = samples
        .into_iter()
        .map(|s| {
            // label を 0 または 1 に検証
            let label = match s.label {
                0 | 1 => s.label as f64,
                other => {
                    eprintln!("[WARN] invalid label {}, treating as 0", other);
                    0.0
                }
            };
            (normalize(&s.title), label)
        })
        .collect();

    // 3. train/test split 前にシャッフル
    processed.shuffle(&mut rand::thread_rng());
    let split_idx = (processed.len() as f64 * 0.8) as usize;
    let (train_data, test_data) = processed.split_at(split_idx);
    let (train_data, test_data) = (train_data.to_vec(), test_data.to_vec());

    eprintln!(
        "[INFO] train: {}, test: {}",
        train_data.len(),
        test_data.len()
    );

    // 4. 語彙構築（訓練データから）
    let vocab = build_vocab(&train_data, params.n_min, params.n_max);
    eprintln!("[INFO] vocab size: {}", vocab.len());

    // 5. 訓練
    let mut model = Model::new(vocab, params.n_min, params.n_max);
    train(&mut model, &train_data, &test_data, &params);

    // 6. モデル保存
    let json = serde_json::to_string_pretty(&model)?;
    std::fs::write(MODEL_OUTPUT, &json)?;
    eprintln!("[INFO] saved model to {}", MODEL_OUTPUT);

    Ok(())
}

fn load_dataset(path: &str) -> Result<Vec<Sample>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

/// NFKC 正規化 + 小文字化
fn normalize(text: &str) -> String {
    text.nfkc().collect::<String>().to_lowercase()
}

/// 訓練データから語彙（n-gram → index）を構築
fn build_vocab(
    samples: &[(String, f64)],
    n_min: usize,
    n_max: usize,
) -> HashMap<String, usize> {
    let mut vocab = HashMap::new();
    for (text, _) in samples {
        for gram in ngram::extract(text, n_min, n_max) {
            let next = vocab.len();
            vocab.entry(gram).or_insert(next);
        }
    }
    vocab
}

/// テキストを語彙に基づきインデックス列に変換（バイナリ特徴量）
fn vectorize(text: &str, vocab: &HashMap<String, usize>, n_min: usize, n_max: usize) -> Vec<usize> {
    ngram::extract(text, n_min, n_max)
        .into_iter()
        .filter_map(|g| vocab.get(&g).copied())
        .collect()
}

/// 訓練ループ + テストデータでの評価 + 早期停止
fn train(
    model: &mut Model,
    train_data: &[(String, f64)],
    test_data: &[(String, f64)],
    params: &HyperParams,
) {
    eprintln!();
    eprintln!("{:<6} {:<12} {:<12} {:<12} {:<12}", "epoch", "train_loss", "train_acc", "test_loss", "test_acc");
    eprintln!("{}", "-".repeat(60));

    let mut rng = rand::thread_rng();
    let mut best_test_loss = f64::INFINITY;
    let mut patience = 0;

    for epoch in 0..params.epochs {
        // データをシャッフル
        let mut shuffled = train_data.to_vec();
        shuffled.shuffle(&mut rng);

        // 訓練
        let (train_loss, train_acc) = evaluate_and_train(model, &shuffled, params);

        // テスト
        let (test_loss, test_acc) = evaluate(model, test_data);

        eprintln!(
            "{:<6} {:<12.4} {:<12.3} {:<12.4} {:<12.3}",
            epoch + 1,
            train_loss,
            train_acc,
            test_loss,
            test_acc
        );

        // 早期停止：test_loss が改善したかチェック
        if test_loss < best_test_loss {
            best_test_loss = test_loss;
            patience = 0;
        } else {
            patience += 1;
            if patience >= params.early_stop_patience {
                eprintln!("[INFO] early stopping at epoch {}", epoch + 1);
                eprintln!();
                break;
            }
        }
    }
    eprintln!();
}

/// 訓練：サンプルを処理して loss/acc を計算、同時にパラメータ更新
fn evaluate_and_train(
    model: &mut Model,
    data: &[(String, f64)],
    params: &HyperParams,
) -> (f64, f64) {
    let mut loss = 0.0;
    let mut correct = 0usize;

    for (text, label) in data {
        let features = vectorize(text, &model.vocab, model.n_min, model.n_max);
        let prob = model.predict_prob(&features);

        // BCE loss
        loss += bce(prob, *label);

        // 精度
        let pred_label = if prob >= 0.5 { 1.0_f64 } else { 0.0_f64 };
        if (pred_label - label).abs() < 0.5 {
            correct += 1;
        }

        // SGD 更新
        model.update(&features, *label, params);
    }

    let n = data.len() as f64;
    (loss / n, correct as f64 / n)
}

/// テスト：loss/acc を計算（パラメータ更新なし）
fn evaluate(model: &Model, data: &[(String, f64)]) -> (f64, f64) {
    let mut loss = 0.0;
    let mut correct = 0usize;

    for (text, label) in data {
        let features = vectorize(text, &model.vocab, model.n_min, model.n_max);
        let prob = model.predict_prob(&features);

        loss += bce(prob, *label);

        let pred_label = if prob >= 0.5 { 1.0_f64 } else { 0.0_f64 };
        if (pred_label - label).abs() < 0.5 {
            correct += 1;
        }
    }

    let n = data.len() as f64;
    (loss / n, correct as f64 / n)
}

/// 学習済みモデルを使って推論
pub fn predict(title: &str) -> Result<(), Box<dyn std::error::Error>> {
    // モデル読み込み
    let model = load_model(MODEL_OUTPUT)?;

    // 前処理
    let normalized = normalize(title);

    // ベクトル化（モデルの n_min/n_max を使用）
    let features = vectorize(&normalized, &model.vocab, model.n_min, model.n_max);

    if features.is_empty() {
        eprintln!("[WARN] title contains no known n-grams");
        println!("Score: 0.50, Label: 0");
        return Ok(());
    }

    // 推論
    let prob = model.predict_prob(&features);
    let label = if prob >= 0.5 { 1 } else { 0 };

    println!("Score: {:.2}, Label: {}", prob, label);
    Ok(())
}

fn load_model(path: &str) -> Result<Model, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

/// Binary Cross Entropy loss（数値安定化）
/// prob を [1e-10, 1-1e-10] の範囲でクリップして log の未定義を防ぐ
fn bce(prob: f64, label: f64) -> f64 {
    let prob = prob.clamp(1e-10, 1.0 - 1e-10);
    -label * prob.ln() - (1.0 - label) * (1.0 - prob).ln()
}

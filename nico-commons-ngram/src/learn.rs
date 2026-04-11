use crate::model::{HyperParams, Metrics, Model};
use crate::ngram;
use crate::classifier;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

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

    // 5. IDF 構築（TF-IDF を使う場合）
    let idf = if params.use_tfidf {
        build_idf(&train_data, &vocab, params.n_min, params.n_max)
    } else {
        vec![]
    };

    // 6. 訓練
    let mut model = Model::new(vocab, params.n_min, params.n_max, idf);
    train(&mut model, &train_data, &test_data, &params);

    // 7. モデル保存
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
fn build_vocab(samples: &[(String, f64)], n_min: usize, n_max: usize) -> HashMap<String, usize> {
    let mut vocab = HashMap::new();
    for (text, _) in samples {
        for gram in ngram::extract(text, n_min, n_max) {
            let next = vocab.len();
            vocab.entry(gram).or_insert(next);
        }
    }
    vocab
}

/// IDF スコアを計算（BM25 スムージング付き）
fn build_idf(
    samples: &[(String, f64)],
    vocab: &HashMap<String, usize>,
    n_min: usize,
    n_max: usize,
) -> Vec<f64> {
    let n = samples.len() as f64;
    let mut df = vec![0usize; vocab.len()];

    for (text, _) in samples {
        let grams = ngram::extract(text, n_min, n_max);
        for gram in grams {
            if let Some(&i) = vocab.get(&gram) {
                df[i] = df[i].saturating_add(1);
            }
        }
    }

    df.iter()
        .map(|&d| (n / (d as f64 + 1.0)).ln() + 1.0)
        .collect()
}

/// テキストを語彙に基づきインデックス列に変換（重み付き特徴量）
fn vectorize(
    text: &str,
    vocab: &HashMap<String, usize>,
    idf: &[f64],
    n_min: usize,
    n_max: usize,
) -> Vec<(usize, f64)> {
    let grams = ngram::extract(text, n_min, n_max);
    let total = grams.len() as f64;

    if total == 0.0 {
        return vec![];
    }

    let mut counts: HashMap<usize, usize> = HashMap::new();
    for gram in grams {
        if let Some(&i) = vocab.get(&gram) {
            *counts.entry(i).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .map(|(i, count)| {
            let tf = count as f64 / total;
            let w = if idf.is_empty() {
                1.0 // binary: weight = 1.0
            } else {
                tf * idf[i] // TF-IDF: weight = TF * IDF
            };
            (i, w)
        })
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
    eprintln!(
        "{:<6} {:<10} {:<10} {:<10} {:<10}",
        "epoch", "train_f1", "train_acc", "test_f1", "test_acc"
    );
    eprintln!("{}", "-".repeat(60));

    let mut rng = rand::thread_rng();
    let mut best_test_f1 = 0.0_f64;
    let mut patience = 0;

    for epoch in 0..params.epochs {
        // データをシャッフル
        let mut shuffled = train_data.to_vec();
        shuffled.shuffle(&mut rng);

        // 訓練
        let train_metrics = evaluate_and_train(model, &shuffled, params);

        // テスト
        let test_metrics = evaluate(model, test_data, params);

        eprintln!(
            "{:<6} {:<10.3} {:<10.3} {:<10.3} {:<10.3}",
            epoch + 1,
            train_metrics.f1,
            train_metrics.accuracy,
            test_metrics.f1,
            test_metrics.accuracy
        );

        // 早期停止：test_f1 が改善したかチェック
        if test_metrics.f1 > best_test_f1 {
            best_test_f1 = test_metrics.f1;
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

/// 訓練：サンプルを処理して Metrics を計算、同時にパラメータ更新
fn evaluate_and_train(model: &mut Model, data: &[(String, f64)], params: &HyperParams) -> Metrics {
    let mut preds = Vec::new();

    for (text, label) in data {
        let features = vectorize(text, &model.vocab, &model.idf, model.n_min, model.n_max);
        let prob = model.predict_prob(&features);
        preds.push((prob, *label));

        // SGD 更新
        model.update(&features, *label, params);
    }

    Metrics::compute(&preds)
}

/// テスト：Metrics を計算（パラメータ更新なし）
fn evaluate(model: &Model, data: &[(String, f64)], _params: &HyperParams) -> Metrics {
    let mut preds = Vec::new();

    for (text, label) in data {
        let features = vectorize(text, &model.vocab, &model.idf, model.n_min, model.n_max);
        let prob = model.predict_prob(&features);
        preds.push((prob, *label));
    }

    Metrics::compute(&preds)
}

/// 学習済みモデルを使って推論
pub fn predict(title: &str) -> Result<(), Box<dyn std::error::Error>> {
    // モデル読み込み
    let model = load_model(MODEL_OUTPUT)?;

    // 前処理
    let normalized = normalize(title);

    // ベクトル化（モデルの n_min/n_max を使用）
    let features = vectorize(
        &normalized,
        &model.vocab,
        &model.idf,
        model.n_min,
        model.n_max,
    );

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

/// グリッドサーチでハイパーパラメータを最適化
pub fn tune() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[INFO] starting hyperparameter grid search...");

    // データセット読み込み
    let samples = load_dataset(DATASET_PATH)?;
    let mut processed: Vec<(String, f64)> = samples
        .into_iter()
        .map(|s| {
            let label = match s.label {
                0 | 1 => s.label as f64,
                _ => 0.0,
            };
            (normalize(&s.title), label)
        })
        .collect();

    // シャッフルと分割（全体で 1 回だけ）
    processed.shuffle(&mut rand::thread_rng());
    let split_idx = (processed.len() as f64 * 0.8) as usize;
    let (train_data, test_data) = processed.split_at(split_idx);
    let (train_data, test_data) = (train_data.to_vec(), test_data.to_vec());

    // 語彙構築
    let vocab = build_vocab(&train_data, 3, 5);

    // グリッドサーチパラメータ
    let lrs = [0.01, 0.05, 0.1, 0.5];
    let lambdas = [1e-5, 1e-4, 1e-3, 1e-2];

    eprintln!();
    eprintln!(
        "{:<10} {:<12} {:<10} {:<10}",
        "learning_rate", "lambda", "test_f1", "test_acc"
    );
    eprintln!("{}", "-".repeat(50));

    let mut best_f1 = 0.0;
    let mut best_lr = 0.0;
    let mut best_lambda = 0.0;

    for &lr in &lrs {
        for &lambda in &lambdas {
            let params = HyperParams {
                learning_rate: lr,
                lambda,
                ..Default::default()
            };

            let idf = vec![]; // バイナリモード
            let mut model = Model::new(vocab.clone(), 3, 5, idf);
            train_simple(&mut model, &train_data, &params);

            let test_metrics = evaluate(&model, &test_data, &params);

            eprintln!(
                "{:<10.4} {:<12.0e} {:<10.3} {:<10.3}",
                lr, lambda, test_metrics.f1, test_metrics.accuracy
            );

            if test_metrics.f1 > best_f1 {
                best_f1 = test_metrics.f1;
                best_lr = lr;
                best_lambda = lambda;
            }
        }
    }

    eprintln!();
    eprintln!(
        "[INFO] best params: lr={}, lambda={}, test_f1={:.3}",
        best_lr, best_lambda, best_f1
    );
    eprintln!();

    Ok(())
}

/// 訓練ループ（ログなし）
fn train_simple(model: &mut Model, train_data: &[(String, f64)], params: &HyperParams) {
    let mut rng = rand::thread_rng();

    for _ in 0..params.epochs {
        let mut shuffled = train_data.to_vec();
        shuffled.shuffle(&mut rng);

        for (text, label) in &shuffled {
            let features = vectorize(text, &model.vocab, &model.idf, model.n_min, model.n_max);
            let prob = model.predict_prob(&features);
            if (prob - label).abs() > 1e-6 {
                model.update(&features, *label, params);
            }
        }
    }
}

/// k-fold クロスバリデーション
pub fn cross_validate(k: usize) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[INFO] starting {}-fold cross-validation...", k);

    // データセット読み込み
    let samples = load_dataset(DATASET_PATH)?;
    let mut processed: Vec<(String, f64)> = samples
        .into_iter()
        .map(|s| {
            let label = match s.label {
                0 | 1 => s.label as f64,
                _ => 0.0,
            };
            (normalize(&s.title), label)
        })
        .collect();

    // シャッフル
    processed.shuffle(&mut rand::thread_rng());
    let fold_size = processed.len() / k;

    let mut all_results = Vec::new();

    eprintln!();
    eprintln!(
        "{:<6} {:<10} {:<10} {:<10} {:<10}",
        "fold", "test_f1", "test_acc", "test_prec", "test_rec"
    );
    eprintln!("{}", "-".repeat(50));

    for fold in 0..k {
        let test_start = fold * fold_size;
        let test_end = if fold == k - 1 {
            processed.len()
        } else {
            (fold + 1) * fold_size
        };

        let test_data = processed[test_start..test_end].to_vec();
        let mut train_data = Vec::new();
        train_data.extend_from_slice(&processed[0..test_start]);
        train_data.extend_from_slice(&processed[test_end..]);

        // 語彙構築
        let vocab = build_vocab(&train_data, 3, 5);

        // 訓練
        let params = HyperParams::default();
        let idf = vec![];
        let mut model = Model::new(vocab, 3, 5, idf);
        train_simple(&mut model, &train_data, &params);

        // 評価
        let metrics = evaluate(&model, &test_data, &params);
        eprintln!(
            "{:<6} {:<10.3} {:<10.3} {:<10.3} {:<10.3}",
            fold + 1,
            metrics.f1,
            metrics.accuracy,
            metrics.precision,
            metrics.recall
        );

        all_results.push(metrics);
    }

    // 平均
    let avg_f1 = all_results.iter().map(|m| m.f1).sum::<f64>() / k as f64;
    let avg_acc = all_results.iter().map(|m| m.accuracy).sum::<f64>() / k as f64;
    let avg_prec = all_results.iter().map(|m| m.precision).sum::<f64>() / k as f64;
    let avg_rec = all_results.iter().map(|m| m.recall).sum::<f64>() / k as f64;

    eprintln!();
    eprintln!(
        "[INFO] average: f1={:.3}, acc={:.3}, prec={:.3}, rec={:.3}",
        avg_f1, avg_acc, avg_prec, avg_rec
    );
    eprintln!();

    Ok(())
}

/// ルールベース vs ニューラルモデルの性能比較
pub fn compare() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[INFO] comparing rule-based vs neural model...");

    // データセット読み込み
    let samples = load_dataset(DATASET_PATH)?;
    let mut processed: Vec<(String, f64)> = samples
        .into_iter()
        .map(|s| {
            let label = match s.label {
                0 | 1 => s.label as f64,
                _ => 0.0,
            };
            (normalize(&s.title), label)
        })
        .collect();

    // シャッフルと分割
    processed.shuffle(&mut rand::thread_rng());
    let split_idx = (processed.len() as f64 * 0.8) as usize;
    let (_train_data, test_data) = processed.split_at(split_idx);
    let test_data = test_data.to_vec();

    eprintln!("[INFO] evaluating on {} test samples\n", test_data.len());

    // === Rule-Based Classifier ===
    eprintln!("=== Rule-Based Classifier (keyword matching) ===");
    let mut rule_preds = Vec::new();
    for (title, label) in &test_data {
        let pred = classifier::classify(title) as i32 as f64;
        rule_preds.push((pred, *label));
    }
    let rule_metrics = Metrics::compute(&rule_preds);

    eprintln!(
        "Precision: {:.3}  Recall: {:.3}  F1: {:.3}  Accuracy: {:.3}\n",
        rule_metrics.precision, rule_metrics.recall, rule_metrics.f1, rule_metrics.accuracy
    );

    // === Neural Model ===
    eprintln!("=== Neural Model (n-gram LR) ===");
    let model = load_model(MODEL_OUTPUT)?;
    let mut nn_preds = Vec::new();
    for (title, label) in &test_data {
        let features = vectorize(title, &model.vocab, &model.idf, model.n_min, model.n_max);
        let prob = model.predict_prob(&features);
        nn_preds.push((prob, *label));
    }
    let nn_metrics = Metrics::compute(&nn_preds);

    eprintln!(
        "Precision: {:.3}  Recall: {:.3}  F1: {:.3}  Accuracy: {:.3}\n",
        nn_metrics.precision, nn_metrics.recall, nn_metrics.f1, nn_metrics.accuracy
    );

    // === 比較表 ===
    eprintln!("=== Comparison ===");
    eprintln!("{:<20} {:<12} {:<12} {:<12}", "Method", "Precision", "Recall", "F1");
    eprintln!("{}", "-".repeat(60));
    eprintln!(
        "{:<20} {:<12.3} {:<12.3} {:<12.3}",
        "Rule-Based", rule_metrics.precision, rule_metrics.recall, rule_metrics.f1
    );
    eprintln!(
        "{:<20} {:<12.3} {:<12.3} {:<12.3}",
        "Neural Model", nn_metrics.precision, nn_metrics.recall, nn_metrics.f1
    );
    eprintln!();

    // 改善度
    let f1_improvement = ((nn_metrics.f1 - rule_metrics.f1) / rule_metrics.f1 * 100.0).abs();
    let acc_improvement = ((nn_metrics.accuracy - rule_metrics.accuracy) / rule_metrics.accuracy * 100.0).abs();

    if nn_metrics.f1 > rule_metrics.f1 {
        eprintln!(
            "[INFO] Neural model is +{:.1}% better in F1 (+{:.1}% in accuracy)",
            f1_improvement, acc_improvement
        );
    } else {
        eprintln!(
            "[INFO] Rule-based is +{:.1}% better in F1",
            f1_improvement
        );
    }
    eprintln!();

    Ok(())
}

/// プルーニング済みモデルを JavaScript ファイルとしてエクスポート
pub fn export(threshold: f64, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[INFO] exporting model with threshold={}", threshold);

    // モデル読み込み
    let model = load_model(MODEL_OUTPUT)?;
    let orig_vocab_size = model.vocab.len();

    // プルーニング：|weight| >= threshold のインデックスを抽出
    let mut kept_indices = Vec::new();
    for (old_idx, &weight) in model.weights.iter().enumerate() {
        if weight.abs() >= threshold {
            kept_indices.push(old_idx);
        }
    }

    eprintln!(
        "[INFO] keeping {} / {} weights",
        kept_indices.len(),
        orig_vocab_size
    );

    // old_idx → new_idx のマッピング
    let mut old_to_new = vec![-1i32; orig_vocab_size];
    for (new_idx, &old_idx) in kept_indices.iter().enumerate() {
        old_to_new[old_idx] = new_idx as i32;
    }

    // 新しい vocab と weights を構築
    let mut new_vocab = HashMap::new();
    let mut new_weights = Vec::new();

    for (gram, &old_idx) in &model.vocab {
        if old_to_new[old_idx] >= 0 {
            let new_idx = old_to_new[old_idx] as usize;
            new_vocab.insert(gram.clone(), new_idx);
            new_weights.push(model.weights[old_idx]);
        }
    }

    // JSON 出力
    let export_model = serde_json::json!({
        "bias": model.bias,
        "n_min": model.n_min,
        "n_max": model.n_max,
        "vocab": new_vocab,
        "weights": new_weights,
    });

    // JS ファイル形式で出力
    let js_content = format!(
        "// @generated by nico-commons-ngram export\n\
         // threshold={}, vocab={}/{}, weights={}\n\
         const MODEL = {};\n",
        threshold,
        new_vocab.len(),
        orig_vocab_size,
        new_weights.len(),
        serde_json::to_string_pretty(&export_model)?
    );

    std::fs::write(output_path, &js_content)?;
    eprintln!("[INFO] exported to {}", output_path);

    let file_size = std::fs::metadata(output_path)?.len();
    eprintln!("[INFO] file size: {:.1}KB", file_size as f64 / 1024.0);

    Ok(())
}

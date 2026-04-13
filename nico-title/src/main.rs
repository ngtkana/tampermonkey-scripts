mod download;
mod analyze;
mod extract;
mod annotate;
mod compare;
mod bio;
mod features;
mod crf;
mod analyze_results;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Download Vocaloid song titles from NicoVideo API
    Download,
    /// Analyze title patterns in the dataset
    Analyze,
    /// Extract song titles using rule-based approach
    Extract,
    /// Generate annotations using Claude API
    #[command(name = "annotate")]
    Annotate {
        /// Number of titles to annotate (default: 200)
        #[arg(short, long, default_value = "200")]
        count: usize,
    },
    /// Compare rule-based and LLM extractions
    Compare,
    /// Convert annotations to BIO tags
    #[command(name = "bio-convert")]
    BioConvert {
        /// Input JSONL file (default: data/nico_api_annotations.jsonl)
        #[arg(short, long, default_value = "data/nico_api_annotations.jsonl")]
        input: String,
        /// Output file (default: data/nico_bio_tags.jsonl)
        #[arg(short, long, default_value = "data/nico_bio_tags.jsonl")]
        output: String,
    },
    /// Train CRF model
    #[command(name = "crf-learn")]
    CrfLearn {
        /// Input BIO tags file (default: data/nico_bio_tags.jsonl)
        #[arg(short, long, default_value = "data/nico_bio_tags.jsonl")]
        input: String,
        /// Output model file (default: data/crf_model.json)
        #[arg(short, long, default_value = "data/crf_model.json")]
        output: String,
        /// Learning rate (default: 0.01)
        #[arg(long, default_value = "0.01")]
        learning_rate: f64,
        /// L2 regularization (default: 0.001)
        #[arg(long, default_value = "0.001")]
        lambda: f64,
        /// Epochs (default: 20)
        #[arg(long, default_value = "20")]
        epochs: usize,
    },
    /// Evaluate CRF model (test accuracy)
    #[command(name = "crf-eval")]
    CrfEval {
        /// Input BIO tags file (default: data/nico_bio_tags.jsonl)
        #[arg(short, long, default_value = "data/nico_bio_tags.jsonl")]
        input: String,
        /// Model file (default: data/crf_model.json)
        #[arg(short, long, default_value = "data/crf_model.json")]
        model: String,
        /// Test split ratio (default: 0.2)
        #[arg(long, default_value = "0.2")]
        test_ratio: f64,
    },
    /// Analyze CRF predictions vs LLM annotations
    #[command(name = "analyze-results")]
    AnalyzeResults {
        /// Input BIO tags file (default: data/nico_bio_tags.jsonl)
        #[arg(short, long, default_value = "data/nico_bio_tags.jsonl")]
        input: String,
        /// Model file (default: data/crf_model.json)
        #[arg(short, long, default_value = "data/crf_model.json")]
        model: String,
        /// Output analysis file (default: data/analysis_results.jsonl)
        #[arg(short, long, default_value = "data/analysis_results.jsonl")]
        output: String,
    },
    /// Show mismatch examples
    #[command(name = "show-mismatches")]
    ShowMismatches {
        /// Input mismatch file (default: data/analysis_mismatches.jsonl)
        #[arg(short, long, default_value = "data/analysis_mismatches.jsonl")]
        input: String,
        /// Number of examples per category (default: 5)
        #[arg(short, long, default_value = "5")]
        count: usize,
    },
    /// Find suspicious annotations (likely incorrect LLM extractions)
    #[command(name = "find-suspicious")]
    FindSuspicious {
        /// Input mismatches file (default: data/analysis_mismatches.jsonl)
        #[arg(short, long, default_value = "data/analysis_mismatches.jsonl")]
        input: String,
        /// Output file (default: data/suspicious_annotations.jsonl)
        #[arg(short, long, default_value = "data/suspicious_annotations.jsonl")]
        output: String,
    },
    /// Check BIO conversion correctness
    #[command(name = "check-bio-conversion")]
    CheckBioConversion {
        /// BIO file (default: data/nico_bio_tags.jsonl)
        #[arg(short, long, default_value = "data/nico_bio_tags.jsonl")]
        bio_file: String,
        /// Mismatches file (default: data/analysis_mismatches.jsonl)
        #[arg(short, long, default_value = "data/analysis_mismatches.jsonl")]
        mismatches_file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Download) => download::download(),
        Some(Commands::Analyze) => analyze::analyze_patterns(),
        Some(Commands::Extract) => extract_all_titles(),
        Some(Commands::Annotate { count }) => annotate::annotate_titles(count),
        Some(Commands::Compare) => compare::compare_methods(),
        Some(Commands::BioConvert { input, output }) => bio::convert_bio(&input, &output),
        Some(Commands::CrfLearn {
            input,
            output,
            learning_rate,
            lambda,
            epochs,
        }) => train_crf(&input, &output, learning_rate, lambda, epochs),
        Some(Commands::CrfEval {
            input,
            model: model_file,
            test_ratio,
        }) => evaluate_crf(&input, &model_file, test_ratio),
        Some(Commands::AnalyzeResults {
            input,
            model,
            output,
        }) => analyze_results::analyze(&input, &model, &output),
        Some(Commands::ShowMismatches { input, count }) => {
            analyze_results::show_mismatches(&input, count);
        }
        Some(Commands::FindSuspicious { input, output }) => {
            analyze_results::find_suspicious(&input, &output);
        }
        Some(Commands::CheckBioConversion {
            bio_file,
            mismatches_file,
        }) => {
            analyze_results::check_bio_conversion(&bio_file, &mismatches_file);
        }
        None => println!("No command specified. Use --help for usage information."),
    }
}

fn extract_all_titles() {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};

    let input_file = match File::open("data/nico_api_result.tsv") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open input file: {}", e);
            return;
        }
    };

    let reader = BufReader::new(input_file);
    let mut output_file = match File::create("data/nico_api_extracted.jsonl") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
            return;
        }
    };

    let mut count = 0;
    for (i, line) in reader.lines().enumerate() {
        if i == 0 {
            continue; // Skip header
        }

        match line {
            Ok(l) => {
                let parts: Vec<&str> = l.split('\t').collect();
                if parts.len() >= 2 {
                    let content_id = parts[0];
                    let title = parts[1];
                    let extracted = extract::extract_song_title(title);

                    let json_output = serde_json::json!({
                        "content_id": content_id,
                        "title": title,
                        "extracted_title": extracted
                    });

                    if let Err(e) = writeln!(output_file, "{}", json_output.to_string()) {
                        eprintln!("Failed to write line: {}", e);
                        return;
                    }
                    count += 1;
                }
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
            }
        }
    }

    println!("Successfully extracted {} titles to data/nico_api_extracted.jsonl", count);
}

fn evaluate_crf(input_file: &str, model_file: &str, test_ratio: f64) {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Read};

    // BIO タグ付きデータを読み込む
    let file = match File::open(input_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open input file: {}", e);
            return;
        }
    };

    let reader = BufReader::new(file);
    let mut documents = Vec::new();
    let mut feature_extractor = features::FeatureExtractor::new();

    println!("Loading data...");
    for (line_num, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading line {}: {}", line_num + 1, e);
                continue;
            }
        };

        match serde_json::from_str::<bio::BioDocument>(&line) {
            Ok(doc) => {
                let chars: Vec<char> = doc.title.chars().collect();
                let mut sequences = Vec::new();
                let mut labels = Vec::new();

                for i in 0..chars.len() {
                    let features = feature_extractor.extract_features(&doc.title, i);
                    sequences.push(features);

                    if let Some(label) = crf::Label::from_str(&doc.tokens[i].tag) {
                        labels.push(label);
                    }
                }

                documents.push((sequences, labels, doc.title.clone()));
            }
            Err(e) => {
                eprintln!("JSON parse error at line {}: {}", line_num + 1, e);
            }
        }
    }

    println!("Loaded {} documents\n", documents.len());

    // テスト・訓練データを分割
    let test_size = (documents.len() as f64 * test_ratio) as usize;
    let (test_docs, _train_docs) = documents.split_at(test_size);

    // モデルを読み込む
    let model: crf::CrfModel = match File::open(model_file) {
        Ok(mut f) => {
            let mut content = String::new();
            f.read_to_string(&mut content).expect("Failed to read model");
            serde_json::from_str(&content).expect("Failed to parse model JSON")
        }
        Err(e) => {
            eprintln!("Failed to open model file: {}", e);
            return;
        }
    };

    println!("Model loaded from {}\n", model_file);

    // 評価指標
    let mut tp = 0;  // B タグが正解
    let mut fp = 0;  // B タグを予測したが不正解
    let mut fn_count = 0;  // B タグを逃した

    for (sequence, gold_labels, _title) in test_docs {
        let pred_labels = model.viterbi(sequence);

        // スパンベースの評価（B タグが曲名の開始を示す）
        for i in 0..gold_labels.len() {
            let gold_is_b = matches!(gold_labels[i], crf::Label::B);
            let pred_is_b = matches!(pred_labels[i], crf::Label::B);

            if gold_is_b && pred_is_b {
                tp += 1;
            } else if pred_is_b && !gold_is_b {
                fp += 1;
            } else if gold_is_b && !pred_is_b {
                fn_count += 1;
            }
        }
    }

    // F1 計算
    let precision = if tp + fp == 0 { 0.0 } else { tp as f64 / (tp + fp) as f64 };
    let recall = if tp + fn_count == 0 { 0.0 } else { tp as f64 / (tp + fn_count) as f64 };
    let f1 = if precision + recall == 0.0 { 0.0 } else { 2.0 * precision * recall / (precision + recall) };

    println!("=== CRF 評価結果 ===");
    println!("テスト件数: {}", test_docs.len());
    println!("TP (正解): {}", tp);
    println!("FP (誤検知): {}", fp);
    println!("FN (見落とし): {}\n", fn_count);

    println!("Precision: {:.1}%", precision * 100.0);
    println!("Recall:    {:.1}%", recall * 100.0);
    println!("F1 score:  {:.1}%\n", f1 * 100.0);

    println!("=== ルールベース比較 ===");
    println!("ルールベース（extract.rs）: 78.1% (compare コマンドより)");
    println!("CRF モデル:                 {:.1}%", f1 * 100.0);

    if f1 > 0.78 {
        println!("\n✅ CRF がルールベースを上回りました！");
        println!("改善度: {:.1} ポイント", (f1 - 0.78) * 100.0);
    } else {
        println!("\n⚠️  ルールベースの方が精度が高い結果です");
        println!("差分: {:.1} ポイント", (0.78 - f1) * 100.0);
    }
}

fn train_crf(
    input_file: &str,
    output_file: &str,
    learning_rate: f64,
    lambda: f64,
    epochs: usize,
) {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};

    // BIO タグ付きデータを読み込む
    let file = match File::open(input_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open input file: {}", e);
            return;
        }
    };

    let reader = BufReader::new(file);
    let mut documents = Vec::new();
    let mut feature_extractor = features::FeatureExtractor::new();

    println!("Loading training data...");
    for (line_num, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading line {}: {}", line_num + 1, e);
                continue;
            }
        };

        match serde_json::from_str::<bio::BioDocument>(&line) {
            Ok(doc) => {
                // 各文字の特徴量を抽出
                let chars: Vec<char> = doc.title.chars().collect();
                let mut sequences = Vec::new();
                let mut labels = Vec::new();

                for (i, _ch) in chars.iter().enumerate() {
                    let features = feature_extractor.extract_features(&doc.title, i);
                    sequences.push(features);

                    if let Some(label) = crf::Label::from_str(&doc.tokens[i].tag) {
                        labels.push(label);
                    }
                }

                documents.push((sequences, labels));
            }
            Err(e) => {
                eprintln!("JSON parse error at line {}: {}", line_num + 1, e);
            }
        }
    }

    println!("Loaded {} documents", documents.len());
    println!("Feature map size: {}", feature_extractor.feature_map.len());

    // CRF モデルを作成・学習
    let mut model = crf::CrfModel::new(feature_extractor.feature_map.clone(), learning_rate, lambda);

    println!("Training CRF for {} epochs...", epochs);
    let batch_size = 32;

    for epoch in 0..epochs {
        let mut total_nll = 0.0;

        // ミニバッチで学習
        for batch_docs in documents.chunks(batch_size) {
            model.train_step(batch_docs);

            // NLL を計算
            for (sequence, labels) in batch_docs {
                total_nll += model.nll(sequence, labels);
            }
        }

        let avg_nll = total_nll / documents.len() as f64;
        println!("Epoch {}: NLL = {:.4}", epoch + 1, avg_nll);
    }

    // モデルをファイルに保存
    println!("Saving model to {}", output_file);
    let model_json = serde_json::json!({
        "feature_weights": model.feature_weights,
        "transition": model.transition,
        "feature_map": model.feature_map,
        "learning_rate": model.learning_rate,
        "lambda": model.lambda,
    });

    match File::create(output_file) {
        Ok(mut f) => {
            if let Err(e) = writeln!(f, "{}", model_json) {
                eprintln!("Failed to write model file: {}", e);
            } else {
                println!("Model saved successfully");
            }
        }
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
        }
    }
}

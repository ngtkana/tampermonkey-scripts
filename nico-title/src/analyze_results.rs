use crate::{bio, crf, features};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisRow {
    pub title: String,
    pub llm_extracted: String,
    pub crf_extracted: String,
    pub match_llm_crf: bool,
    pub gold_labels: Vec<String>,
    pub pred_labels: Vec<String>,
}

/// BIO タグからスパンを抽出
fn extract_span_from_bio(title: &str, labels: &[crf::Label]) -> String {
    let chars: Vec<char> = title.chars().collect();
    let mut start = None;

    for (i, &label) in labels.iter().enumerate() {
        if label == crf::Label::B {
            start = Some(i);
        } else if label == crf::Label::O && start.is_some() {
            // スパンを終了
            if let Some(s) = start {
                return chars[s..i].iter().collect();
            }
            start = None;
        }
    }

    // 末尾までのスパンを返す
    if let Some(s) = start {
        chars[s..].iter().collect()
    } else {
        String::new()
    }
}

pub fn analyze(input_file: &str, model_file: &str, output_file: &str) {
    use std::io::Read;

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

    // BIO タグ付きデータを読み込む
    let file = match File::open(input_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open input file: {}", e);
            return;
        }
    };

    let reader = BufReader::new(file);
    let mut output = match File::create(output_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
            return;
        }
    };

    let mut feature_extractor = features::FeatureExtractor::new();
    let mut total = 0;
    let mut mismatches = 0;

    println!("Analyzing predictions...\n");

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
                let mut gold_labels = Vec::new();

                // 特徴量と正解ラベルを抽出
                for i in 0..chars.len() {
                    let features = feature_extractor.extract_features(&doc.title, i);
                    sequences.push(features);

                    if let Some(label) = crf::Label::from_str(&doc.tokens[i].tag) {
                        gold_labels.push(label);
                    }
                }

                // CRF で予測
                let pred_labels = model.viterbi(&sequences);

                // LLM の抽出結果
                let llm_extracted = doc.extracted_title.clone();

                // CRF の抽出結果（最初の B-I スパン）
                let crf_extracted = extract_span_from_bio(&doc.title, &pred_labels);

                // マッチ判定
                let match_llm_crf = llm_extracted == crf_extracted;
                if !match_llm_crf {
                    mismatches += 1;
                }
                total += 1;

                let analysis = AnalysisRow {
                    title: doc.title.clone(),
                    llm_extracted,
                    crf_extracted,
                    match_llm_crf,
                    gold_labels: gold_labels
                        .iter()
                        .map(|l| {
                            match l {
                                crf::Label::B => "B",
                                crf::Label::I => "I",
                                crf::Label::O => "O",
                            }
                            .to_string()
                        })
                        .collect(),
                    pred_labels: pred_labels
                        .iter()
                        .map(|l| {
                            match l {
                                crf::Label::B => "B",
                                crf::Label::I => "I",
                                crf::Label::O => "O",
                            }
                            .to_string()
                        })
                        .collect(),
                };

                if let Err(e) = writeln!(
                    output,
                    "{}",
                    serde_json::to_string(&analysis).unwrap_or_default()
                ) {
                    eprintln!("Failed to write result: {}", e);
                }
            }
            Err(e) => {
                eprintln!("JSON parse error at line {}: {}", line_num + 1, e);
            }
        }
    }

    println!("=== 分析結果 ===");
    println!("総件数: {}", total);
    println!("LLM と CRF が一致: {} ({}%)", total - mismatches,
        if total > 0 { ((total - mismatches) as f64 / total as f64 * 100.0) as u32 } else { 0 });
    println!("LLM と CRF が不一致: {} ({}%)\n", mismatches,
        if total > 0 { (mismatches as f64 / total as f64 * 100.0) as u32 } else { 0 });

    println!("分析結果を {} に出力しました", output_file);

    // 別ファイルに不一致ケースだけを出力
    if let Ok(mut mismatch_file) = File::create("data/analysis_mismatches.jsonl") {
        let file = match File::open(input_file) {
            Ok(f) => f,
            Err(_) => return,
        };

        let reader = BufReader::new(file);
        let mut feature_extractor = features::FeatureExtractor::new();

        for line in reader.lines().flatten() {
            if let Ok(doc) = serde_json::from_str::<bio::BioDocument>(&line) {
                let chars: Vec<char> = doc.title.chars().collect();
                let mut sequences = Vec::new();
                let mut gold_labels = Vec::new();

                for i in 0..chars.len() {
                    let features = feature_extractor.extract_features(&doc.title, i);
                    sequences.push(features);

                    if let Some(label) = crf::Label::from_str(&doc.tokens[i].tag) {
                        gold_labels.push(label);
                    }
                }

                let pred_labels = model.viterbi(&sequences);
                let llm_extracted = doc.extracted_title.clone();
                let crf_extracted = extract_span_from_bio(&doc.title, &pred_labels);

                if llm_extracted != crf_extracted {
                    let mismatch = serde_json::json!({
                        "title": doc.title,
                        "llm_extracted": llm_extracted,
                        "crf_extracted": crf_extracted,
                        "gold_tags": doc.tokens.iter().map(|t| t.tag.as_str()).collect::<Vec<_>>(),
                        "pred_labels": pred_labels.iter().map(|l| {
                            match l {
                                crf::Label::B => "B",
                                crf::Label::I => "I",
                                crf::Label::O => "O",
                            }
                        }).collect::<Vec<_>>(),
                    });

                    let _ = writeln!(mismatch_file, "{}", mismatch.to_string());
                }
            }
        }

        println!("不一致ケースを data/analysis_mismatches.jsonl に出力しました");
    }
}

pub fn show_mismatches(input_file: &str, count: usize) {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = match File::open(input_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open file: {}", e);
            return;
        }
    };

    let reader = BufReader::new(file);
    let mut empty_crf = Vec::new();
    let mut partial_crf = Vec::new();
    let mut different = Vec::new();

    // Parse JSON and categorize
    for line in reader.lines().flatten() {
        if let Ok(row) = serde_json::from_str::<serde_json::Value>(&line) {
            let crf_extracted = row["crf_extracted"].as_str().unwrap_or("");
            let llm_extracted = row["llm_extracted"].as_str().unwrap_or("");

            if crf_extracted.is_empty() {
                empty_crf.push(row);
            } else if crf_extracted.len() < llm_extracted.len()
                && llm_extracted.starts_with(crf_extracted)
            {
                partial_crf.push(row);
            } else if crf_extracted != llm_extracted {
                different.push(row);
            }
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("CRF モデル不一致分析");
    println!("{}\n", "=".repeat(80));

    // Pattern 1: CRF が空を予測
    println!("✗ パターン1: CRF が空を予測 ({} 件)", empty_crf.len());
    println!("{}", "-".repeat(80));
    for row in empty_crf.iter().take(count) {
        println!(
            "Title: {}",
            row["title"].as_str().unwrap_or("(N/A)")
        );
        println!(
            "  LLM期待値: {}",
            row["llm_extracted"].as_str().unwrap_or("(N/A)")
        );
        println!("  CRF予測:   （空）");
        if let Some(pred) = row["pred_labels"].as_array() {
            let pred_str: String = pred
                .iter()
                .take(20)
                .filter_map(|v| v.as_str())
                .collect();
            println!("  ラベル:     {}", pred_str);
        }
        println!();
    }

    // Pattern 2: CRF が部分文字列を予測
    println!("\n✗ パターン2: CRF が部分文字列を予測 ({} 件)", partial_crf.len());
    println!("{}", "-".repeat(80));
    for row in partial_crf.iter().take(count) {
        println!(
            "Title: {}",
            row["title"].as_str().unwrap_or("(N/A)")
        );
        println!(
            "  LLM期待値: {}",
            row["llm_extracted"].as_str().unwrap_or("(N/A)")
        );
        println!(
            "  CRF予測:   {}",
            row["crf_extracted"].as_str().unwrap_or("(N/A)")
        );
        if let Some(pred) = row["pred_labels"].as_array() {
            let pred_str: String = pred
                .iter()
                .take(20)
                .filter_map(|v| v.as_str())
                .collect();
            println!("  ラベル:     {}", pred_str);
        }
        println!();
    }

    // Pattern 3: CRF が異なるスパンを予測
    println!(
        "\n✗ パターン3: CRF が異なるスパンを予測 ({} 件)",
        different.len()
    );
    println!("{}", "-".repeat(80));
    for row in different.iter().take(count) {
        println!(
            "Title: {}",
            row["title"].as_str().unwrap_or("(N/A)")
        );
        println!(
            "  LLM期待値: {}",
            row["llm_extracted"].as_str().unwrap_or("(N/A)")
        );
        println!(
            "  CRF予測:   {}",
            row["crf_extracted"].as_str().unwrap_or("(N/A)")
        );
        println!();
    }

    println!("{}", "=".repeat(80));
    println!("詳細は data/analysis_mismatches.jsonl をご覧ください");
    println!("分析報告は data/ANALYSIS_REPORT.md をご覧ください");
}

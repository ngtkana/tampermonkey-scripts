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
            f.read_to_string(&mut content)
                .expect("Failed to read model");
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
    println!(
        "LLM と CRF が一致: {} ({}%)",
        total - mismatches,
        if total > 0 {
            ((total - mismatches) as f64 / total as f64 * 100.0) as u32
        } else {
            0
        }
    );
    println!(
        "LLM と CRF が不一致: {} ({}%)\n",
        mismatches,
        if total > 0 {
            (mismatches as f64 / total as f64 * 100.0) as u32
        } else {
            0
        }
    );

    println!("分析結果を {} に出力しました", output_file);

    // 別ファイルに不一致ケースだけを出力
    if let Ok(mut mismatch_file) = File::create("data/analysis_mismatches.jsonl") {
        let file = match File::open(input_file) {
            Ok(f) => f,
            Err(_) => return,
        };

        let reader = BufReader::new(file);
        let mut feature_extractor = features::FeatureExtractor::new();

        for line in reader.lines().map_while(Result::ok) {
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

                    let _ = writeln!(mismatch_file, "{}", mismatch);
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
    for line in reader.lines().map_while(Result::ok) {
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
        println!("Title: {}", row["title"].as_str().unwrap_or("(N/A)"));
        println!(
            "  LLM期待値: {}",
            row["llm_extracted"].as_str().unwrap_or("(N/A)")
        );
        println!("  CRF予測:   （空）");
        if let Some(pred) = row["pred_labels"].as_array() {
            let pred_str: String = pred.iter().take(20).filter_map(|v| v.as_str()).collect();
            println!("  ラベル:     {}", pred_str);
        }
        println!();
    }

    // Pattern 2: CRF が部分文字列を予測
    println!(
        "\n✗ パターン2: CRF が部分文字列を予測 ({} 件)",
        partial_crf.len()
    );
    println!("{}", "-".repeat(80));
    for row in partial_crf.iter().take(count) {
        println!("Title: {}", row["title"].as_str().unwrap_or("(N/A)"));
        println!(
            "  LLM期待値: {}",
            row["llm_extracted"].as_str().unwrap_or("(N/A)")
        );
        println!(
            "  CRF予測:   {}",
            row["crf_extracted"].as_str().unwrap_or("(N/A)")
        );
        if let Some(pred) = row["pred_labels"].as_array() {
            let pred_str: String = pred.iter().take(20).filter_map(|v| v.as_str()).collect();
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
        println!("Title: {}", row["title"].as_str().unwrap_or("(N/A)"));
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

pub fn find_suspicious(input_file: &str, output_file: &str) {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};

    let file = match File::open(input_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open file: {}", e);
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

    let mut suspicious_count = 0;
    let mut gold_starts_with_o = 0; // Gold が O で始まる（曲名が後ろ）
    let mut both_nonempty_differ = 0; // 両方が非空だが異なる

    println!("\n{}", "=".repeat(80));
    println!("怪しいAnnotation検出");
    println!("{}\n", "=".repeat(80));

    let empty_vec = vec![];

    for line in reader.lines().map_while(Result::ok) {
        if let Ok(row) = serde_json::from_str::<serde_json::Value>(&line) {
            let gold_tags = row["gold_tags"].as_array().unwrap_or(&empty_vec);
            let llm_extracted = row["llm_extracted"].as_str().unwrap_or("");
            let crf_extracted = row["crf_extracted"].as_str().unwrap_or("");
            let title = row["title"].as_str().unwrap_or("");

            let is_suspicious = if gold_tags.is_empty() {
                false
            } else if gold_tags[0].as_str() == Some("O") && !llm_extracted.is_empty() {
                // Gold が O で始まる（曲名が後ろにある）のに、LLM が何かを抽出している
                // → LLM が誤った可能性が高い
                gold_starts_with_o += 1;
                true
            } else if !llm_extracted.is_empty()
                && !crf_extracted.is_empty()
                && llm_extracted != crf_extracted
            {
                // Gold が B で始まるなら、とりあえず LLM は正しい可能性がある
                // ただし、CRF との差が大きい場合は要確認
                if gold_tags.is_empty() || gold_tags[0].as_str() != Some("B") {
                    both_nonempty_differ += 1;
                    true
                } else {
                    false
                }
            } else {
                false
            };

            if is_suspicious {
                suspicious_count += 1;

                let record = serde_json::json!({
                    "title": title,
                    "llm_extracted": llm_extracted,
                    "crf_extracted": crf_extracted,
                    "gold_tag_start": gold_tags.first().and_then(|v| v.as_str()).unwrap_or("N/A"),
                    "reason": if gold_tags.first().and_then(|v| v.as_str()) == Some("O") {
                        "Gold が O で始まる（曲名が後ろ）のに LLM が抽出した"
                    } else {
                        "両方が非空だが異なり、Gold が不明確"
                    }
                });

                let _ = writeln!(output, "{}", record);
            }
        }
    }

    println!("怪しいAnnotation: {} 件", suspicious_count);
    println!(
        "  - Gold が O で始まる（曲名が後ろ）: {} 件",
        gold_starts_with_o
    );
    println!("  - その他（要確認）: {} 件\n", both_nonempty_differ);

    println!("詳細は {} をご覧ください", output_file);

    if suspicious_count > 0 {
        println!(
            "\n修正方法:\n\
             1. {} を開く\n\
             2. 各ケースを目視確認\n\
             3. LLM の抽出が間違っていれば、correct な値に修正\n\
             4. data/nico_api_annotations.jsonl の該当行も修正\n\
             5. bio-convert を実行して BIO タグを再生成",
            output_file
        );
    }
}

pub fn check_bio_conversion(bio_file: &str, mismatches_file: &str) {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    // BIO ファイルを読み込んでマップを作成
    let file = match File::open(bio_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open BIO file: {}", e);
            return;
        }
    };

    let reader = BufReader::new(file);
    let mut bio_data: HashMap<String, String> = HashMap::new();

    for line in reader.lines().map_while(Result::ok) {
        if let Ok(row) = serde_json::from_str::<serde_json::Value>(&line)
            && let Some(title) = row["title"].as_str()
            && let Some(extracted) = row["extracted_title"].as_str()
        {
            bio_data.insert(title.to_string(), extracted.to_string());
        }
    }

    // Mismatches ファイルを読み込んで検証
    let file = match File::open(mismatches_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open mismatches file: {}", e);
            return;
        }
    };

    let reader = BufReader::new(file);
    let mut bio_missing = 0; // BIO ファイルに見つからない
    let mut bio_mismatch = 0; // BIO ファイルの extracted が LLM と異なる
    let mut bio_correct = 0; // BIO ファイルの extracted が LLM と同じ

    println!("\n{}", "=".repeat(80));
    println!("BIO 変換の正確性チェック");
    println!("{}\n", "=".repeat(80));

    for line in reader.lines().map_while(Result::ok) {
        if let Ok(row) = serde_json::from_str::<serde_json::Value>(&line)
            && let Some(title) = row["title"].as_str()
            && let Some(llm_extracted) = row["llm_extracted"].as_str()
        {
            if let Some(bio_extracted) = bio_data.get(title) {
                if bio_extracted == llm_extracted {
                    bio_correct += 1;
                } else {
                    bio_mismatch += 1;
                    // Debug output
                    if bio_mismatch <= 3 {
                        eprintln!("BIO mismatch:");
                        eprintln!("  Title: {}", title);
                        eprintln!("  LLM:   {}", llm_extracted);
                        eprintln!("  BIO:   {}\n", bio_extracted);
                    }
                }
            } else {
                bio_missing += 1;
                if bio_missing <= 3 {
                    eprintln!("BIO missing:");
                    eprintln!("  Title: {}", title);
                    eprintln!("  LLM:   {}\n", llm_extracted);
                }
            }
        }
    }

    println!("結果:");
    println!(
        "  BIO ファイルの extracted が LLM と一致: {} 件",
        bio_correct
    );
    println!(
        "  BIO ファイルの extracted が LLM と不一致: {} 件",
        bio_mismatch
    );
    println!("  BIO ファイルに見つからない: {} 件\n", bio_missing);

    if bio_mismatch == 0 && bio_missing == 0 {
        println!("✓ BIO 変換は正常です！");
        println!(
            "  つまり、問題は **BIO 変換ではなく、LLM annotation または CRF モデル**にあります。"
        );
    } else {
        println!("✗ BIO 変換にバグがある可能性があります。");
        println!("  確認してください：");
        println!("  1. HTML エンティティの処理（&amp; → &）");
        println!("  2. Unicode 正規化（NFKC）");
        println!("  3. 部分文字列マッチの順序（複数マッチの場合）");
    }
}

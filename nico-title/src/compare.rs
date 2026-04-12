use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct AnnotationRow {
    title: String,
    extracted_title: String,
}

#[derive(Debug)]
struct ComparisonResult {
    title: String,
    rule_based: String,
    llm_based: String,
    match_flag: bool,
}

pub fn compare_methods() {
    // LLM アノテーション結果を読み込む
    let llm_results = match load_jsonl("data/nico_api_annotations.jsonl") {
        Ok(results) => results,
        Err(e) => {
            eprintln!("Failed to load LLM annotations: {}", e);
            return;
        }
    };

    // ルールベース抽出結果を読み込む（JSONL）
    let rule_results = match load_jsonl("data/nico_api_extracted.jsonl") {
        Ok(results) => results,
        Err(e) => {
            eprintln!("Failed to load rule-based extractions: {}", e);
            return;
        }
    };

    println!("LLM annotations: {} entries", llm_results.len());
    println!("Rule-based extractions: {} entries", rule_results.len());
    println!();

    // マッチングを比較
    let mut comparisons = Vec::new();
    let mut matches = 0;
    let mut mismatches = 0;

    for (title, llm_extracted) in &llm_results {
        if let Some(rule_extracted) = rule_results.get(title) {
            let is_match = llm_extracted == rule_extracted;
            comparisons.push(ComparisonResult {
                title: title.clone(),
                rule_based: rule_extracted.clone(),
                llm_based: llm_extracted.clone(),
                match_flag: is_match,
            });

            if is_match {
                matches += 1;
            } else {
                mismatches += 1;
            }
        }
    }

    // 結果を表示
    println!("Comparison Results:");
    println!("==================");
    println!("Total items compared: {}", comparisons.len());
    println!("Matches: {} ({:.1}%)", matches, matches as f64 * 100.0 / comparisons.len() as f64);
    println!("Mismatches: {} ({:.1}%)\n", mismatches, mismatches as f64 * 100.0 / comparisons.len() as f64);

    // ミスマッチの例を表示
    if mismatches > 0 {
        println!("Mismatches (showing first 15):");
        println!("==============================");
        let mut count = 0;
        for comp in &comparisons {
            if !comp.match_flag && count < 15 {
                println!();
                println!("Title: {}", comp.title);
                println!("  Rule-based:   '{}'", comp.rule_based);
                println!("  LLM-based:    '{}'", comp.llm_based);
                count += 1;
            }
        }
    }

    // 合致の例を表示
    println!("\n\nMatches (showing first 10):");
    println!("===========================");
    let mut count = 0;
    for comp in &comparisons {
        if comp.match_flag && count < 10 {
            println!("  '{}' → '{}'", comp.title, comp.rule_based);
            count += 1;
        }
    }

    // 統計情報
    println!("\n\nStatistics:");
    println!("===========");

    // 抽出されたテキストの長さの分布
    let mut rule_lengths = Vec::new();
    let mut llm_lengths = Vec::new();

    for comp in &comparisons {
        rule_lengths.push(comp.rule_based.len());
        llm_lengths.push(comp.llm_based.len());
    }

    rule_lengths.sort();
    llm_lengths.sort();

    if !rule_lengths.is_empty() {
        let mid = rule_lengths.len() / 2;
        println!("Rule-based extraction lengths:");
        println!("  Min: {}, Median: {}, Max: {}",
            rule_lengths.first().unwrap_or(&0),
            rule_lengths.get(mid).unwrap_or(&0),
            rule_lengths.last().unwrap_or(&0)
        );
    }

    if !llm_lengths.is_empty() {
        let mid = llm_lengths.len() / 2;
        println!("LLM-based extraction lengths:");
        println!("  Min: {}, Median: {}, Max: {}",
            llm_lengths.first().unwrap_or(&0),
            llm_lengths.get(mid).unwrap_or(&0),
            llm_lengths.last().unwrap_or(&0)
        );
    }
}

fn load_jsonl(file_path: &str) -> Result<HashMap<String, String>, String> {
    let file = File::open(file_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut results = HashMap::new();

    for line in reader.lines() {
        match line {
            Ok(l) => {
                if let Ok(row) = serde_json::from_str::<AnnotationRow>(&l) {
                    results.insert(row.title, row.extracted_title);
                }
            }
            Err(e) => return Err(format!("Error reading line: {}", e)),
        }
    }

    Ok(results)
}


use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn analyze_patterns() {
    let file = match File::open("data/nico_api_result.tsv") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open data file: {}", e);
            return;
        }
    };

    let reader = BufReader::new(file);
    let mut titles = Vec::new();
    let mut patterns: HashMap<&str, usize> = HashMap::new();

    // ファイルを読み込み
    for (i, line) in reader.lines().enumerate() {
        if i == 0 {
            continue; // ヘッダースキップ
        }
        match line {
            Ok(l) => {
                let parts: Vec<&str> = l.split('\t').collect();
                if parts.len() >= 2 {
                    titles.push(parts[1].to_string());
                }
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
            }
        }
    }

    println!("Total titles: {}\n", titles.len());

    // パターン分析
    for title in &titles {
        if title.contains(" / ") {
            *patterns.entry("slash_separator").or_insert(0) += 1;
        }
        if title.contains(" - ") {
            *patterns.entry("dash_separator").or_insert(0) += 1;
        }
        if title.contains("feat.") || title.contains("feat ") {
            *patterns.entry("feat").or_insert(0) += 1;
        }
        if title.contains("【") && title.contains("】") {
            *patterns.entry("japanese_brackets").or_insert(0) += 1;
        }
        if title.contains("（") && title.contains("）") {
            *patterns.entry("japanese_parens").or_insert(0) += 1;
        }
        if title.starts_with("【") {
            *patterns.entry("starts_with_bracket").or_insert(0) += 1;
        }
        if title.contains('[') && title.contains(']') {
            *patterns.entry("square_brackets").or_insert(0) += 1;
        }
        if title.contains('(') && title.contains(')') {
            *patterns.entry("round_parens").or_insert(0) += 1;
        }
    }

    // パターンをソートして出力
    let mut pattern_vec: Vec<_> = patterns.iter().collect();
    pattern_vec.sort_by_key(|&(_, count)| std::cmp::Reverse(*count));

    println!("Pattern frequency:");
    for (pattern, count) in pattern_vec {
        let pct = *count as f64 * 100.0 / titles.len() as f64;
        println!("  {:<30} {:>4} ({:>5.1}%)", pattern, count, pct);
    }

    println!("\nSample titles with specific patterns:\n");

    // スラッシュで分けるパターン
    println!("== Slash separator (曲名 / ボーカル) ==");
    let mut count = 0;
    for title in &titles {
        if title.contains(" / ") && count < 10 {
            let parts: Vec<&str> = title.split(" / ").collect();
            println!("  '{}' | '{}'", parts[0], parts.get(1).unwrap_or(&""));
            count += 1;
        }
    }

    println!("\n== Japanese brackets (【】) ==");
    count = 0;
    for title in &titles {
        if title.contains("【") && count < 10 {
            println!("  {}", title);
            count += 1;
        }
    }

    println!("\n== feat. pattern ==");
    count = 0;
    for title in &titles {
        if (title.contains("feat.") || title.contains("feat ")) && count < 10 {
            println!("  {}", title);
            count += 1;
        }
    }

    println!("\n== Dash separator (曲名 - subtitle) ==");
    count = 0;
    for title in &titles {
        if title.contains(" - ") && !title.contains("【") && count < 10 {
            let parts: Vec<&str> = title.split(" - ").collect();
            println!("  '{}' | '{}'", parts[0], parts.get(1).unwrap_or(&""));
            count += 1;
        }
    }
}

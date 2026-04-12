use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

#[derive(Debug, Serialize, Deserialize)]
pub struct AnnotationRow {
    pub title: String,
    pub extracted_title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BioToken {
    pub char: String,
    pub tag: String, // "B", "I", "O"
}

/// BIO タグ付けの結果を JSON Lines フォーマットで保存
#[derive(Debug, Serialize, Deserialize)]
pub struct BioDocument {
    pub title: String,
    pub extracted_title: String,
    pub tokens: Vec<BioToken>,
}

/// title と extracted_title から BIO タグを自動生成
///
/// 返り値: Some(Document) if extracted_title が title の部分文字列か完全一致
///        None if 自動生成できない（例外ケース）
fn convert_to_bio(title: &str, extracted_title: &str) -> Option<BioDocument> {
    // 完全一致の場合
    if title == extracted_title {
        let tokens: Vec<BioToken> = title
            .chars()
            .enumerate()
            .map(|(i, ch)| {
                let tag = if i == 0 { "B".to_string() } else { "I".to_string() };
                BioToken {
                    char: ch.to_string(),
                    tag,
                }
            })
            .collect();

        return Some(BioDocument {
            title: title.to_string(),
            extracted_title: extracted_title.to_string(),
            tokens,
        });
    }

    // 部分文字列マッチの場合
    // 文字単位での検索を行う
    let title_chars: Vec<char> = title.chars().collect();
    let extracted_chars: Vec<char> = extracted_title.chars().collect();

    // スライディングウィンドウで部分文字列を検索
    let mut start_char_idx = None;
    for i in 0..=title_chars.len().saturating_sub(extracted_chars.len()) {
        if title_chars[i..].starts_with(&extracted_chars) {
            start_char_idx = Some(i);
            break;
        }
    }

    if let Some(start_idx) = start_char_idx {
        let end_idx = start_idx + extracted_chars.len();
        let tokens: Vec<BioToken> = title_chars
            .iter()
            .enumerate()
            .map(|(char_idx, ch)| {
                let tag = if char_idx >= start_idx && char_idx < end_idx {
                    if char_idx == start_idx { "B" } else { "I" }
                } else {
                    "O"
                };
                BioToken {
                    char: ch.to_string(),
                    tag: tag.to_string(),
                }
            })
            .collect();

        return Some(BioDocument {
            title: title.to_string(),
            extracted_title: extracted_title.to_string(),
            tokens,
        });
    }

    None
}

pub fn convert_bio(input_file: &str, output_file: &str) {
    // annotations.jsonl を読み込む
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

    let mut converted = 0;
    let mut failed = 0;
    let mut exceptions = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading line {}: {}", line_num + 1, e);
                continue;
            }
        };

        match serde_json::from_str::<AnnotationRow>(&line) {
            Ok(row) => {
                match convert_to_bio(&row.title, &row.extracted_title) {
                    Some(doc) => {
                        if let Err(e) = writeln!(output, "{}", serde_json::to_string(&doc).unwrap()) {
                            eprintln!("Failed to write line: {}", e);
                        } else {
                            converted += 1;
                        }
                    }
                    None => {
                        failed += 1;
                        exceptions.push((row.title.clone(), row.extracted_title.clone()));
                    }
                }
            }
            Err(e) => {
                eprintln!("JSON parse error at line {}: {}", line_num + 1, e);
            }
        }
    }

    println!("Converted: {} entries", converted);
    println!("Failed (exceptions): {} entries", failed);

    if !exceptions.is_empty() {
        println!("\nExceptions (first 10):");
        for (title, extracted) in exceptions.iter().take(10) {
            println!("  Title: {}", title);
            println!("  Extracted: {}", extracted);
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let doc = convert_to_bio("曲名", "曲名").unwrap();
        assert_eq!(doc.tokens.len(), 2);
        assert_eq!(doc.tokens[0].tag, "B");
        assert_eq!(doc.tokens[1].tag, "I");
    }

    #[test]
    fn test_substring_at_start() {
        let doc = convert_to_bio("曲名 / ボカロ名", "曲名").unwrap();
        assert_eq!(doc.tokens[0].tag, "B");
        assert_eq!(doc.tokens[1].tag, "I");
        assert_eq!(doc.tokens[2].tag, "O");
        assert_eq!(doc.tokens[3].tag, "O");
    }

    #[test]
    fn test_substring_in_middle() {
        let doc = convert_to_bio("【MV】曲名【オリジナル】", "曲名").unwrap();
        let b_count = doc.tokens.iter().filter(|t| t.tag == "B").count();
        let i_count = doc.tokens.iter().filter(|t| t.tag == "I").count();
        assert_eq!(b_count, 1);
        assert_eq!(i_count, 1);
    }

    #[test]
    fn test_no_match_returns_none() {
        let result = convert_to_bio("タイトル", "別の曲名");
        assert!(result.is_none());
    }
}

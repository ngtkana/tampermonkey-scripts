/// Rule-based song title extractor for Vocaloid content
///
/// Heuristic rules:
/// 1. If "曲名 / ボーカル" pattern exists, extract the left part
/// 2. Remove content in Japanese brackets 【】and parentheses
/// 3. Remove content in square brackets [MV], [Official], etc.
/// 4. If "曲名 - subtitle" pattern exists, extract the left part
/// 5. Clean up trailing/leading whitespace and special characters
pub fn extract_song_title(full_title: &str) -> String {
    let mut title = full_title.to_string();

    // Rule 1: スラッシュ分割 (曲名 / ボーカル名)
    if let Some(pos) = title.find(" / ") {
        title = title[..pos].trim().to_string();
    }

    // Rule 2: 日本語括弧を削除 【】
    title = remove_japanese_brackets(&title);

    // Rule 3: 丸括弧を削除 （）
    title = remove_japanese_parens(&title);

    // Rule 4: 角括弧を削除 []
    title = remove_square_brackets(&title);

    // Rule 5: ダッシュで分割は行わない
    // 理由：括弧削除後のダッシュが曲名の一部なのか区切りなのか判定困難
    // 分析結果（9.2%）から、重要度が低い

    // Cleanup: 末尾の special characters を削除
    title = title.trim().to_string();
    while title.ends_with("&amp;") || title.ends_with("&") {
        if title.ends_with("&amp;") {
            title = title[..title.len() - 5].to_string();
        } else {
            title.pop();
        }
        title = title.trim_end().to_string();
    }

    title.trim().to_string()
}

fn remove_japanese_brackets(s: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;

    for ch in s.chars() {
        if ch == '【' {
            depth += 1;
        } else if ch == '】' {
            depth -= 1;
        } else if depth == 0 {
            result.push(ch);
        }
    }

    result.trim().to_string()
}

fn remove_japanese_parens(s: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;

    for ch in s.chars() {
        if ch == '（' {
            depth += 1;
        } else if ch == '）' {
            depth -= 1;
        } else if depth == 0 {
            result.push(ch);
        }
    }

    result.trim().to_string()
}

fn remove_square_brackets(s: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;

    for ch in s.chars() {
        if ch == '[' {
            depth += 1;
        } else if ch == ']' {
            depth -= 1;
        } else if depth == 0 {
            result.push(ch);
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slash_separator() {
        assert_eq!(extract_song_title("千本桜 / 初音ミク"), "千本桜");
    }

    #[test]
    fn test_japanese_brackets() {
        assert_eq!(extract_song_title("【初音ミク】波【オリジナル】"), "波");
    }

    #[test]
    fn test_feat_pattern() {
        assert_eq!(
            extract_song_title("曲名 feat. 初音ミク"),
            "曲名 feat. 初音ミク"
        );
    }

    #[test]
    fn test_combined_patterns() {
        assert_eq!(
            extract_song_title("【MV】彼は誰メロディ / 重音テト"),
            "彼は誰メロディ"
        );
    }

    #[test]
    fn test_square_brackets() {
        assert_eq!(
            extract_song_title("[MV] DIVA - Nebula"),
            "DIVA - Nebula" // ダッシュは分割しないので残される
        );
    }

    #[test]
    fn test_no_extraction_needed() {
        assert_eq!(extract_song_title("シンプルな曲名"), "シンプルな曲名");
    }
}

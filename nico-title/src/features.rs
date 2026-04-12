use std::collections::HashMap;

/// 文字の種類を分類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharType {
    Kanji,         // 漢字
    Hiragana,      // ひらがな
    Katakana,      // カタカナ
    AsciiLetter,   // A-Z, a-z
    AsciiDigit,    // 0-9
    AsciiSymbol,   // ! @ # ...
    FullwidthKana, // ＡＢＣ等（全角英数字）
    FullwidthSymbol, // 【】（）など
    Space,         // 空白
    Other,         // その他
}

impl CharType {
    pub fn of(c: char) -> Self {
        match c {
            // 漢字（CJK Unified Ideographs）
            '\u{4E00}'..='\u{9FFF}' => CharType::Kanji,

            // ひらがな （含む句読点など U+3000-309F）
            '\u{3000}'..='\u{309F}' => CharType::Hiragana,

            // カタカナ
            '\u{30A0}'..='\u{30FF}' => CharType::Katakana,

            // ASCII 英字
            'A'..='Z' | 'a'..='z' => CharType::AsciiLetter,

            // ASCII 数字
            '0'..='9' => CharType::AsciiDigit,

            // ASCII 記号
            '!' | '@' | '#' | '$' | '%' | '&' | '*' | '-' | '_' | '=' | '+' |
            '{' | '}' | ';' | ':' | '\'' | '"' | '<' | '>' |
            '\\' | '|' | '`' | '^' => CharType::AsciiSymbol,

            // 括弧と記号は ASCII 記号内に含む
            '(' | ')' | '[' | ']' | ',' | '.' | '/' | '?' | '~' => CharType::AsciiSymbol,

            // 全角英数字・記号
            '\u{FF21}'..='\u{FF3A}' | '\u{FF41}'..='\u{FF5A}' | // Ａ-Ｚ、ａ-ｚ
            '\u{FF10}'..='\u{FF19}' | // １-９
            '\u{FF1A}'..='\u{FF20}' | // ：；＜＝＞？＠
            '\u{FF3B}'..='\u{FF40}' | // ［＼］＾＿｀
            '\u{FF5B}'..='\u{FF65}' => CharType::FullwidthKana,

            // スペース
            ' ' | '\t' => CharType::Space,

            _ => CharType::Other,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            CharType::Kanji => "kanji",
            CharType::Hiragana => "hiragana",
            CharType::Katakana => "katakana",
            CharType::AsciiLetter => "ascii_letter",
            CharType::AsciiDigit => "ascii_digit",
            CharType::AsciiSymbol => "ascii_symbol",
            CharType::FullwidthKana => "fullwidth_kana",
            CharType::FullwidthSymbol => "fullwidth_symbol",
            CharType::Space => "space",
            CharType::Other => "other",
        }
    }
}

pub struct FeatureExtractor {
    /// 特徴量文字列 → ID のマッピング
    pub feature_map: HashMap<String, usize>,
}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self {
            feature_map: HashMap::new(),
        }
    }

    /// 特徴量文字列を特徴量 ID に変換
    fn get_feature_id(&mut self, feature: String) -> usize {
        let len = self.feature_map.len();
        *self.feature_map.entry(feature).or_insert(len)
    }

    /// タイトル内の位置 idx に対して特徴量を抽出
    pub fn extract_features(&mut self, title: &str, idx: usize) -> Vec<usize> {
        let chars: Vec<char> = title.chars().collect();
        if idx >= chars.len() {
            return vec![];
        }

        let mut features = Vec::new();

        let curr_char = chars[idx];
        let curr_type = CharType::of(curr_char);

        // 特徴1: 現在の文字型
        features.push(self.get_feature_id(format!("char_type:{}", curr_type.name())));

        // 特徴2: 前の文字型
        if idx > 0 {
            let prev_type = CharType::of(chars[idx - 1]);
            features.push(self.get_feature_id(format!("prev_type:{}", prev_type.name())));
        } else {
            features.push(self.get_feature_id("is_start".to_string()));
        }

        // 特徴3: 次の文字型
        if idx + 1 < chars.len() {
            let next_type = CharType::of(chars[idx + 1]);
            features.push(self.get_feature_id(format!("next_type:{}", next_type.name())));
        } else {
            features.push(self.get_feature_id("is_end".to_string()));
        }

        // 特徴4: セパレータとの距離
        let separators = vec!["/", "／", "【", "】", "feat.", "Vo.", "_"];
        for sep in separators {
            if let Some(pos) = title.find(sep) {
                // バイト位置を文字位置に変換
                let sep_char_idx = title[..pos].chars().count();
                let distance = (idx as isize - sep_char_idx as isize).abs();
                if distance <= 5 {
                    features.push(self.get_feature_id(format!("sep_dist:{}:{}",sep.replace(".", "_"), distance)));
                }
            }
        }

        // 特徴5: 相対位置
        let title_len = chars.len();
        if idx < 3 {
            features.push(self.get_feature_id("pos:start".to_string()));
        }
        if idx >= title_len - 3 {
            features.push(self.get_feature_id("pos:end".to_string()));
        }
        if idx == title_len / 2 {
            features.push(self.get_feature_id("pos:middle".to_string()));
        }

        features
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_type_kanji() {
        assert_eq!(CharType::of('曲'), CharType::Kanji);
    }

    #[test]
    fn test_char_type_hiragana() {
        assert_eq!(CharType::of('ぎ'), CharType::Hiragana);
    }

    #[test]
    fn test_char_type_katakana() {
        assert_eq!(CharType::of('ミ'), CharType::Katakana);
    }

    #[test]
    fn test_char_type_ascii() {
        assert_eq!(CharType::of('a'), CharType::AsciiLetter);
        assert_eq!(CharType::of('5'), CharType::AsciiDigit);
    }

    #[test]
    fn test_feature_extractor() {
        let mut ext = FeatureExtractor::new();
        let features = ext.extract_features("曲名 / ボカロ", 0);
        assert!(!features.is_empty());
        // 先頭なので is_start を含むべき
        assert!(ext.feature_map.contains_key("is_start"));
    }

    #[test]
    fn test_separator_distance() {
        let mut ext = FeatureExtractor::new();
        let features = ext.extract_features("曲名 / ボカロ", 3); // "/" の直前
        assert!(!features.is_empty());
    }
}

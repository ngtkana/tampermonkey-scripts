/// Rule-based song title extractor for Vocaloid content on NicoNico Douga
///
/// ニコ動ボカロ曲のタイトルには歴史的に2大流派がある（参考: さかじょん 2022）:
///   【】流派 (〜2014): 【ボカロ名】曲名【オリジナル曲】
///   /  流派 (2014〜): 曲名 / ボカロ名
/// さらにクリエイター名が先頭に来るパターンも増加:
///   クリエイター - 曲名 feat. ボカロ
///   クリエイター「曲名」feat. ボカロ
///   クリエイター『曲名』Vo. ボカロ
pub fn extract_song_title(full_title: &str) -> String {
    // Rule 0: 「」or『』内に曲名がある場合
    // 開き括弧の前にテキストがある = クリエイター名が先頭にいるパターン
    // 例: utage「mirroring」- 初音ミク → mirroring
    // 例: 稲葉曇『ラグトレイン』Vo. 歌愛ユキ → ラグトレイン
    if let Some(quoted) = extract_from_quotes(full_title) {
        return quoted;
    }

    let mut title = full_title.to_string();

    // Rule 1: スラッシュ分割（曲名 / ボカロ名 パターン）
    // 半角スペースあり " / " を優先し、なければ任意の / および全角 ／ にも対応
    // 例: 好きです / 歌愛ユキ → 好きです
    // 例: あなたがいなくなって/初音ミク → あなたがいなくなって
    // 例: 怠惰でありたい／初音ミク → 怠惰でありたい
    if let Some(pos) = title.find(" / ") {
        title = title[..pos].trim().to_string();
    } else if let Some(pos) = title.find('/') {
        title = title[..pos].trim().to_string();
    } else if let Some(pos) = title.find('／') {
        title = title[..pos].trim().to_string();
    }

    // Rule 2: 日本語括弧を削除 【】
    // 例: 【MV】彼は誰メロディ → 彼は誰メロディ
    // 例: 【初音ミク】ごめんなさい【オリジナル曲】 → ごめんなさい
    title = remove_brackets(&title, '【', '】');

    // Rule 3: 日本語丸括弧を削除 （）
    title = remove_brackets(&title, '（', '）');

    // Rule 4: 角括弧を削除 []
    title = remove_brackets(&title, '[', ']');

    // Rule 5: アンダースコア分割（曲名_ボーカル名 パターン）
    // 例: 桜のrunway_初音ミク → 桜のrunway
    if let Some(pos) = title.find('_') {
        title = title[..pos].trim().to_string();
    }

    // Rule 6: feat./ft./Vo. 以降を削除（スラッシュのない feat パターン）
    // 例: 傀儡的フォルティシモ　feat.初音ミク → 傀儡的フォルティシモ
    // 例: 稲葉曇「ラグトレイン」Vo. 歌愛ユキ は Rule 0 で処理済みだが念のため
    for sep in &[
        " feat. ", " feat.", "(feat.", "　feat.", " feat ", " ft. ", " ft.", " ft ", " Vo. ",
        "　Vo.", " vo. ", " vo.",
    ] {
        if let Some(pos) = title.find(sep) {
            title = title[..pos].trim().to_string();
            break;
        }
    }

    title.trim().to_string()
}

/// 「」または『』の内容を抽出する
/// 開き括弧の前にテキストがある（pos > 0）ときのみ適用
fn extract_from_quotes(s: &str) -> Option<String> {
    for (open, close) in [('「', '」'), ('『', '』')] {
        let open_len = open.len_utf8();
        if let (Some(start), Some(end)) = (s.find(open), s.rfind(close))
            && start < end
            && start > 0
        {
            let content = s[start + open_len..end].trim().to_string();
            if !content.is_empty() {
                return Some(content);
            }
        }
    }
    None
}

fn remove_brackets(s: &str, open: char, close: char) -> String {
    let mut result = String::new();
    let mut depth = 0;

    for ch in s.chars() {
        if ch == open {
            depth += 1;
        } else if ch == close {
            if depth > 0 {
                depth -= 1;
            }
        } else if depth == 0 {
            result.push(ch);
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- 【】流派 ---

    #[test]
    fn test_bracket_prefix() {
        // 【ボカロ名】曲名
        assert_eq!(
            extract_song_title("【初音ミク】ごめんなさい 【オリジナル曲】"),
            "ごめんなさい"
        );
    }

    #[test]
    fn test_bracket_mv_prefix() {
        // 【MV】曲名
        assert_eq!(extract_song_title("【MV】彼は誰メロディ"), "彼は誰メロディ");
    }

    // --- / 流派 ---

    #[test]
    fn test_slash_with_spaces() {
        assert_eq!(extract_song_title("千本桜 / 初音ミク"), "千本桜");
    }

    #[test]
    fn test_slash_no_space() {
        // スペースなしスラッシュ
        assert_eq!(extract_song_title("テトライド/重音テト"), "テトライド");
    }

    #[test]
    fn test_slash_fullwidth() {
        // 全角スラッシュ
        assert_eq!(
            extract_song_title("怠惰でありたい／初音ミク"),
            "怠惰でありたい"
        );
    }

    #[test]
    fn test_slash_feat_combo() {
        // 曲名 / クリエイター feat. ボカロ
        assert_eq!(
            extract_song_title("アフターブーケ / 何番サンダー feat. 夏色花梨"),
            "アフターブーケ"
        );
    }

    // --- クリエイター名先頭パターン（「」/『』）---

    #[test]
    fn test_creator_japanese_quotes() {
        // utage「曲名」- ボカロ
        assert_eq!(
            extract_song_title("utage「mirroring」- 初音ミク"),
            "mirroring"
        );
    }

    #[test]
    fn test_creator_japanese_double_quotes() {
        // クリエイター『曲名』Vo. ボカロ
        assert_eq!(
            extract_song_title("稲葉曇『ラグトレイン』Vo. 歌愛ユキ"),
            "ラグトレイン"
        );
    }

    #[test]
    fn test_creator_quotes_feat() {
        // クリエイター「曲名」feat. ボカロ
        assert_eq!(
            extract_song_title("Chinozo「グッバイ宣言」feat. flower"),
            "グッバイ宣言"
        );
    }

    // --- アンダースコア ---

    #[test]
    fn test_underscore_separator() {
        assert_eq!(extract_song_title("桜のrunway_初音ミク"), "桜のrunway");
    }

    // --- feat. / Vo. / ft. ---

    #[test]
    fn test_feat_no_slash() {
        // スラッシュなしの feat パターン
        assert_eq!(
            extract_song_title("傀儡的フォルティシモ　feat.初音ミク"),
            "傀儡的フォルティシモ"
        );
    }

    #[test]
    fn test_square_brackets() {
        assert_eq!(extract_song_title("[MV] DIVA - Nebula"), "DIVA - Nebula");
    }

    #[test]
    fn test_no_extraction_needed() {
        assert_eq!(extract_song_title("シンプルな曲名"), "シンプルな曲名");
    }
}

/// ルールベース分類器
/// JS の nico-commons-content-tree.user.js から移植

const POSITIVE_KEYWORDS: &[&str] = &[
    "歌って", "うたって", "唄って",
    "歌った", "うたった", "唄った",
    "歌いました", "うたいました", "唄いました",
    "歌わせていただき", "うたわせていただき",
    "歌いますた", "歌いなおし",
    "弾き語り",
    "カバー", "cover",
    "歌えている", "原キーで",
];

const NEGATIVE_KEYWORDS: &[&str] = &[
    // 非カバー動画
    "まとめ", "音源", "講座", "配布", "メドレー", "予告", "人力",
    // 合成音声系ソフトウェア・規格
    "utau", "vocaloid", "ボカロ", "neutrino", "synthesizerv", "synthv",
    "voiceroid", "ボイスロイド", "a.i.voice", "合成音声", "nnsvs", "voicevox",
];

/// ルールベースで「歌ってみた」っぽいかを判定
/// 負のキーワードがあれば false、正のキーワードがあれば true、それ以外は false
pub fn classify(title: &str) -> bool {
    let title_lower = title.to_lowercase();

    // NEGATIVE_KEYWORDS に含まれていたら false
    if NEGATIVE_KEYWORDS.iter().any(|k| title_lower.contains(&k.to_lowercase())) {
        return false;
    }

    // POSITIVE_KEYWORDS に含まれていたら true
    POSITIVE_KEYWORDS.iter().any(|k| title_lower.contains(&k.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive() {
        assert!(classify("【歌ってみた】千本桜"));
        assert!(classify("千本桜カバー【歌った】"));
        assert!(classify("弾き語り"));
    }

    #[test]
    fn test_negative() {
        assert!(!classify("【MMD】千本桜")); // no positive keyword
        assert!(!classify("【歌ってみた】ボカロ版")); // has negative keyword
        assert!(!classify("千本桜 メドレー"));
    }
}

use std::collections::HashSet;

/// Unicode `.chars()` ベースで n=n_min..=n_max の char n-gram を返す
pub fn extract(text: &str, n_min: usize, n_max: usize) -> HashSet<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut grams = HashSet::new();
    for n in n_min..=n_max {
        for window in chars.windows(n) {
            grams.insert(window.iter().collect());
        }
    }
    grams
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigram_basic() {
        let g = extract("abcde", 3, 3);
        assert!(g.contains("abc"));
        assert!(g.contains("bcd"));
        assert!(g.contains("cde"));
        assert_eq!(g.len(), 3);
    }

    #[test]
    fn range_3_5() {
        let g = extract("abcde", 3, 5);
        assert!(g.contains("abc"));
        assert!(g.contains("abcd"));
        assert!(g.contains("abcde"));
    }

    #[test]
    fn shorter_than_n_returns_empty() {
        assert!(extract("ab", 3, 5).is_empty());
    }
}

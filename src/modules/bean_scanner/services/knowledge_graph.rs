/// Stage 3: Origin Geography & Terroir Knowledge Graph (Sabang-Merauke + 65 World Countries)
/// Maps geographic keywords to canonical origin, terroir details, and default varietals.
use strsim::{jaro_winkler, levenshtein};

use super::origin_profiles_id::INDONESIA_PROFILES;
use super::origin_profiles_world::WORLD_PROFILES;

const NON_COFFEE_STOPWORDS: &[&str] = &[
    "struk", "belanja", "total", "subtotal", "terima", "kasih", "harga",
    "bayar", "tunai", "kembalian", "diskon", "promo", "tanggal", "kasir",
    "invoice", "receipt", "change", "thank", "thanks",
];

#[derive(Debug, Clone)]
pub struct TerroirProfile {
    pub canonical_name: &'static str,
    pub region_province: &'static str,
    pub default_altitude: &'static str,
    pub default_varietals: &'static str,
    pub keywords: &'static [&'static str],
}

pub struct KnowledgeGraph;

impl KnowledgeGraph {
    fn all_profiles() -> impl Iterator<Item = &'static TerroirProfile> {
        INDONESIA_PROFILES.iter().chain(WORLD_PROFILES.iter())
    }

    fn contains_word_boundary(haystack: &str, needle: &str) -> bool {
        let mut start = 0;
        while let Some(pos) = haystack[start..].find(needle) {
            let abs = start + pos;
            let before_ok = abs == 0
                || !haystack[..abs]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_alphanumeric);
            let after_idx = abs + needle.len();
            let after_ok = after_idx >= haystack.len()
                || !haystack[after_idx..]
                    .chars()
                    .next()
                    .is_some_and(char::is_alphanumeric);
            if before_ok && after_ok {
                return true;
            }
            start = abs + needle.len().max(1);
        }
        false
    }

    /// Grounded lookup against Terroir Knowledge Graph using word-boundary longest-match
    /// + dual-gated fuzzy tolerance (Jaro-Winkler >= 0.91 AND Levenshtein <= 1/2).
    pub fn find_origin(text: &str) -> Option<&'static TerroirProfile> {
        let clean = text.to_lowercase();
        let words: Vec<&str> = clean
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() >= 5 && !NON_COFFEE_STOPWORDS.contains(w))
            .collect();

        // 1. Word-boundary direct matching — longest keyword wins.
        let mut best_substring: Option<(&'static TerroirProfile, usize)> = None;
        for profile in Self::all_profiles() {
            for &kw in profile.keywords {
                if Self::contains_word_boundary(&clean, kw) {
                    let len = kw.len();
                    if best_substring.map_or(true, |(_, best_len)| len > best_len) {
                        best_substring = Some((profile, len));
                    }
                }
            }
        }
        if let Some((profile, _)) = best_substring {
            return Some(profile);
        }

        // 2. Strict dual-gated fuzzy match on single-word tokens to fix OCR typos without hallucinating.
        let mut best_fuzzy: Option<(&'static TerroirProfile, usize, f64)> = None;
        for profile in Self::all_profiles() {
            for &kw in profile.keywords {
                if kw.contains(' ') || kw.len() < 5 {
                    continue;
                }
                let max_dist = if kw.len() >= 9 { 2 } else { 1 };
                for &word in &words {
                    if (word.len() as isize - kw.len() as isize).abs() > max_dist as isize {
                        continue;
                    }
                    let score = jaro_winkler(word, kw);
                    if score >= 0.91 && levenshtein(word, kw) <= max_dist {
                        let len = kw.len();
                        let is_better = match best_fuzzy {
                            None => true,
                            Some((_, best_len, best_score)) => {
                                len > best_len || (len == best_len && score > best_score)
                            }
                        };
                        if is_better {
                            best_fuzzy = Some((profile, len, score));
                        }
                    }
                }
            }
        }
        best_fuzzy.map(|(profile, _, _)| profile)
    }
}

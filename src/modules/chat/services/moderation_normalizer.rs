use crate::modules::chat::services::profanity_lexicon::{
    EXACT_TOKEN_TERMS, SAFE_COFFEE_WHITELIST, SEVERE_SUBSTRING_TERMS, WILDCARD_PATTERNS,
};

fn is_zero_width(c: char) -> bool {
    matches!(
        c,
        '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}' | '\u{FEFF}' | '\u{00AD}'
    )
}

fn fold_diacritic_or_leet(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | '@' | '4' => 'a',
        'é' | 'è' | 'ê' | 'ë' | '3' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'ı' | '1' | '!' | '|' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | '0' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ñ' => 'n',
        'ç' | 'ć' => 'c',
        'ş' | 'š' | '$' | '5' => 's',
        'ğ' => 'g',
        'ł' => 'l',
        '7' => 't',
        '8' => 'b',
        other => other,
    }
}

pub fn strip_invisible_lower(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if is_zero_width(c) {
            continue;
        }
        if c == 'ß' || c == 'ẞ' {
            out.push_str("ss");
        } else {
            for lc in c.to_lowercase() {
                out.push(lc);
            }
        }
    }
    out
}

pub fn decode_leet_only(token: &str) -> String {
    token
        .chars()
        .map(fold_diacritic_or_leet)
        .filter(|c| c.is_alphabetic())
        .collect()
}

pub fn collapse_repeats(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev: Option<char> = None;
    for c in s.chars() {
        if prev != Some(c) {
            out.push(c);
            prev = Some(c);
        }
    }
    out
}

fn matches_masked_wildcard(raw_token: &str) -> bool {
    if !raw_token.contains('*') && !raw_token.contains('#') && !raw_token.contains('_') {
        return false;
    }
    let cleaned: String = raw_token
        .chars()
        .map(|c| match c {
            '#' | '_' | '.' | '-' => '*',
            other => fold_diacritic_or_leet(other),
        })
        .filter(|c| c.is_alphabetic() || *c == '*')
        .collect();
    if cleaned.len() < 4 || !cleaned.contains('*') {
        return false;
    }
    WILDCARD_PATTERNS
        .iter()
        .any(|(pre, suf)| cleaned.starts_with(pre) && cleaned.ends_with(suf))
}

fn matches_severe_ascii(decoded: &str, collapsed: &str) -> bool {
    for &term in SEVERE_SUBSTRING_TERMS {
        if !term.is_ascii() || term.contains(' ') {
            continue;
        }
        if decoded.contains(term) || collapsed.contains(&collapse_repeats(term)) {
            return true;
        }
    }
    false
}

/// Mendeteksi kata kasar multibahasa (leetspeak, huruf berulang, wildcard, dan spasi patah).
pub fn contains_multilingual_profanity(raw_text: &str) -> bool {
    let clean = strip_invisible_lower(raw_text);
    if clean.trim().is_empty() {
        return false;
    }

    for &term in SEVERE_SUBSTRING_TERMS {
        if (!term.is_ascii() || term.contains(' ')) && clean.contains(term) {
            return true;
        }
    }

    let raw_tokens: Vec<&str> = clean
        .split(|c: char| {
            c.is_whitespace()
                || matches!(c, ',' | ';' | ':' | '?' | '"' | '\'' | '(' | ')' | '[' | ']' | '/')
        })
        .filter(|s| !s.is_empty())
        .collect();

    for &tok in &raw_tokens {
        if matches_masked_wildcard(tok) {
            return true;
        }
        let alpha_only: String = tok.chars().filter(|c| c.is_alphabetic()).collect();
        if SAFE_COFFEE_WHITELIST.contains(&alpha_only.as_str()) {
            continue;
        }
        let decoded = decode_leet_only(tok);
        let collapsed = collapse_repeats(&decoded);
        if decoded.is_empty()
            || SAFE_COFFEE_WHITELIST.contains(&decoded.as_str())
            || SAFE_COFFEE_WHITELIST.contains(&collapsed.as_str())
        {
            continue;
        }
        if EXACT_TOKEN_TERMS.contains(&decoded.as_str())
            || EXACT_TOKEN_TERMS
                .iter()
                .any(|t| collapse_repeats(t) == collapsed)
        {
            return true;
        }
        if matches_severe_ascii(&decoded, &collapsed) {
            return true;
        }
    }

    // Anti-spacing bypass (`f u c k`, `k.o.n.t.o.l`, `b a n g s a t`)
    let mut short_run = String::new();
    let mut short_count = 0usize;
    for &tok in &raw_tokens {
        let dec = decode_leet_only(tok);
        if !dec.is_empty() && dec.len() <= 2 && dec.is_ascii() {
            short_run.push_str(&dec);
            short_count += 1;
            if short_count >= 3 && short_run.len() >= 4 {
                let col_run = collapse_repeats(&short_run);
                for &term in SEVERE_SUBSTRING_TERMS {
                    if term.is_ascii()
                        && !term.contains(' ')
                        && term.len() >= 4
                        && (short_run.contains(term) || col_run.contains(&collapse_repeats(term)))
                    {
                        return true;
                    }
                }
            }
        } else {
            short_run.clear();
            short_count = 0;
        }
    }

    false
}

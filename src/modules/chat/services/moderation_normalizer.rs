use crate::modules::chat::services::profanity_lexicon::{
    EXACT_TOKEN_TERMS, SAFE_COFFEE_WHITELIST, SEVERE_SUBSTRING_TERMS,
};

fn is_zero_width(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}' | '\u{FEFF}' | '\u{00AD}')
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
            out.extend(c.to_lowercase());
        }
    }
    out
}

pub fn decode_leet_only(token: &str) -> String {
    token.chars().map(fold_diacritic_or_leet).filter(|c| c.is_alphabetic()).collect()
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

fn wildcard_fits_term(pattern: &str, term: &str) -> bool {
    if term.len() < 4 || term.len() > pattern.len() + 3 {
        return false;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    if let Some(first) = parts.first().filter(|s| !s.is_empty()) {
        if !term.starts_with(first) { return false; }
    }
    if let Some(last) = parts.last().filter(|s| !s.is_empty()) {
        if !term.ends_with(last) { return false; }
    }
    let mut cursor = 0usize;
    for (idx, seg) in parts.iter().enumerate() {
        if seg.is_empty() {
            cursor += 1;
            continue;
        }
        let start = if idx == 0 { 0 } else { cursor + 1 };
        let Some(pos) = term.get(start..).and_then(|slice| slice.find(seg)) else {
            return false;
        };
        cursor = start + pos + seg.len();
    }
    true
}

fn matches_masked_wildcard(raw_token: &str) -> bool {
    if !raw_token.contains('*') && !raw_token.contains('#') && !raw_token.contains('_') {
        return false;
    }
    let mut cleaned = String::with_capacity(raw_token.len());
    let mut alpha_count = 0usize;
    let mut prev_star = false;
    for c in raw_token.chars() {
        if matches!(c, '*' | '#' | '_' | '.' | '-') {
            if !prev_star { cleaned.push('*'); prev_star = true; }
        } else {
            let mapped = fold_diacritic_or_leet(c);
            if mapped.is_ascii_alphabetic() {
                cleaned.push(mapped);
                alpha_count += 1;
                prev_star = false;
            }
        }
    }
    if cleaned.len() < 4 || alpha_count < 2 || !cleaned.contains('*') {
        return false;
    }
    SEVERE_SUBSTRING_TERMS
        .iter()
        .any(|&term| term.is_ascii() && !term.contains(' ') && wildcard_fits_term(&cleaned, term))
}

fn matches_severe_ascii(decoded: &str, collapsed: &str) -> bool {
    SEVERE_SUBSTRING_TERMS.iter().any(|&term| {
        term.is_ascii()
            && !term.contains(' ')
            && (decoded.contains(term) || collapsed.contains(&collapse_repeats(term)))
    })
}

/// Mendeteksi kata kasar multibahasa (leetspeak, huruf berulang, multi-wildcard, dan spasi patah).
pub fn contains_multilingual_profanity(raw_text: &str) -> bool {
    let clean = strip_invisible_lower(raw_text);
    if clean.trim().is_empty() {
        return false;
    }

    if SEVERE_SUBSTRING_TERMS
        .iter()
        .any(|&t| (!t.is_ascii() || t.contains(' ')) && clean.contains(t))
    {
        return true;
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
            || EXACT_TOKEN_TERMS.iter().any(|t| collapse_repeats(t) == collapsed)
            || matches_severe_ascii(&decoded, &collapsed)
        {
            return true;
        }
    }

    let mut short_run = String::new();
    let mut short_count = 0usize;
    for &tok in &raw_tokens {
        let dec = decode_leet_only(tok);
        if !dec.is_empty() && dec.len() <= 2 && dec.is_ascii() {
            short_run.push_str(&dec);
            short_count += 1;
            if short_count >= 3 && short_run.len() >= 4 {
                let col_run = collapse_repeats(&short_run);
                if SEVERE_SUBSTRING_TERMS.iter().any(|&t| {
                    t.is_ascii()
                        && !t.contains(' ')
                        && t.len() >= 4
                        && (short_run.contains(t) || col_run.contains(&collapse_repeats(t)))
                }) {
                    return true;
                }
            }
        } else {
            short_run.clear();
            short_count = 0;
        }
    }
    false
}

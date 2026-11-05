/// Stage 6 Helper: Clean Title & Roastery Composition
use regex::Regex;
use crate::modules::bean_scanner::services::knowledge_graph::TerroirProfile;
use crate::modules::bean_scanner::services::noise_filter::NoiseFilter;

pub struct TitleComposer;

impl TitleComposer {
    pub fn compose_bean_name(
        title: Option<&str>,
        origin: Option<&TerroirProfile>,
        process: &str,
        roastery: &str,
    ) -> String {
        let mut base = if let Some(t) = title {
            NoiseFilter::clean_title(t)
        } else if let Some(p) = origin {
            p.canonical_name.to_string()
        } else {
            "Specialty Coffee".to_string()
        };

        // Remove redundant roastery name from bean title
        let r_low = roastery.to_lowercase();
        if base.to_lowercase().starts_with(&r_low) {
            base = base[r_low.len()..].trim().to_string();
        }

        if base.is_empty() {
            base = origin.map(|o| o.canonical_name.to_string()).unwrap_or_else(|| "Specialty Coffee".to_string());
        }

        // Clean process to natural short form (e.g. "Wine Process" -> "Wine")
        let p_short = match process {
            "Wine Process" => "Wine",
            "Honey Process" => "Honey",
            "Full Washed" => "Washed",
            other => other,
        };

        if !p_short.is_empty() && !base.to_lowercase().contains(&p_short.to_lowercase()) {
            format!("{} {}", base, p_short)
        } else {
            base
        }
    }

    pub fn extract_roastery_from_text(text: &str) -> Option<String> {
        for line in text.lines().map(|l| l.trim()).take(4) {
            let lower = line.to_lowercase();
            if (lower.contains("coffee") || lower.contains("bean") || lower.contains("roastery") || lower.contains("roasters"))
                && !NoiseFilter::is_noise(line)
            {
                return Some(Self::title_case(line));
            }
        }
        None
    }

    pub fn extract_explicit_altitude(text: &str) -> Option<String> {
        if let Ok(re) = Regex::new(r"(?i)(\d{3,4})\s*(?:-\s*(\d{3,4}))?\s*(?:mdpl|masl|m\b)") {
            if let Some(caps) = re.captures(text) {
                if let Some(m) = caps.get(0) {
                    return Some(m.as_str().trim().to_uppercase());
                }
            }
        }
        None
    }

    pub fn extract_explicit_varietal(text: &str) -> Option<String> {
        let lower = text.to_lowercase();
        const KNOWN: &[&str] = &[
            "geisha", "gesha", "pink bourbon", "bourbon", "typica", "sl28", "sl34",
            "sigararutang", "kartika", "andungsari", "lini s795", "caturra", "catimor",
        ];
        for &v in KNOWN {
            if lower.contains(v) {
                return Some(Self::title_case(v));
            }
        }
        None
    }

    pub fn title_case(s: &str) -> String {
        s.split_whitespace()
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

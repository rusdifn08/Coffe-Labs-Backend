/// Stage 1: Boilerplate & Packaging Noise Blacklist Filter
/// Strips out marketing slogans, packaging boilerplate, and non-informative text.

const NOISE_PHRASES: &[&str] = &[
    "fresh bean",
    "better brew",
    "roasting date",
    "roast date",
    "best before",
    "production date",
    "exp date",
    "net weight",
    "netto",
    "berat bersih",
    "100% arabica",
    "pure arabica",
    "specialty coffee",
    "single origin",
    "whole bean",
    "whole beans",
    "biji kopi",
    "biji kopi pilihan",
    "bubuk kopi",
    "ground coffee",
    "drip coffee",
    "filter coffee",
    "espresso roast",
    "espresso",
    "diproduksi oleh",
    "p-irt",
    "pirt",
    "halal",
    "dari gunung",
    "untuk cita rasa terbaik",
    "premium quality",
    "fresh roasted",
    "master your brew",
    "indonesia",
    "coffee bean",
    "coffee beans",
];

pub struct NoiseFilter;

impl NoiseFilter {
    /// Returns true if the given line is purely boilerplate / packaging noise
    pub fn is_noise(text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.len() <= 1 {
            return true;
        }

        let lower = trimmed.to_lowercase();

        // Exact or near-exact match on noise phrases
        for &noise in NOISE_PHRASES {
            if lower == noise {
                return true;
            }
        }

        // Catch common multi-word slogans
        if lower.contains("fresh bean") && lower.contains("better brew") {
            return true;
        }
        if lower.contains("roasting date") || lower.contains("roast date") {
            return true;
        }
        if lower.contains("dari gunung") || lower.contains("cita rasa") {
            return true;
        }
        if lower.contains("diproduksi oleh") || lower.contains("p-irt") {
            return true;
        }

        // Empty checkbox noise: [ ] washed, [ ] wine, etc.
        if (lower.starts_with("[ ]") || lower.starts_with("( )") || lower.starts_with("[]"))
            && !lower.contains('x') && !lower.contains('v') && !lower.contains('✓')
        {
            return true;
        }

        false
    }

    /// Cleans slogan substrings from candidate coffee names
    pub fn clean_title(title: &str) -> String {
        let mut result = title.to_string();
        for &noise in NOISE_PHRASES {
            if let Some(pos) = result.to_lowercase().find(noise) {
                result.replace_range(pos..pos + noise.len(), " ");
            }
        }
        result.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

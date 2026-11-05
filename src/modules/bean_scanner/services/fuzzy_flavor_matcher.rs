/// Stage 4 & 5: Fuzzy Tasting Notes & Process Sensory Matrix
use strsim::jaro_winkler;
use super::noise_filter::NoiseFilter;

pub struct ProcessSensoryProfile {
    pub canonical_name: &'static str,
    pub recommended_temp_c: i32,
    pub top_recipe_name: &'static str,
    pub top_recipe_slug: &'static str,
    pub signature_notes: &'static [&'static str],
}

pub struct FuzzyFlavorMatcher;

const FLAVOR_LEXICON: &[(&str, &[&str])] = &[
    ("Blueberry", &["blueberry", "bluberry", "bluebery", "beri", "berries"]),
    ("Strawberry", &["strawberry", "starberry", "strawbery", "stroberi"]),
    ("Jasmine", &["jasmine", "jasmin", "jasmne", "melati"]),
    ("Black Tea", &["tea", "blacktea", "earlgrey", "teh", "black tea", "floral tea"]),
    ("Winey", &["wine", "winey", "winery", "red wine", "fermented"]),
    ("Red Grape", &["grape", "red grape", "anggur"]),
    ("Blackcurrant", &["blackcurrant", "currant", "kismis"]),
    ("Peach", &["peach", "apricot", "persik", "stone fruit"]),
    ("Mango", &["mango", "mangga", "tropical mango"]),
    ("Pineapple", &["pineapple", "nanas", "passion fruit", "markisa"]),
    ("Lemon Citrus", &["lemon", "lime", "citrus", "jeruk", "bergamot", "orange"]),
    ("Milk Chocolate", &["milk chocolate", "chocolate", "choco", "cokelat"]),
    ("Dark Cocoa", &["dark cocoa", "cacao", "dark chocolate", "cocoa nibs"]),
    ("Caramel", &["caramel", "karamel", "toffee"]),
    ("Brown Sugar", &["brown sugar", "gula aren", "palm sugar", "cane sugar"]),
    ("Wild Honey", &["honey", "madu", "wild honey"]),
    ("Chestnut", &["chestnut", "almond", "hazelnut", "kacang"]),
    ("Red Fruit", &["red fruit", "buah merah", "plum", "cherry"]),
    ("Cinnamon", &["cinnamon", "kayu manis", "spice"]),
    ("Vanilla", &["vanilla", "vanila"]),
    ("Floral", &["floral", "flower", "bunga", "rose", "chamomile"]),
];

impl FuzzyFlavorMatcher {
    /// Detect process and return canonical name & recommended brewing params
    pub fn detect_process(text: &str) -> Option<ProcessSensoryProfile> {
        let clean = text.to_lowercase();

        // 1. Checkbox pattern: e.g. [x] NATURAL, [v] WINE, [X] WASHED
        for line in clean.lines() {
            let l = line.trim();
            if l.contains("[x]") || l.contains("[v]") || l.contains("[✓]") {
                if l.contains("natural") {
                    return Some(Self::profile_natural());
                }
                if l.contains("wine") {
                    return Some(Self::profile_wine());
                }
                if l.contains("washed") {
                    return Some(Self::profile_washed());
                }
                if l.contains("honey") {
                    return Some(Self::profile_honey());
                }
            }
        }

        // 2. Direct keyword search
        if clean.contains("wine") || clean.contains("winery") || clean.contains("extended fermentation") {
            return Some(Self::profile_wine());
        }
        if clean.contains("anaerobic") || clean.contains("carbonic") || clean.contains("lactic") || clean.contains("thermal shock") {
            return Some(Self::profile_anaerobic());
        }
        if clean.contains("honey") {
            return Some(Self::profile_honey());
        }
        if clean.contains("natural") || clean.contains("dry process") {
            return Some(Self::profile_natural());
        }
        if clean.contains("washed") || clean.contains("wet process") {
            return Some(Self::profile_washed());
        }
        if clean.contains("giling basah") || clean.contains("wet hulled") || clean.contains("semi washed") {
            return Some(Self::profile_wet_hulled());
        }

        None
    }

    pub fn match_tasting_notes(text: &str) -> Vec<String> {
        let mut results = Vec::new();

        for line in text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
            if NoiseFilter::is_noise(line) {
                continue;
            }

            let l_lower = line.to_lowercase();
            // Skip line if it's purely a single process word (e.g. "WINE" or "NATURAL" or "[x] NATURAL")
            if l_lower == "wine" || l_lower == "natural" || l_lower == "washed" || l_lower == "honey" || l_lower.starts_with("[x]") {
                continue;
            }

            let words: Vec<&str> = line
                .split(|c: char| c == ',' || c == '•' || c == '/' || c == '-' || c == ';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && s.len() >= 3)
                .collect();

            for candidate in words {
                let c_clean = candidate.to_lowercase();
                for &(canonical, synonyms) in FLAVOR_LEXICON {
                    let mut matched = false;
                    for &syn in synonyms {
                        if c_clean == syn || c_clean.contains(syn) || jaro_winkler(&c_clean, syn) >= 0.83 {
                            matched = true;
                            break;
                        }
                    }
                    if matched && !results.contains(&canonical.to_string()) {
                        results.push(canonical.to_string());
                    }
                }
            }
        }

        results
    }

    pub fn profile_wine() -> ProcessSensoryProfile {
        ProcessSensoryProfile {
            canonical_name: "Wine Process",
            recommended_temp_c: 89,
            top_recipe_name: "Dual-Temperature Thermal Drop V60 (93°C -> 70°C)",
            top_recipe_slug: "dual-temperature-thermal-drop-v60",
            signature_notes: &["Winey", "Red Grape", "Strawberry", "Blackcurrant", "Dark Cocoa"],
        }
    }

    pub fn profile_natural() -> ProcessSensoryProfile {
        ProcessSensoryProfile {
            canonical_name: "Natural",
            recommended_temp_c: 90,
            top_recipe_name: "Tetsu Kasuya 4:6 Method (Sweet Focus)",
            top_recipe_slug: "tetsu-kasuya-46",
            signature_notes: &["Blueberry", "Strawberry Jam", "Tropical Fruit", "Milk Chocolate"],
        }
    }

    pub fn profile_honey() -> ProcessSensoryProfile {
        ProcessSensoryProfile {
            canonical_name: "Honey Process",
            recommended_temp_c: 91,
            top_recipe_name: "Smoking Barrels V60",
            top_recipe_slug: "smoking-barrels-v60",
            signature_notes: &["Wild Honey", "Sweet Peach", "Brown Sugar", "Caramel", "Plum"],
        }
    }

    pub fn profile_anaerobic() -> ProcessSensoryProfile {
        ProcessSensoryProfile {
            canonical_name: "Anaerobic Maceration",
            recommended_temp_c: 89,
            top_recipe_name: "Dual-Temperature Thermal Drop V60 (93°C -> 70°C)",
            top_recipe_slug: "dual-temperature-thermal-drop-v60",
            signature_notes: &["Strawberry Candy", "Tropical Mango", "Winey Berry", "Cinnamon", "Floral"],
        }
    }

    pub fn profile_washed() -> ProcessSensoryProfile {
        ProcessSensoryProfile {
            canonical_name: "Full Washed",
            recommended_temp_c: 92,
            top_recipe_name: "WBrC Sandwich Technique",
            top_recipe_slug: "wbrc-sandwich-v60",
            signature_notes: &["Jasmine", "Black Tea", "Lemon Citrus", "Bergamot", "Cane Sugar"],
        }
    }

    pub fn profile_wet_hulled() -> ProcessSensoryProfile {
        ProcessSensoryProfile {
            canonical_name: "Wet Hulled (Giling Basah)",
            recommended_temp_c: 91,
            top_recipe_name: "Tetsu Kasuya 4:6 Method (Rich Body)",
            top_recipe_slug: "tetsu-kasuya-46",
            signature_notes: &["Dark Cocoa", "Caramel", "Earthy Spice", "Herbal Tea"],
        }
    }
}

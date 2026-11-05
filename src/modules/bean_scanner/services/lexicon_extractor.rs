use aho_corasick::{AhoCorasickBuilder, MatchKind};
use regex::Regex;
use crate::modules::bean_scanner::dto::OcrTextBlockDto;
use crate::modules::bean_scanner::models::ExtractedLexiconData;

const PROCESSES: &[&str] = &[
    "Carbonic Maceration", "Natural Anaerobic", "Anaerobic Lactic", "Thermal Shock",
    "Full Washed", "Semi Washed", "Giling Basah", "Wet Hulled", "Black Honey",
    "Yellow Honey", "Red Honey", "Anaerobic", "Natural", "Washed", "Honey", "Koji",
];

const VARIETALS: &[&str] = &[
    "Pink Bourbon", "Yellow Bourbon", "Bourbon", "Geisha", "Gesha", "Typica",
    "SL28", "SL34", "Pacamara", "Abyssinia", "Sigararutang", "Kartika",
    "Andungsari", "Lini S795", "Caturra", "Catuai", "Catimor", "Heirloom", "Castillo",
];

const TASTING_NOTES: &[&str] = &[
    "Jasmine", "Bergamot", "Peach", "Apricot", "Strawberry", "Blueberry",
    "Tropical Fruit", "Lemon Peel", "Lemon", "Orange Blossom", "Orange Peel",
    "Milk Chocolate", "Dark Chocolate", "Cacao Nibs", "Caramel", "Brown Sugar",
    "Honey", "Black Tea", "Earl Grey", "Rose Wine", "Rose", "Floral", "Winey",
    "Cinnamon", "Hazelnut", "Green Apple", "Blackcurrant", "Mango", "Pineapple",
];

const ORIGINS: &[&str] = &[
    "Flores Bajawa", "Java Preanger", "Papua Wamena", "Bali Kintamani", "Ethiopia Yirgacheffe",
    "Kintamani", "Gayo", "Toraja", "Mandheling", "Halmahera", "Yirgacheffe", "Guji",
    "Sidamo", "Huila", "Boquete", "Kiambu", "Colombia", "Panama", "Kenya", "Ethiopia", "Indonesia",
];

pub struct LexiconExtractor;

impl LexiconExtractor {
    pub fn extract(full_text: &str, blocks: &[OcrTextBlockDto]) -> ExtractedLexiconData {
        let clean_text = full_text.trim();
        let (extracted_roastery, extracted_bean_name) = Self::extract_names_from_blocks(blocks, clean_text);

        let process = Self::find_first_match(clean_text, PROCESSES);
        let variety = Self::find_first_match(clean_text, VARIETALS);
        let origin = Self::find_first_match(clean_text, ORIGINS);
        let tasting_notes = Self::find_all_matches(clean_text, TASTING_NOTES);
        let altitude = Self::extract_altitude(clean_text);

        ExtractedLexiconData {
            roastery: extracted_roastery,
            bean_name: extracted_bean_name,
            origin,
            variety,
            process,
            altitude,
            tasting_notes,
        }
    }

    fn find_first_match(text: &str, dictionary: &[&str]) -> Option<String> {
        if let Ok(ac) = AhoCorasickBuilder::new().match_kind(MatchKind::LeftmostLongest).ascii_case_insensitive(true).build(dictionary) {
            if let Some(m) = ac.find(text) {
                return dictionary.get(m.pattern().as_usize()).map(|s| s.to_string());
            }
        }
        None
    }

    fn find_all_matches(text: &str, dictionary: &[&str]) -> Vec<String> {
        let mut results = Vec::new();
        if let Ok(ac) = AhoCorasickBuilder::new().match_kind(MatchKind::LeftmostLongest).ascii_case_insensitive(true).build(dictionary) {
            for m in ac.find_iter(text) {
                if let Some(val) = dictionary.get(m.pattern().as_usize()) {
                    let s = val.to_string();
                    if !results.contains(&s) {
                        results.push(s);
                    }
                }
            }
        }
        results
    }

    fn extract_altitude(text: &str) -> Option<String> {
        if let Ok(re) = Regex::new(r"(?i)(\d{3,4})\s*(?:-\s*(\d{3,4}))?\s*(?:mdpl|masl|m\b)") {
            if let Some(caps) = re.captures(text) {
                if let Some(m) = caps.get(0) {
                    return Some(m.as_str().trim().to_uppercase());
                }
            }
        }
        None
    }

    fn extract_names_from_blocks(blocks: &[OcrTextBlockDto], full_text: &str) -> (Option<String>, Option<String>) {
        if blocks.is_empty() {
            let lines: Vec<&str> = full_text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
            let roastery = lines.first().map(|s| s.to_string());
            let name = lines.get(1).or_else(|| lines.first()).map(|s| s.to_string());
            return (roastery, name);
        }

        // Sort by font size descending, top_y ascending
        let mut sorted = blocks.to_vec();
        sorted.sort_by(|a, b| {
            b.normalized_height.partial_cmp(&a.normalized_height)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.top_y.partial_cmp(&b.top_y).unwrap_or(std::cmp::Ordering::Equal))
        });

        let primary_name = sorted.first().map(|b| b.text.trim().to_string());
        let secondary = sorted.get(1).map(|b| b.text.trim().to_string());

        (secondary, primary_name)
    }
}

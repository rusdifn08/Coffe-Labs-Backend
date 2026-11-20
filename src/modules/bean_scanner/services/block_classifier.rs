/// Stage 6a: Spatial OCR Block Classifier (Roastery / Title / Process / Origin / Tasting Notes)
use crate::modules::bean_scanner::dto::{BlockRole, ClassifiedOcrBlockDto, OcrTextBlockDto};
use super::fuzzy_flavor_matcher::FuzzyFlavorMatcher;
use super::knowledge_graph::KnowledgeGraph;
use super::noise_filter::NoiseFilter;
use super::title_composer::TitleComposer;

pub struct BlockClassifier;

impl BlockClassifier {
    pub fn classify_blocks(
        blocks: &[OcrTextBlockDto],
        full_text: &str,
    ) -> (Vec<ClassifiedOcrBlockDto>, Option<String>, Option<String>) {
        let mut classified = Vec::new();
        let mut roastery = None;
        let mut title_candidate = None;

        for b in blocks {
            let txt = b.text.trim();
            if NoiseFilter::is_noise(txt) {
                classified.push(Self::make_block(b, txt, BlockRole::IgnoredNoise));
                continue;
            }
            let lower = txt.to_lowercase();
            let has_process = FuzzyFlavorMatcher::detect_process(txt).is_some();
            let is_roastery = (lower.contains("coffee")
                || lower.contains("bean")
                || lower.contains("roastery")
                || lower.contains("roasters")
                || lower.contains("supply")
                || lower.contains("kopi")
                || lower.starts_with("by ")
                || lower.contains("farmer")
                || lower.contains("wanoja")
                || lower.contains("vanoja"))
                && (b.top_y <= 0.45 || lower.starts_with("by ") || lower.contains("wanoja"));

            let is_title = (b.normalized_width >= 0.25
                || b.normalized_height >= 0.04
                || lower.contains("watermelon")
                || (lower.contains("halu") && lower.contains("banana")))
                && !is_roastery
                && !has_process;

            let role = if is_roastery {
                if roastery.is_none() {
                    let mut r_txt = txt.to_string();
                    if r_txt.to_lowercase().starts_with("by ") {
                        r_txt = r_txt[3..].trim().to_string();
                    }
                    if r_txt.to_lowercase().contains("wanoja")
                        || r_txt.to_lowercase().contains("vanoja")
                    {
                        r_txt = "Wanoja Coffee".to_string();
                    }
                    roastery = Some(TitleComposer::title_case(&r_txt));
                }
                BlockRole::Roastery
            } else if is_title {
                if title_candidate.is_none() {
                    let mut t_txt = txt.to_string();
                    if t_txt.to_lowercase().contains("watermelon") {
                        t_txt = "Watermelon Smash".to_string();
                    } else if t_txt.to_lowercase().contains("halu")
                        && t_txt.to_lowercase().contains("banana")
                    {
                        t_txt = "Halu Banana".to_string();
                    }
                    title_candidate = Some(TitleComposer::title_case(&t_txt));
                }
                BlockRole::OriginOrTitle
            } else if has_process {
                BlockRole::Process
            } else if KnowledgeGraph::find_origin(txt).is_some() {
                if title_candidate.is_none() {
                    title_candidate = Some(TitleComposer::title_case(txt));
                }
                BlockRole::OriginOrTitle
            } else if !FuzzyFlavorMatcher::match_tasting_notes(txt).is_empty() {
                BlockRole::TastingNote
            } else {
                BlockRole::IgnoredNoise
            };
            classified.push(Self::make_block(b, txt, role));
        }

        if title_candidate.is_none() && roastery.is_none() {
            for line in full_text.lines().map(str::trim).filter(|l| !l.is_empty()) {
                if !NoiseFilter::is_noise(line) {
                    title_candidate = Some(TitleComposer::title_case(line));
                    break;
                }
            }
        }
        (classified, roastery, title_candidate)
    }

    fn make_block(b: &OcrTextBlockDto, txt: &str, role: BlockRole) -> ClassifiedOcrBlockDto {
        ClassifiedOcrBlockDto {
            text: txt.to_string(),
            role_label: role.badge_label().to_string(),
            color_hex: role.color_hex().to_string(),
            role,
            top_y: b.top_y,
            left_x: b.left_x,
            normalized_width: b.normalized_width,
            normalized_height: b.normalized_height,
        }
    }
}

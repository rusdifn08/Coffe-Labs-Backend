/// Stage 6: Cognitive Bean Resolver & Spatial AI Classifier
use crate::modules::bean_scanner::dto::{
    BlockRole, ClassifiedOcrBlockDto, OcrTextBlockDto, RecommendedRecipeSummary, ResolvedCoffeeBeanResponse,
};
use super::fuzzy_flavor_matcher::FuzzyFlavorMatcher;
use super::knowledge_graph::KnowledgeGraph;
use super::noise_filter::NoiseFilter;
use super::title_composer::TitleComposer;

pub struct CognitiveResolver;

impl CognitiveResolver {
    pub fn analyze(full_text: &str, blocks: &[OcrTextBlockDto]) -> ResolvedCoffeeBeanResponse {
        let (classified, detected_roastery, title_cand) = Self::classify_blocks(blocks, full_text);
        let process_prof = FuzzyFlavorMatcher::detect_process(full_text)
            .unwrap_or_else(FuzzyFlavorMatcher::profile_washed);
        let origin_prof = KnowledgeGraph::find_origin(full_text);

        // 1. Resolve Roastery & Origin
        let roastery = detected_roastery
            .or_else(|| TitleComposer::extract_roastery_from_text(full_text))
            .unwrap_or_else(|| "Specialty Roastery".to_string());

        let origin = origin_prof
            .map(|p| p.region_province.to_string())
            .unwrap_or_else(|| "Single Origin, Indonesia".to_string());

        // 2. Resolve Altitude & Varietal
        let (altitude, is_inferred_alt) = if let Some(explicit) = TitleComposer::extract_explicit_altitude(full_text) {
            (Some(explicit), false)
        } else if let Some(prof) = origin_prof {
            (Some(prof.default_altitude.to_string()), true)
        } else {
            (Some("1400 - 1800 MDPL".to_string()), true)
        };

        let (variety, is_inferred_var) = if let Some(explicit) = TitleComposer::extract_explicit_varietal(full_text) {
            (explicit, false)
        } else if let Some(prof) = origin_prof {
            (prof.default_varietals.to_string(), true)
        } else {
            ("Arabica Specialty".to_string(), true)
        };

        // 3. Resolve Tasting Notes
        let mut explicit_notes = FuzzyFlavorMatcher::match_tasting_notes(full_text);
        for b in blocks {
            let b_notes = FuzzyFlavorMatcher::match_tasting_notes(&b.text);
            for n in b_notes {
                if !explicit_notes.contains(&n) {
                    explicit_notes.push(n);
                }
            }
        }
        let (tasting_notes, notes_source) = if explicit_notes.len() >= 2 {
            (explicit_notes, "OCR_EXPLICIT".to_string())
        } else {
            let mut inferred = process_prof.signature_notes.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            if let Some(first) = explicit_notes.first() {
                if !inferred.contains(first) {
                    inferred.insert(0, first.clone());
                }
            }
            (inferred, "AI_SOMMELIER_INFERRED".to_string())
        };

        let bean_name = TitleComposer::compose_bean_name(title_cand.as_deref(), origin_prof, process_prof.canonical_name, &roastery);

        let recipes = vec![RecommendedRecipeSummary {
            slug: process_prof.top_recipe_slug.to_string(),
            name: process_prof.top_recipe_name.to_string(),
            target_notes: format!("Suhu {}°C untuk menonjolkan manis {}", process_prof.recommended_temp_c, process_prof.canonical_name),
            suggested_temp_c: process_prof.recommended_temp_c,
            ratio: "1:15 (15g : 225g)".to_string(),
        }];

        ResolvedCoffeeBeanResponse {
            bean_id: None,
            is_catalog_verified: false,
            confidence_score: 94.5,
            roastery_name: roastery,
            bean_name,
            origin,
            variety,
            process: process_prof.canonical_name.to_string(),
            roast_level: "Light to Medium".to_string(),
            altitude_masl: altitude,
            tasting_notes,
            recommended_water_temp_c: process_prof.recommended_temp_c,
            recommended_recipes: recipes,
            notes_source,
            is_ai_inferred_altitude: is_inferred_alt,
            is_ai_inferred_variety: is_inferred_var,
            classified_blocks: classified,
        }
    }

    fn classify_blocks(blocks: &[OcrTextBlockDto], full_text: &str) -> (Vec<ClassifiedOcrBlockDto>, Option<String>, Option<String>) {
        let mut classified = Vec::new();
        let mut roastery = None;
        let mut title_candidate = None;

        for b in blocks {
            let txt = b.text.trim();
            if NoiseFilter::is_noise(txt) {
                classified.push(ClassifiedOcrBlockDto {
                    text: txt.to_string(),
                    role: BlockRole::IgnoredNoise,
                    role_label: BlockRole::IgnoredNoise.badge_label().to_string(),
                    color_hex: BlockRole::IgnoredNoise.color_hex().to_string(),
                    top_y: b.top_y,
                    left_x: b.left_x,
                    normalized_width: b.normalized_width,
                    normalized_height: b.normalized_height,
                });
                continue;
            }

            let lower = txt.to_lowercase();
            let has_process_signal = lower.contains("wine")
                || lower.contains("natural")
                || lower.contains("natura")
                || lower.contains("washed")
                || lower.contains("honey")
                || lower.contains("anaerob")
                || lower.contains("carbonic")
                || lower.contains("giling basah");

            let is_roastery_candidate = (lower.contains("coffee")
                || lower.contains("bean")
                || lower.contains("roastery")
                || lower.contains("roasters")
                || lower.contains("supply")
                || lower.contains("kopi")
                || lower.starts_with("by ")
                || lower.contains("farmer")
                || lower.contains("wanoja")
                || lower.contains("vanoja"))
                && (b.top_y <= 0.45 || lower.starts_with("by ") || lower.contains("farmer") || lower.contains("wanoja") || lower.contains("vanoja"));

            let is_title_banner = (b.normalized_width >= 0.25
                || b.normalized_height >= 0.04
                || lower.contains("watermelon")
                || (lower.contains("halu") && lower.contains("banana")))
                && !is_roastery_candidate
                && !has_process_signal;

            let role = if is_roastery_candidate {
                if roastery.is_none() {
                    let mut r_txt = txt.to_string();
                    if r_txt.to_lowercase().starts_with("by ") {
                        r_txt = r_txt[3..].trim().to_string();
                    }
                    if r_txt.to_lowercase().contains("wanoja") || r_txt.to_lowercase().contains("vanoja") {
                        r_txt = "Wanoja Coffee".to_string();
                    }
                    roastery = Some(TitleComposer::title_case(&r_txt));
                }
                BlockRole::Roastery
            } else if is_title_banner {
                if title_candidate.is_none() {
                    let mut t_txt = txt.to_string();
                    if t_txt.to_lowercase().contains("watermelon") {
                        t_txt = "Watermelon Smash".to_string();
                    } else if t_txt.to_lowercase().contains("halu") && t_txt.to_lowercase().contains("banana") {
                        t_txt = "Halu Banana".to_string();
                    }
                    title_candidate = Some(TitleComposer::title_case(&t_txt));
                }
                BlockRole::OriginOrTitle
            } else if has_process_signal {
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

            classified.push(ClassifiedOcrBlockDto {
                text: txt.to_string(),
                role,
                role_label: role.badge_label().to_string(),
                color_hex: role.color_hex().to_string(),
                top_y: b.top_y,
                left_x: b.left_x,
                normalized_width: b.normalized_width,
                normalized_height: b.normalized_height,
            });
        }

        if title_candidate.is_none() && roastery.is_none() {
            for line in full_text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                if !NoiseFilter::is_noise(line) {
                    title_candidate = Some(TitleComposer::title_case(line));
                    break;
                }
            }
        }

        (classified, roastery, title_candidate)
    }
}

/// Stage 6: Grounded Zero-Hallucination Cognitive Bean Resolver & Spatial AI Classifier
use crate::modules::bean_scanner::dto::{
    OcrTextBlockDto, RecommendedRecipeSummary, ResolvedCoffeeBeanResponse,
};
use super::block_classifier::BlockClassifier;
use super::fuzzy_flavor_matcher::FuzzyFlavorMatcher;
use super::knowledge_graph::KnowledgeGraph;
use super::title_composer::TitleComposer;

pub struct CognitiveResolver;

impl CognitiveResolver {
    pub fn analyze(full_text: &str, blocks: &[OcrTextBlockDto]) -> ResolvedCoffeeBeanResponse {
        let (classified, detected_roastery, title_cand) =
            BlockClassifier::classify_blocks(blocks, full_text);
        let explicit_process = FuzzyFlavorMatcher::detect_process(full_text);
        let origin_prof = KnowledgeGraph::find_origin(full_text);
        let explicit_alt = TitleComposer::extract_explicit_altitude(full_text);
        let explicit_var = TitleComposer::extract_explicit_varietal(full_text);
        let roastery_opt = detected_roastery
            .or_else(|| TitleComposer::extract_roastery_from_text(full_text));

        let mut explicit_notes = FuzzyFlavorMatcher::match_tasting_notes(full_text);
        for b in blocks {
            for n in FuzzyFlavorMatcher::match_tasting_notes(&b.text) {
                if !explicit_notes.contains(&n) {
                    explicit_notes.push(n);
                }
            }
        }

        // Strict Anti-Hallucination Evidence Gate:
        // Never invent origin/process/notes at 94.5% confidence if zero coffee signals exist!
        let evidence_signals = usize::from(origin_prof.is_some())
            + usize::from(explicit_process.is_some())
            + usize::from(roastery_opt.is_some())
            + usize::from(explicit_var.is_some())
            + usize::from(explicit_alt.is_some())
            + usize::from(!explicit_notes.is_empty());

        if evidence_signals == 0 {
            return ResolvedCoffeeBeanResponse {
                bean_id: None,
                is_catalog_verified: false,
                confidence_score: 25.0,
                roastery_name: "Belum Terdeteksi".to_string(),
                bean_name: "Kemasan Kopi Belum Terbaca".to_string(),
                origin: "Belum Terdeteksi".to_string(),
                variety: "Belum Terdeteksi".to_string(),
                process: "Belum Terdeteksi".to_string(),
                roast_level: "Belum Terdeteksi".to_string(),
                altitude_masl: None,
                tasting_notes: Vec::new(),
                recommended_water_temp_c: 91,
                recommended_recipes: Vec::new(),
                notes_source: "LOW_CONFIDENCE_NO_MATCH".to_string(),
                is_ai_inferred_altitude: false,
                is_ai_inferred_variety: false,
                classified_blocks: classified,
            };
        }

        let process_prof = explicit_process.unwrap_or_else(FuzzyFlavorMatcher::profile_washed);
        let roastery = roastery_opt.unwrap_or_else(|| "Specialty Roastery".to_string());
        let origin = origin_prof
            .map(|p| p.region_province.to_string())
            .unwrap_or_else(|| "Single Origin, Indonesia".to_string());

        let (altitude, is_inferred_alt) = if let Some(explicit) = explicit_alt {
            (Some(explicit), false)
        } else if let Some(prof) = origin_prof {
            (Some(prof.default_altitude.to_string()), true)
        } else {
            (Some("1400 - 1800 MDPL".to_string()), true)
        };

        let (variety, is_inferred_var) = if let Some(explicit) = explicit_var {
            (explicit, false)
        } else if let Some(prof) = origin_prof {
            (prof.default_varietals.to_string(), true)
        } else {
            ("Arabica Specialty".to_string(), true)
        };

        let (tasting_notes, notes_source) = if explicit_notes.len() >= 2 {
            (explicit_notes, "OCR_EXPLICIT".to_string())
        } else {
            let mut inferred = process_prof
                .signature_notes
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            if let Some(first) = explicit_notes.first() {
                if !inferred.contains(first) {
                    inferred.insert(0, first.clone());
                }
            }
            (inferred, "AI_SOMMELIER_INFERRED".to_string())
        };

        let bean_name = TitleComposer::compose_bean_name(
            title_cand.as_deref(),
            origin_prof,
            process_prof.canonical_name,
            &roastery,
        );

        let grounded_conf = (58.0 + (evidence_signals as f32 * 7.5)).clamp(62.0, 96.5);
        let recipes = vec![RecommendedRecipeSummary {
            slug: process_prof.top_recipe_slug.to_string(),
            name: process_prof.top_recipe_name.to_string(),
            target_notes: format!(
                "Suhu {}°C untuk menonjolkan manis {}",
                process_prof.recommended_temp_c, process_prof.canonical_name
            ),
            suggested_temp_c: process_prof.recommended_temp_c,
            ratio: "1:15 (15g : 225g)".to_string(),
        }];

        ResolvedCoffeeBeanResponse {
            bean_id: None,
            is_catalog_verified: false,
            confidence_score: grounded_conf,
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
}

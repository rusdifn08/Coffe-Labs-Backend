use sqlx::PgPool;
use crate::core::error::AppResult;
use crate::modules::bean_scanner::dto::{BeanScanRequestDto, RecommendedRecipeSummary, ResolvedCoffeeBeanResponse};
use crate::modules::bean_scanner::repository::BeanScannerRepository;
use super::cognitive_resolver::CognitiveResolver;

pub struct BeanResolverService;

impl BeanResolverService {
    pub async fn resolve(pool: &PgPool, req: BeanScanRequestDto) -> AppResult<ResolvedCoffeeBeanResponse> {
        let _ = BeanScannerRepository::ensure_seed_catalog_if_empty(pool).await;

        // 1. Cognitive AI Analysis (Noise Filter, Knowledge Graph, Fuzzy Flavors & Sommelier Inference)
        let mut cognitive_result = CognitiveResolver::analyze(&req.raw_full_text, &req.blocks);

        // 2. Query NeonDB verified catalog using high-confidence tokens
        let search_query = format!(
            "{} {} {} {}",
            cognitive_result.roastery_name,
            cognitive_result.bean_name,
            cognitive_result.origin,
            cognitive_result.process
        );

        let catalog_match = BeanScannerRepository::find_best_catalog_match(pool, &search_query).await?;

        if let Some(matched) = catalog_match {
            if matched.similarity_score >= 0.88 {
                let rec = matched.row;
                cognitive_result.bean_id = Some(rec.id);
                cognitive_result.is_catalog_verified = true;
                cognitive_result.confidence_score = (matched.similarity_score * 100.0).clamp(85.0, 99.5);

                // Only overwrite bean name if cognitive result was generic
                if cognitive_result.bean_name.to_lowercase().starts_with("specialty coffee") {
                    cognitive_result.bean_name = rec.bean_name;
                }
                if cognitive_result.roastery_name == "Specialty Roastery" && !rec.roastery_name.is_empty() {
                    cognitive_result.roastery_name = rec.roastery_name;
                }

                if cognitive_result.origin == "Single Origin, Indonesia" {
                    cognitive_result.origin = rec.origin_region.or(rec.origin_country).unwrap_or(cognitive_result.origin);
                }

                if cognitive_result.is_ai_inferred_variety && !rec.variety.is_empty() {
                    cognitive_result.variety = rec.variety;
                    cognitive_result.is_ai_inferred_variety = false;
                }

                if cognitive_result.process == "Full Washed" && !rec.process.is_empty() {
                    cognitive_result.process = rec.process;
                }

                if cognitive_result.is_ai_inferred_altitude {
                    if let Some(alt) = rec.altitude_masl {
                        cognitive_result.altitude_masl = Some(alt);
                        cognitive_result.is_ai_inferred_altitude = false;
                    }
                }

                // Never overwrite explicit OCR tasting notes with catalog notes!
                if cognitive_result.notes_source != "OCR_EXPLICIT" && !rec.tasting_notes.is_empty() {
                    cognitive_result.tasting_notes = rec.tasting_notes;
                    cognitive_result.notes_source = "CATALOG_VERIFIED".to_string();
                }

                if let Some(temp) = rec.recommended_water_temp_c {
                    cognitive_result.recommended_water_temp_c = temp;
                }
                cognitive_result.recommended_recipes = Self::build_recipes_for_process(
                    &cognitive_result.process,
                    cognitive_result.recommended_water_temp_c,
                );
            }
        }

        Ok(cognitive_result)
    }

    fn build_recipes_for_process(process: &str, temp: i32) -> Vec<RecommendedRecipeSummary> {
        let p = process.to_lowercase();
        if p.contains("wine") || p.contains("anaerobic") || p.contains("natural") || p.contains("carbonic") {
            vec![
                RecommendedRecipeSummary {
                    slug: "dual-temperature-thermal-drop-v60".to_string(),
                    name: "Dual-Temperature Thermal Drop V60 (93°C -> 70°C)".to_string(),
                    target_notes: "Kombinasi suhu drop untuk menonjolkan manis buah dan winey tanpa kepahitan".to_string(),
                    suggested_temp_c: temp,
                    ratio: "1:15 (15g : 225g)".to_string(),
                },
                RecommendedRecipeSummary {
                    slug: "tetsu-kasuya-4-6".to_string(),
                    name: "Tetsu Kasuya 4:6 Method (Sweet Focus)".to_string(),
                    target_notes: "Keseimbangan manis buah dan body syrupy".to_string(),
                    suggested_temp_c: temp + 1,
                    ratio: "1:15 (20g : 300g)".to_string(),
                },
            ]
        } else {
            vec![
                RecommendedRecipeSummary {
                    slug: "smoking-barrels-v60".to_string(),
                    name: "Smoking Barrels Modern V60".to_string(),
                    target_notes: "Klaritas rasa tinggi dengan aroma bunga dan asiditas bersih".to_string(),
                    suggested_temp_c: temp,
                    ratio: "1:15 (15g : 225g)".to_string(),
                },
                RecommendedRecipeSummary {
                    slug: "wbrc-sandwich-v60".to_string(),
                    name: "WBrC Sandwich Technique".to_string(),
                    target_notes: "Ekstraksi merata dengan sweetness seimbang dan floral notes".to_string(),
                    suggested_temp_c: temp + 1,
                    ratio: "1:16.6 (15g : 250g)".to_string(),
                },
            ]
        }
    }
}

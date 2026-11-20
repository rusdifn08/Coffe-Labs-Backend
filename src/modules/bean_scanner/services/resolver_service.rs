use sqlx::PgPool;
use crate::core::error::AppResult;
use crate::modules::bean_scanner::dto::{
    BeanScanRequestDto, RecommendedRecipeSummary, ResolvedCoffeeBeanResponse,
};
use crate::modules::bean_scanner::repository::BeanScannerRepository;
use super::cognitive_resolver::CognitiveResolver;

pub struct BeanResolverService;

impl BeanResolverService {
    pub async fn resolve(
        pool: &PgPool,
        req: BeanScanRequestDto,
    ) -> AppResult<ResolvedCoffeeBeanResponse> {
        let _ = BeanScannerRepository::ensure_seed_catalog_if_empty(pool).await;

        // 1. Grounded On-Engine Cognitive AI Analysis (Sabang-Merauke + 65 World Countries)
        let mut res = CognitiveResolver::analyze(&req.raw_full_text, &req.blocks);

        // 2. Query 171,000+ NeonDB `coffee_ocr_lexicon` GIN pg_trgm tokens to recover OCR typos
        let lines: Vec<String> = req
            .raw_full_text
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| l.len() >= 3)
            .collect();
        let lex_hits = BeanScannerRepository::lookup_ocr_lexicon(pool, &lines).await?;
        for hit in &lex_hits {
            if hit.sim_score < 0.65 {
                continue;
            }
            match hit.entity_type.as_str() {
                "ROASTERY" if res.roastery_name == "Specialty Roastery" || res.roastery_name == "Belum Terdeteksi" => {
                    res.roastery_name = hit.canonical_value.clone();
                    res.confidence_score = (res.confidence_score + 12.0).min(97.0);
                }
                "ORIGIN" if res.origin == "Single Origin, Indonesia" || res.origin == "Belum Terdeteksi" => {
                    res.origin = hit.canonical_value.clone();
                    res.confidence_score = (res.confidence_score + 14.0).min(97.0);
                }
                "VARIETY" if res.is_ai_inferred_variety || res.variety == "Belum Terdeteksi" => {
                    res.variety = hit.canonical_value.clone();
                    res.is_ai_inferred_variety = false;
                    res.confidence_score = (res.confidence_score + 8.0).min(97.0);
                }
                "PROCESS" if res.process == "Belum Terdeteksi" => {
                    res.process = hit.canonical_value.clone();
                    res.confidence_score = (res.confidence_score + 10.0).min(97.0);
                }
                "FLAVOR" if !res.tasting_notes.contains(&hit.canonical_value) && res.tasting_notes.len() < 5 => {
                    res.tasting_notes.push(hit.canonical_value.clone());
                }
                _ => {}
            }
        }

        // 3. Query 2,155+ verified `coffee_beans_catalog` using pg_trgm GIN index
        if res.notes_source == "LOW_CONFIDENCE_NO_MATCH" && lex_hits.is_empty() {
            return Ok(res);
        }

        let search_query = format!(
            "{} {} {} {}",
            res.roastery_name, res.bean_name, res.origin, res.process
        );
        if let Some(matched) = BeanScannerRepository::find_best_catalog_match(pool, &search_query).await? {
            if matched.similarity_score >= 0.48 {
                let rec = matched.row;
                res.bean_id = Some(rec.id);
                res.is_catalog_verified = matched.similarity_score >= 0.58;
                res.confidence_score = ((matched.similarity_score * 100.0).max(res.confidence_score)).clamp(72.0, 99.5);

                if res.bean_name.to_lowercase().starts_with("specialty coffee")
                    || res.bean_name == "Kemasan Kopi Belum Terbaca"
                {
                    res.bean_name = rec.bean_name;
                }
                if (res.roastery_name == "Specialty Roastery" || res.roastery_name == "Belum Terdeteksi")
                    && !rec.roastery_name.is_empty()
                {
                    res.roastery_name = rec.roastery_name;
                }
                if res.origin == "Single Origin, Indonesia" || res.origin == "Belum Terdeteksi" {
                    res.origin = rec.origin_region.or(rec.origin_country).unwrap_or(res.origin);
                }
                if (res.is_ai_inferred_variety || res.variety == "Belum Terdeteksi") && !rec.variety.is_empty() {
                    res.variety = rec.variety;
                    res.is_ai_inferred_variety = false;
                }
                if (res.process == "Full Washed" || res.process == "Belum Terdeteksi") && !rec.process.is_empty() {
                    res.process = rec.process;
                }
                if res.is_ai_inferred_altitude || res.altitude_masl.is_none() {
                    if let Some(alt) = rec.altitude_masl {
                        res.altitude_masl = Some(alt);
                        res.is_ai_inferred_altitude = false;
                    }
                }
                if res.notes_source != "OCR_EXPLICIT" && !rec.tasting_notes.is_empty() {
                    res.tasting_notes = rec.tasting_notes;
                    res.notes_source = "CATALOG_VERIFIED".to_string();
                }
                if let Some(temp) = rec.recommended_water_temp_c {
                    res.recommended_water_temp_c = temp;
                }
                res.recommended_recipes = Self::build_recipes_for_process(
                    &res.process,
                    res.recommended_water_temp_c,
                );
            }
        }

        Ok(res)
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

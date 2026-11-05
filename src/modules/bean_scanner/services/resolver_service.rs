use sqlx::PgPool;
use crate::core::error::AppResult;
use crate::modules::bean_scanner::dto::{BeanScanRequestDto, RecommendedRecipeSummary, ResolvedCoffeeBeanResponse};
use crate::modules::bean_scanner::repository::BeanScannerRepository;
use crate::modules::bean_scanner::services::lexicon_extractor::LexiconExtractor;

pub struct BeanResolverService;

impl BeanResolverService {
    pub async fn resolve(pool: &PgPool, req: BeanScanRequestDto) -> AppResult<ResolvedCoffeeBeanResponse> {
        let _ = BeanScannerRepository::ensure_seed_catalog_if_empty(pool).await;

        let lexicon = LexiconExtractor::extract(&req.raw_full_text, &req.blocks);

        let query = format!(
            "{} {} {} {} {}",
            lexicon.roastery.as_deref().unwrap_or(""),
            lexicon.bean_name.as_deref().unwrap_or(""),
            lexicon.origin.as_deref().unwrap_or(""),
            lexicon.variety.as_deref().unwrap_or(""),
            lexicon.process.as_deref().unwrap_or("")
        );
        let final_query = if query.trim().is_empty() {
            req.raw_full_text.chars().take(120).collect::<String>()
        } else {
            query
        };

        let catalog_match = BeanScannerRepository::find_best_catalog_match(pool, &final_query).await?;

        if let Some(matched) = catalog_match {
            if matched.similarity_score >= 0.72 {
                let rec = matched.row;
                let recipes = Self::build_recipes_for_process(&rec.process, rec.recommended_water_temp_c.unwrap_or(91));
                return Ok(ResolvedCoffeeBeanResponse {
                    bean_id: Some(rec.id),
                    is_catalog_verified: true,
                    confidence_score: (matched.similarity_score * 100.0).clamp(75.0, 99.5),
                    roastery_name: rec.roastery_name,
                    bean_name: rec.bean_name,
                    origin: rec.origin_region.or(rec.origin_country).unwrap_or_else(|| "Single Origin".to_string()),
                    variety: rec.variety,
                    process: rec.process,
                    roast_level: rec.roast_level,
                    altitude_masl: rec.altitude_masl,
                    tasting_notes: rec.tasting_notes,
                    recommended_water_temp_c: rec.recommended_water_temp_c.unwrap_or(91),
                    recommended_recipes: recipes,
                });
            }
        }

        // Unlisted or independent roastery: construct from rich Lexicon Extractor
        let process_name = lexicon.process.unwrap_or_else(|| "Washed".to_string());
        let water_temp = Self::suggest_temp(&process_name);
        let recipes = Self::build_recipes_for_process(&process_name, water_temp);
        let notes = if lexicon.tasting_notes.is_empty() {
            vec!["Caramel".to_string(), "Citrus".to_string(), "Floral".to_string()]
        } else {
            lexicon.tasting_notes
        };

        Ok(ResolvedCoffeeBeanResponse {
            bean_id: None,
            is_catalog_verified: false,
            confidence_score: 88.5,
            roastery_name: lexicon.roastery.unwrap_or_else(|| "Specialty Roastery".to_string()),
            bean_name: lexicon.bean_name.unwrap_or_else(|| "Single Origin Selection".to_string()),
            origin: lexicon.origin.unwrap_or_else(|| "Specialty Origin".to_string()),
            variety: lexicon.variety.unwrap_or_else(|| "Specialty Arabica".to_string()),
            process: process_name,
            roast_level: "Light to Medium".to_string(),
            altitude_masl: lexicon.altitude,
            tasting_notes: notes,
            recommended_water_temp_c: water_temp,
            recommended_recipes: recipes,
        })
    }

    fn suggest_temp(process: &str) -> i32 {
        let p = process.to_lowercase();
        if p.contains("anaerobic") || p.contains("natural") || p.contains("carbonic") {
            90
        } else if p.contains("honey") {
            91
        } else {
            92
        }
    }

    fn build_recipes_for_process(process: &str, temp: i32) -> Vec<RecommendedRecipeSummary> {
        let p = process.to_lowercase();
        if p.contains("anaerobic") || p.contains("natural") || p.contains("carbonic") {
            vec![
                RecommendedRecipeSummary {
                    slug: "dual-temperature-thermal-drop-v60".to_string(),
                    name: "Dual-Temperature Thermal Drop V60".to_string(),
                    target_notes: "Sangat cocok untuk kopi proses fermentasi/natural agar manis maksimal tanpa astringency pahit".to_string(),
                    suggested_temp_c: temp,
                    ratio: "1:15 (15g : 225g)".to_string(),
                },
                RecommendedRecipeSummary {
                    slug: "tetsu-kasuya-4-6".to_string(),
                    name: "Tetsu Kasuya 4:6 Method".to_string(),
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
                    slug: "james-hoffmann-better-1-cup".to_string(),
                    name: "James Hoffmann Better 1-Cup V60".to_string(),
                    target_notes: "Ekstraksi merata dengan sweetness seimbang".to_string(),
                    suggested_temp_c: temp + 1,
                    ratio: "1:16.6 (15g : 250g)".to_string(),
                },
            ]
        }
    }
}

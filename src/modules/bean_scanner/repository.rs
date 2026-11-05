use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use crate::core::error::AppResult;
use crate::modules::bean_scanner::models::CoffeeBeanCatalogRow;

pub struct BeanScannerRepository;

#[derive(Debug, Clone, FromRow)]
pub struct CatalogMatchDbRow {
    pub id: Uuid,
    pub roastery_id: Option<Uuid>,
    pub roastery_name: String,
    pub bean_name: String,
    pub origin_country: Option<String>,
    pub origin_region: Option<String>,
    pub variety: String,
    pub process: String,
    pub roast_level: String,
    pub altitude_masl: Option<String>,
    pub tasting_notes: Vec<String>,
    pub recommended_water_temp_c: Option<i32>,
    pub recommended_recipe_slugs: Vec<String>,
    pub packaging_image_url: Option<String>,
    pub search_vector_text: Option<String>,
    pub is_verified: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub sim_score: f32,
}

#[derive(Debug, Clone)]
pub struct CatalogMatchResult {
    pub row: CoffeeBeanCatalogRow,
    pub similarity_score: f32,
}

impl BeanScannerRepository {
    pub async fn find_best_catalog_match(pool: &PgPool, query: &str) -> AppResult<Option<CatalogMatchResult>> {
        let clean = query.trim().to_lowercase();
        if clean.is_empty() {
            return Ok(None);
        }

        let sql = r#"
            SELECT 
                id, roastery_id, roastery_name, bean_name, origin_country, origin_region,
                variety, process, roast_level, altitude_masl, tasting_notes,
                recommended_water_temp_c, recommended_recipe_slugs, packaging_image_url,
                search_vector_text, is_verified, created_at,
                similarity(search_vector_text, $1) AS sim_score
            FROM coffee_beans_catalog
            ORDER BY sim_score DESC
            LIMIT 1
        "#;

        let record = sqlx::query_as::<_, CatalogMatchDbRow>(sql)
            .bind(&clean)
            .fetch_optional(pool)
            .await?;

        if let Some(r) = record {
            let row = CoffeeBeanCatalogRow {
                id: r.id,
                roastery_id: r.roastery_id,
                roastery_name: r.roastery_name,
                bean_name: r.bean_name,
                origin_country: r.origin_country,
                origin_region: r.origin_region,
                variety: r.variety,
                process: r.process,
                roast_level: r.roast_level,
                altitude_masl: r.altitude_masl,
                tasting_notes: r.tasting_notes,
                recommended_water_temp_c: r.recommended_water_temp_c,
                recommended_recipe_slugs: r.recommended_recipe_slugs,
                packaging_image_url: r.packaging_image_url,
                search_vector_text: r.search_vector_text,
                is_verified: r.is_verified,
                created_at: r.created_at,
            };
            return Ok(Some(CatalogMatchResult {
                row,
                similarity_score: r.sim_score,
            }));
        }

        Ok(None)
    }

    pub async fn ensure_seed_catalog_if_empty(pool: &PgPool) -> AppResult<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM coffee_beans_catalog")
            .fetch_one(pool)
            .await?;

        if count.0 == 0 {
            let seed_sql = r#"
                INSERT INTO coffee_beans_catalog (
                    roastery_name, bean_name, origin_country, origin_region, variety, process, 
                    roast_level, altitude_masl, tasting_notes, recommended_water_temp_c, recommended_recipe_slugs, is_verified
                )
                VALUES
                ('Smoking Barrels', 'Halmahera Spice Island', 'Indonesia', 'Halmahera', 'Kartika & Abyssinia', 'Natural Anaerobic', 'Light', '1500 - 1700 MDPL', ARRAY['Strawberry Jam', 'Cacao Nibs', 'Rose Wine', 'Spices'], 90, ARRAY['dual-temperature-thermal-drop-v60', 'tetsu-kasuya-4-6'], true),
                ('Smoking Barrels', 'Flores Bajawa', 'Indonesia', 'Bajawa', 'Typica & S795', 'Full Washed', 'Light to Medium', '1400 - 1600 MDPL', ARRAY['Milk Chocolate', 'Caramel', 'Hazelnut', 'Green Apple'], 92, ARRAY['smoking-barrels-v60', 'james-hoffmann-better-1-cup'], true),
                ('Common Grounds', 'Finca Deborah Nirvana', 'Panama', 'Boquete', 'Geisha', 'Nitrogen Anaerobic', 'Light', '1900 - 2000 MASL', ARRAY['Jasmine', 'Bergamot', 'Peach', 'Orange Blossom'], 91, ARRAY['lance-hedrick-1-pour-v60', 'tetsu-kasuya-4-6'], true),
                ('Instinct Roastery', 'Gayo Honey Reserve', 'Indonesia', 'Aceh Gayo', 'Abyssinia & Typica', 'Black Honey', 'Light to Medium', '1600 MDPL', ARRAY['Tropical Fruit', 'Honey', 'Melon', 'Sweet Orange'], 91, ARRAY['dual-temperature-thermal-drop-v60', 'scott-rao-v60'], true),
                ('Fuglen', 'Ethiopia Yirgacheffe Chelchele', 'Ethiopia', 'Yirgacheffe', 'Heirloom', 'Full Washed', 'Ultra Light', '1950 - 2100 MASL', ARRAY['Floral', 'Earl Grey', 'Lemon Peel', 'White Peach'], 93, ARRAY['james-hoffmann-better-1-cup', 'lance-hedrick-1-pour-v60'], true)
                ON CONFLICT DO NOTHING;
            "#;
            let _ = sqlx::query(seed_sql).execute(pool).await;
        }
        Ok(())
    }
}

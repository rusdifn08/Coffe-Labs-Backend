use sqlx::PgPool;

use crate::modules::brew_methods_audit::{
    error::BrewAuditError, models::BrewMethodCatalogRow,
};

pub async fn list_all_methods(pool: &PgPool) -> Result<Vec<BrewMethodCatalogRow>, BrewAuditError> {
    let rows = sqlx::query_as::<_, BrewMethodCatalogRow>(
        r#"
        SELECT
            id,
            slug,
            display_name,
            display_name_id,
            archetype::text AS archetype,
            icon_asset_path,
            filter_key,
            is_background_alarm_mode,
            default_min_duration_sec,
            default_max_duration_sec,
            sort_order
        FROM brewing_methods_catalog
        ORDER BY sort_order ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

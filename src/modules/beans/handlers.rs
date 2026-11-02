use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use sqlx::PgPool;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::ValidatedJson,
    },
    modules::beans::{
        dto::{BeanCatalogQuery, DialInCalculationRequest},
        service,
    },
};

pub async fn get_beans_catalog(
    Query(query): Query<BeanCatalogQuery>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let beans = service::get_catalog(&pool, query).await?;
    Ok(ApiResponse::ok("Katalog biji kopi specialty berhasil dimuat.", beans))
}

pub async fn get_bean_radar(
    Query(query): Query<BeanCatalogQuery>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let beans = service::get_catalog(&pool, query).await?;
    Ok(ApiResponse::ok("Data Bean Radar roastery sekitar berhasil dimuat.", beans))
}

pub async fn calculate_ai_dial_in(
    ValidatedJson(payload): ValidatedJson<DialInCalculationRequest>,
) -> AppResult<impl IntoResponse> {
    let recommendation = service::calculate_dial_in(payload);
    Ok(ApiResponse::ok(
        "Rekomendasi ekstraksi AI Barista berhasil dihitung.",
        recommendation,
    ))
}

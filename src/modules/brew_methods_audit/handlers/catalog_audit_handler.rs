use axum::{extract::State, response::IntoResponse, Json};

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        AppState,
    },
    modules::brew_methods_audit::{
        archetype_validator::validate_recipe_audit,
        coldbrew_math::compute_cold_brew_plan,
        dto::{AuditRecipeRequest, CalculateColdBrewRequest},
        repositories::method_catalog_repo,
    },
};

pub async fn get_methods_catalog(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let methods = method_catalog_repo::list_all_methods(&state.pool)
        .await
        .map_err(AppError::from)?;
    Ok(ApiResponse::ok("Katalog 18 metode seduh & archetype berhasil dimuat.", methods))
}

pub async fn post_audit_recipe(
    Json(req): Json<AuditRecipeRequest>,
) -> AppResult<impl IntoResponse> {
    let report = validate_recipe_audit(&req);
    Ok(ApiResponse::ok("Audit resep berbasis archetype selesai.", report))
}

pub async fn post_calculate_coldbrew_plan(
    Json(req): Json<CalculateColdBrewRequest>,
) -> AppResult<impl IntoResponse> {
    let plan = compute_cold_brew_plan(
        &req.brew_mode,
        req.coffee_dose_g,
        req.water_ml,
        req.ambient_temp_c,
        req.drip_rate_bpm,
    );
    Ok(ApiResponse::ok("Kalkulasi Arrhenius Cold Brew & Cold Drip siap.", plan))
}

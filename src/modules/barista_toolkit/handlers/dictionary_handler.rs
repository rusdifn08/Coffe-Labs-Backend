use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        redis::{redis_get_bounded, redis_set_ex_bounded},
        AppState,
    },
    modules::barista_toolkit::{
        dto::{CoffeeDictionaryEntryDto, DictionaryFilterQuery},
        models::CoffeeDictionaryRow,
        repositories::ebook_ast_repo,
    },
};

fn to_dto(r: CoffeeDictionaryRow) -> CoffeeDictionaryEntryDto {
    CoffeeDictionaryEntryDto {
        id: r.id,
        term_slug: r.term_slug,
        alphabet_letter: r.alphabet_letter,
        term_title: r.term_title,
        category: r.category,
        short_definition_id: r.short_definition_id,
        deep_explanation_id: r.deep_explanation_id,
        related_formula_latex: r.related_formula_latex,
        illustration_cloudinary_url: r.illustration_cloudinary_url,
        related_book_code: r.related_book_code,
        related_chapter_num: r.related_chapter_num,
    }
}

pub async fn list_dictionary_entries(
    State(state): State<AppState>,
    Query(query): Query<DictionaryFilterQuery>,
) -> AppResult<impl IntoResponse> {
    let unfiltered = query.letter.is_none() && query.q.is_none();
    let cache_key = "coffee_dict:v3:all";
    if unfiltered {
        if let Some(pool) = state.redis.active_pool() {
            if let Some(cached) = redis_get_bounded(&pool, cache_key).await {
                if let Ok(items) = serde_json::from_str::<Vec<CoffeeDictionaryEntryDto>>(&cached) {
                    return Ok(ApiResponse::ok("Kamus Kopi A-Z dimuat dari cache.", items));
                }
            }
        }
    }

    let rows = ebook_ast_repo::fetch_dictionary_entries(
        &state.pool,
        query.letter.as_deref(),
        query.q.as_deref(),
    )
    .await
    .map_err(AppError::from)?;
    let out: Vec<CoffeeDictionaryEntryDto> = rows.into_iter().map(to_dto).collect();

    if unfiltered {
        if let Some(pool) = state.redis.active_pool() {
            if let Ok(json_str) = serde_json::to_string(&out) {
                redis_set_ex_bounded(&pool, cache_key, json_str, 900).await;
            }
        }
    }

    Ok(ApiResponse::ok("Kamus Istilah Kopi Spesialti A-Z siap digunakan.", out))
}

pub async fn get_dictionary_term(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<impl IntoResponse> {
    let row = ebook_ast_repo::fetch_dictionary_entry_by_slug(&state.pool, &slug)
        .await
        .map_err(AppError::from)?;
    Ok(ApiResponse::ok("Definisi istilah kopi ditemukan.", to_dto(row)))
}

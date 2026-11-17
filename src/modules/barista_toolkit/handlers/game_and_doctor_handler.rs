use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
        AppState,
    },
    modules::barista_toolkit::{
        dto::{
            BrewDoctorDiagnoseRequest, GameBrew3DSessionSubmissionDto, GameBrewChapterDto,
            SubmitGameBrewScoreRequest, SubmitGameBrewScoreResponse,
        },
        error::BaristaToolkitError,
        repositories::game_brew_repo,
        services::{brew_doctor_engine, game_brew_service},
    },
};

pub async fn list_game_chapters(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let (chapters, progress) = tokio::try_join!(
        game_brew_repo::fetch_all_chapters(&state.pool),
        game_brew_repo::fetch_user_game_progress(&state.pool, claims.sub),
    )
    .map_err(AppError::from)?;

    let prog_map: HashMap<i16, _> = progress
        .into_iter()
        .map(|p| (p.chapter_number, p))
        .collect();

    let list: Vec<GameBrewChapterDto> = chapters
        .into_iter()
        .map(|c| {
            let p = prog_map.get(&c.chapter_number);
            let unlocked = c.chapter_number == 1 || p.is_some_and(|r| r.is_unlocked);
            GameBrewChapterDto {
                chapter_number: c.chapter_number,
                slug: c.slug,
                title: c.title,
                story_location: c.story_location,
                npc_mentor_name: c.npc_mentor_name,
                target_recipe_slug: c.target_recipe_slug,
                physics_config: c.physics_config_json,
                story_dialogue: c.story_dialogue_json,
                xp_reward_3_stars: c.xp_reward_3_stars,
                stars_earned: p.map(|r| r.stars_earned).unwrap_or(0),
                high_score_accuracy_pct: p.map(|r| r.high_score_accuracy_pct).unwrap_or(0.0),
                best_extraction_yield_pct: p.and_then(|r| r.best_extraction_yield_pct),
                is_unlocked: unlocked,
            }
        })
        .collect();

    Ok(ApiResponse::ok(
        "Daftar chapter Game Brew: Kedai Senja Malabar berhasil dimuat.",
        list,
    ))
}

pub async fn submit_game_chapter_score(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(chapter_number): Path<i16>,
    ValidatedJson(req): ValidatedJson<SubmitGameBrewScoreRequest>,
) -> AppResult<impl IntoResponse> {
    let chapter = game_brew_repo::fetch_single_chapter(&state.pool, chapter_number)
        .await
        .map_err(AppError::from)?;

    let progress = game_brew_repo::fetch_user_game_progress(&state.pool, claims.sub)
        .await
        .map_err(AppError::from)?;

    let prev = progress.iter().find(|p| p.chapter_number == chapter_number);
    let is_unlocked = chapter_number == 1 || prev.is_some_and(|p| p.is_unlocked);
    if !is_unlocked {
        return Err(AppError::from(BaristaToolkitError::ChapterLocked));
    }

    let prev_stars = prev.map(|p| p.stars_earned).unwrap_or(0);
    let eval = game_brew_service::evaluate_game_brew_submission(
        chapter_number,
        req.pour_accuracy_pct,
        req.extraction_yield_est,
        req.thermal_stability_score,
        prev_stars,
        chapter.xp_reward_3_stars,
    );

    game_brew_repo::save_chapter_submission(
        &state.pool,
        claims.sub,
        chapter_number,
        eval.stars_earned,
        eval.final_accuracy_pct,
        req.extraction_yield_est,
        eval.unlocked_next_chapter,
        eval.xp_awarded,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiResponse::ok(
        "Hasil seduhan Game Brew berhasil dievaluasi.",
        SubmitGameBrewScoreResponse {
            chapter_number,
            stars_earned: eval.stars_earned,
            final_accuracy_pct: eval.final_accuracy_pct,
            rank_title: eval.rank_title,
            unlocked_next_chapter: eval.unlocked_next_chapter,
            xp_awarded: eval.xp_awarded,
            mentor_feedback: eval.mentor_feedback,
            physics_verified: eval.physics_verified,
        },
    ))
}

pub async fn submit_game_chapter_3d_score(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(chapter_number): Path<i16>,
    ValidatedJson(req): ValidatedJson<GameBrew3DSessionSubmissionDto>,
) -> AppResult<impl IntoResponse> {
    let chapter = game_brew_repo::fetch_single_chapter(&state.pool, chapter_number)
        .await
        .map_err(AppError::from)?;

    let progress = game_brew_repo::fetch_user_game_progress(&state.pool, claims.sub)
        .await
        .map_err(AppError::from)?;

    let prev = progress.iter().find(|p| p.chapter_number == chapter_number);
    let is_unlocked = chapter_number == 1 || prev.is_some_and(|p| p.is_unlocked);
    if !is_unlocked {
        return Err(AppError::from(BaristaToolkitError::ChapterLocked));
    }

    let prev_stars = prev.map(|p| p.stars_earned).unwrap_or(0);
    let eval = game_brew_service::evaluate_game_brew_3d_submission(
        chapter_number,
        &req,
        prev_stars,
        chapter.xp_reward_3_stars,
    );

    if eval.physics_verified {
        game_brew_repo::save_chapter_submission(
            &state.pool,
            claims.sub,
            chapter_number,
            eval.stars_earned,
            eval.final_accuracy_pct,
            req.extraction_yield_est,
            eval.unlocked_next_chapter,
            eval.xp_awarded,
        )
        .await
        .map_err(AppError::from)?;
    }

    Ok(ApiResponse::ok(
        "Evaluasi telemetri 3D Gyroscope Game Brew V60 selesai.",
        SubmitGameBrewScoreResponse {
            chapter_number,
            stars_earned: eval.stars_earned,
            final_accuracy_pct: eval.final_accuracy_pct,
            rank_title: eval.rank_title,
            unlocked_next_chapter: eval.unlocked_next_chapter,
            xp_awarded: eval.xp_awarded,
            mentor_feedback: eval.mentor_feedback,
            physics_verified: eval.physics_verified,
        },
    ))
}

pub async fn diagnose_brew_doctor(
    ValidatedJson(req): ValidatedJson<BrewDoctorDiagnoseRequest>,
) -> AppResult<impl IntoResponse> {
    let diagnosis = brew_doctor_engine::diagnose_extraction(&req);
    Ok(ApiResponse::ok(
        "Diagnosa ekstraksi Brew Doctor & SCA Compass selesai.",
        diagnosis,
    ))
}

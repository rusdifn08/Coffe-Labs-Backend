use std::collections::HashMap;
use axum::{extract::{Path, State}, response::IntoResponse};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use uuid::Uuid;
use crate::{
    core::{error::{ApiResponse, AppError, AppResult}, extractors::{AuthUser, ValidatedJson}, AppState},
    modules::barista_toolkit::{
        cloudinary_signer::generate_hls_stream_url,
        dto::{LmsCourseDto, LmsLessonSummaryDto, SignedHlsLessonResponse, SyncLmsProgressRequest, SyncLmsProgressResponse},
        error::BaristaToolkitError,
        models::{has_tier_access, LmsLessonRow, UserLmsProgressRow},
        repositories::lms_repo,
        services::{
            lms_progress_service::{decode_bitmap_13, evaluate_completion, grade_quiz_answers, merge_bitmap_or, popcount_100bit, sign_hls_manifest_url, BITMAP_BYTES_LEN},
            object_storage_stream::resolve_local_lms_stream_path,
            rabbitmq_lms_worker::publish_lms_lesson_completed,
            tier_access_resolver::{resolve_live_effective_tier, user_has_tier_access},
        },
    },
};

fn build_lesson_summary(l: &LmsLessonRow, p: Option<&UserLmsProgressRow>) -> LmsLessonSummaryDto {
    LmsLessonSummaryDto {
        id: l.id,
        course_id: l.course_id,
        module_number: l.module_number,
        lesson_number: l.lesson_number,
        subchapter_code: l.subchapter_code.clone(),
        title: l.title.clone(),
        description: l.description.clone(),
        kind: l.kind.clone(),
        duration_sec: l.duration_sec,
        completion_pct: p.map(|r| r.completion_pct).unwrap_or(0),
        quiz_score_pct: p.and_then(|r| r.quiz_score_pct),
        is_completed: p.is_some_and(|r| r.is_completed),
        xp_reward: l.xp_reward,
        key_takeaways_markdown: l.key_takeaways_markdown.clone().unwrap_or_default(),
        formulas_latex: l.formulas_latex_json.clone(),
        storyboard_scenes: l.storyboard_scenes_json.clone(),
        simulation_config: l.simulation_config_json.clone(),
        chapter_markers: l.chapter_markers_json.clone(),
        quiz_questions: l.quiz_questions_json.clone(),
        linked_ebook_code: l.linked_ebook_code.clone(),
        linked_ebook_chapter: l.linked_ebook_chapter,
    }
}

pub async fn get_lms_courses(State(state): State<AppState>, AuthUser(claims): AuthUser) -> AppResult<impl IntoResponse> {
    let (loaded, (live_tier, live_role)) = tokio::join!(
        async {
            tokio::try_join!(
                lms_repo::fetch_published_courses(&state.pool),
                lms_repo::fetch_all_lessons(&state.pool),
                lms_repo::fetch_user_lms_progress(&state.pool, claims.sub),
            )
            .map_err(AppError::from)
        },
        resolve_live_effective_tier(&state.pool, &claims),
    );
    let (courses, lessons, progress_list) = loaded?;

    let prog_map: HashMap<Uuid, UserLmsProgressRow> = progress_list.into_iter().map(|p| (p.lesson_id, p)).collect();
    let out: Vec<LmsCourseDto> = courses
        .into_iter()
        .map(|c| {
            let is_unlocked = has_tier_access(&live_tier, &live_role, &c.required_tier);
            let course_lessons: Vec<LmsLessonSummaryDto> = lessons
                .iter()
                .filter(|l| l.course_id == c.id)
                .map(|l| build_lesson_summary(l, prog_map.get(&l.id)))
                .collect();
            let completed = course_lessons.iter().filter(|l| l.is_completed).count();

            LmsCourseDto {
                id: c.id,
                slug: c.slug,
                title: c.title,
                subtitle: c.subtitle,
                instructor_name: c.instructor_name,
                difficulty: c.difficulty,
                required_tier: c.required_tier,
                is_unlocked,
                thumbnail_url: c.thumbnail_url,
                competency_level: c.competency_level,
                focus_summary: c.focus_summary,
                interactive_engine_type: c.interactive_engine_type,
                linked_ebook_id: c.linked_ebook_id,
                total_modules_count: c.total_modules_count,
                total_duration_sec: c.total_duration_sec,
                xp_reward_on_complete: c.xp_reward_on_complete,
                completed_lessons_count: completed,
                lessons: course_lessons,
            }
        })
        .collect();

    Ok(ApiResponse::ok("Katalog kursus Barista Academy Manual Brew 9 Bab berhasil dimuat.", out))
}

pub async fn get_lesson_stream(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(lesson_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let (lesson, required_tier) = lms_repo::fetch_lesson_and_course_tier(&state.pool, lesson_id)
        .await
        .map_err(AppError::from)?;
    if !user_has_tier_access(&state.pool, &claims, &required_tier).await {
        return Err(AppError::from(BaristaToolkitError::TierUpgradeRequired(required_tier)));
    }

    let prev = lms_repo::fetch_single_lesson_progress(&state.pool, claims.sub, lesson_id)
        .await
        .map_err(AppError::from)?;
    let manifest = lesson.hls_manifest_path.unwrap_or_else(|| format!("coffeelabs/lms/{lesson_id}"));
    let cloud_creds = state.config.cloudinary_config().ok().flatten();
    let (signed_hls_url, expires_at) = if let Some(local_obj_url) = resolve_local_lms_stream_path(&manifest).await {
        // Prioritaskan Object Storage backend bila berkas HLS/MP4 tersedia langsung di server
        // agar tidak mengembalikan URL Cloudinary 404 ketika CLOUDINARY_URL hanya dipakai avatar.
        (local_obj_url, chrono::Utc::now() + chrono::Duration::minutes(30))
    } else if cloud_creds.is_some() {
        (generate_hls_stream_url(cloud_creds.as_ref(), &manifest), chrono::Utc::now() + chrono::Duration::minutes(30))
    } else {
        sign_hls_manifest_url(&manifest, claims.sub, state.config.jwt_secret_str())
    };
    let bitmap_bytes = prev.as_ref().map(|p| p.watched_segments_bitmap.clone()).unwrap_or_else(|| vec![0_u8; BITMAP_BYTES_LEN]);

    Ok(ApiResponse::ok(
        "Signed HLS URL & Laboratorium Simulasi Sub-Bab siap digunakan.",
        SignedHlsLessonResponse {
            lesson_id: lesson.id,
            title: lesson.title,
            description: lesson.description,
            kind: lesson.kind,
            signed_hls_url,
            expires_at,
            duration_sec: lesson.duration_sec,
            key_takeaways_markdown: lesson.key_takeaways_markdown.unwrap_or_default(),
            formulas_latex: lesson.formulas_latex_json,
            storyboard_scenes: lesson.storyboard_scenes_json,
            simulation_config: lesson.simulation_config_json,
            chapter_markers: lesson.chapter_markers_json,
            quiz_questions: lesson.quiz_questions_json,
            watched_bitmap_b64: B64.encode(bitmap_bytes),
            last_position_sec: prev.as_ref().map(|p| p.last_position_sec).unwrap_or(0),
            completion_pct: prev.as_ref().map(|p| p.completion_pct).unwrap_or(0),
            is_completed: prev.is_some_and(|p| p.is_completed),
        },
    ))
}

pub async fn sync_lesson_progress(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(lesson_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<SyncLmsProgressRequest>,
) -> AppResult<impl IntoResponse> {
    let (lesson, required_tier) = lms_repo::fetch_lesson_and_course_tier(&state.pool, lesson_id)
        .await
        .map_err(AppError::from)?;
    if !user_has_tier_access(&state.pool, &claims, &required_tier).await {
        return Err(AppError::from(BaristaToolkitError::TierUpgradeRequired(required_tier)));
    }

    let prev = lms_repo::fetch_single_lesson_progress(&state.pool, claims.sub, lesson_id)
        .await
        .map_err(AppError::from)?;
    let incoming = decode_bitmap_13(&req.watched_bitmap_b64);
    let existing = prev.as_ref().map(|p| p.watched_segments_bitmap.as_slice()).unwrap_or(&[]);
    let merged = merge_bitmap_or(existing, &incoming);
    let completion_pct = popcount_100bit(&merged);

    let quiz_score = req
        .quiz_answers
        .as_deref()
        .and_then(|ans| grade_quiz_answers(&lesson.quiz_questions_json, ans))
        .or_else(|| prev.as_ref().and_then(|p| p.quiz_score_pct));

    let was_done = prev.as_ref().is_some_and(|p| p.is_completed);
    let now_done = was_done || evaluate_completion(completion_pct, &lesson.quiz_questions_json, quiz_score);
    let xp_awarded = if !was_done && now_done { lesson.xp_reward } else { 0 };

    lms_repo::upsert_lesson_progress(
        &state.pool, claims.sub, lesson_id, lesson.course_id, &merged,
        req.last_position_sec, completion_pct, quiz_score, now_done, xp_awarded,
    )
    .await
    .map_err(AppError::from)?;

    if !was_done && now_done {
        publish_lms_lesson_completed(
            &state.rabbitmq, claims.sub, lesson_id, lesson.course_id,
            xp_awarded, completion_pct, quiz_score,
        );
    }

    Ok(ApiResponse::ok(
        "Progres belajar 100-bit segmen & status kelulusan disinkronkan.",
        SyncLmsProgressResponse {
            lesson_id,
            watched_bitmap_b64: B64.encode(merged),
            completion_pct,
            quiz_score_pct: quiz_score,
            is_completed: now_done,
            xp_awarded,
        },
    ))
}

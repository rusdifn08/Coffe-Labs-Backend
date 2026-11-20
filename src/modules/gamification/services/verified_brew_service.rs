use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    core::error::{AppError, AppResult},
    modules::gamification::{
        level_curve_math::compute_level_snapshot,
        proof_of_brew_validator::{min_valid_duration_sec, validate_proof_of_brew, ProofOfBrewVerdict},
        verified_brew_dto::{
            CompleteVerifiedBrewDto, GamificationRewardDeltaDto, StartVerifiedBrewDto,
            StartVerifiedBrewResponseDto, XpBreakdownItemDto,
        },
    },
};

pub async fn start_session(
    pool: &PgPool,
    user_id: Uuid,
    req: StartVerifiedBrewDto,
) -> AppResult<StartVerifiedBrewResponseDto> {
    let session_id = Uuid::new_v4();
    let archetype = req.archetype.unwrap_or_else(|| "POUR_OVER".to_string());
    let min_sec = min_valid_duration_sec(&archetype, req.target_duration_sec);
    let steps = req.total_steps_count.unwrap_or(1).max(1);

    let row = sqlx::query(
        r#"
        INSERT INTO brew_verification_sessions
            (id, user_id, method_id, method_display_name, archetype, recipe_id, recipe_title,
             bean_name, roastery_name, target_duration_sec, total_steps_count, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'ACTIVE')
        RETURNING server_started_at
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .bind(&req.method_id)
    .bind(&req.method_display_name)
    .bind(&archetype)
    .bind(&req.recipe_id)
    .bind(&req.recipe_title)
    .bind(&req.bean_name)
    .bind(&req.roastery_name)
    .bind(req.target_duration_sec)
    .bind(steps)
    .fetch_one(pool)
    .await?;

    let server_started_at: DateTime<Utc> = row.try_get("server_started_at")?;
    Ok(StartVerifiedBrewResponseDto {
        session_id,
        server_started_at,
        min_valid_duration_sec: min_sec,
    })
}

pub async fn complete_session(
    pool: &PgPool,
    user_id: Uuid,
    session_id: Uuid,
    req: CompleteVerifiedBrewDto,
) -> AppResult<GamificationRewardDeltaDto> {
    let mut tx = pool.begin().await?;

    let session = sqlx::query(
        r#"
        SELECT method_id, method_display_name, archetype, recipe_id, recipe_title,
               bean_name, roastery_name, target_duration_sec, server_started_at, status::text AS status
        FROM brew_verification_sessions
        WHERE id = $1 AND user_id = $2
        FOR UPDATE
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Sesi seduhan tidak ditemukan.".into()))?;

    let status: String = session.try_get("status")?;
    if status != "ACTIVE" {
        return Err(AppError::BadRequest("Sesi seduhan ini sudah pernah diklaim atau berakhir.".into()));
    }

    let archetype: String = session.try_get("archetype")?;
    let target_sec: i32 = session.try_get("target_duration_sec")?;
    let started_at: DateTime<Utc> = session.try_get("server_started_at")?;
    let wall_sec = (Utc::now() - started_at).num_seconds().max(0);

    if let ProofOfBrewVerdict::RejectedInvalidDuration(reason) =
        validate_proof_of_brew(&archetype, target_sec, wall_sec, req.active_elapsed_ms)
    {
        sqlx::query(
            "UPDATE brew_verification_sessions SET status = 'REJECTED_INVALID_DURATION', rejection_reason = $2 WHERE id = $1",
        )
        .bind(session_id)
        .bind(&reason)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Err(AppError::BadRequest(reason));
    }

    let method_id: String = session.try_get("method_id")?;
    let method_name: String = session.try_get("method_display_name")?;
    let recipe_id: String = session.try_get("recipe_id")?;
    let recipe_title: String = session.try_get("recipe_title")?;
    let bean_name: Option<String> = session.try_get("bean_name")?;
    let roastery_name: Option<String> = session.try_get("roastery_name")?;
    let actual_sec = (req.active_elapsed_ms / 1000).max(1) as i32;

    sqlx::query(
        r#"
        UPDATE brew_verification_sessions
        SET status = 'VERIFIED_COMPLETED', server_completed_at = NOW(),
            reported_active_elapsed_ms = $2, reported_paused_ms = $3,
            completed_steps_mask = $4, pour_accuracy_pct = $5
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .bind(req.active_elapsed_ms)
    .bind(req.paused_ms.unwrap_or(0))
    .bind(req.completed_steps_mask.unwrap_or(0))
    .bind(req.accuracy_score)
    .execute(&mut *tx)
    .await?;

    let method_row = sqlx::query(
        r#"
        INSERT INTO user_method_brew_stats (user_id, method_id, completed_count, total_brew_seconds)
        VALUES ($1, $2, 1, $3)
        ON CONFLICT (user_id, method_id) DO UPDATE
        SET completed_count = user_method_brew_stats.completed_count + 1,
            total_brew_seconds = user_method_brew_stats.total_brew_seconds + EXCLUDED.total_brew_seconds,
            last_brewed_at = NOW()
        RETURNING completed_count
        "#,
    )
    .bind(user_id)
    .bind(&method_id)
    .bind(actual_sec as i64)
    .fetch_one(&mut *tx)
    .await?;
    let method_new_count: i32 = method_row.try_get("completed_count")?;

    let recipe_row = sqlx::query(
        r#"
        INSERT INTO user_tried_recipes (user_id, recipe_id, recipe_title, method_id, times_completed, best_accuracy_pct, last_duration_sec)
        VALUES ($1, $2, $3, $4, 1, $5, $6)
        ON CONFLICT (user_id, recipe_id) DO UPDATE
        SET times_completed = user_tried_recipes.times_completed + 1,
            best_accuracy_pct = GREATEST(COALESCE(user_tried_recipes.best_accuracy_pct, 0), COALESCE(EXCLUDED.best_accuracy_pct, 0)),
            last_duration_sec = EXCLUDED.last_duration_sec,
            last_completed_at = NOW()
        RETURNING times_completed
        "#,
    )
    .bind(user_id)
    .bind(&recipe_id)
    .bind(&recipe_title)
    .bind(&method_id)
    .bind(req.accuracy_score)
    .bind(actual_sec)
    .fetch_one(&mut *tx)
    .await?;
    let times_completed: i32 = recipe_row.try_get("times_completed")?;

    let mut breakdown = vec![XpBreakdownItemDto { label: "Seduhan Selesai".into(), xp: 100 }];
    let mut xp_gained = 100i32;
    if times_completed == 1 {
        xp_gained += 50;
        breakdown.push(XpBreakdownItemDto { label: "Resep Baru Pertama Kali".into(), xp: 50 });
    }
    if method_new_count == 1 {
        xp_gained += 40;
        breakdown.push(XpBreakdownItemDto { label: "Metode Seduh Baru".into(), xp: 40 });
    }

    sqlx::query(
        r#"
        INSERT INTO verified_brew_completions
            (id, session_id, user_id, method_id, method_display_name, recipe_id, recipe_title,
             bean_name, roastery_name, coffee_dose_g, water_total_ml, water_temp_c,
             actual_duration_sec, pour_accuracy_pct, xp_awarded)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(session_id)
    .bind(user_id)
    .bind(&method_id)
    .bind(&method_name)
    .bind(&recipe_id)
    .bind(&recipe_title)
    .bind(&bean_name)
    .bind(&roastery_name)
    .bind(req.coffee_dose_g.unwrap_or(15.0))
    .bind(req.water_total_ml.unwrap_or(225.0))
    .bind(req.water_temp_c.unwrap_or(91))
    .bind(actual_sec)
    .bind(req.accuracy_score)
    .bind(xp_gained)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO user_gamification_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let prof = sqlx::query(
        "SELECT total_xp, current_streak_days FROM user_gamification_profiles WHERE user_id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    let xp_before: i64 = prof.try_get("total_xp")?;
    let streak_days: i32 = prof.try_get("current_streak_days")?;
    let xp_after = xp_before + xp_gained as i64;
    let snap_before = compute_level_snapshot(xp_before);
    let snap_after = compute_level_snapshot(xp_after);

    sqlx::query(
        r#"
        UPDATE user_gamification_profiles
        SET total_xp = $2, current_level = $3, barista_rank_title = $4,
            level_start_xp = $5, next_level_xp = $6, level_progress_ratio = $7,
            total_verified_brews_count = total_verified_brews_count + 1, updated_at = NOW()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(xp_after)
    .bind(snap_after.level)
    .bind(snap_after.rank_title)
    .bind(snap_after.level_start_xp)
    .bind(snap_after.next_level_xp)
    .bind(snap_after.progress_ratio as f32)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(GamificationRewardDeltaDto {
        source_kind: "VERIFIED_BREW".into(),
        source_label: format!("Seduhan {method_name} Selesai!"),
        xp_before,
        xp_gained,
        xp_after,
        level_before: snap_before.level,
        level_after: snap_after.level,
        did_level_up: snap_after.level > snap_before.level,
        rank_before: snap_before.rank_title.into(),
        rank_after: snap_after.rank_title.into(),
        progress_ratio_before: snap_before.progress_ratio,
        progress_ratio_after: snap_after.progress_ratio,
        method_id: Some(method_id),
        method_new_completed_count: Some(method_new_count),
        new_streak_days: streak_days.max(1),
        breakdown_items: breakdown,
    })
}

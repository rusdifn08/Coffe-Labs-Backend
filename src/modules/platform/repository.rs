use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::platform::{
        dto::{UpdateAppVersionConfigRequest, UpdateTrialPolicyRequest},
        models::{AppVersionConfig, Feedback, FeedbackStats, TrialAccessPolicy},
    },
};

pub async fn fetch_version_config(pool: &PgPool) -> AppVersionConfig {
    sqlx::query_as::<_, AppVersionConfig>(
        "SELECT * FROM app_version_configs WHERE id = 'default'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or_else(|_| AppVersionConfig::fallback_default())
}

pub async fn update_version_config(
    pool: &PgPool,
    req: &UpdateAppVersionConfigRequest,
) -> AppResult<AppVersionConfig> {
    let updated = sqlx::query_as::<_, AppVersionConfig>(
        r#"UPDATE app_version_configs
           SET min_base_version = COALESCE($1, min_base_version),
               latest_base_version = COALESCE($2, latest_base_version),
               min_build_number = COALESCE($3, min_build_number),
               latest_build_number = COALESCE($4, latest_build_number),
               latest_patch_number = COALESCE($5, latest_patch_number),
               is_force_update = COALESCE($6, is_force_update),
               is_patch_required = COALESCE($7, is_patch_required),
               release_notes = COALESCE($8, release_notes),
               download_url = COALESCE($9, download_url),
               checksum_sha256 = COALESCE($10, checksum_sha256),
               updated_at = NOW()
           WHERE id = 'default'
           RETURNING *"#,
    )
    .bind(&req.min_base_version)
    .bind(&req.latest_base_version)
    .bind(req.min_build_number)
    .bind(req.latest_build_number)
    .bind(req.latest_patch_number)
    .bind(req.is_force_update)
    .bind(req.is_patch_required)
    .bind(&req.release_notes)
    .bind(&req.download_url)
    .bind(&req.checksum_sha256)
    .fetch_one(pool)
    .await?;
    Ok(updated)
}

pub async fn fetch_trial_policy(pool: &PgPool) -> TrialAccessPolicy {
    sqlx::query_as::<_, TrialAccessPolicy>(
        "SELECT * FROM trial_access_policies WHERE id = 'default'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or_else(|_| TrialAccessPolicy::fallback_default())
}

pub async fn update_trial_policy(
    pool: &PgPool,
    req: &UpdateTrialPolicyRequest,
) -> AppResult<TrialAccessPolicy> {
    sqlx::query("INSERT INTO trial_access_policies (id) VALUES ('default') ON CONFLICT (id) DO NOTHING")
        .execute(pool)
        .await?;

    let existing = fetch_trial_policy(pool).await;
    let raw_mode = req
        .policy_mode
        .as_deref()
        .unwrap_or(&existing.policy_mode)
        .trim()
        .to_lowercase();
    let mode = match raw_mode.as_str() {
        "full_trial_7d_non_business" | "trial_7d" | "full_7d" => {
            "full_trial_7d_non_business".to_string()
        }
        "custom" => "custom".to_string(),
        _ => "strict_limited".to_string(),
    };

    let (
        unlock_all_recipes,
        unlock_smart_brew_dial_in,
        unlock_ai_scanner,
        unlock_toolkit_pro,
        unlock_learning_academy,
        unlock_brew_doctor,
        unlimited_brews,
        max_free_recipes,
        max_trial_brews,
        allow_business_suite,
    ) = if mode == "full_trial_7d_non_business" {
        (
            req.unlock_all_recipes.unwrap_or(true),
            req.unlock_smart_brew_dial_in.unwrap_or(true),
            req.unlock_ai_scanner.unwrap_or(true),
            req.unlock_toolkit_pro.unwrap_or(true),
            req.unlock_learning_academy.unwrap_or(true),
            req.unlock_brew_doctor.unwrap_or(true),
            req.unlimited_brews.unwrap_or(true),
            req.max_free_recipes.unwrap_or(999).clamp(1, 5000),
            req.max_trial_brews.unwrap_or(999).clamp(1, 5000),
            false,
        )
    } else if mode == "strict_limited" {
        (
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            req.max_free_recipes.unwrap_or(10).clamp(1, 100),
            req.max_trial_brews.unwrap_or(10).clamp(1, 100),
            false,
        )
    } else {
        (
            req.unlock_all_recipes.unwrap_or(existing.unlock_all_recipes),
            req.unlock_smart_brew_dial_in
                .unwrap_or(existing.unlock_smart_brew_dial_in),
            req.unlock_ai_scanner.unwrap_or(existing.unlock_ai_scanner),
            req.unlock_toolkit_pro.unwrap_or(existing.unlock_toolkit_pro),
            req.unlock_learning_academy
                .unwrap_or(existing.unlock_learning_academy),
            req.unlock_brew_doctor.unwrap_or(existing.unlock_brew_doctor),
            req.unlimited_brews.unwrap_or(existing.unlimited_brews),
            req.max_free_recipes
                .unwrap_or(existing.max_free_recipes)
                .clamp(1, 5000),
            req.max_trial_brews
                .unwrap_or(existing.max_trial_brews)
                .clamp(1, 5000),
            req.allow_business_suite
                .unwrap_or(existing.allow_business_suite),
        )
    };

    let trial_duration_days = req
        .trial_duration_days
        .unwrap_or_else(|| {
            if mode == "full_trial_7d_non_business" {
                7
            } else {
                existing.trial_duration_days
            }
        })
        .clamp(0, 365);
    let apply_to_guests = req.apply_to_guests.unwrap_or(existing.apply_to_guests);
    let apply_to_registered_trial = req
        .apply_to_registered_trial
        .unwrap_or(existing.apply_to_registered_trial);
    let policy_note = req
        .policy_note
        .clone()
        .unwrap_or_else(|| match mode.as_str() {
            "full_trial_7d_non_business" => {
                format!("Trial Full Access {} Hari (Non-Bisnis): seluruh fitur brewer terbuka, POS & Kiosk tetap terkunci.", trial_duration_days)
            }
            "strict_limited" => {
                "Kebijakan standar akun Trial Coffee Labs (10 resep & fitur terbatas).".to_string()
            }
            _ => existing.policy_note.clone(),
        });
    let reset_activated = req.reset_activated_at.unwrap_or(false)
        || (mode == "full_trial_7d_non_business" && existing.policy_mode != mode);

    let updated = sqlx::query_as::<_, TrialAccessPolicy>(
        r#"UPDATE trial_access_policies
           SET policy_mode = $1,
               trial_duration_days = $2,
               unlock_all_recipes = $3,
               unlock_smart_brew_dial_in = $4,
               unlock_ai_scanner = $5,
               unlock_toolkit_pro = $6,
               unlock_learning_academy = $7,
               unlock_brew_doctor = $8,
               unlimited_brews = $9,
               max_free_recipes = $10,
               max_trial_brews = $11,
               allow_business_suite = $12,
               apply_to_guests = $13,
               apply_to_registered_trial = $14,
               policy_note = $15,
               activated_at = CASE WHEN $16 THEN NOW() ELSE activated_at END,
               updated_at = NOW()
           WHERE id = 'default'
           RETURNING *"#,
    )
    .bind(&mode)
    .bind(trial_duration_days)
    .bind(unlock_all_recipes)
    .bind(unlock_smart_brew_dial_in)
    .bind(unlock_ai_scanner)
    .bind(unlock_toolkit_pro)
    .bind(unlock_learning_academy)
    .bind(unlock_brew_doctor)
    .bind(unlimited_brews)
    .bind(max_free_recipes)
    .bind(max_trial_brews)
    .bind(allow_business_suite)
    .bind(apply_to_guests)
    .bind(apply_to_registered_trial)
    .bind(&policy_note)
    .bind(reset_activated)
    .fetch_one(pool)
    .await?;

    Ok(updated)
}

pub async fn insert_feedback(
    pool: &PgPool,
    user_id: Option<Uuid>,
    username: &str,
    user_role: &str,
    category: &str,
    feature_affected: &str,
    title: &str,
    description: &str,
    rating: i32,
    app_version: &str,
    device_info: &str,
) -> AppResult<Feedback> {
    let fb = sqlx::query_as::<_, Feedback>(
        r#"INSERT INTO feedbacks (
            user_id, username, user_role, category, feature_affected,
            title, description, rating, app_version, device_info, status
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'pending')
        RETURNING *"#,
    )
    .bind(user_id)
    .bind(username)
    .bind(user_role)
    .bind(category)
    .bind(feature_affected)
    .bind(title)
    .bind(description)
    .bind(rating)
    .bind(app_version)
    .bind(device_info)
    .fetch_one(pool)
    .await?;
    Ok(fb)
}

pub async fn list_feedbacks(
    pool: &PgPool,
    status: Option<&str>,
    category: Option<&str>,
) -> AppResult<Vec<Feedback>> {
    let status_filter = status.filter(|s| !s.trim().is_empty() && *s != "all");
    let cat_filter = category.filter(|c| !c.trim().is_empty() && *c != "all");

    let rows = sqlx::query_as::<_, Feedback>(
        r#"SELECT * FROM feedbacks
           WHERE ($1::text IS NULL OR status = $1)
             AND ($2::text IS NULL OR category = $2)
           ORDER BY created_at DESC"#,
    )
    .bind(status_filter)
    .bind(cat_filter)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn update_feedback_status(
    pool: &PgPool,
    id: Uuid,
    status: &str,
    admin_notes: Option<&str>,
) -> AppResult<Option<Feedback>> {
    let row = sqlx::query_as::<_, Feedback>(
        r#"UPDATE feedbacks
           SET status = $1, admin_notes = COALESCE($2, admin_notes), updated_at = NOW()
           WHERE id = $3
           RETURNING *"#,
    )
    .bind(status)
    .bind(admin_notes)
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn delete_feedback(pool: &PgPool, id: Uuid) -> AppResult<u64> {
    let res = sqlx::query("DELETE FROM feedbacks WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub async fn get_feedback_stats(pool: &PgPool) -> AppResult<FeedbackStats> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks").fetch_one(pool).await?;
    let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE status = 'pending'").fetch_one(pool).await?;
    let in_progress: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE status = 'in_progress'").fetch_one(pool).await?;
    let resolved: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE status = 'resolved'").fetch_one(pool).await?;
    let bug_reports: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE category = 'bug_report'").fetch_one(pool).await?;
    let feature_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE category = 'feature_request'").fetch_one(pool).await?;
    let avg: f64 = sqlx::query_scalar("SELECT COALESCE(AVG(rating)::FLOAT8, 5.0) FROM feedbacks WHERE rating IS NOT NULL").fetch_one(pool).await?;

    Ok(FeedbackStats {
        total,
        pending,
        in_progress,
        resolved,
        bug_reports,
        feature_requests,
        average_rating: (avg * 10.0).round() / 10.0,
    })
}

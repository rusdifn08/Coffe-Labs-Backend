use sqlx::PgPool;

use crate::{
    core::error::AppResult,
    modules::platform::{
        dto::UpdateTrialPolicyRequest,
        models::TrialAccessPolicy,
    },
};

pub use crate::modules::platform::feedback_repo::{
    delete_feedback, get_feedback_stats, insert_feedback, list_feedbacks, update_feedback_status,
};
pub use crate::modules::platform::version_config_repo::{
    fetch_version_config, update_version_config,
};

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

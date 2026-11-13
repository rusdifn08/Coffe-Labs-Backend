use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::notifications::{
    dto::UpdatePreferencesReq,
    error::NotifResult,
    models::NotificationPreferenceRow,
};

pub async fn get_or_default_preferences(
    pool: &PgPool,
    user_id: Uuid,
) -> NotifResult<NotificationPreferenceRow> {
    let found = sqlx::query_as::<_, NotificationPreferenceRow>(
        "SELECT * FROM user_notification_preferences WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(found.unwrap_or_else(|| NotificationPreferenceRow::default_for(user_id)))
}

pub async fn upsert_preferences(
    pool: &PgPool,
    user_id: Uuid,
    req: &UpdatePreferencesReq,
) -> NotifResult<NotificationPreferenceRow> {
    let tz = req.timezone.trim();
    let row = sqlx::query_as::<_, NotificationPreferenceRow>(
        r#"
        INSERT INTO user_notification_preferences (
            user_id, enable_chat_messages, enable_follow_follback,
            enable_thread_replies, enable_likes_and_recipe_tries,
            enable_live_brew_and_streak, enable_transactional_billing,
            quiet_hours_start_minute, quiet_hours_end_minute, timezone, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
        ON CONFLICT (user_id) DO UPDATE SET
            enable_chat_messages = EXCLUDED.enable_chat_messages,
            enable_follow_follback = EXCLUDED.enable_follow_follback,
            enable_thread_replies = EXCLUDED.enable_thread_replies,
            enable_likes_and_recipe_tries = EXCLUDED.enable_likes_and_recipe_tries,
            enable_live_brew_and_streak = EXCLUDED.enable_live_brew_and_streak,
            enable_transactional_billing = EXCLUDED.enable_transactional_billing,
            quiet_hours_start_minute = EXCLUDED.quiet_hours_start_minute,
            quiet_hours_end_minute = EXCLUDED.quiet_hours_end_minute,
            timezone = EXCLUDED.timezone,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(req.enable_chat_messages)
    .bind(req.enable_follow_follback)
    .bind(req.enable_thread_replies)
    .bind(req.enable_likes_and_recipe_tries)
    .bind(req.enable_live_brew_and_streak)
    .bind(req.enable_transactional_billing)
    .bind(req.quiet_hours_start_minute)
    .bind(req.quiet_hours_end_minute)
    .bind(if tz.is_empty() { "Asia/Jakarta" } else { tz })
    .fetch_one(pool)
    .await?;

    Ok(row)
}

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::notifications::{
    dto::{InboxPageResp, NotificationCursor, UpdatePreferencesReq},
    error::{NotifError, NotifResult},
    events::NotificationCategory,
    models::NotificationPreferenceRow,
    repositories::{device_token_repo, inbox_repo},
};

pub async fn fetch_inbox_page(
    pool: &PgPool,
    user_id: Uuid,
    category: Option<&str>,
    cursor_token: Option<&str>,
    limit: Option<i64>,
) -> NotifResult<InboxPageResp> {
    let normalized_cat = match category.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => Some(
            NotificationCategory::parse_lossy(raw)
                .ok_or_else(|| NotifError::InvalidPayload("Kategori notifikasi tidak valid.".to_string()))?
                .as_str(),
        ),
        None => None,
    };

    let cursor = match cursor_token.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => Some(NotificationCursor::decode(raw)?),
        None => None,
    };

    let (items, next_cursor, has_more) =
        inbox_repo::list_inbox(pool, user_id, normalized_cat, cursor, limit.unwrap_or(25)).await?;
    let unread_count = inbox_repo::count_unread(pool, user_id).await?;

    Ok(InboxPageResp {
        items,
        next_cursor,
        has_more,
        unread_count,
    })
}

pub async fn get_unread_count(pool: &PgPool, user_id: Uuid) -> NotifResult<i64> {
    inbox_repo::count_unread(pool, user_id).await
}

pub async fn mark_read(pool: &PgPool, user_id: Uuid, notif_id: Uuid) -> NotifResult<i64> {
    inbox_repo::mark_one_read(pool, user_id, notif_id).await
}

pub async fn mark_all_read(pool: &PgPool, user_id: Uuid) -> NotifResult<i64> {
    inbox_repo::mark_all_read(pool, user_id).await
}

pub async fn get_preferences(pool: &PgPool, user_id: Uuid) -> NotifResult<NotificationPreferenceRow> {
    device_token_repo::get_or_default_preferences(pool, user_id).await
}

pub async fn update_preferences(
    pool: &PgPool,
    user_id: Uuid,
    req: &UpdatePreferencesReq,
) -> NotifResult<NotificationPreferenceRow> {
    if req.quiet_hours_start_minute.is_some() != req.quiet_hours_end_minute.is_some() {
        return Err(NotifError::InvalidPayload(
            "Jam mulai dan jam selesai Quiet Hours harus diisi bersamaan.".to_string(),
        ));
    }
    let _check_tz_now: DateTime<Utc> = Utc::now();
    device_token_repo::upsert_preferences(pool, user_id, req).await
}

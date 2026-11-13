use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::notifications::{
    dto::NotificationCursor,
    error::NotifResult,
    events::OmnichannelNotifEvent,
    models::NotificationInboxRow,
};

pub fn build_action_payload_json(event: &OmnichannelNotifEvent) -> serde_json::Value {
    let buttons: Vec<&str> = event.action_buttons.iter().map(|b| b.wire_str()).collect();
    json!({
        "event_id": event.event_id,
        "entity_id": event.entity_id,
        "actor_id": event.actor.as_ref().map(|a| a.user_id),
        "conversation_id": event.conversation_id,
        "collapse_key": event.collapse_key,
        "is_follback_opportunity": buttons.contains(&"QUICK_FOLLBACK"),
        "action_buttons": buttons,
    })
}

pub async fn insert_or_upsert_inbox(
    pool: &PgPool,
    event: &OmnichannelNotifEvent,
) -> NotifResult<NotificationInboxRow> {
    let action_payload = build_action_payload_json(event);
    let actors_preview = serde_json::to_value(&event.actors_preview).unwrap_or_else(|_| json!([]));
    let agg_count = event.aggregated_count.max(1);

    let row = sqlx::query_as::<_, NotificationInboxRow>(
        r#"
        INSERT INTO notification_inbox (
            id, user_id, coalesce_group_key, category, event_type,
            title, body, deep_link_path, action_payload, actors_preview,
            aggregated_count, is_read, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, FALSE, NOW(), NOW())
        ON CONFLICT (user_id, coalesce_group_key)
        WHERE coalesce_group_key IS NOT NULL
        DO UPDATE SET
            title = EXCLUDED.title,
            body = EXCLUDED.body,
            deep_link_path = EXCLUDED.deep_link_path,
            action_payload = EXCLUDED.action_payload,
            actors_preview = EXCLUDED.actors_preview,
            aggregated_count = GREATEST(EXCLUDED.aggregated_count, notification_inbox.aggregated_count + 1),
            is_read = FALSE,
            read_at = NULL,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(event.event_id)
    .bind(event.target_user_id)
    .bind(event.coalesce_key.as_deref())
    .bind(event.category.as_str())
    .bind(&event.event_type)
    .bind(&event.title)
    .bind(&event.body)
    .bind(&event.deep_link_path)
    .bind(action_payload)
    .bind(actors_preview)
    .bind(agg_count)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn list_inbox(
    pool: &PgPool,
    user_id: Uuid,
    category: Option<&str>,
    cursor: Option<NotificationCursor>,
    limit: i64,
) -> NotifResult<(Vec<NotificationInboxRow>, Option<String>, bool)> {
    let clamped = limit.clamp(1, 50);
    let fetch_limit = clamped + 1;
    let cat_filter = category.map(str::trim).filter(|s| !s.is_empty());

    let mut rows = sqlx::query_as::<_, NotificationInboxRow>(
        r#"
        SELECT * FROM notification_inbox
        WHERE user_id = $1
          AND ($2::text IS NULL OR category = $2)
          AND (
              $3::timestamptz IS NULL
              OR (updated_at, id) < ($3, $4)
          )
        ORDER BY updated_at DESC, id DESC
        LIMIT $5
        "#,
    )
    .bind(user_id)
    .bind(cat_filter)
    .bind(cursor.as_ref().map(|c| c.updated_at))
    .bind(cursor.as_ref().map(|c| c.id))
    .bind(fetch_limit)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() as i64 > clamped;
    if has_more {
        rows.truncate(clamped as usize);
    }
    let next_cursor = if has_more {
        rows.last().map(|r| {
            NotificationCursor {
                updated_at: r.updated_at,
                id: r.id,
            }
            .encode()
        })
    } else {
        None
    };

    Ok((rows, next_cursor, has_more))
}

pub async fn count_unread(pool: &PgPool, user_id: Uuid) -> NotifResult<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::bigint FROM notification_inbox WHERE user_id = $1 AND is_read = FALSE",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn mark_one_read(pool: &PgPool, user_id: Uuid, notif_id: Uuid) -> NotifResult<i64> {
    sqlx::query(
        r#"
        UPDATE notification_inbox
        SET is_read = TRUE, read_at = COALESCE(read_at, NOW())
        WHERE id = $1 AND user_id = $2 AND is_read = FALSE
        "#,
    )
    .bind(notif_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    count_unread(pool, user_id).await
}

pub async fn mark_all_read(pool: &PgPool, user_id: Uuid) -> NotifResult<i64> {
    sqlx::query(
        r#"
        UPDATE notification_inbox
        SET is_read = TRUE, read_at = COALESCE(read_at, NOW())
        WHERE user_id = $1 AND is_read = FALSE
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    count_unread(pool, user_id).await
}

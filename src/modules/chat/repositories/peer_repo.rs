use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::chat::{
    error::ChatError,
    models::{ChatPeerRow, ParticipantWatermarkRow},
};

pub async fn get_participant_ids(
    pool: &PgPool,
    conversation_id: Uuid,
) -> Result<Vec<Uuid>, ChatError> {
    let ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM conversation_participants WHERE conversation_id = $1",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;
    Ok(ids)
}

pub async fn get_peer_contact_ids(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Uuid>, ChatError> {
    let ids = sqlx::query_scalar::<_, Uuid>(
        r#"SELECT DISTINCT cp2.user_id
           FROM conversation_participants cp1
           INNER JOIN conversation_participants cp2
               ON cp1.conversation_id = cp2.conversation_id
           WHERE cp1.user_id = $1 AND cp2.user_id <> $1
           LIMIT 200"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(ids)
}

pub async fn advance_watermark(
    pool: &PgPool,
    conversation_id: Uuid,
    user_id: Uuid,
    up_to_seq_id: i64,
    is_read: bool,
) -> Result<ParticipantWatermarkRow, ChatError> {
    let read_target = if is_read { up_to_seq_id } else { 0 };
    let row = sqlx::query_as::<_, ParticipantWatermarkRow>(
        r#"UPDATE conversation_participants
           SET last_delivered_seq_id = GREATEST(last_delivered_seq_id, $3),
               last_read_seq_id = GREATEST(last_read_seq_id, $4)
           WHERE conversation_id = $1 AND user_id = $2
           RETURNING conversation_id, user_id, last_delivered_seq_id, last_read_seq_id"#,
    )
    .bind(conversation_id)
    .bind(user_id)
    .bind(up_to_seq_id)
    .bind(read_target)
    .fetch_optional(pool)
    .await?
    .ok_or(ChatError::NotParticipant)?;

    Ok(row)
}

pub async fn search_peers(
    pool: &PgPool,
    exclude_user_id: Uuid,
    query: Option<&str>,
    limit: i64,
) -> Result<Vec<ChatPeerRow>, ChatError> {
    let pattern = format!("%{}%", query.unwrap_or("").trim());
    let rows = sqlx::query_as::<_, ChatPeerRow>(
        r#"SELECT id, username, full_name, avatar_url, role
           FROM users
           WHERE id <> $1
             AND (status = 'approved' OR status = 'active')
             AND ($2 = '%%' OR username ILIKE $2 OR full_name ILIKE $2)
           ORDER BY updated_at DESC
           LIMIT $3"#,
    )
    .bind(exclude_user_id)
    .bind(&pattern)
    .bind(limit.clamp(1, 50))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::chat::{
    error::ChatError,
    models::{ChatMessageRow, ParticipantWatermarkRow},
    protocol::{ConversationCursorDto, InteractiveAttachment, MessageType},
};

#[derive(Debug)]
pub struct PersistMessageParams<'a> {
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub client_msg_id: Uuid,
    pub reply_to_msg_id: Option<Uuid>,
    pub message_type: MessageType,
    pub body_text: Option<&'a str>,
    pub attachment: Option<&'a InteractiveAttachment>,
    pub preview_text: &'a str,
}

pub async fn persist_message_idempotent(
    pool: &PgPool,
    params: PersistMessageParams<'_>,
) -> Result<ChatMessageRow, ChatError> {
    let mut tx = pool.begin().await?;

    if let Some(existing) = sqlx::query_as::<_, ChatMessageRow>(
        "SELECT * FROM chat_messages WHERE conversation_id = $1 AND client_msg_id = $2",
    )
    .bind(params.conversation_id)
    .bind(params.client_msg_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        tx.commit().await?;
        return Ok(existing);
    }

    let next_seq_id: i64 = sqlx::query_scalar(
        r#"UPDATE conversations
           SET last_seq_id = last_seq_id + 1,
               last_message_preview = $2,
               last_message_at = NOW(),
               updated_at = NOW()
           WHERE id = $1
           RETURNING last_seq_id"#,
    )
    .bind(params.conversation_id)
    .bind(params.preview_text)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ChatError::ConversationNotFound)?;

    let attachment_json = params
        .attachment
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| ChatError::InvalidFrame(format!("Attachment tidak valid: {}", e)))?;

    let row = sqlx::query_as::<_, ChatMessageRow>(
        r#"INSERT INTO chat_messages (
               id, conversation_id, seq_id, sender_id, client_msg_id,
               reply_to_msg_id, message_type, body_text, attachment_payload
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           ON CONFLICT (conversation_id, client_msg_id)
           DO UPDATE SET is_edited = chat_messages.is_edited
           RETURNING *"#,
    )
    .bind(Uuid::now_v7())
    .bind(params.conversation_id)
    .bind(next_seq_id)
    .bind(params.sender_id)
    .bind(params.client_msg_id)
    .bind(params.reply_to_msg_id)
    .bind(params.message_type.as_db_str())
    .bind(params.body_text)
    .bind(attachment_json)
    .fetch_one(&mut *tx)
    .await?;

    // Pengirim otomatis memiliki watermark delivered/read sampai pesannya sendiri.
    sqlx::query(
        r#"UPDATE conversation_participants
           SET last_delivered_seq_id = GREATEST(last_delivered_seq_id, $3),
               last_read_seq_id = GREATEST(last_read_seq_id, $3)
           WHERE conversation_id = $1 AND user_id = $2"#,
    )
    .bind(params.conversation_id)
    .bind(params.sender_id)
    .bind(row.seq_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(row)
}

pub async fn fetch_conversation_history(
    pool: &PgPool,
    conversation_id: Uuid,
    before_seq: Option<i64>,
    limit: i64,
) -> Result<Vec<ChatMessageRow>, ChatError> {
    let clamped = limit.clamp(1, 100);
    let rows = match before_seq {
        Some(cursor_seq) => {
            sqlx::query_as::<_, ChatMessageRow>(
                r#"SELECT * FROM chat_messages
                   WHERE conversation_id = $1 AND seq_id < $2
                   ORDER BY seq_id DESC
                   LIMIT $3"#,
            )
            .bind(conversation_id)
            .bind(cursor_seq)
            .bind(clamped)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, ChatMessageRow>(
                r#"SELECT * FROM chat_messages
                   WHERE conversation_id = $1
                   ORDER BY seq_id DESC
                   LIMIT $2"#,
            )
            .bind(conversation_id)
            .bind(clamped)
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows)
}

pub async fn fetch_delta_messages(
    pool: &PgPool,
    user_id: Uuid,
    cursors: &[ConversationCursorDto],
) -> Result<Vec<ChatMessageRow>, ChatError> {
    if cursors.is_empty() {
        return Ok(Vec::new());
    }
    let conv_ids: Vec<Uuid> = cursors.iter().map(|c| c.conversation_id).collect();
    let min_seqs: Vec<i64> = cursors.iter().map(|c| c.local_max_seq_id).collect();

    let rows = sqlx::query_as::<_, ChatMessageRow>(
        r#"SELECT m.*
           FROM chat_messages m
           INNER JOIN unnest($2::uuid[], $3::bigint[]) AS cur(conv_id, min_seq)
               ON m.conversation_id = cur.conv_id AND m.seq_id > cur.min_seq
           INNER JOIN conversation_participants cp
               ON cp.conversation_id = m.conversation_id AND cp.user_id = $1
           ORDER BY m.conversation_id, m.seq_id ASC
           LIMIT 500"#,
    )
    .bind(user_id)
    .bind(&conv_ids)
    .bind(&min_seqs)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn fetch_watermarks_for_user(
    pool: &PgPool,
    user_id: Uuid,
    conv_ids: &[Uuid],
) -> Result<Vec<ParticipantWatermarkRow>, ChatError> {
    if conv_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, ParticipantWatermarkRow>(
        r#"SELECT cp.conversation_id, cp.user_id, cp.last_delivered_seq_id, cp.last_read_seq_id
           FROM conversation_participants cp
           WHERE cp.conversation_id = ANY($1)
             AND EXISTS (
                 SELECT 1 FROM conversation_participants me
                 WHERE me.conversation_id = cp.conversation_id AND me.user_id = $2
             )"#,
    )
    .bind(conv_ids)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

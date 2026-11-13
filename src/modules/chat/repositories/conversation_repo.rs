use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::chat::{error::ChatError, models::ConversationSummaryRow};

pub fn compute_direct_pair_key(user_a: Uuid, user_b: Uuid) -> String {
    if user_a <= user_b {
        format!("{}:{}", user_a, user_b)
    } else {
        format!("{}:{}", user_b, user_a)
    }
}

pub async fn find_or_create_direct_room(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<Uuid, ChatError> {
    let pair_key = compute_direct_pair_key(user_a, user_b);
    let mut tx = pool.begin().await?;

    let candidate_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO conversations (id, kind, direct_pair_key)
           VALUES ($1, 'DIRECT', $2)
           ON CONFLICT (direct_pair_key) DO NOTHING"#,
    )
    .bind(candidate_id)
    .bind(&pair_key)
    .execute(&mut *tx)
    .await?;

    let conv_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM conversations WHERE direct_pair_key = $1",
    )
    .bind(&pair_key)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO conversation_participants (conversation_id, user_id, role)
           VALUES ($1, $2, 'MEMBER') ON CONFLICT DO NOTHING"#,
    )
    .bind(conv_id)
    .bind(user_a)
    .execute(&mut *tx)
    .await?;

    if user_b != user_a {
        sqlx::query(
            r#"INSERT INTO conversation_participants (conversation_id, user_id, role)
               VALUES ($1, $2, 'MEMBER') ON CONFLICT DO NOTHING"#,
        )
        .bind(conv_id)
        .bind(user_b)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(conv_id)
}

pub async fn create_group_room(
    pool: &PgPool,
    creator_id: Uuid,
    title: &str,
    avatar_url: Option<&str>,
    member_ids: &[Uuid],
) -> Result<Uuid, ChatError> {
    let mut tx = pool.begin().await?;
    let conv_id = Uuid::now_v7();

    sqlx::query(
        r#"INSERT INTO conversations (id, kind, title, avatar_url)
           VALUES ($1, 'GROUP', $2, $3)"#,
    )
    .bind(conv_id)
    .bind(title.trim())
    .bind(avatar_url)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO conversation_participants (conversation_id, user_id, role)
           VALUES ($1, $2, 'ADMIN') ON CONFLICT DO NOTHING"#,
    )
    .bind(conv_id)
    .bind(creator_id)
    .execute(&mut *tx)
    .await?;

    for &member_id in member_ids {
        if member_id == creator_id {
            continue;
        }
        sqlx::query(
            r#"INSERT INTO conversation_participants (conversation_id, user_id, role)
               VALUES ($1, $2, 'MEMBER') ON CONFLICT DO NOTHING"#,
        )
        .bind(conv_id)
        .bind(member_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(conv_id)
}

pub async fn list_user_conversations(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ConversationSummaryRow>, ChatError> {
    let rows = sqlx::query_as::<_, ConversationSummaryRow>(
        r#"SELECT
            c.id,
            c.kind::text AS kind,
            c.title,
            c.avatar_url,
            c.last_seq_id,
            c.last_message_preview,
            c.last_message_at,
            my_cp.last_read_seq_id AS my_read_seq_id,
            my_cp.last_delivered_seq_id AS my_delivered_seq_id,
            peer.id AS peer_user_id,
            peer.username AS peer_username,
            peer.full_name AS peer_full_name,
            peer.avatar_url AS peer_avatar_url,
            COALESCE(peer_wm.max_read, 0) AS peer_read_seq_id,
            COALESCE(peer_wm.max_delivered, 0) AS peer_delivered_seq_id
        FROM conversations c
        INNER JOIN conversation_participants my_cp
            ON my_cp.conversation_id = c.id AND my_cp.user_id = $1
        LEFT JOIN LATERAL (
            SELECT u.id, u.username, u.full_name, u.avatar_url
            FROM conversation_participants cp2
            INNER JOIN users u ON u.id = cp2.user_id
            WHERE cp2.conversation_id = c.id AND cp2.user_id <> $1
            ORDER BY cp2.joined_at ASC
            LIMIT 1
        ) peer ON TRUE
        LEFT JOIN LATERAL (
            SELECT
                MAX(cp3.last_read_seq_id) AS max_read,
                MAX(cp3.last_delivered_seq_id) AS max_delivered
            FROM conversation_participants cp3
            WHERE cp3.conversation_id = c.id AND cp3.user_id <> $1
        ) peer_wm ON TRUE
        ORDER BY COALESCE(c.last_message_at, c.created_at) DESC
        LIMIT 100"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

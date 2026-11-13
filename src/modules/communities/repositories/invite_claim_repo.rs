use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::communities::{error::CommunitiesError, models::ClaimedInviteRow};

pub async fn enroll_member_in_club_conversations(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    community_id: Uuid,
    user_id: Uuid,
    chat_role: &str,
) -> Result<Option<Uuid>, CommunitiesError> {
    let conv_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"SELECT linked_conversation_id
           FROM community_channels
           WHERE community_id = $1 AND linked_conversation_id IS NOT NULL
           ORDER BY display_order ASC"#,
    )
    .bind(community_id)
    .fetch_all(&mut **tx)
    .await?;

    for conv_id in &conv_ids {
        sqlx::query(
            r#"INSERT INTO conversation_participants (conversation_id, user_id, role)
               VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"#,
        )
        .bind(conv_id)
        .bind(user_id)
        .bind(chat_role)
        .execute(&mut **tx)
        .await?;
    }

    Ok(conv_ids.into_iter().next())
}

/// Klaim Secret ID atau Link Token dalam Single-CTE transaksi atomik.
pub async fn claim_invite_secret_atomic(
    pg: &PgPool,
    secret_sha256: Option<&[u8]>,
    invite_url_token: Option<&str>,
    user_id: Uuid,
) -> Result<Option<(ClaimedInviteRow, Option<Uuid>)>, CommunitiesError> {
    let mut tx = pg.begin().await?;

    let claimed = sqlx::query_as::<_, ClaimedInviteRow>(
        r#"WITH valid_secret AS (
            SELECT id, community_id, role_granted
            FROM community_invite_secrets
            WHERE is_revoked = FALSE
              AND (expires_at IS NULL OR expires_at > NOW())
              AND (max_uses IS NULL OR used_count < max_uses)
              AND (
                    ($1::bytea IS NOT NULL AND secret_code_sha256 = $1)
                 OR ($2::text IS NOT NULL AND invite_url_token = $2)
              )
            FOR UPDATE
        ),
        inserted_member AS (
            INSERT INTO community_members (community_id, user_id, role)
            SELECT vs.community_id, $3, vs.role_granted
            FROM valid_secret vs
            ON CONFLICT (community_id, user_id) DO NOTHING
            RETURNING community_id, role::text AS role
        ),
        bumped_secret AS (
            UPDATE community_invite_secrets cis
            SET used_count = cis.used_count + 1
            FROM inserted_member im, valid_secret vs
            WHERE cis.id = vs.id AND im.community_id = vs.community_id
        ),
        bumped_club AS (
            UPDATE coffee_communities cc
            SET members_count = cc.members_count + 1,
                updated_at = NOW()
            FROM inserted_member im
            WHERE cc.id = im.community_id
        )
        SELECT vs.id AS invite_id,
               vs.community_id,
               COALESCE(
                   (SELECT role FROM inserted_member),
                   cm.role::text,
                   vs.role_granted::text
               ) AS assigned_role,
               (NOT EXISTS (SELECT 1 FROM inserted_member)) AS already_member
        FROM valid_secret vs
        LEFT JOIN community_members cm
          ON cm.community_id = vs.community_id AND cm.user_id = $3"#,
    )
    .bind(secret_sha256)
    .bind(invite_url_token)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = claimed else {
        tx.rollback().await?;
        return Ok(None);
    };

    let chat_role = if row.assigned_role == "OWNER" || row.assigned_role == "ADMIN" {
        "ADMIN"
    } else {
        "MEMBER"
    };
    let default_conv_id =
        enroll_member_in_club_conversations(&mut tx, row.community_id, user_id, chat_role).await?;

    tx.commit().await?;
    Ok(Some((row, default_conv_id)))
}

pub async fn join_public_club_atomic(
    pg: &PgPool,
    community_id: Uuid,
    user_id: Uuid,
) -> Result<(bool, String, Option<Uuid>), CommunitiesError> {
    let mut tx = pg.begin().await?;

    let inserted: Option<String> = sqlx::query_scalar(
        r#"INSERT INTO community_members (community_id, user_id, role)
           VALUES ($1, $2, 'BARISTA_MEMBER'::community_member_role)
           ON CONFLICT (community_id, user_id) DO NOTHING
           RETURNING role::text"#,
    )
    .bind(community_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let already_member = inserted.is_none();
    let role = if let Some(r) = inserted {
        sqlx::query(
            "UPDATE coffee_communities SET members_count = members_count + 1, updated_at = NOW() WHERE id = $1",
        )
        .bind(community_id)
        .execute(&mut *tx)
        .await?;
        r
    } else {
        sqlx::query_scalar::<_, String>(
            "SELECT role::text FROM community_members WHERE community_id = $1 AND user_id = $2",
        )
        .bind(community_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?
    };

    let default_conv_id =
        enroll_member_in_club_conversations(&mut tx, community_id, user_id, "MEMBER").await?;

    tx.commit().await?;
    Ok((already_member, role, default_conv_id))
}

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::communities::{
    error::CommunitiesError,
    models::CommunityInviteSecretRow,
    secret_id_crypto::GeneratedSecretId,
};

pub async fn list_active_secrets(
    pg: &PgPool,
    community_id: Uuid,
) -> Result<Vec<CommunityInviteSecretRow>, CommunitiesError> {
    let rows = sqlx::query_as::<_, CommunityInviteSecretRow>(
        r#"SELECT id, community_id, created_by, secret_code_display, invite_url_token,
                  role_granted::text AS role_granted, max_uses, used_count,
                  expires_at, is_revoked, created_at
           FROM community_invite_secrets
           WHERE community_id = $1 AND is_revoked = FALSE
           ORDER BY created_at DESC
           LIMIT 25"#,
    )
    .bind(community_id)
    .fetch_all(pg)
    .await?;
    Ok(rows)
}

pub async fn insert_invite_secret(
    pg: &PgPool,
    community_id: Uuid,
    created_by: Uuid,
    generated: &GeneratedSecretId,
    role_granted: &str,
    max_uses: Option<i32>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<CommunityInviteSecretRow, CommunitiesError> {
    let row = sqlx::query_as::<_, CommunityInviteSecretRow>(
        r#"INSERT INTO community_invite_secrets (
            id, community_id, created_by, secret_code_display, secret_code_sha256,
            invite_url_token, role_granted, max_uses, expires_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7::community_member_role, $8, $9)
        RETURNING id, community_id, created_by, secret_code_display, invite_url_token,
                  role_granted::text AS role_granted, max_uses, used_count,
                  expires_at, is_revoked, created_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(community_id)
    .bind(created_by)
    .bind(&generated.display_code)
    .bind(&generated.secret_sha256)
    .bind(&generated.invite_url_token)
    .bind(role_granted)
    .bind(max_uses)
    .bind(expires_at)
    .fetch_one(pg)
    .await?;
    Ok(row)
}

pub async fn revoke_invite_secret(
    pg: &PgPool,
    community_id: Uuid,
    invite_id: Uuid,
) -> Result<bool, CommunitiesError> {
    let res = sqlx::query(
        r#"UPDATE community_invite_secrets
           SET is_revoked = TRUE
           WHERE id = $1 AND community_id = $2 AND is_revoked = FALSE"#,
    )
    .bind(invite_id)
    .bind(community_id)
    .execute(pg)
    .await?;
    Ok(res.rows_affected() > 0)
}

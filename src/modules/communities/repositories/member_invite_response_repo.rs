use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::communities::{
    error::CommunitiesError,
    repositories::invite_claim_repo,
};

pub async fn accept_invitation_atomic(
    pg: &PgPool,
    invitation_id: Uuid,
    user_id: Uuid,
) -> Result<Option<(Uuid, Uuid, bool, String, Option<Uuid>)>, CommunitiesError> {
    let mut tx = pg.begin().await?;

    let accepted: Option<(Uuid, Uuid, String)> = sqlx::query_as(
        r#"UPDATE community_member_invitations
           SET status = 'ACCEPTED'::community_invite_status,
               responded_at = NOW(),
               updated_at = NOW()
           WHERE id = $1 AND invitee_id = $2 AND status = 'PENDING'::community_invite_status
           RETURNING community_id, inviter_id, role_granted::text"#,
    )
    .bind(invitation_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((community_id, inviter_id, role)) = accepted else {
        tx.rollback().await?;
        return Ok(None);
    };

    let inserted: Option<String> = sqlx::query_scalar(
        r#"INSERT INTO community_members (community_id, user_id, role)
           VALUES ($1, $2, $3::community_member_role)
           ON CONFLICT (community_id, user_id) DO NOTHING
           RETURNING role::text"#,
    )
    .bind(community_id)
    .bind(user_id)
    .bind(&role)
    .fetch_optional(&mut *tx)
    .await?;

    let already_member = inserted.is_none();
    if !already_member {
        sqlx::query(
            "UPDATE coffee_communities SET members_count = members_count + 1, updated_at = NOW() WHERE id = $1",
        )
        .bind(community_id)
        .execute(&mut *tx)
        .await?;
    }

    let chat_role = if role == "OWNER" || role == "ADMIN" {
        "ADMIN"
    } else {
        "MEMBER"
    };
    let default_conv_id = invite_claim_repo::enroll_member_in_club_conversations(
        &mut tx,
        community_id,
        user_id,
        chat_role,
    )
    .await?;

    tx.commit().await?;
    Ok(Some((
        community_id,
        inviter_id,
        already_member,
        role,
        default_conv_id,
    )))
}

pub async fn decline_invitation_atomic(
    pg: &PgPool,
    invitation_id: Uuid,
    user_id: Uuid,
) -> Result<Option<(Uuid, Uuid)>, CommunitiesError> {
    let declined: Option<(Uuid, Uuid)> = sqlx::query_as(
        r#"UPDATE community_member_invitations
           SET status = 'DECLINED'::community_invite_status,
               responded_at = NOW(),
               updated_at = NOW()
           WHERE id = $1 AND invitee_id = $2 AND status = 'PENDING'::community_invite_status
           RETURNING community_id, inviter_id"#,
    )
    .bind(invitation_id)
    .bind(user_id)
    .fetch_optional(pg)
    .await?;
    Ok(declined)
}

pub async fn cancel_invitation(
    pg: &PgPool,
    community_id: Uuid,
    invitation_id: Uuid,
) -> Result<bool, CommunitiesError> {
    let res = sqlx::query(
        r#"UPDATE community_member_invitations
           SET status = 'CANCELLED'::community_invite_status,
               updated_at = NOW()
           WHERE id = $1 AND community_id = $2 AND status = 'PENDING'::community_invite_status"#,
    )
    .bind(invitation_id)
    .bind(community_id)
    .execute(pg)
    .await?;
    Ok(res.rows_affected() > 0)
}

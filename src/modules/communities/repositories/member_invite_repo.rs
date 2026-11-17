use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::communities::{
    error::CommunitiesError,
    invite_dto::{ClubMemberInvitationRow, ClubSentInvitationRow},
};

pub async fn upsert_invitations_batch(
    pg: &PgPool,
    community_id: Uuid,
    inviter_id: Uuid,
    invitee_ids: &[Uuid],
    role_granted: &str,
    message: &str,
) -> Result<Vec<ClubSentInvitationRow>, CommunitiesError> {
    if invitee_ids.is_empty() {
        return Ok(Vec::new());
    }

    let generated_ids: Vec<Uuid> = (0..invitee_ids.len()).map(|_| Uuid::now_v7()).collect();
    let rows = sqlx::query_as::<_, ClubSentInvitationRow>(
        r#"WITH candidates AS (
               SELECT u.id AS invitee_id, c.gen_id
               FROM unnest($2::uuid[], $3::uuid[]) AS c(invitee_id, gen_id)
               INNER JOIN users u ON u.id = c.invitee_id
               WHERE u.id <> $4
                 AND (u.status = 'approved' OR u.status = 'active')
                 AND NOT EXISTS (
                     SELECT 1 FROM community_members cm
                     WHERE cm.community_id = $1 AND cm.user_id = u.id
                 )
           ),
           upserted AS (
               INSERT INTO community_member_invitations (
                   id, community_id, inviter_id, invitee_id, role_granted, message, status,
                   responded_at, created_at, updated_at
               )
               SELECT gen_id, $1, $4, invitee_id, $5::community_member_role, $6,
                      'PENDING'::community_invite_status, NULL, NOW(), NOW()
               FROM candidates
               ON CONFLICT (community_id, invitee_id) DO UPDATE
               SET inviter_id = EXCLUDED.inviter_id,
                   role_granted = EXCLUDED.role_granted,
                   message = EXCLUDED.message,
                   status = 'PENDING'::community_invite_status,
                   responded_at = NULL,
                   updated_at = NOW()
               WHERE community_member_invitations.status <> 'ACCEPTED'::community_invite_status
               RETURNING id, community_id, invitee_id, role_granted::text AS role_granted,
                         status::text AS status, message, created_at
           )
           SELECT up.id, up.community_id, up.invitee_id,
                  COALESCE(NULLIF(p.username, ''), u.username) AS invitee_username,
                  COALESCE(NULLIF(p.display_name, ''), NULLIF(u.full_name, ''), u.username) AS invitee_display_name,
                  COALESCE(p.avatar_url, u.avatar_url) AS invitee_avatar_url,
                  up.role_granted, up.status, up.message, up.created_at
           FROM upserted up
           INNER JOIN users u ON u.id = up.invitee_id
           LEFT JOIN user_profile_projections p ON p.user_id = up.invitee_id"#,
    )
    .bind(community_id)
    .bind(invitee_ids)
    .bind(&generated_ids)
    .bind(inviter_id)
    .bind(role_granted)
    .bind(message.trim())
    .fetch_all(pg)
    .await?;

    Ok(rows)
}

pub async fn list_pending_for_invitee(
    pg: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ClubMemberInvitationRow>, CommunitiesError> {
    let rows = sqlx::query_as::<_, ClubMemberInvitationRow>(
        r#"SELECT mi.id, mi.community_id, c.name AS community_name,
                  c.handle_slug AS community_slug, c.description AS community_description,
                  c.privacy_level::text AS privacy_level, c.members_count, c.focus_tags,
                  mi.inviter_id,
                  COALESCE(NULLIF(p.username, ''), u.username) AS inviter_username,
                  COALESCE(NULLIF(p.display_name, ''), NULLIF(u.full_name, ''), u.username) AS inviter_display_name,
                  COALESCE(p.avatar_url, u.avatar_url) AS inviter_avatar_url,
                  mi.role_granted::text AS role_granted, mi.message,
                  mi.status::text AS status, mi.created_at
           FROM community_member_invitations mi
           INNER JOIN coffee_communities c ON c.id = mi.community_id
           INNER JOIN users u ON u.id = mi.inviter_id
           LEFT JOIN user_profile_projections p ON p.user_id = mi.inviter_id
           WHERE mi.invitee_id = $1 AND mi.status = 'PENDING'::community_invite_status
           ORDER BY mi.updated_at DESC
           LIMIT 50"#,
    )
    .bind(user_id)
    .fetch_all(pg)
    .await?;
    Ok(rows)
}

pub async fn list_pending_for_club(
    pg: &PgPool,
    community_id: Uuid,
) -> Result<Vec<ClubSentInvitationRow>, CommunitiesError> {
    let rows = sqlx::query_as::<_, ClubSentInvitationRow>(
        r#"SELECT mi.id, mi.community_id, mi.invitee_id,
                  COALESCE(NULLIF(p.username, ''), u.username) AS invitee_username,
                  COALESCE(NULLIF(p.display_name, ''), NULLIF(u.full_name, ''), u.username) AS invitee_display_name,
                  COALESCE(p.avatar_url, u.avatar_url) AS invitee_avatar_url,
                  mi.role_granted::text AS role_granted, mi.status::text AS status,
                  mi.message, mi.created_at
           FROM community_member_invitations mi
           INNER JOIN users u ON u.id = mi.invitee_id
           LEFT JOIN user_profile_projections p ON p.user_id = mi.invitee_id
           WHERE mi.community_id = $1 AND mi.status = 'PENDING'::community_invite_status
           ORDER BY mi.updated_at DESC
           LIMIT 50"#,
    )
    .bind(community_id)
    .fetch_all(pg)
    .await?;
    Ok(rows)
}

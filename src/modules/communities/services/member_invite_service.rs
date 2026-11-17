use sqlx::PgPool;
use std::{collections::HashSet, sync::Arc};
use uuid::Uuid;

use crate::modules::{
    communities::{
        error::CommunitiesError,
        invite_dto::{
            ClubMemberInvitationDto, ClubSentInvitationDto, ClubSentInvitationRow,
            SendClubInvitesResponseDto,
        },
        models::{is_privileged_role, normalize_club_role},
        repositories::{community_query_repo, member_invite_repo},
        services::club_cache_service::ClubCacheService,
    },
    notifications::{
        club_builders::build_club_invitation_event,
        repositories::actor_lookup,
        services::producer_service::NotificationProducerService,
    },
};

fn map_sent_dto(r: ClubSentInvitationRow) -> ClubSentInvitationDto {
    ClubSentInvitationDto {
        id: r.id,
        community_id: r.community_id,
        invitee_id: r.invitee_id,
        invitee_username: r.invitee_username,
        invitee_display_name: r.invitee_display_name,
        invitee_avatar_url: r.invitee_avatar_url,
        role_granted: r.role_granted,
        status: r.status,
        message: r.message,
        created_at: r.created_at,
    }
}

pub async fn send_direct_invitations(
    pg: &PgPool,
    cache: &ClubCacheService,
    notifications: &Arc<NotificationProducerService>,
    actor_id: Uuid,
    community_id: Uuid,
    raw_user_ids: &[Uuid],
    raw_role: Option<&str>,
    raw_message: Option<&str>,
) -> Result<SendClubInvitesResponseDto, CommunitiesError> {
    let club = cache.get_club(pg, community_id).await?;
    let actor_role = community_query_repo::fetch_viewer_role(pg, community_id, actor_id)
        .await?
        .ok_or(CommunitiesError::Forbidden)?;

    if club.privacy_level != "PUBLIC" && !is_privileged_role(&actor_role) {
        return Err(CommunitiesError::Forbidden);
    }

    let role_granted = match raw_role {
        Some(r) => normalize_club_role(r).ok_or_else(|| {
            CommunitiesError::BadRequest("role_granted tidak valid.".into())
        })?,
        None => "BARISTA_MEMBER",
    };
    if matches!(role_granted, "OWNER" | "ADMIN") && actor_role != "OWNER" {
        return Err(CommunitiesError::Forbidden);
    }

    let mut seen = HashSet::new();
    let unique_targets: Vec<Uuid> = raw_user_ids
        .iter()
        .copied()
        .filter(|id| *id != actor_id && seen.insert(*id))
        .take(50)
        .collect();

    let message = raw_message.unwrap_or("").trim();
    let rows = member_invite_repo::upsert_invitations_batch(
        pg,
        community_id,
        actor_id,
        &unique_targets,
        role_granted,
        message,
    )
    .await?;

    let invited_count = rows.len();
    let skipped = unique_targets.len().saturating_sub(invited_count);
    let invitations: Vec<ClubSentInvitationDto> = rows.into_iter().map(map_sent_dto).collect();

    if !invitations.is_empty() {
        let pool_bg = pg.clone();
        let notif_bg = Arc::clone(notifications);
        let club_name = club.name.clone();
        let msg_owned = message.to_string();
        let targets: Vec<(Uuid, Uuid)> = invitations.iter().map(|i| (i.invitee_id, i.id)).collect();
        tokio::spawn(async move {
            let Some(actor) = actor_lookup::lookup_actor_summary(&pool_bg, actor_id).await else {
                return;
            };
            for (target_uid, invite_id) in targets {
                notif_bg.publish(build_club_invitation_event(
                    target_uid,
                    community_id,
                    &club_name,
                    invite_id,
                    &msg_owned,
                    actor.clone(),
                ));
            }
        });
    }

    Ok(SendClubInvitesResponseDto {
        invited_count,
        skipped_already_member: skipped,
        invitations,
    })
}

pub async fn list_my_pending_invitations(
    pg: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ClubMemberInvitationDto>, CommunitiesError> {
    let rows = member_invite_repo::list_pending_for_invitee(pg, user_id).await?;
    Ok(rows
        .into_iter()
        .map(|r| ClubMemberInvitationDto {
            id: r.id,
            community_id: r.community_id,
            community_name: r.community_name,
            community_slug: r.community_slug,
            community_description: r.community_description,
            privacy_level: r.privacy_level,
            members_count: r.members_count,
            focus_tags: serde_json::from_value(r.focus_tags).unwrap_or_default(),
            inviter_id: r.inviter_id,
            inviter_username: r.inviter_username,
            inviter_display_name: r.inviter_display_name,
            inviter_avatar_url: r.inviter_avatar_url,
            role_granted: r.role_granted,
            message: r.message,
            status: r.status,
            created_at: r.created_at,
        })
        .collect())
}

pub async fn list_club_sent_invitations(
    pg: &PgPool,
    actor_id: Uuid,
    community_id: Uuid,
) -> Result<Vec<ClubSentInvitationDto>, CommunitiesError> {
    let actor_role = community_query_repo::fetch_viewer_role(pg, community_id, actor_id)
        .await?
        .ok_or(CommunitiesError::Forbidden)?;
    if !is_privileged_role(&actor_role) {
        return Err(CommunitiesError::Forbidden);
    }
    let rows = member_invite_repo::list_pending_for_club(pg, community_id).await?;
    Ok(rows.into_iter().map(map_sent_dto).collect())
}

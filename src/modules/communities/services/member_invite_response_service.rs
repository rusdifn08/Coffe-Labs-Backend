use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::modules::{
    communities::{
        dto::JoinClubResponseDto,
        error::CommunitiesError,
        invite_dto::RespondInvitationResponseDto,
        models::is_privileged_role,
        repositories::{community_query_repo, member_invite_response_repo},
        services::{
            club_cache_service::ClubCacheService,
            club_discovery_service::summary_from_row,
        },
    },
    notifications::{
        club_builders::build_club_invite_response_event,
        repositories::actor_lookup,
        services::producer_service::NotificationProducerService,
    },
};

pub async fn accept_invitation(
    pg: &PgPool,
    cache: &ClubCacheService,
    notifications: &Arc<NotificationProducerService>,
    invitation_id: Uuid,
    user_id: Uuid,
) -> Result<RespondInvitationResponseDto, CommunitiesError> {
    let Some((community_id, inviter_id, already_member, assigned_role, default_conv_id)) =
        member_invite_response_repo::accept_invitation_atomic(pg, invitation_id, user_id).await?
    else {
        return Err(CommunitiesError::InvitationNotFound);
    };

    cache.invalidate_club(community_id).await;
    let club = cache.get_club(pg, community_id).await?;
    let club_name = club.name.clone();
    let pool_bg = pg.clone();
    let notif_bg = Arc::clone(notifications);
    tokio::spawn(async move {
        if let Some(actor) = actor_lookup::lookup_actor_summary(&pool_bg, user_id).await {
            notif_bg.publish(build_club_invite_response_event(
                inviter_id,
                community_id,
                &club_name,
                true,
                actor,
            ));
        }
    });

    let joined_club = JoinClubResponseDto {
        joined: true,
        already_member,
        assigned_role: assigned_role.clone(),
        club: summary_from_row(&club, Some(assigned_role)),
        default_conversation_id: default_conv_id,
    };
    Ok(RespondInvitationResponseDto {
        invitation_id,
        community_id,
        status: "ACCEPTED".to_string(),
        joined_club: Some(joined_club),
    })
}

pub async fn decline_invitation(
    pg: &PgPool,
    invitation_id: Uuid,
    user_id: Uuid,
) -> Result<RespondInvitationResponseDto, CommunitiesError> {
    let Some((community_id, _)) =
        member_invite_response_repo::decline_invitation_atomic(pg, invitation_id, user_id).await?
    else {
        return Err(CommunitiesError::InvitationNotFound);
    };
    Ok(RespondInvitationResponseDto {
        invitation_id,
        community_id,
        status: "DECLINED".to_string(),
        joined_club: None,
    })
}

pub async fn cancel_invitation(
    pg: &PgPool,
    actor_id: Uuid,
    community_id: Uuid,
    invitation_id: Uuid,
) -> Result<bool, CommunitiesError> {
    let actor_role = community_query_repo::fetch_viewer_role(pg, community_id, actor_id)
        .await?
        .ok_or(CommunitiesError::Forbidden)?;
    if !is_privileged_role(&actor_role) {
        return Err(CommunitiesError::Forbidden);
    }
    member_invite_response_repo::cancel_invitation(pg, community_id, invitation_id).await
}

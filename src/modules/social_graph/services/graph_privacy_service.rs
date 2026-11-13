use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::social_graph::{
    dto::{FollowMutationResponseDto, PendingRequestActionResponseDto, PrivacySettingResponseDto},
    error::SocialGraphError,
    models::RelationshipState,
    repositories::{follow_edge_repo, follow_privacy_repo},
    services::graph_command_service::GraphCommandService,
};

pub async fn handle_follow_request(
    cmd: &GraphCommandService,
    pool: &PgPool,
    owner_id: Uuid,
    requester_id: Uuid,
    accept: bool,
) -> Result<PendingRequestActionResponseDto, SocialGraphError> {
    if owner_id == requester_id {
        return Err(SocialGraphError::SelfFollowNotAllowed);
    }
    let owner_summary = follow_edge_repo::fetch_actor_summary(pool, owner_id).await?;
    let outcome = if accept {
        follow_privacy_repo::accept_follow_request_idempotent(pool, owner_id, requester_id).await?
    } else {
        follow_privacy_repo::reject_follow_request_idempotent(pool, owner_id, requester_id).await?
    };

    if outcome.changed {
        if accept {
            cmd.cache.on_request_accepted(owner_id, requester_id).await;
            let pair = follow_edge_repo::fetch_pair_edge_state(pool, requester_id, owner_id).await?;
            let req_state = RelationshipState::resolve(Some(requester_id), owner_id, true, pair.in_accepted, false);
            let msg = format!("{} menyetujui permintaan mengikuti Anda.", owner_summary.display_name);
            cmd.notif_dispatcher
                .notify_relationship_event(&owner_summary, requester_id, "request_accepted", req_state, &msg)
                .await;
        } else {
            cmd.cache.on_follow_removed(requester_id, owner_id).await;
            cmd.notif_dispatcher.invalidate_pair_profiles(owner_id, requester_id).await;
        }
    }
    let pair = follow_edge_repo::fetch_pair_edge_state(pool, owner_id, requester_id).await?;
    let state = RelationshipState::resolve(Some(owner_id), requester_id, pair.out_accepted, pair.in_accepted, pair.out_pending);
    Ok(PendingRequestActionResponseDto {
        requester_id,
        action: if accept { "accepted" } else { "rejected" }.to_string(),
        state,
        pending_follow_requests_count: outcome.pending_follow_requests_count,
        followers_count: outcome.followers_count,
    })
}

pub async fn remove_follower(
    cmd: &GraphCommandService,
    pool: &PgPool,
    owner_id: Uuid,
    follower_id: Uuid,
) -> Result<FollowMutationResponseDto, SocialGraphError> {
    if owner_id == follower_id {
        return Err(SocialGraphError::SelfFollowNotAllowed);
    }
    let outcome = follow_edge_repo::unfollow_user_idempotent(pool, follower_id, owner_id).await?;
    if outcome.changed {
        cmd.cache.on_follow_removed(follower_id, owner_id).await;
    }
    cmd.notif_dispatcher.invalidate_pair_profiles(owner_id, follower_id).await;
    let pair = follow_edge_repo::fetch_pair_edge_state(pool, owner_id, follower_id).await?;
    let state = RelationshipState::resolve(Some(owner_id), follower_id, pair.out_accepted, false, pair.out_pending);
    Ok(FollowMutationResponseDto {
        target_user_id: follower_id,
        state,
        status: "REMOVED".to_string(),
        is_idempotent_replay: !outcome.changed,
        target_followers_count: outcome.target_followers_count,
        target_following_count: outcome.target_following_count,
        actor_following_count: outcome.actor_following_count,
        mutual_followers_count: 0,
    })
}

pub async fn update_privacy(
    cmd: &GraphCommandService,
    pool: &PgPool,
    owner_id: Uuid,
    is_private: bool,
) -> Result<PrivacySettingResponseDto, SocialGraphError> {
    let row = follow_privacy_repo::update_privacy_setting(pool, owner_id, is_private).await?;
    cmd.notif_dispatcher.invalidate_pair_profiles(owner_id, owner_id).await;
    Ok(PrivacySettingResponseDto {
        is_private_account: row.is_private_account,
        pending_follow_requests_count: row.pending_follow_requests_count,
    })
}

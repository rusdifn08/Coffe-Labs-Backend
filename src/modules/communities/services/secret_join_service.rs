use chrono::{Duration as ChronoDuration, Utc};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use sqlx::PgPool;
use std::{num::NonZeroU32, sync::Arc};
use uuid::Uuid;

use crate::modules::communities::{
    dto::{CommunityInviteSecretDto, CreateInviteSecretRequest, JoinClubResponseDto},
    error::CommunitiesError,
    models::{is_privileged_role, normalize_club_role, CommunityInviteSecretRow},
    repositories::{community_query_repo, invite_claim_repo, invite_secret_repo},
    secret_id_crypto::{generate_secret_id, normalize_and_verify_secret_id},
    services::{
        club_cache_service::ClubCacheService,
        club_discovery_service::summary_from_row,
    },
};

const MAX_ATTEMPTS_PER_MINUTE: NonZeroU32 = NonZeroU32::MIN.saturating_add(4);

pub struct SecretJoinService {
    pg: PgPool,
    hmac_key: [u8; 32],
    cache: Arc<ClubCacheService>,
    brute_force_limiter: DefaultKeyedRateLimiter<Uuid>,
}

impl SecretJoinService {
    pub fn new(pg: PgPool, hmac_key: [u8; 32], cache: Arc<ClubCacheService>) -> Self {
        let quota = Quota::per_minute(MAX_ATTEMPTS_PER_MINUTE);
        let brute_force_limiter = RateLimiter::keyed(quota);
        Self {
            pg,
            hmac_key,
            cache,
            brute_force_limiter,
        }
    }

    pub fn hmac_key(&self) -> &[u8; 32] {
        &self.hmac_key
    }

    fn map_invite_dto(row: CommunityInviteSecretRow) -> CommunityInviteSecretDto {
        CommunityInviteSecretDto {
            id: row.id,
            community_id: row.community_id,
            secret_code_display: row.secret_code_display,
            invite_deep_link: format!("coffeelabs://club/join/{}", row.invite_url_token),
            invite_web_link: format!("https://coffeelabs.app/c/{}", row.invite_url_token),
            invite_url_token: row.invite_url_token,
            role_granted: row.role_granted,
            max_uses: row.max_uses,
            used_count: row.used_count,
            expires_at: row.expires_at,
            is_revoked: row.is_revoked,
            created_at: row.created_at,
        }
    }

    pub async fn create_invite(
        &self,
        actor_id: Uuid,
        community_id: Uuid,
        req: CreateInviteSecretRequest,
    ) -> Result<CommunityInviteSecretDto, CommunitiesError> {
        let _club = self.cache.get_club(&self.pg, community_id).await?;
        let actor_role = community_query_repo::fetch_viewer_role(&self.pg, community_id, actor_id)
            .await?
            .ok_or(CommunitiesError::Forbidden)?;

        if !is_privileged_role(&actor_role) {
            return Err(CommunitiesError::Forbidden);
        }

        let role_granted = match req.role_granted.as_deref() {
            Some(r) => normalize_club_role(r).ok_or_else(|| {
                CommunitiesError::BadRequest("role_granted tidak valid.".into())
            })?,
            None => "BARISTA_MEMBER",
        };

        if matches!(role_granted, "OWNER" | "ADMIN") && actor_role != "OWNER" {
            return Err(CommunitiesError::Forbidden);
        }

        let max_uses = req.max_uses.map(|n| n.clamp(1, 10_000));
        let expires_at = req
            .expires_in_hours
            .map(|h| Utc::now() + ChronoDuration::hours(h.clamp(1, 24 * 365)));

        let generated = generate_secret_id(&self.hmac_key)?;
        let row = invite_secret_repo::insert_invite_secret(
            &self.pg,
            community_id,
            actor_id,
            &generated,
            role_granted,
            max_uses,
            expires_at,
        )
        .await?;

        Ok(Self::map_invite_dto(row))
    }

    pub async fn list_invites(
        &self,
        actor_id: Uuid,
        community_id: Uuid,
    ) -> Result<Vec<CommunityInviteSecretDto>, CommunitiesError> {
        let _club = self.cache.get_club(&self.pg, community_id).await?;
        let actor_role = community_query_repo::fetch_viewer_role(&self.pg, community_id, actor_id)
            .await?
            .ok_or(CommunitiesError::Forbidden)?;
        if !is_privileged_role(&actor_role) {
            return Err(CommunitiesError::Forbidden);
        }
        let rows = invite_secret_repo::list_active_secrets(&self.pg, community_id).await?;
        Ok(rows.into_iter().map(Self::map_invite_dto).collect())
    }

    pub async fn revoke_invite(
        &self,
        actor_id: Uuid,
        community_id: Uuid,
        invite_id: Uuid,
    ) -> Result<bool, CommunitiesError> {
        let actor_role = community_query_repo::fetch_viewer_role(&self.pg, community_id, actor_id)
            .await?
            .ok_or(CommunitiesError::Forbidden)?;
        if !is_privileged_role(&actor_role) {
            return Err(CommunitiesError::Forbidden);
        }
        invite_secret_repo::revoke_invite_secret(&self.pg, community_id, invite_id).await
    }

    pub async fn join_by_secret_code(
        &self,
        user_id: Uuid,
        raw_secret: &str,
    ) -> Result<JoinClubResponseDto, CommunitiesError> {
        self.brute_force_limiter
            .check_key(&user_id)
            .map_err(|_| CommunitiesError::SecretRateLimited)?;

        let (_, digest) = normalize_and_verify_secret_id(raw_secret, &self.hmac_key)
            .ok_or(CommunitiesError::InvalidOrExpiredSecret)?;

        let Some((claimed, default_conversation_id)) =
            invite_claim_repo::claim_invite_secret_atomic(&self.pg, Some(&digest), None, user_id)
                .await?
        else {
            return Err(CommunitiesError::InvalidOrExpiredSecret);
        };

        self.cache.invalidate_club(claimed.community_id).await;
        let club = self.cache.get_club(&self.pg, claimed.community_id).await?;
        Ok(JoinClubResponseDto {
            joined: true,
            already_member: claimed.already_member,
            assigned_role: claimed.assigned_role.clone(),
            club: summary_from_row(&club, Some(claimed.assigned_role)),
            default_conversation_id,
        })
    }

    pub async fn join_by_invite_token(
        &self,
        user_id: Uuid,
        raw_token: &str,
    ) -> Result<JoinClubResponseDto, CommunitiesError> {
        self.brute_force_limiter
            .check_key(&user_id)
            .map_err(|_| CommunitiesError::SecretRateLimited)?;

        let token = raw_token.trim();
        if token.len() < 12 || token.len() > 64 {
            return Err(CommunitiesError::InvalidOrExpiredSecret);
        }

        let Some((claimed, default_conversation_id)) =
            invite_claim_repo::claim_invite_secret_atomic(&self.pg, None, Some(token), user_id)
                .await?
        else {
            return Err(CommunitiesError::InvalidOrExpiredSecret);
        };

        self.cache.invalidate_club(claimed.community_id).await;
        let club = self.cache.get_club(&self.pg, claimed.community_id).await?;
        Ok(JoinClubResponseDto {
            joined: true,
            already_member: claimed.already_member,
            assigned_role: claimed.assigned_role.clone(),
            club: summary_from_row(&club, Some(claimed.assigned_role)),
            default_conversation_id,
        })
    }
}

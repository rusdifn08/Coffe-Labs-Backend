use std::{num::NonZeroU32, sync::Arc, time::Duration};

use governor::{
    clock::{Clock, DefaultClock},
    state::keyed::DefaultKeyedStateStore,
    Quota, RateLimiter,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::social_graph::{
    dto::FollowMutationResponseDto,
    error::SocialGraphError,
    models::{FollowEdgeStatus, RelationshipState},
    repositories::follow_edge_repo,
    services::{
        follow_notif_dispatcher::FollowNotifDispatcher,
        redis_graph_cache::RedisGraphCache,
    },
};

type KeyedLimiter = RateLimiter<Uuid, DefaultKeyedStateStore<Uuid>, DefaultClock>;

const fn nz(v: u32) -> NonZeroU32 {
    match NonZeroU32::new(v) {
        Some(n) => n,
        None => NonZeroU32::MIN,
    }
}

pub struct GraphCommandService {
    pub(crate) cache: Arc<RedisGraphCache>,
    pub(crate) notif_dispatcher: Arc<FollowNotifDispatcher>,
    minute_limiter: KeyedLimiter,
    daily_limiter: KeyedLimiter,
}

impl GraphCommandService {
    pub fn new(cache: Arc<RedisGraphCache>, notif_dispatcher: Arc<FollowNotifDispatcher>) -> Self {
        let minute_limiter = RateLimiter::keyed(Quota::per_minute(nz(15)));
        let daily_quota = Quota::with_period(Duration::from_secs(432))
            .unwrap_or_else(|| Quota::per_hour(nz(10)))
            .allow_burst(nz(200));
        Self {
            cache,
            notif_dispatcher,
            minute_limiter,
            daily_limiter: RateLimiter::keyed(daily_quota),
        }
    }

    pub fn enforce_rate_limit(&self, user_id: Uuid) -> Result<(), SocialGraphError> {
        if let Err(neg) = self.minute_limiter.check_key(&user_id) {
            let secs = neg.wait_time_from(DefaultClock::default().now()).as_secs().max(1);
            return Err(SocialGraphError::TooManyRequests {
                message: format!("Terlalu banyak aksi follow/unfollow. Coba lagi dalam {secs} detik."),
                retry_after_secs: secs,
            });
        }
        if let Err(neg) = self.daily_limiter.check_key(&user_id) {
            let secs = neg.wait_time_from(DefaultClock::default().now()).as_secs().max(60);
            return Err(SocialGraphError::TooManyRequests {
                message: "Batas harian interaksi sosial (200 aksi/hari) tercapai.".to_string(),
                retry_after_secs: secs,
            });
        }
        Ok(())
    }

    pub async fn follow_user(
        &self,
        pool: &PgPool,
        actor_id: Uuid,
        target: &str,
    ) -> Result<FollowMutationResponseDto, SocialGraphError> {
        let target_row = follow_edge_repo::resolve_target_user(pool, target).await?;
        if actor_id == target_row.user_id {
            return Err(SocialGraphError::SelfFollowNotAllowed);
        }
        self.enforce_rate_limit(actor_id)?;

        let actor_summary = follow_edge_repo::fetch_actor_summary(pool, actor_id).await?;
        let outcome = follow_edge_repo::follow_user_idempotent(
            pool,
            actor_id,
            target_row.user_id,
            target_row.is_private_account,
        )
        .await?;

        let is_pending = matches!(outcome.resulting_status, Some(FollowEdgeStatus::Pending));
        if outcome.changed {
            self.cache.on_follow_created(actor_id, target_row.user_id, is_pending).await;
        }
        let snap = self.cache.get_snapshot_and_mutuals(pool, actor_id, target_row.user_id).await?;
        let state = RelationshipState::resolve(Some(actor_id), target_row.user_id, !is_pending, snap.pair.in_accepted, is_pending);

        if outcome.changed {
            let (action, recipient_state, msg) = if is_pending {
                ("follow_request", RelationshipState::None, format!("{} meminta untuk mengikuti Anda.", actor_summary.display_name))
            } else if snap.pair.in_accepted {
                ("follback", RelationshipState::Mutual, format!("{} mengikuti Anda balik — kalian kini Mutual!", actor_summary.display_name))
            } else {
                ("follow", RelationshipState::FollowedByTarget, format!("{} mulai mengikuti Anda.", actor_summary.display_name))
            };
            self.notif_dispatcher
                .notify_relationship_event(&actor_summary, target_row.user_id, action, recipient_state, &msg)
                .await;
        } else {
            self.notif_dispatcher.invalidate_pair_profiles(actor_id, target_row.user_id).await;
        }

        Ok(FollowMutationResponseDto {
            target_user_id: target_row.user_id,
            state,
            status: if is_pending { "PENDING" } else { "ACCEPTED" }.to_string(),
            is_idempotent_replay: !outcome.changed,
            target_followers_count: outcome.target_followers_count,
            target_following_count: outcome.target_following_count,
            actor_following_count: outcome.actor_following_count,
            mutual_followers_count: snap.mutual_count,
        })
    }

    pub async fn unfollow_user(
        &self,
        pool: &PgPool,
        actor_id: Uuid,
        target: &str,
    ) -> Result<FollowMutationResponseDto, SocialGraphError> {
        let target_row = follow_edge_repo::resolve_target_user(pool, target).await?;
        if actor_id == target_row.user_id {
            return Err(SocialGraphError::SelfFollowNotAllowed);
        }
        self.enforce_rate_limit(actor_id)?;

        let outcome = follow_edge_repo::unfollow_user_idempotent(pool, actor_id, target_row.user_id).await?;
        if outcome.changed {
            self.cache.on_follow_removed(actor_id, target_row.user_id).await;
        }
        self.notif_dispatcher.invalidate_pair_profiles(actor_id, target_row.user_id).await;
        let snap = self.cache.get_snapshot_and_mutuals(pool, actor_id, target_row.user_id).await?;
        let state = RelationshipState::resolve(Some(actor_id), target_row.user_id, false, snap.pair.in_accepted, false);

        Ok(FollowMutationResponseDto {
            target_user_id: target_row.user_id,
            state,
            status: "NONE".to_string(),
            is_idempotent_replay: !outcome.changed,
            target_followers_count: outcome.target_followers_count,
            target_following_count: outcome.target_following_count,
            actor_following_count: outcome.actor_following_count,
            mutual_followers_count: snap.mutual_count,
        })
    }
}

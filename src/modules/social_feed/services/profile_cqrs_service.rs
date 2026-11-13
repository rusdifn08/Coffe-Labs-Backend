use fred::{clients::RedisPool, interfaces::KeysInterface, types::Expiration};
use moka::future::Cache;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    core::{cache::AppCaches, redis::PROFILE_CACHE_TTL_SECS},
    modules::{
        chat::services::moderation_guard::{ModerationGuard, ModerationMode},
        social_feed::{
            attachments::{BadgeMiniDto, RecentBrewSnapshot},
            dto::{FollowToggleResponseDto, MaterializedProfileDto, UpdateMyProfileRequest},
            error::SocialFeedError,
            models::{
                barista_rank_for_level, compute_level_progress_ratio, decode_roaring_ids,
                level_from_total_xp, xp_req_for_level, UserProfileProjectionRow,
            },
            repositories::{follow_repo, profile_projection_repo},
        },
    },
};

const REDIS_PROFILE_PREFIX: &str = "social:profile:mat_view:";

/// Lightweight CQRS service: L1 (`moka`, 3s) + L2 (`fred` Redis, 300s) + L3 (NeonDB projection).
pub struct ProfileCqrsService {
    l1_cache: Cache<Uuid, Arc<serde_json::Value>>,
    redis_pool: Option<RedisPool>,
}

impl ProfileCqrsService {
    #[allow(dead_code)]
    pub fn new(redis_pool: Option<RedisPool>) -> Self {
        Self::with_l1_cache(redis_pool, AppCaches::build_profile_l1())
    }

    pub fn with_l1_cache(redis_pool: Option<RedisPool>, l1_cache: Cache<Uuid, Arc<serde_json::Value>>) -> Self {
        Self { l1_cache, redis_pool }
    }

    pub async fn invalidate_profile(&self, user_id: Uuid) {
        self.l1_cache.invalidate(&user_id).await;
        if let Some(pool) = &self.redis_pool {
            let key = format!("{REDIS_PROFILE_PREFIX}{user_id}");
            let _ = pool.del::<i64, _>(key).await;
        }
    }

    pub async fn get_materialized_profile(
        &self,
        pg: &PgPool,
        viewer_id: Option<Uuid>,
        target: &str,
    ) -> Result<MaterializedProfileDto, SocialFeedError> {
        let target_id = profile_projection_repo::resolve_target_user_id(pg, target, viewer_id).await?;
        let base = self.load_cached_base_profile(pg, target_id).await?;
        let mut dto = (*base).clone();
        if let Some(vid) = viewer_id {
            dto.is_own_profile = vid == target_id;
            let (followed, following) = follow_repo::fetch_follow_relationship(pg, vid, target_id).await?;
            dto.is_followed_by_viewer = followed;
            dto.is_following_viewer = following;
        }
        Ok(dto)
    }

    async fn load_cached_base_profile(&self, pg: &PgPool, target_id: Uuid) -> Result<Arc<MaterializedProfileDto>, SocialFeedError> {
        if let Some(hit) = self.l1_cache.get(&target_id).await {
            if let Ok(parsed) = serde_json::from_value::<MaterializedProfileDto>(hit.as_ref().clone()) {
                return Ok(Arc::new(parsed));
            }
        }
        if let Some(redis) = &self.redis_pool {
            let key = format!("{REDIS_PROFILE_PREFIX}{target_id}");
            if let Ok(Some(raw)) = redis.get::<Option<String>, _>(&key).await {
                if let Ok(parsed) = serde_json::from_str::<MaterializedProfileDto>(&raw) {
                    let arc = Arc::new(parsed);
                    if let Ok(val) = serde_json::to_value(arc.as_ref()) { self.l1_cache.insert(target_id, Arc::new(val)).await; }
                    return Ok(arc);
                }
            }
        }
        let arc = Arc::new(self.fetch_l3_projection(pg, target_id).await?);
        if let Ok(val) = serde_json::to_value(arc.as_ref()) { self.l1_cache.insert(target_id, Arc::new(val)).await; }
        if let Some(redis) = &self.redis_pool {
            if let Ok(serialized) = serde_json::to_string(arc.as_ref()) {
                let key = format!("{REDIS_PROFILE_PREFIX}{target_id}");
                let _ = redis.set::<(), _, _>(key, serialized, Some(Expiration::EX(PROFILE_CACHE_TTL_SECS)), None, false).await;
            }
        }
        Ok(arc)
    }

    async fn fetch_l3_projection(
        &self,
        pg: &PgPool,
        target_id: Uuid,
    ) -> Result<MaterializedProfileDto, SocialFeedError> {
        let row = profile_projection_repo::ensure_projection_exists(pg, target_id).await?;
        let badge_ids = decode_roaring_ids(row.unlocked_badges_bitmap.as_deref());
        let db_badges = profile_projection_repo::fetch_badges_for_ids(pg, &badge_ids).await?;
        let featured: Vec<BadgeMiniDto> = if db_badges.is_empty() {
            serde_json::from_value(row.top_featured_badges.clone()).unwrap_or_default()
        } else {
            db_badges
                .into_iter()
                .map(|b| BadgeMiniDto { id: b.id, slug: b.slug, title: b.title, rarity: b.rarity, rive_artboard: b.rive_artboard })
                .collect()
        };
        Ok(map_row_to_dto(row, badge_ids, featured))
    }

    pub async fn update_my_profile(
        &self,
        pg: &PgPool,
        guard: &ModerationGuard,
        user_id: Uuid,
        req: UpdateMyProfileRequest,
    ) -> Result<MaterializedProfileDto, SocialFeedError> {
        let clean_name = check_optional(guard, req.display_name.as_deref())?;
        let clean_bio = check_optional(guard, req.bio.as_deref())?;
        profile_projection_repo::update_projection_profile(
            pg, user_id, &req, clean_name.as_deref(), clean_bio.as_deref(),
        ).await?;
        self.invalidate_profile(user_id).await;
        self.get_materialized_profile(pg, Some(user_id), "me").await
    }

    pub async fn toggle_follow(
        &self,
        pg: &PgPool,
        viewer_id: Uuid,
        target: &str,
    ) -> Result<FollowToggleResponseDto, SocialFeedError> {
        let target_id = profile_projection_repo::resolve_target_user_id(pg, target, Some(viewer_id)).await?;
        profile_projection_repo::ensure_projection_exists(pg, viewer_id).await?;
        profile_projection_repo::ensure_projection_exists(pg, target_id).await?;
        let (is_followed, followers_count) = follow_repo::toggle_follow_atomic(pg, viewer_id, target_id).await?;
        self.invalidate_profile(viewer_id).await;
        self.invalidate_profile(target_id).await;
        Ok(FollowToggleResponseDto { target_user_id: target_id, is_followed_by_viewer: is_followed, followers_count })
    }
}

fn check_optional(guard: &ModerationGuard, raw: Option<&str>) -> Result<Option<String>, SocialFeedError> {
    raw.map(|s| guard.check_text(s, ModerationMode::RejectMessage))
        .transpose()
        .map_err(|e| SocialFeedError::ModerationRejected(e.to_string()))
}

fn map_row_to_dto(
    row: UserProfileProjectionRow,
    unlocked_badge_ids: Vec<u32>,
    top_featured_badges: Vec<BadgeMiniDto>,
) -> MaterializedProfileDto {
    let level = level_from_total_xp(row.total_xp).max(row.barista_level);
    let recent_brew_snapshot = row
        .recent_brew_snapshot
        .and_then(|v| serde_json::from_value::<RecentBrewSnapshot>(v).ok());
    MaterializedProfileDto {
        user_id: row.user_id,
        username: row.username,
        display_name: row.display_name,
        bio: row.bio,
        avatar_url: row.avatar_url,
        banner_url: row.banner_url,
        account_tier: row.account_tier,
        total_xp: row.total_xp,
        barista_level: level,
        barista_rank_title: barista_rank_for_level(level).to_string(),
        level_progress_ratio: compute_level_progress_ratio(level, row.total_xp),
        next_level_xp: xp_req_for_level((level.max(1) + 1) as u32),
        current_streak_days: row.current_streak_days,
        unlocked_badge_ids,
        top_featured_badges,
        followers_count: row.followers_count,
        following_count: row.following_count,
        total_threads_count: row.total_threads_count,
        total_brews_count: row.total_brews_count,
        total_recipes_created_count: row.total_recipes_created_count,
        total_recipe_tries_received: row.total_recipe_tries_received,
        recent_brew_snapshot,
        is_followed_by_viewer: false,
        is_following_viewer: false,
        is_own_profile: false,
        updated_at: row.updated_at,
    }
}

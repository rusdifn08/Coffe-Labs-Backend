use moka::future::Cache;
use sqlx::PgPool;
use std::{sync::OnceLock, time::Duration};
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::{
        gamification::{
            dto::{BadgeDto, GamificationProfileResponse},
            models::LeaderboardRow,
            repository,
            services::xp_calculator::{
                decode_badges_bitmap, default_freezes_for_tier, level_title_for,
                tier_xp_multiplier, xp_for_level,
            },
        },
        rbac::models::AccountTier,
    },
};

static LEADERBOARD_CACHE: OnceLock<Cache<String, Vec<LeaderboardRow>>> = OnceLock::new();

fn lb_cache() -> &'static Cache<String, Vec<LeaderboardRow>> {
    LEADERBOARD_CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(16)
            .time_to_live(Duration::from_secs(60))
            .build()
    })
}

pub async fn invalidate_leaderboards() {
    lb_cache().invalidate_all();
}

pub async fn get_cached_leaderboard(
    pool: &PgPool,
    scope: &str,
    limit: i64,
) -> AppResult<Vec<LeaderboardRow>> {
    let clamped_limit = limit.clamp(1, 100);
    let weekly_only = scope.eq_ignore_ascii_case("weekly");
    let cache_key = format!("{}:{}", if weekly_only { "weekly" } else { "global" }, clamped_limit);

    if let Some(cached) = lb_cache().get(&cache_key).await {
        return Ok(cached);
    }

    let rows = repository::query_leaderboard(pool, weekly_only, clamped_limit).await?;
    lb_cache().insert(cache_key, rows.clone()).await;
    Ok(rows)
}

pub async fn get_full_profile(
    pool: &PgPool,
    user_id: Uuid,
    username: &str,
    tier: AccountTier,
    timezone: &str,
) -> AppResult<GamificationProfileResponse> {
    let default_freezes = default_freezes_for_tier(tier);
    let profile = repository::get_or_create_profile(pool, user_id, default_freezes, timezone).await?;
    let catalog = repository::list_catalog(pool).await?;

    let bitmap = decode_badges_bitmap(&profile.unlocked_badges_bitmap);
    let unlocked_ids: Vec<u32> = bitmap.iter().collect();

    let lvl = (profile.current_level.max(1)) as u32;
    let floor_xp = xp_for_level(lvl);
    let next_xp = xp_for_level((lvl + 1).min(100));
    let span = (next_xp - floor_xp).max(1) as f64;
    let progress = (((profile.total_xp - floor_xp).max(0) as f64 / span) * 100.0).clamp(0.0, 100.0);

    let badges = catalog
        .into_iter()
        .map(|c| BadgeDto {
            id: c.id,
            slug: c.slug,
            title: c.title,
            category: c.category,
            description: c.description,
            criteria_json: c.criteria_json,
            xp_reward: c.xp_reward,
            rive_artboard: c.rive_artboard,
            rarity: c.rarity,
            tier_required: c.tier_required,
            is_unlocked: bitmap.contains(c.id as u32),
        })
        .collect();

    Ok(GamificationProfileResponse {
        user_id,
        username: username.to_string(),
        tier: tier.as_str().to_string(),
        tier_multiplier: tier_xp_multiplier(tier),
        total_xp: profile.total_xp,
        current_level: profile.current_level,
        level_title: level_title_for(lvl).to_string(),
        current_level_floor_xp: floor_xp,
        next_level_xp: next_xp,
        level_progress_pct: (progress * 100.0).round() / 100.0,
        current_streak: profile.current_streak,
        longest_streak: profile.longest_streak,
        streak_freezes_available: profile.streak_freezes_available,
        last_brew_local_date: profile.last_brew_local_date,
        timezone: profile.timezone,
        unlocked_badge_ids: unlocked_ids,
        badges,
    })
}

use chrono::Utc;
use sqlx::PgPool;
use ulid::Ulid;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::{
        gamification::{
            dto::{
                BadgeDto, ClaimDomainActionRequest, CompleteBrewSessionRequest,
                StartBrewSessionRequest, StartBrewSessionResponse, XpRewardResponse,
            },
            error::GamificationError,
            models::XpActionType,
            repository,
            services::{
                leaderboard_service::invalidate_leaderboards,
                streak_service::{evaluate_daily_streak, resolve_local_date},
                xp_calculator::{
                    daily_xp_cap_for_tier, decode_badges_bitmap, default_freezes_for_tier,
                    encode_badges_bitmap, level_from_total_xp, level_title_for,
                    tier_xp_multiplier, xp_for_level,
                },
            },
        },
        rbac::models::AccountTier,
    },
};

fn collect_candidate_badges(
    req: &CompleteBrewSessionRequest,
    actual_sec: i64,
    target_sec: i32,
    new_streak: i32,
    tier: AccountTier,
) -> Vec<u32> {
    let mut ids = vec![1u32]; // #1 First Golden Bloom
    if (actual_sec - i64::from(target_sec)).abs() <= 5 { ids.push(2); }
    if req.water_temp_c.is_some_and(|t| (91..=93).contains(&t)) { ids.push(4); }
    if req.sensory_score.is_some_and(|s| s >= 4.75) && tier.is_at_least(AccountTier::Pro) { ids.push(5); }
    if req.origin.as_deref().is_some_and(|o| o.to_lowercase().contains("kintamani")) { ids.push(6); }
    if req.origin.as_deref().is_some_and(|o| o.to_lowercase().contains("gayo")) { ids.push(7); }
    if req.process.as_deref().is_some_and(|p| p.to_lowercase().contains("anaerobic") || p.to_lowercase().contains("carbonic")) && tier.is_at_least(AccountTier::Pro) { ids.push(9); }
    if req.variety.as_deref().is_some_and(|v| v.to_lowercase().contains("geisha")) && tier.is_at_least(AccountTier::Pro) { ids.push(10); }
    if req.technique.as_deref().is_some_and(|t| t.contains("4_6")) { ids.push(11); }
    if new_streak >= 7 { ids.push(18); }
    if new_streak >= 30 && tier.is_at_least(AccountTier::Pro) { ids.push(19); }
    ids
}

pub async fn start_brew_session(
    pool: &PgPool,
    user_id: Uuid,
    req: StartBrewSessionRequest,
) -> AppResult<StartBrewSessionResponse> {
    let recipe_id = req.recipe_id.unwrap_or_else(Uuid::new_v4);
    let session = repository::insert_brew_session(pool, user_id, recipe_id, req.target_duration_sec).await?;
    let min_valid = ((req.target_duration_sec as f64) * 0.80).floor() as i32;
    let idem_key = format!("brew_sess_{}", session.id.simple());
    let token = format!("cl_bs_{}_{}", session.id.simple(), session.started_at.timestamp());

    Ok(StartBrewSessionResponse {
        session_id: session.id,
        session_token: token,
        idempotency_key: idem_key,
        recipe_id: session.recipe_id,
        target_duration_sec: session.target_duration_sec,
        min_valid_duration_sec: min_valid,
        started_at: session.started_at,
    })
}

pub async fn complete_brew_session(
    pool: &PgPool,
    user_id: Uuid,
    tier: AccountTier,
    req: CompleteBrewSessionRequest,
) -> AppResult<XpRewardResponse> {
    let tz = req.timezone.as_deref().unwrap_or("Asia/Jakarta");
    let profile = repository::get_or_create_profile(pool, user_id, default_freezes_for_tier(tier), tz).await?;
    let catalog = repository::list_catalog(pool).await?;

    let mut tx = pool.begin().await?;
    let session = repository::find_brew_session_for_update(&mut tx, req.session_id, user_id)
        .await?
        .ok_or(GamificationError::SessionNotFound)?;
    if session.status == "COMPLETED" {
        return Err(GamificationError::SessionAlreadyCompleted.into());
    }
    let elapsed_wall = (Utc::now() - session.started_at).num_seconds().max(0);
    let actual_sec = req.actual_duration_sec.map(i64::from).unwrap_or(elapsed_wall).max(elapsed_wall);
    let min_required = ((session.target_duration_sec as f64) * 0.80).floor() as i64;
    if actual_sec < min_required {
        repository::mark_session_completed(&mut tx, session.id, false, "REJECTED_TOO_FAST").await?;
        tx.commit().await?;
        return Err(GamificationError::AntiCheatDurationTooShort { actual: actual_sec, min_required }.into());
    }
    repository::mark_session_completed(&mut tx, session.id, true, "COMPLETED").await?;

    let today_xp = repository::sum_today_xp(&mut tx, user_id).await?;
    let cap = daily_xp_cap_for_tier(tier);
    if today_xp >= i64::from(cap) {
        return Err(GamificationError::DailyXpCapReached { cap }.into());
    }

    let today_local = resolve_local_date(tz);
    let streak_eval = evaluate_daily_streak(
        profile.current_streak, profile.longest_streak, profile.streak_freezes_available,
        profile.last_brew_local_date, today_local, tier,
    );

    let mut bitmap = decode_badges_bitmap(&profile.unlocked_badges_bitmap);
    let candidate_badges = collect_candidate_badges(&req, actual_sec, session.target_duration_sec, streak_eval.new_streak, tier);
    let mut newly_unlocked = Vec::new();
    let mut badges_bonus_xp = 0i32;

    for bid in candidate_badges {
        if !bitmap.contains(bid) {
            if let Some(cat) = catalog.iter().find(|c| c.id as u32 == bid) {
                bitmap.insert(bid);
                badges_bonus_xp += cat.xp_reward;
                newly_unlocked.push(BadgeDto {
                    id: cat.id, slug: cat.slug.clone(), title: cat.title.clone(),
                    category: cat.category.clone(), description: cat.description.clone(),
                    criteria_json: cat.criteria_json.clone(), xp_reward: cat.xp_reward,
                    rive_artboard: cat.rive_artboard.clone(), rarity: cat.rarity.clone(),
                    tier_required: cat.tier_required.clone(), is_unlocked: true,
                });
            }
        }
    }

    let base_xp = XpActionType::BrewCompleted.base_xp();
    let mult = tier_xp_multiplier(tier);
    let total_gained = (((base_xp as f64) * mult).round() as i32) + streak_eval.streak_bonus_xp + badges_bonus_xp;
    let ulid_str = Ulid::new().to_string();

    if !repository::append_xp_ledger(&mut tx, &ulid_str, user_id, XpActionType::BrewCompleted.as_str(), base_xp, mult, total_gained, Some(session.id), &req.idempotency_key).await? {
        return Err(GamificationError::DuplicateIdempotencyKey(req.idempotency_key).into());
    }

    let new_total_xp = profile.total_xp + i64::from(total_gained);
    let prev_level = profile.current_level.max(1);
    let new_level = level_from_total_xp(new_total_xp) as i32;
    let encoded_bmap = encode_badges_bitmap(&bitmap);

    repository::update_profile_tx(&mut tx, user_id, new_total_xp, new_level, streak_eval.new_streak, streak_eval.new_longest, streak_eval.freezes_remaining, today_local, tz, &encoded_bmap).await?;
    tx.commit().await?;
    invalidate_leaderboards().await;

    Ok(XpRewardResponse {
        transaction_ulid: ulid_str, action_type: XpActionType::BrewCompleted.as_str().to_string(),
        base_xp, tier_multiplier: mult, streak_bonus_xp: streak_eval.streak_bonus_xp,
        badges_bonus_xp, xp_gained: total_gained, total_xp: new_total_xp,
        previous_level: prev_level, new_level, leveled_up: new_level > prev_level,
        level_title: level_title_for(new_level as u32).to_string(),
        current_level_floor_xp: xp_for_level(new_level as u32), next_level_xp: xp_for_level(((new_level + 1) as u32).min(100)),
        current_streak: streak_eval.new_streak, longest_streak: streak_eval.new_longest,
        streak_updated: streak_eval.streak_updated, streak_status: streak_eval.streak_status.to_string(),
        streak_freezes_available: streak_eval.freezes_remaining, newly_unlocked_badges: newly_unlocked,
    })
}

pub async fn claim_domain_action(pool: &PgPool, user_id: Uuid, tier: AccountTier, req: ClaimDomainActionRequest) -> AppResult<XpRewardResponse> {
    let action = match req.action_type.to_uppercase().as_str() {
        "MASTERCLASS_FINISHED" => XpActionType::MasterclassFinished,
        "RECIPE_SHARED" => XpActionType::RecipeShared,
        "QUEST_REWARD" => XpActionType::QuestReward,
        _ => XpActionType::StreakBonus,
    };
    let tz = req.timezone.as_deref().unwrap_or("Asia/Jakarta");
    let profile = repository::get_or_create_profile(pool, user_id, default_freezes_for_tier(tier), tz).await?;
    let mut tx = pool.begin().await?;
    let (base_xp, mult) = (action.base_xp(), tier_xp_multiplier(tier));
    let gained = ((base_xp as f64) * mult).round() as i32;
    let ulid_str = Ulid::new().to_string();

    if !repository::append_xp_ledger(&mut tx, &ulid_str, user_id, action.as_str(), base_xp, mult, gained, req.reference_id, &req.idempotency_key).await? {
        return Err(GamificationError::DuplicateIdempotencyKey(req.idempotency_key).into());
    }

    let new_total_xp = profile.total_xp + i64::from(gained);
    let (prev_level, new_level) = (profile.current_level.max(1), level_from_total_xp(new_total_xp) as i32);
    let today_local = resolve_local_date(tz);
    repository::update_profile_tx(&mut tx, user_id, new_total_xp, new_level, profile.current_streak, profile.longest_streak, profile.streak_freezes_available, profile.last_brew_local_date.unwrap_or(today_local), tz, &profile.unlocked_badges_bitmap).await?;
    tx.commit().await?;
    invalidate_leaderboards().await;

    Ok(XpRewardResponse {
        transaction_ulid: ulid_str, action_type: action.as_str().to_string(), base_xp, tier_multiplier: mult,
        streak_bonus_xp: 0, badges_bonus_xp: 0, xp_gained: gained, total_xp: new_total_xp,
        previous_level: prev_level, new_level, leveled_up: new_level > prev_level,
        level_title: level_title_for(new_level as u32).to_string(),
        current_level_floor_xp: xp_for_level(new_level as u32), next_level_xp: xp_for_level(((new_level + 1) as u32).min(100)),
        current_streak: profile.current_streak, longest_streak: profile.longest_streak,
        streak_updated: false, streak_status: "UNCHANGED".to_string(),
        streak_freezes_available: profile.streak_freezes_available, newly_unlocked_badges: Vec::new(),
    })
}

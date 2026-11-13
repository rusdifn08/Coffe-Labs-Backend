use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::{
    chat::services::moderation_guard::{ModerationGuard, ModerationMode},
    communities::{
        dto::{ClubFeedQuery, CreateClubThreadRequest},
        error::CommunitiesError,
        repositories::club_feed_repo::{self, InsertClubThreadParams, CLUB_POST_XP_REWARD},
        services::club_cache_service::ClubCacheService,
    },
    recommendation::{scoring_math, taste_profiler_service::TasteProfilerService},
    social_feed::{
        dto::{CursorPaginatedFeedDto, SocialThreadItemDto},
        repositories::{profile_projection_repo, thread_repo},
        services::{profile_cqrs_service::ProfileCqrsService, thread_hydrator},
    },
};

async fn ensure_club_read_access(
    pg: &PgPool,
    cache: &ClubCacheService,
    viewer_id: Option<Uuid>,
    community_id: Uuid,
) -> Result<(), CommunitiesError> {
    let club = cache.get_club(pg, community_id).await?;
    if club.privacy_level == "PUBLIC" {
        return Ok(());
    }
    let Some(vid) = viewer_id else {
        return Err(CommunitiesError::Forbidden);
    };
    if !cache.is_member(pg, community_id, vid).await? {
        return Err(CommunitiesError::Forbidden);
    }
    Ok(())
}

pub async fn get_club_feed(
    pg: &PgPool,
    cache: &ClubCacheService,
    viewer_id: Option<Uuid>,
    community_id: Uuid,
    query: ClubFeedQuery,
) -> Result<CursorPaginatedFeedDto<SocialThreadItemDto>, CommunitiesError> {
    ensure_club_read_access(pg, cache, viewer_id, community_id).await?;
    let limit = query.limit.unwrap_or(20).clamp(1, 50);
    let cursor = query
        .cursor
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| CommunitiesError::BadRequest("Cursor paginasi tidak valid.".into()))?;

    let mut rows =
        club_feed_repo::fetch_club_feed_rows(pg, community_id, cursor, limit + 1).await?;
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    let next_cursor = rows.last().map(|r| r.id.to_string());
    let items = thread_hydrator::hydrate_threads_batch(pg, viewer_id, rows)
        .await
        .map_err(|e| CommunitiesError::BadRequest(e.to_string()))?;

    Ok(CursorPaginatedFeedDto {
        items,
        next_cursor,
        has_more,
    })
}

pub async fn get_club_recipe_vault(
    pg: &PgPool,
    cache: &ClubCacheService,
    viewer_id: Option<Uuid>,
    community_id: Uuid,
) -> Result<Vec<SocialThreadItemDto>, CommunitiesError> {
    ensure_club_read_access(pg, cache, viewer_id, community_id).await?;
    let rows = club_feed_repo::fetch_club_recipe_vault_rows(pg, community_id, 40).await?;
    thread_hydrator::hydrate_threads_batch(pg, viewer_id, rows)
        .await
        .map_err(|e| CommunitiesError::BadRequest(e.to_string()))
}

pub async fn create_club_post(
    pg: &PgPool,
    cache: &ClubCacheService,
    cqrs: &ProfileCqrsService,
    guard: &ModerationGuard,
    taste_profiler: &TasteProfilerService,
    author_id: Uuid,
    community_id: Uuid,
    req: CreateClubThreadRequest,
) -> Result<SocialThreadItemDto, CommunitiesError> {
    let _club = cache.get_club(pg, community_id).await?;
    if !cache.is_member(pg, community_id, author_id).await? {
        return Err(CommunitiesError::Forbidden);
    }

    profile_projection_repo::ensure_projection_exists(pg, author_id)
        .await
        .map_err(|e| CommunitiesError::BadRequest(e.to_string()))?;

    let clean_text = guard
        .check_text(&req.body_text, ModerationMode::RejectMessage)
        .map_err(|e| CommunitiesError::Validation(e.to_string()))?;
    if clean_text.is_empty() && req.attachment.is_none() && req.media_urls.is_empty() {
        return Err(CommunitiesError::BadRequest(
            "Postingan club tidak boleh kosong.".into(),
        ));
    }

    let kind_upper = match req.kind.trim().to_ascii_uppercase().as_str() {
        "BREW_LOG" => "BREW_LOG",
        "CREATED_RECIPE" => "CREATED_RECIPE",
        "REPOST_QUOTE" => "REPOST_QUOTE",
        _ => "STANDARD_POST",
    };

    let mut media_urls = req.media_urls;
    for item in &mut media_urls {
        item.url = crate::core::storage::auto_optimize_url(&item.url);
    }
    let has_media = !media_urls.is_empty();
    let media_urls_json = serde_json::to_value(&media_urls).unwrap_or_default();
    let attachment_type = req.attachment.as_ref().map(|a| a.attachment_type_tag());
    let attachment_payload = req
        .attachment
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| CommunitiesError::BadRequest("Lampiran tidak valid.".into()))?;

    let is_recipe_vault_item = attachment_type == Some("RECIPE_CARD");
    let is_brew_activity =
        kind_upper == "BREW_LOG" || kind_upper == "CREATED_RECIPE" || is_recipe_vault_item;

    let mut taste_source = clean_text.clone();
    if let Some(snap) = &req.recent_brew_snapshot {
        taste_source.push(' ');
        taste_source.push_str(&snap.bean_name);
        taste_source.push(' ');
        taste_source.push_str(&snap.process);
        taste_source.push(' ');
        taste_source.push_str(&snap.method);
    }
    let taste_vec = scoring_math::extract_16d_vector_from_text(&taste_source);
    let post_id = Uuid::now_v7();

    club_feed_repo::insert_club_thread_atomic(
        pg,
        InsertClubThreadParams {
            id: post_id,
            community_id,
            author_id,
            kind: kind_upper,
            quoted_thread_id: req.quoted_thread_id,
            body_text: &clean_text,
            has_media,
            media_urls_json,
            attachment_type,
            attachment_payload,
            is_pinned: req.pin_in_club,
            is_recipe_vault_item,
            is_brew_activity,
            taste_feature_vector: &taste_vec,
        },
    )
    .await?;

    let _ = profile_projection_repo::award_xp_on_projection(pg, author_id, CLUB_POST_XP_REWARD).await;
    cqrs.invalidate_profile(author_id).await;
    cache.invalidate_club(community_id).await;
    taste_profiler.record_signal_best_effort(author_id, &taste_vec).await;

    let row = thread_repo::fetch_thread_by_id(pg, post_id)
        .await
        .map_err(|e| CommunitiesError::BadRequest(e.to_string()))?;
    let mut items = thread_hydrator::hydrate_threads_batch(pg, Some(author_id), vec![row])
        .await
        .map_err(|e| CommunitiesError::BadRequest(e.to_string()))?;
    items.pop().ok_or(CommunitiesError::ClubNotFound)
}

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::{
    chat::services::moderation_guard::{ModerationGuard, ModerationMode},
    social_feed::{
        attachments::{RecentBrewSnapshot, ThreadAttachmentPayload},
        dto::{CreateThreadRequest, SocialThreadItemDto},
        error::SocialFeedError,
        repositories::{
            profile_projection_repo,
            thread_repo,
            thread_write_repo::{self, InsertThreadParams},
        },
        services::{profile_cqrs_service::ProfileCqrsService, thread_hydrator},
    },
};

const XP_BREW_LOG_REWARD: i64 = 20;
const XP_RECIPE_POST_REWARD: i64 = 25;

pub async fn create_thread(
    pg: &PgPool,
    cqrs: &ProfileCqrsService,
    guard: &ModerationGuard,
    author_id: Uuid,
    req: CreateThreadRequest,
) -> Result<SocialThreadItemDto, SocialFeedError> {
    profile_projection_repo::ensure_projection_exists(pg, author_id).await?;

    let clean_text = guard
        .check_text(&req.body_text, ModerationMode::RejectMessage)
        .map_err(|e| SocialFeedError::ModerationRejected(e.to_string()))?;

    let kind_upper = normalize_thread_kind(&req.kind, req.parent_thread_id.is_some())?;
    if clean_text.is_empty() && req.attachment.is_none() && req.media_urls.is_empty() {
        return Err(SocialFeedError::BadRequest(
            "Thread tidak boleh kosong tanpa teks, media, atau lampiran kopi.".into(),
        ));
    }

    let (root_thread_id, depth_level) = if let Some(parent_id) = req.parent_thread_id {
        let parent = thread_repo::fetch_thread_by_id(pg, parent_id).await?;
        let root_id = parent.root_thread_id.unwrap_or(parent.id);
        let depth = (parent.depth_level + 1).clamp(1, 3);
        (Some(root_id), depth)
    } else {
        (None, 0)
    };

    let mut normalized_media = req.media_urls;
    for item in &mut normalized_media {
        item.url = crate::core::storage::auto_optimize_url(&item.url);
    }
    let media_urls_json = serde_json::to_value(&normalized_media)
        .map_err(|_| SocialFeedError::BadRequest("Format media_urls tidak valid".into()))?;
    let has_media = !normalized_media.is_empty();
    let attachment_type = req.attachment.as_ref().map(|a| a.attachment_type_tag());
    let attachment_payload = req
        .attachment
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| SocialFeedError::BadRequest("Payload attachment tidak valid".into()))?;

    let derived_snapshot = req.recent_brew_snapshot.or_else(|| {
        if kind_upper == "BREW_LOG" {
            derive_brew_snapshot(req.attachment.as_ref())
        } else {
            None
        }
    });
    let snapshot_json = derived_snapshot
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| SocialFeedError::BadRequest("Snapshot seduh tidak valid".into()))?;

    let new_id = Uuid::now_v7();
    thread_write_repo::insert_thread_and_update_projection(
        pg,
        InsertThreadParams {
            id: new_id,
            author_id,
            kind: kind_upper,
            parent_thread_id: req.parent_thread_id,
            root_thread_id,
            quoted_thread_id: req.quoted_thread_id,
            depth_level,
            body_text: &clean_text,
            has_media,
            media_urls_json,
            attachment_type,
            attachment_payload,
            is_pinned_on_profile: req.pin_on_profile && depth_level == 0,
            recent_brew_snapshot: snapshot_json,
        },
    )
    .await?;

    let bonus_xp = match kind_upper {
        "BREW_LOG" => XP_BREW_LOG_REWARD,
        "CREATED_RECIPE" => XP_RECIPE_POST_REWARD,
        _ => 0,
    };
    if bonus_xp > 0 {
        let _ = profile_projection_repo::award_xp_on_projection(pg, author_id, bonus_xp).await;
    }

    cqrs.invalidate_profile(author_id).await;

    let row = thread_repo::fetch_thread_by_id(pg, new_id).await?;
    let mut items = thread_hydrator::hydrate_threads_batch(pg, Some(author_id), vec![row]).await?;
    items.pop().ok_or(SocialFeedError::ThreadNotFound)
}

fn normalize_thread_kind(raw: &str, has_parent: bool) -> Result<&'static str, SocialFeedError> {
    if has_parent {
        return Ok("REPLY");
    }
    match raw.trim().to_uppercase().as_str() {
        "STANDARD_POST" | "" => Ok("STANDARD_POST"),
        "BREW_LOG" => Ok("BREW_LOG"),
        "CREATED_RECIPE" => Ok("CREATED_RECIPE"),
        "REPOST_QUOTE" => Ok("REPOST_QUOTE"),
        "REPLY" => Err(SocialFeedError::BadRequest(
            "Balasan wajib menyertakan parent_thread_id.".into(),
        )),
        _ => Err(SocialFeedError::BadRequest(
            "Jenis thread tidak dikenali.".into(),
        )),
    }
}

fn derive_brew_snapshot(attachment: Option<&ThreadAttachmentPayload>) -> Option<RecentBrewSnapshot> {
    match attachment {
        Some(ThreadAttachmentPayload::PouringTelemetryCurve {
            recipe_title,
            bean_name,
            accuracy_score_pct,
            total_duration_sec,
            ..
        }) => Some(RecentBrewSnapshot {
            recipe_id: None,
            recipe_title: recipe_title.clone(),
            bean_name: bean_name.clone(),
            roastery: "Coffee Labs Community".to_string(),
            process: "Pour-Over".to_string(),
            method: "V60".to_string(),
            primary_temp_c: Some(92),
            secondary_temp_c: None,
            coffee_dose_g: Some(15.0),
            total_water_ml: Some(240.0),
            target_duration_sec: Some(*total_duration_sec),
            brewed_at: Utc::now(),
            is_live: false,
            accuracy_score: Some(*accuracy_score_pct),
        }),
        Some(ThreadAttachmentPayload::InteractiveRecipeCard {
            recipe_id,
            title,
            brewer_name,
            method,
            coffee_dose_g,
            total_water_ml,
            primary_temp_c,
            secondary_temp_c,
            target_duration_sec,
            ..
        }) => Some(RecentBrewSnapshot {
            recipe_id: Some(recipe_id.clone()),
            recipe_title: title.clone(),
            bean_name: brewer_name.clone(),
            roastery: "Coffee Labs".to_string(),
            process: method.clone(),
            method: method.clone(),
            primary_temp_c: Some(*primary_temp_c),
            secondary_temp_c: *secondary_temp_c,
            coffee_dose_g: Some(*coffee_dose_g),
            total_water_ml: Some(*total_water_ml),
            target_duration_sec: Some(*target_duration_sec),
            brewed_at: Utc::now(),
            is_live: false,
            accuracy_score: None,
        }),
        _ => None,
    }
}

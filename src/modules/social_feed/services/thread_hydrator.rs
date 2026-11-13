use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::modules::social_feed::{
    attachments::{MediaItemDto, ThreadAttachmentPayload},
    dto::{QuotedThreadPreviewDto, SocialThreadItemDto},
    error::SocialFeedError,
    models::SocialThreadWithAuthorRow,
    repositories::{interaction_repo, thread_repo},
};

pub async fn hydrate_threads_batch(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    rows: Vec<SocialThreadWithAuthorRow>,
) -> Result<Vec<SocialThreadItemDto>, SocialFeedError> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let thread_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let quoted_ids: Vec<Uuid> = rows
        .iter()
        .filter_map(|r| r.quoted_thread_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let (liked_set, reposted_set) = match viewer_id {
        Some(vid) => {
            let liked = interaction_repo::fetch_user_liked_ids(pg, vid, &thread_ids).await?;
            let reposted =
                interaction_repo::fetch_user_reposted_ids(pg, vid, &thread_ids).await?;
            (liked, reposted)
        }
        None => (HashSet::new(), HashSet::new()),
    };

    let quoted_map: HashMap<Uuid, QuotedThreadPreviewDto> =
        thread_repo::fetch_quoted_previews(pg, &quoted_ids)
            .await?
            .into_iter()
            .map(|q| (q.id, q))
            .collect();

    Ok(rows
        .into_iter()
        .map(|r| {
            let media_urls =
                serde_json::from_value::<Vec<MediaItemDto>>(r.media_urls).unwrap_or_default();
            let attachment = r
                .attachment_payload
                .and_then(|v| serde_json::from_value::<ThreadAttachmentPayload>(v).ok());
            let quoted_preview = r.quoted_thread_id.and_then(|qid| quoted_map.get(&qid).cloned());

            SocialThreadItemDto {
                id: r.id,
                author_id: r.author_id,
                author_username: r.author_username,
                author_display_name: r.author_display_name,
                author_avatar_url: r.author_avatar_url,
                author_tier: r.author_tier,
                author_barista_level: r.author_barista_level,
                author_barista_rank_title: r.author_barista_rank_title,
                kind: r.kind,
                parent_thread_id: r.parent_thread_id,
                root_thread_id: r.root_thread_id,
                quoted_thread_id: r.quoted_thread_id,
                quoted_preview,
                depth_level: r.depth_level,
                body_text: r.body_text,
                has_media: r.has_media,
                media_urls,
                attachment,
                likes_count: r.likes_count,
                replies_count: r.replies_count,
                reposts_count: r.reposts_count,
                try_recipe_count: r.try_recipe_count,
                is_pinned_on_profile: r.is_pinned_on_profile,
                is_liked_by_me: liked_set.contains(&r.id),
                is_reposted_by_me: reposted_set.contains(&r.id),
                created_at: r.created_at,
            }
        })
        .collect())
}

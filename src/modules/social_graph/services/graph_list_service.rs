use std::collections::HashMap;

use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::{
    social_feed::models::barista_rank_for_level,
    social_graph::{
        dto::{
            FollowRequestItemDto, PrivacySettingResponseDto, SocialGraphCursorPageDto,
            SocialGraphUserItemDto,
        },
        error::SocialGraphError,
        models::{FollowEdgeUserRow, GraphCursor, RelationshipState},
        repositories::{follow_cursor_repo, follow_edge_repo, follow_privacy_repo},
        services::graph_query_service::GraphQueryService,
    },
};

pub async fn list_connections(
    query_svc: &GraphQueryService,
    pool: &PgPool,
    viewer_id: Option<Uuid>,
    target: &str,
    kind: &str,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<SocialGraphCursorPageDto<SocialGraphUserItemDto>, SocialGraphError> {
    let target_row = follow_edge_repo::resolve_target_user(pool, target).await?;
    let parsed_cursor = cursor.as_deref().map(GraphCursor::decode).transpose()?;
    let clamped_limit = limit.unwrap_or(20).clamp(1, 50);

    if kind != "mutual" && target_row.is_private_account && viewer_id != Some(target_row.user_id) {
        let can_view = match viewer_id {
            Some(vid) => {
                follow_edge_repo::fetch_pair_edge_state(pool, vid, target_row.user_id)
                    .await?
                    .out_accepted
            }
            None => false,
        };
        if !can_view {
            return Err(SocialGraphError::Forbidden(
                "Daftar pengikut/diikuti akun privat hanya tersedia bagi pengikut yang disetujui."
                    .to_string(),
            ));
        }
    }

    let mut rows: Vec<FollowEdgeUserRow> = match kind {
        "followers" => {
            follow_cursor_repo::list_followers_keyset(pool, target_row.user_id, parsed_cursor, clamped_limit + 1).await?
        }
        "following" => {
            follow_cursor_repo::list_following_keyset(pool, target_row.user_id, parsed_cursor, clamped_limit + 1).await?
        }
        "mutual" => {
            let vid = viewer_id.ok_or_else(|| {
                SocialGraphError::Unauthorized("Masuk terlebih dahulu untuk melihat koneksi mutual.".to_string())
            })?;
            follow_cursor_repo::list_mutual_connections_keyset(pool, vid, target_row.user_id, parsed_cursor, clamped_limit + 1).await?
        }
        _ => return Err(SocialGraphError::BadRequest("Jenis tab grafik tidak valid.".to_string())),
    };

    let has_more = rows.len() as i64 > clamped_limit;
    if has_more {
        rows.truncate(clamped_limit as usize);
    }
    let next_cursor = rows
        .last()
        .filter(|_| has_more)
        .map(|r| GraphCursor { created_at: r.edge_created_at, user_id: r.user_id }.encode());

    let state_map: HashMap<Uuid, RelationshipState> = if let Some(vid) = viewer_id {
        let ids: Vec<Uuid> = rows.iter().map(|r| r.user_id).collect();
        query_svc
            .batch_check_relationships(pool, vid, ids)
            .await?
            .items
            .into_iter()
            .map(|b| (b.target_user_id, b.state))
            .collect()
    } else {
        HashMap::new()
    };

    let items = rows
        .into_iter()
        .map(|r| SocialGraphUserItemDto {
            user_id: r.user_id,
            username: r.username,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            barista_level: r.barista_level,
            barista_rank_title: barista_rank_for_level(r.barista_level).to_string(),
            account_tier: r.account_tier,
            is_private_account: r.is_private_account,
            relationship_state: state_map.get(&r.user_id).copied().unwrap_or(RelationshipState::None),
            followed_at: r.edge_created_at,
        })
        .collect();

    Ok(SocialGraphCursorPageDto { items, next_cursor, has_more })
}

pub async fn list_pending_requests(
    pool: &PgPool,
    owner_id: Uuid,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<SocialGraphCursorPageDto<FollowRequestItemDto>, SocialGraphError> {
    let parsed_cursor = cursor.as_deref().map(GraphCursor::decode).transpose()?;
    let clamped_limit = limit.unwrap_or(20).clamp(1, 50);
    let mut rows = follow_cursor_repo::list_pending_requests_keyset(
        pool,
        owner_id,
        parsed_cursor,
        clamped_limit + 1,
    )
    .await?;
    let has_more = rows.len() as i64 > clamped_limit;
    if has_more {
        rows.truncate(clamped_limit as usize);
    }
    let next_cursor = rows
        .last()
        .filter(|_| has_more)
        .map(|r| GraphCursor { created_at: r.edge_created_at, user_id: r.user_id }.encode());

    let items = rows
        .into_iter()
        .map(|r| FollowRequestItemDto {
            requester_id: r.user_id,
            username: r.username,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            barista_level: r.barista_level,
            barista_rank_title: barista_rank_for_level(r.barista_level).to_string(),
            account_tier: r.account_tier,
            requested_at: r.edge_created_at,
        })
        .collect();
    Ok(SocialGraphCursorPageDto { items, next_cursor, has_more })
}

pub async fn get_my_privacy_setting(
    pool: &PgPool,
    owner_id: Uuid,
) -> Result<PrivacySettingResponseDto, SocialGraphError> {
    let row = follow_privacy_repo::fetch_privacy_setting(pool, owner_id).await?;
    Ok(PrivacySettingResponseDto {
        is_private_account: row.is_private_account,
        pending_follow_requests_count: row.pending_follow_requests_count,
    })
}

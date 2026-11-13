use std::{collections::HashMap, sync::Arc, time::Duration};

use moka::future::Cache;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::{
    social_feed::models::barista_rank_for_level,
    social_graph::{
        dto::{
            BatchRelationshipItemDto, BatchRelationshipsResponseDto, MiniUserAvatarDto,
            RelationshipSnapshotDto,
        },
        error::SocialGraphError,
        models::RelationshipState,
        repositories::{follow_edge_repo, follow_list_repo},
        services::redis_graph_cache::RedisGraphCache,
    },
};

pub struct GraphQueryService {
    cache: Arc<RedisGraphCache>,
    mini_profile_cache: Cache<Uuid, MiniUserAvatarDto>,
}

impl GraphQueryService {
    pub fn new(cache: Arc<RedisGraphCache>) -> Self {
        let mini_profile_cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(300))
            .build();
        Self {
            cache,
            mini_profile_cache,
        }
    }

    async fn resolve_mini_avatars(
        &self,
        pool: &PgPool,
        ids: &[Uuid],
    ) -> Result<Vec<MiniUserAvatarDto>, SocialGraphError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut resolved = HashMap::with_capacity(ids.len());
        let mut missing = Vec::new();
        for &id in ids {
            if let Some(cached) = self.mini_profile_cache.get(&id).await {
                resolved.insert(id, cached);
            } else {
                missing.push(id);
            }
        }
        if !missing.is_empty() {
            let rows = follow_list_repo::fetch_mini_profiles_by_ids(pool, &missing).await?;
            for r in rows {
                let dto = MiniUserAvatarDto {
                    user_id: r.user_id,
                    username: r.username,
                    display_name: r.display_name,
                    avatar_url: r.avatar_url,
                    barista_rank_title: barista_rank_for_level(r.barista_level).to_string(),
                };
                self.mini_profile_cache.insert(r.user_id, dto.clone()).await;
                resolved.insert(r.user_id, dto);
            }
        }
        Ok(ids.iter().filter_map(|id| resolved.get(id).cloned()).collect())
    }

    pub async fn get_relationship_snapshot(
        &self,
        pool: &PgPool,
        viewer_id: Option<Uuid>,
        target: &str,
    ) -> Result<RelationshipSnapshotDto, SocialGraphError> {
        let target_row = follow_edge_repo::resolve_target_user(pool, target).await?;
        let Some(vid) = viewer_id else {
            return Ok(Self::empty_snapshot(&target_row, RelationshipState::None));
        };
        if vid == target_row.user_id {
            return Ok(Self::empty_snapshot(&target_row, RelationshipState::SelfProfile));
        }

        let snap = self.cache.get_snapshot_and_mutuals(pool, vid, target_row.user_id).await?;
        let state = RelationshipState::resolve(
            Some(vid),
            target_row.user_id,
            snap.pair.out_accepted,
            snap.pair.in_accepted,
            snap.pair.out_pending,
        );
        let mutual_avatars = self.resolve_mini_avatars(pool, &snap.mutual_sample_ids).await?;

        Ok(RelationshipSnapshotDto {
            target_user_id: target_row.user_id,
            state,
            is_mutual: matches!(state, RelationshipState::Mutual),
            is_followed_by_target: snap.pair.in_accepted,
            has_pending_incoming_request: snap.pair.in_pending,
            target_is_private: target_row.is_private_account,
            mutual_followers_count: snap.mutual_count,
            mutual_avatars,
            followers_count: target_row.followers_count as i64,
            following_count: target_row.following_count as i64,
        })
    }

    fn empty_snapshot(
        target_row: &crate::modules::social_graph::models::TargetUserRow,
        state: RelationshipState,
    ) -> RelationshipSnapshotDto {
        RelationshipSnapshotDto {
            target_user_id: target_row.user_id,
            state,
            is_mutual: false,
            is_followed_by_target: false,
            has_pending_incoming_request: false,
            target_is_private: target_row.is_private_account,
            mutual_followers_count: 0,
            mutual_avatars: Vec::new(),
            followers_count: target_row.followers_count as i64,
            following_count: target_row.following_count as i64,
        }
    }

    pub async fn batch_check_relationships(
        &self,
        pool: &PgPool,
        viewer_id: Uuid,
        target_ids: Vec<Uuid>,
    ) -> Result<BatchRelationshipsResponseDto, SocialGraphError> {
        if target_ids.len() > 50 {
            return Err(SocialGraphError::BadRequest(
                "Maksimal 50 target_ids dalam satu permintaan batch.".to_string(),
            ));
        }
        let mut unique = Vec::with_capacity(target_ids.len());
        for id in target_ids {
            if !unique.contains(&id) {
                unique.push(id);
            }
        }
        let vectors = self.cache.batch_membership(pool, viewer_id, &unique).await?;
        let items = vectors
            .into_iter()
            .map(|(tid, out_acc, in_acc, out_pen)| {
                let state = RelationshipState::resolve(Some(viewer_id), tid, out_acc, in_acc, out_pen);
                BatchRelationshipItemDto {
                    target_user_id: tid,
                    state,
                    is_mutual: matches!(state, RelationshipState::Mutual),
                    is_followed_by_target: in_acc,
                }
            })
            .collect();
        Ok(BatchRelationshipsResponseDto { items })
    }
}

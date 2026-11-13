use fred::{clients::RedisPool, interfaces::KeysInterface};
use moka::future::Cache;
use roaring::RoaringBitmap;
use sqlx::PgPool;
use std::{collections::HashSet, sync::Arc, time::Duration};
use uuid::Uuid;

use crate::modules::communities::{
    error::CommunitiesError,
    models::CommunityRow,
    repositories::community_query_repo,
};

#[derive(Debug, Clone)]
pub struct CachedClubMembers {
    pub exact_ids: HashSet<Uuid>,
    pub bitmap: RoaringBitmap,
}

impl CachedClubMembers {
    pub fn from_ids(ids: Vec<Uuid>) -> Self {
        let mut bitmap = RoaringBitmap::new();
        let mut exact_ids = HashSet::with_capacity(ids.len());
        for uid in ids {
            bitmap.insert(fnv1a_uuid_bucket(uid));
            exact_ids.insert(uid);
        }
        Self { exact_ids, bitmap }
    }

    pub fn contains_user(&self, user_id: Uuid) -> bool {
        let bucket = fnv1a_uuid_bucket(user_id);
        self.bitmap.contains(bucket) && self.exact_ids.contains(&user_id)
    }
}

pub fn fnv1a_uuid_bucket(uid: Uuid) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in uid.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

pub struct ClubCacheService {
    clubs: Cache<Uuid, CommunityRow>,
    members: Cache<Uuid, Arc<CachedClubMembers>>,
    redis: Option<RedisPool>,
}

impl ClubCacheService {
    pub fn new(redis: Option<RedisPool>) -> Self {
        let clubs = Cache::builder()
            .max_capacity(2_000)
            .time_to_live(Duration::from_secs(300))
            .build();
        let members = Cache::builder()
            .max_capacity(2_000)
            .time_to_live(Duration::from_secs(120))
            .build();
        Self {
            clubs,
            members,
            redis,
        }
    }

    pub async fn get_club(
        &self,
        pg: &PgPool,
        community_id: Uuid,
    ) -> Result<CommunityRow, CommunitiesError> {
        if let Some(hit) = self.clubs.get(&community_id).await {
            return Ok(hit);
        }
        let club = community_query_repo::fetch_community_by_id(pg, community_id)
            .await?
            .ok_or(CommunitiesError::ClubNotFound)?;
        self.clubs.insert(community_id, club.clone()).await;
        Ok(club)
    }

    pub async fn put_club(&self, club: CommunityRow) {
        self.clubs.insert(club.id, club).await;
    }

    pub async fn get_members(
        &self,
        pg: &PgPool,
        community_id: Uuid,
    ) -> Result<Arc<CachedClubMembers>, CommunitiesError> {
        if let Some(hit) = self.members.get(&community_id).await {
            return Ok(hit);
        }
        let ids = community_query_repo::fetch_club_member_ids(pg, community_id).await?;
        let arc = Arc::new(CachedClubMembers::from_ids(ids));
        self.members.insert(community_id, arc.clone()).await;
        Ok(arc)
    }

    pub async fn is_member(
        &self,
        pg: &PgPool,
        community_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, CommunitiesError> {
        let cached = self.get_members(pg, community_id).await?;
        if cached.contains_user(user_id) {
            return Ok(true);
        }
        // Fallback DB lookup jika baru saja join pada node lain
        Ok(community_query_repo::fetch_viewer_role(pg, community_id, user_id)
            .await?
            .is_some())
    }

    pub async fn invalidate_club(&self, community_id: Uuid) {
        self.clubs.invalidate(&community_id).await;
        self.members.invalidate(&community_id).await;
        if let Some(redis) = &self.redis {
            let key = format!("club:meta:{community_id}");
            let _: Result<i64, _> = redis.del(&key).await;
        }
    }
}

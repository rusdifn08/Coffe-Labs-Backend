use fred::{
    interfaces::LuaInterface,
    prelude::{Expiration, KeysInterface, RedisPool, RedisValue, SetsInterface},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::social_graph::{
    error::SocialGraphError,
    models::EdgePairState,
    repositories::{follow_edge_repo, follow_list_repo},
};

const CACHE_TTL_SECS: i64 = 6 * 3600;

const SNAPSHOT_AND_MUTUAL_LUA: &str = r#"
if redis.call('EXISTS', KEYS[4]) == 0 or redis.call('EXISTS', KEYS[5]) == 0 then
    return {'WARMUP_NEEDED'}
end
local out_acc = redis.call('SISMEMBER', KEYS[1], ARGV[2])
local in_acc = redis.call('SISMEMBER', KEYS[2], ARGV[1])
local out_pen = redis.call('SISMEMBER', KEYS[3], ARGV[2])
local inter = redis.call('SINTER', KEYS[1], KEYS[2])
local res = {'OK', tostring(out_acc), tostring(in_acc), tostring(out_pen), tostring(#inter)}
for i = 1, math.min(#inter, 3) do
    res[#res + 1] = tostring(inter[i])
end
return res
"#;

#[derive(Debug, Clone)]
pub struct SnapshotCacheHit {
    pub pair: EdgePairState,
    pub mutual_count: i64,
    pub mutual_sample_ids: Vec<Uuid>,
}

pub struct RedisGraphCache {
    redis: Option<RedisPool>,
}

impl RedisGraphCache {
    pub fn new(redis: Option<RedisPool>) -> Self { Self { redis } }
    pub fn redis_pool(&self) -> Option<&RedisPool> { self.redis.as_ref() }
    fn key_following(uid: Uuid) -> String { format!("graph:following:{uid}") }
    fn key_followers(uid: Uuid) -> String { format!("graph:followers:{uid}") }
    fn key_pending_out(uid: Uuid) -> String { format!("graph:pending_out:{uid}") }
    fn key_warm(uid: Uuid) -> String { format!("graph:warm:{uid}") }

    async fn warmup_user(&self, pool: &PgPool, redis: &RedisPool, user_id: Uuid) -> bool {
        let Ok((following, followers, pending_out)) =
            follow_list_repo::fetch_edge_sets_for_warmup(pool, user_id).await
        else {
            return false;
        };
        let (fk, rk, pk, wk) = (
            Self::key_following(user_id),
            Self::key_followers(user_id),
            Self::key_pending_out(user_id),
            Self::key_warm(user_id),
        );
        let pipe = redis.next().pipeline();
        if pipe.del::<RedisValue, _>(vec![fk.clone(), rk.clone(), pk.clone()]).await.is_err() {
            return false;
        }
        let mut ok = true;
        for (key, ids) in [(&fk, following), (&rk, followers), (&pk, pending_out)] {
            if !ids.is_empty() {
                let members: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
                ok &= pipe.sadd::<RedisValue, _, _>(key, members).await.is_ok();
                ok &= pipe.expire::<RedisValue, _>(key, CACHE_TTL_SECS).await.is_ok();
            }
        }
        ok &= pipe
            .set::<RedisValue, _, _>(&wk, "1", Some(Expiration::EX(CACHE_TTL_SECS)), None, false)
            .await
            .is_ok();
        ok && pipe.last::<RedisValue>().await.is_ok()
    }

    async fn run_lua_snapshot(&self, redis: &RedisPool, viewer_id: Uuid, target_id: Uuid) -> Option<Vec<String>> {
        let keys = vec![
            Self::key_following(viewer_id),
            Self::key_followers(target_id),
            Self::key_pending_out(viewer_id),
            Self::key_warm(viewer_id),
            Self::key_warm(target_id),
        ];
        let args = vec![viewer_id.to_string(), target_id.to_string()];
        redis.eval::<Vec<String>, _, _, _>(SNAPSHOT_AND_MUTUAL_LUA, keys, args).await.ok()
    }

    pub async fn get_snapshot_and_mutuals(
        &self,
        pool: &PgPool,
        viewer_id: Uuid,
        target_id: Uuid,
    ) -> Result<SnapshotCacheHit, SocialGraphError> {
        if let Some(redis) = &self.redis {
            if let Some(mut raw) = self.run_lua_snapshot(redis, viewer_id, target_id).await {
                if raw.first().map(|s| s.as_str()) == Some("WARMUP_NEEDED") {
                    if self.warmup_user(pool, redis, viewer_id).await && self.warmup_user(pool, redis, target_id).await {
                        raw = self.run_lua_snapshot(redis, viewer_id, target_id).await.unwrap_or_default();
                    }
                }
                if raw.first().map(|s| s.as_str()) == Some("OK") && raw.len() >= 5 {
                    let out_accepted = raw.get(1).map(|s| s == "1").unwrap_or(false);
                    let in_accepted = raw.get(2).map(|s| s == "1").unwrap_or(false);
                    let out_pending = raw.get(3).map(|s| s == "1").unwrap_or(false);
                    let mutual_count = raw.get(4).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                    let mutual_sample_ids = raw.iter().skip(5).filter_map(|s| Uuid::parse_str(s).ok()).collect();
                    let in_pending = redis
                        .sismember::<bool, _, _>(Self::key_pending_out(target_id), viewer_id.to_string())
                        .await
                        .unwrap_or(false);
                    return Ok(SnapshotCacheHit {
                        pair: EdgePairState { out_accepted, in_accepted, out_pending, in_pending },
                        mutual_count,
                        mutual_sample_ids,
                    });
                }
            }
        }
        let pair = follow_edge_repo::fetch_pair_edge_state(pool, viewer_id, target_id).await?;
        let (mutual_count, mutual_sample_ids) = follow_list_repo::fetch_mutual_stats_sql(pool, viewer_id, target_id).await?;
        Ok(SnapshotCacheHit { pair, mutual_count, mutual_sample_ids })
    }

    pub async fn batch_membership(
        &self,
        pool: &PgPool,
        viewer_id: Uuid,
        target_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, bool, bool, bool)>, SocialGraphError> {
        if target_ids.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(redis) = &self.redis {
            let warmed = match redis.exists::<bool, _>(Self::key_warm(viewer_id)).await {
                Ok(true) => true,
                _ => self.warmup_user(pool, redis, viewer_id).await,
            };
            if warmed {
                let members: Vec<String> = target_ids.iter().map(|id| id.to_string()).collect();
                let pipe = redis.next().pipeline();
                let q1 = pipe.smismember::<Vec<bool>, _, _>(Self::key_following(viewer_id), members.clone()).await.is_ok();
                let q2 = pipe.smismember::<Vec<bool>, _, _>(Self::key_followers(viewer_id), members.clone()).await.is_ok();
                let q3 = pipe.smismember::<Vec<bool>, _, _>(Self::key_pending_out(viewer_id), members).await.is_ok();
                if q1 && q2 && q3 {
                    let res = pipe.try_all::<Vec<bool>>().await;
                    if let [Ok(out_v), Ok(in_v), Ok(pen_v)] = &res[..] {
                        return Ok(target_ids
                            .iter()
                            .enumerate()
                            .map(|(i, tid)| (*tid, out_v.get(i).copied().unwrap_or(false), in_v.get(i).copied().unwrap_or(false), pen_v.get(i).copied().unwrap_or(false)))
                            .collect());
                    }
                }
            }
        }
        follow_list_repo::fetch_batch_edge_vectors_sql(pool, viewer_id, target_ids).await
    }

    pub async fn on_follow_created(&self, actor_id: Uuid, target_id: Uuid, is_pending: bool) {
        let Some(redis) = &self.redis else { return };
        if is_pending {
            self.add_if_warm(redis, actor_id, &Self::key_pending_out(actor_id), target_id).await;
        } else {
            self.add_if_warm(redis, actor_id, &Self::key_following(actor_id), target_id).await;
            self.add_if_warm(redis, target_id, &Self::key_followers(target_id), actor_id).await;
        }
    }

    pub async fn on_follow_removed(&self, actor_id: Uuid, target_id: Uuid) {
        let Some(redis) = &self.redis else { return };
        let _ = redis.srem::<i64, _, _>(Self::key_following(actor_id), target_id.to_string()).await;
        let _ = redis.srem::<i64, _, _>(Self::key_pending_out(actor_id), target_id.to_string()).await;
        let _ = redis.srem::<i64, _, _>(Self::key_followers(target_id), actor_id.to_string()).await;
    }

    pub async fn on_request_accepted(&self, owner_id: Uuid, requester_id: Uuid) {
        let Some(redis) = &self.redis else { return };
        let _ = redis.srem::<i64, _, _>(Self::key_pending_out(requester_id), owner_id.to_string()).await;
        self.add_if_warm(redis, requester_id, &Self::key_following(requester_id), owner_id).await;
        self.add_if_warm(redis, owner_id, &Self::key_followers(owner_id), requester_id).await;
    }

    async fn add_if_warm(&self, redis: &RedisPool, owner_id: Uuid, set_key: &str, member: Uuid) {
        if let Ok(true) = redis.exists::<bool, _>(Self::key_warm(owner_id)).await {
            let _ = redis.sadd::<i64, _, _>(set_key, member.to_string()).await;
            let _ = redis.expire::<bool, _>(set_key, CACHE_TTL_SECS).await;
        }
    }
}

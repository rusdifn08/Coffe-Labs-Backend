use std::collections::HashMap;

use fred::{
    interfaces::KeysInterface,
    prelude::{Expiration, RedisPool, RedisValue},
};
use h3o::{LatLng, Resolution};
use moka::future::Cache;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::beans_radar::{
        models::CoffeeRadarPlaceRow,
        repositories::radar_place_repo,
    },
};

const REDIS_PLACE_TTL_SECS: i64 = 1800;

/// Layanan Cache Geospasial Uber H3 + L1 Moka (`AppCaches.radar_place_l1`) + L2 Redis (`radar:place:{id}`).
pub struct H3CacheService;

impl H3CacheService {
    /// Menghitung indeks string heksagonal Uber H3 Resolusi 7 untuk kunci sel spasial.
    pub fn cell_res7_string(lat: f64, lng: f64) -> String {
        LatLng::new(lat, lng)
            .map(|c| c.to_cell(Resolution::Seven).to_string())
            .unwrap_or_else(|_| "0".to_string())
    }

    /// Menghidrasi batch ID kedai kopi dari L1 Moka -> L2 Redis -> NeonDB PostGIS secara berurutan.
    pub async fn hydrate_ordered_places(
        place_l1: &Cache<Uuid, CoffeeRadarPlaceRow>,
        redis: Option<RedisPool>,
        pool: &PgPool,
        ids: &[Uuid],
    ) -> AppResult<(Vec<CoffeeRadarPlaceRow>, &'static str)> {
        if ids.is_empty() {
            return Ok((Vec::new(), "EMPTY"));
        }

        let mut map: HashMap<Uuid, CoffeeRadarPlaceRow> = HashMap::with_capacity(ids.len());
        let mut missing_after_l1 = Vec::new();

        for id in ids {
            if let Some(row) = place_l1.get(id).await {
                map.insert(*id, row);
            } else {
                missing_after_l1.push(*id);
            }
        }

        if missing_after_l1.is_empty() {
            return Ok((Self::preserve_order(ids, &map), "L1_RTREE_MOKA"));
        }

        let mut missing_after_l2 = Vec::new();
        if let Some(ref redis_pool) = redis {
            for id in &missing_after_l1 {
                let key = format!("radar:place:{}", id);
                if let Ok(Some(json_str)) = redis_pool.get::<Option<String>, _>(&key).await {
                    if let Ok(row) = serde_json::from_str::<CoffeeRadarPlaceRow>(&json_str) {
                        place_l1.insert(*id, row.clone()).await;
                        map.insert(*id, row);
                        continue;
                    }
                }
                missing_after_l2.push(*id);
            }
        } else {
            missing_after_l2 = missing_after_l1;
        }

        if missing_after_l2.is_empty() {
            return Ok((Self::preserve_order(ids, &map), "L2_H3_REDIS"));
        }

        let db_rows = radar_place_repo::batch_hydrate_by_ids(pool, &missing_after_l2).await?;
        for row in db_rows {
            place_l1.insert(row.id, row.clone()).await;
            if let Some(ref redis_pool) = redis {
                if let Ok(serialized) = serde_json::to_string(&row) {
                    let key = format!("radar:place:{}", row.id);
                    let _: Result<RedisValue, _> = redis_pool
                        .set(
                            &key,
                            serialized,
                            Some(Expiration::EX(REDIS_PLACE_TTL_SECS)),
                            None,
                            false,
                        )
                        .await;
                }
            }
            map.insert(row.id, row);
        }

        Ok((Self::preserve_order(ids, &map), "L3_POSTGIS_HYBRID"))
    }

    fn preserve_order(
        ids: &[Uuid],
        map: &HashMap<Uuid, CoffeeRadarPlaceRow>,
    ) -> Vec<CoffeeRadarPlaceRow> {
        ids.iter().filter_map(|id| map.get(id).cloned()).collect()
    }
}

use moka::future::Cache;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use crate::modules::recommendation::scoring_math::{
    extract_16d_vector_from_text, neutral_taste_vector, update_ema_l2, vec_from_slice,
    TASTE_DIMENSION_LABELS, TASTE_DIM, TASTE_EMA_ALPHA,
};

pub struct TasteProfilerService {
    pg: PgPool,
    cache: Cache<Uuid, [f32; TASTE_DIM]>,
}

impl TasteProfilerService {
    pub fn new(pg: PgPool) -> Self {
        let cache = Cache::builder()
            .max_capacity(5_000)
            .time_to_live(Duration::from_secs(600))
            .build();
        Self { pg, cache }
    }

    pub async fn get_or_load_user_taste(&self, user_id: Uuid) -> [f32; TASTE_DIM] {
        if let Some(hit) = self.cache.get(&user_id).await {
            return hit;
        }
        let row: Option<Vec<f32>> = sqlx::query_scalar(
            "SELECT affinity_vector FROM user_taste_affinity_profiles WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pg)
        .await
        .ok()
        .flatten();

        let vec = match row {
            Some(raw) => vec_from_slice(&raw),
            None => neutral_taste_vector(),
        };
        self.cache.insert(user_id, vec).await;
        vec
    }

    pub async fn record_signal_text_best_effort(&self, user_id: Uuid, text: &str) {
        let sig = extract_16d_vector_from_text(text);
        self.record_signal_best_effort(user_id, &sig).await;
    }

    pub async fn record_signal_best_effort(&self, user_id: Uuid, signal: &[f32; TASTE_DIM]) {
        let current = self.get_or_load_user_taste(user_id).await;
        let next = update_ema_l2(&current, signal, TASTE_EMA_ALPHA);
        if next == current {
            return;
        }
        self.cache.insert(user_id, next).await;
        let as_vec: Vec<f32> = next.to_vec();
        let _ = sqlx::query(
            r#"INSERT INTO user_taste_affinity_profiles (user_id, affinity_vector, updated_at)
               VALUES ($1, $2, NOW())
               ON CONFLICT (user_id) DO UPDATE
               SET affinity_vector = EXCLUDED.affinity_vector,
                   updated_at = NOW()"#,
        )
        .bind(user_id)
        .bind(&as_vec)
        .execute(&self.pg)
        .await;
    }

    pub fn top_taste_labels(vec: &[f32; TASTE_DIM], limit: usize) -> Vec<String> {
        let mut pairs: Vec<(usize, f32)> = vec.iter().copied().enumerate().collect();
        pairs.sort_by(|a, b| b.1.total_cmp(&a.1));
        pairs
            .into_iter()
            .filter(|(_, score)| *score > 0.12)
            .take(limit)
            .filter_map(|(idx, _)| TASTE_DIMENSION_LABELS.get(idx).map(|s| (*s).to_string()))
            .collect()
    }
}

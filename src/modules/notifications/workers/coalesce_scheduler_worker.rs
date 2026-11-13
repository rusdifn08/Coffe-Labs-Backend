use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use fred::interfaces::{HashesInterface, KeysInterface, SortedSetsInterface, StreamsInterface};
use tracing::info;

use crate::modules::notifications::{
    events::{ActorSummary, OmnichannelNotifEvent},
    services::producer_service::{
        merge_coalesced_event, NOTIF_DELAYED_ZSET_KEY, NOTIF_STREAM_KEY,
    },
};

pub async fn run_coalesce_scheduler(client: fred::clients::RedisClient) {
    info!("Notification Sliding-Window Coalesce Scheduler (15s) aktif");
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let now_ms = Utc::now().timestamp_millis() as f64;

        let due_keys = match client
            .zrangebyscore::<Vec<String>, _, _, _>(
                NOTIF_DELAYED_ZSET_KEY,
                "-inf",
                now_ms,
                false,
                Some((0_i64, 50_i64)),
            )
            .await
        {
            Ok(keys) => keys,
            Err(_) => continue,
        };

        for coalesce_key in due_keys {
            let removed = client
                .zrem::<i64, _, _>(NOTIF_DELAYED_ZSET_KEY, &coalesce_key)
                .await
                .unwrap_or(0);
            if removed == 0 {
                continue;
            }

            let hash_key = format!("notif:coalesce:{coalesce_key}");
            let raw_fields = match client
                .hgetall::<HashMap<String, String>, _>(&hash_key)
                .await
            {
                Ok(map) => map,
                Err(_) => continue,
            };
            let _ = client.del::<i64, _>(&hash_key).await;

            let Some(meta_json) = raw_fields.get("__meta__") else {
                continue;
            };
            let Ok(template) = serde_json::from_str::<OmnichannelNotifEvent>(meta_json) else {
                continue;
            };

            let mut actors: Vec<ActorSummary> = raw_fields
                .iter()
                .filter(|(k, _)| k.as_str() != "__meta__")
                .filter_map(|(_, val)| serde_json::from_str::<ActorSummary>(val).ok())
                .collect();
            if actors.is_empty() {
                if let Some(first) = template.actor.clone() {
                    actors.push(first);
                }
            }

            let merged = merge_coalesced_event(template, &actors);
            if let Ok(payload) = serde_json::to_string(&merged) {
                let _ = client
                    .xadd::<String, _, _, _, _>(
                        NOTIF_STREAM_KEY,
                        false,
                        ("MAXLEN", "~", crate::core::redis::REDIS_STREAM_MAXLEN as i64),
                        "*",
                        ("payload", payload),
                    )
                    .await;
            }
        }
    }
}

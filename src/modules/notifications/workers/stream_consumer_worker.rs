use std::{collections::HashMap, sync::Arc, time::Duration};

use fred::interfaces::StreamsInterface;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::modules::notifications::{
    error::NotifError,
    events::OmnichannelNotifEvent,
    services::{
        backoff::{compute_full_jitter_delay_ms, R_MAX},
        omnichannel_router::OmnichannelRouter,
        producer_service::{NOTIF_DLQ_KEY, NOTIF_STREAM_KEY},
    },
};

const CONSUMER_GROUP: &str = "notif_workers";

pub async fn run_stream_consumer(
    redis: Option<fred::clients::RedisClient>,
    mut local_rx: mpsc::UnboundedReceiver<OmnichannelNotifEvent>,
    router: Arc<OmnichannelRouter>,
) {
    if let Some(client) = redis {
        let consumer_id = format!("worker-{}", uuid::Uuid::new_v4());
        let _ = client
            .xgroup_create::<String, _, _, _>(NOTIF_STREAM_KEY, CONSUMER_GROUP, "0", true)
            .await;

        let router_local = Arc::clone(&router);
        let client_local = client.clone();
        tokio::spawn(async move {
            while let Some(evt) = local_rx.recv().await {
                process_with_retry_or_dlq(evt, &router_local, Some(&client_local)).await;
            }
        });

        let mut reclaim_tick = 0_u64;
        info!(consumer_id = %consumer_id, "Notification Redis Stream consumer worker aktif");
        loop {
            match client
                .xreadgroup_map::<String, String, String, String, _, _, _, _>(
                    CONSUMER_GROUP,
                    &consumer_id,
                    Some(50),
                    Some(2_000),
                    false,
                    NOTIF_STREAM_KEY,
                    ">",
                )
                .await
            {
                Ok(map) => {
                    for (_stream, entries) in map {
                        for (msg_id, fields) in entries {
                            handle_stream_entry(&client, &router, &msg_id, &fields).await;
                        }
                    }
                }
                Err(err) => {
                    warn!(error = %err, "Redis XREADGROUP error, jeda 1s sebelum retry");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }

            reclaim_tick = reclaim_tick.wrapping_add(1);
            if reclaim_tick % 10 == 0 {
                reclaim_stale_entries(&client, &router, &consumer_id).await;
            }
        }
    } else {
        info!("Notification engine berjalan dengan local in-memory worker (REDIS_URL kosong)");
        while let Some(evt) = local_rx.recv().await {
            process_with_retry_or_dlq(evt, &router, None).await;
        }
    }
}

async fn handle_stream_entry(
    client: &fred::clients::RedisClient,
    router: &Arc<OmnichannelRouter>,
    msg_id: &str,
    fields: &HashMap<String, String>,
) {
    if let Some(raw_json) = fields.get("payload") {
        if let Ok(evt) = serde_json::from_str::<OmnichannelNotifEvent>(raw_json) {
            process_with_retry_or_dlq(evt, router, Some(client)).await;
        }
    }
    let _ = client
        .xack::<i64, _, _, _>(NOTIF_STREAM_KEY, CONSUMER_GROUP, msg_id)
        .await;
}

async fn reclaim_stale_entries(
    client: &fred::clients::RedisClient,
    router: &Arc<OmnichannelRouter>,
    consumer_id: &str,
) {
    if let Ok((_next, entries)) = client
        .xautoclaim_values::<String, String, String, _, _, _, _>(
            NOTIF_STREAM_KEY,
            CONSUMER_GROUP,
            consumer_id,
            30_000,
            "0-0",
            Some(25),
            false,
        )
        .await
    {
        for (msg_id, fields) in entries {
            handle_stream_entry(client, router, &msg_id, &fields).await;
        }
    }
}

async fn process_with_retry_or_dlq(
    mut evt: OmnichannelNotifEvent,
    router: &Arc<OmnichannelRouter>,
    redis: Option<&fred::clients::RedisClient>,
) {
    match router.route_event(&evt).await {
        Ok(()) => {}
        Err(NotifError::RetryableDelivery {
            retry_after_secs,
            reason,
        }) => {
            if evt.attempt >= R_MAX {
                error!(
                    event_id = %evt.event_id,
                    reason = %reason,
                    "Batas R_max=5 tercapai; memindahkan event notifikasi ke Dead Letter Queue"
                );
                if let (Some(client), Ok(payload)) = (redis, serde_json::to_string(&evt)) {
                    let _ = client
                        .xadd::<String, _, _, _, _>(
                            NOTIF_DLQ_KEY,
                            false,
                            ("MAXLEN", "~", crate::core::redis::REDIS_STREAM_MAXLEN as i64),
                            "*",
                            ("payload", payload),
                        )
                        .await;
                }
                return;
            }

            let delay_ms = compute_full_jitter_delay_ms(evt.attempt, retry_after_secs);
            evt.attempt += 1;
            let router_clone = Arc::clone(router);
            let redis_clone = redis.cloned();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                if let Some(client) = redis_clone {
                    if let Ok(payload) = serde_json::to_string(&evt) {
                        if client
                            .xadd::<String, _, _, _, _>(
                                NOTIF_STREAM_KEY,
                                false,
                                ("MAXLEN", "~", crate::core::redis::REDIS_STREAM_MAXLEN as i64),
                                "*",
                                ("payload", payload),
                            )
                            .await
                            .is_ok()
                        {
                            return;
                        }
                    }
                }
                let _ = router_clone.route_event(&evt).await;
            });
        }
        Err(err) => {
            warn!(event_id = %evt.event_id, error = %err, "Event notifikasi gagal diproses");
        }
    }
}

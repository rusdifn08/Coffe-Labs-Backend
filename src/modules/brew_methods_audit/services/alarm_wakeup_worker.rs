use std::{sync::Arc, time::Duration};

use chrono::Utc;
use fred::interfaces::SortedSetsInterface;
use fred::prelude::RedisPool;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::modules::{
    brew_methods_audit::{
        repositories::long_alarm_repo,
        services::alarm_scheduler_service::BREW_ALARM_ZSET_KEY,
    },
    notifications::{
        events::{NotificationCategory, OmnichannelNotifEvent},
        services::producer_service::NotificationProducerService,
    },
};

const POLL_INTERVAL_SECS: u64 = 15;

pub fn spawn_cold_brew_alarm_worker(
    pool: PgPool,
    redis_pool: Option<RedisPool>,
    notifications: Arc<NotificationProducerService>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(POLL_INTERVAL_SECS));
        loop {
            interval.tick().await;
            process_due_alarms(&pool, redis_pool.as_ref(), &notifications).await;
        }
    });
}

async fn process_due_alarms(
    pool: &PgPool,
    redis_pool: Option<&RedisPool>,
    notifications: &Arc<NotificationProducerService>,
) {
    match long_alarm_repo::claim_due_alarms(pool).await {
        Ok(due) if due.is_empty() => {}
        Ok(due) => {
            info!(count = due.len(), "Triggering due Cold Brew background alarms");
            for alarm in due {
                if let Some(rpool) = redis_pool {
                    let _ = rpool
                        .zrem::<i64, _, _>(BREW_ALARM_ZSET_KEY, alarm.id.to_string())
                        .await;
                }
                let ev = OmnichannelNotifEvent {
                    event_id: Uuid::new_v4(),
                    target_user_id: alarm.user_id,
                    category: NotificationCategory::Gamification,
                    event_type: "COLD_BREW_ALARM_RING".to_string(),
                    title: format!("Cold Brew Selesai: {}", alarm.recipe_title),
                    body: format!(
                        "Ekstraksi {:.0}g : {:.0}ml selesai! Segera saring konsentrat agar tidak pahit berlebih.",
                        alarm.coffee_dose_g, alarm.water_ml
                    ),
                    deep_link_path: format!("/brew/cold-brew-alarm/{}", alarm.id),
                    coalesce_key: None,
                    collapse_key: Some(format!("coldbrew_{}", alarm.id)),
                    priority_high: true,
                    ttl_secs: 7_200,
                    created_at: Utc::now(),
                    actor: None,
                    actors_preview: Vec::new(),
                    aggregated_count: 1,
                    entity_id: Some(alarm.id.to_string()),
                    conversation_id: None,
                    action_buttons: Vec::new(),
                    is_critical_transactional: true,
                    attempt: 0,
                    first_enqueued_at: Utc::now(),
                };
                notifications.publish_immediate(ev).await;
            }
        }
        Err(err) => {
            warn!(error = %err, "Gagal memeriksa antrean alarm Cold Brew di PostgreSQL");
        }
    }
}

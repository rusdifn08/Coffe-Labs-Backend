use chrono::{DateTime, Utc};
use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    Channel, ExchangeKind,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DEFAULT_EXCHANGE: &str = "coffee_labs.events";
pub const DEFAULT_QUEUE: &str = "coffee_labs.async_tasks";
pub const DEFAULT_ROUTING_KEY: &str = "events.#";
pub const CONSUMER_PREFETCH_COUNT: u16 = 5;
pub const RECONNECT_BASE_MS: u64 = 2_000;
pub const RECONNECT_MAX_MS: u64 = 60_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RabbitMqEventEnvelope {
    pub event_id: Uuid,
    pub event_type: String,
    pub source_module: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl RabbitMqEventEnvelope {
    pub fn new(
        event_type: impl Into<String>,
        source_module: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::now_v7(),
            event_type: event_type.into(),
            source_module: source_module.into(),
            payload,
            created_at: Utc::now(),
        }
    }
}

/// Mendeklarasikan Topic Exchange durable + Queue durable + Binding secara idempoten.
pub async fn declare_topology(channel: &Channel) -> Result<(), lapin::Error> {
    channel
        .exchange_declare(
            DEFAULT_EXCHANGE,
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_declare(
            DEFAULT_QUEUE,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            DEFAULT_QUEUE,
            DEFAULT_EXCHANGE,
            DEFAULT_ROUTING_KEY,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    Ok(())
}

/// Exponential backoff + full jitter untuk loop reconnect ke CloudAMQP.
pub fn compute_reconnect_backoff_ms(attempt: u32) -> u64 {
    let shift = attempt.min(5);
    let cap = RECONNECT_BASE_MS
        .saturating_mul(1_u64 << shift)
        .min(RECONNECT_MAX_MS);
    let mut rng = rand::thread_rng();
    rng.gen_range((RECONNECT_BASE_MS / 2)..=cap)
}

/// Menyembunyikan username & password dari AMQP URL sebelum dicatat pada tracing log.
pub fn sanitize_amqp_url(raw: &str) -> String {
    let trimmed = raw.trim();
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return "[REDACTED_AMQP_URL]".to_string();
    };
    if let Some((_, host_and_vhost)) = rest.rsplit_once('@') {
        format!("{scheme}://***:***@{host_and_vhost}")
    } else {
        format!("{scheme}://{rest}")
    }
}

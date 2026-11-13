use coffe_labs_backend::core::rabbitmq::{
    topology::{
        compute_reconnect_backoff_ms, sanitize_amqp_url, DEFAULT_ROUTING_KEY, RECONNECT_BASE_MS,
        RECONNECT_MAX_MS,
    },
    RabbitMqClient, RabbitMqEventEnvelope, CONSUMER_PREFETCH_COUNT, DEFAULT_EXCHANGE,
    DEFAULT_QUEUE,
};
use secrecy::SecretString;
use serde_json::json;

#[test]
fn rabbitmq_topology_constants_match_render_and_cloudamqp_limits() {
    assert_eq!(DEFAULT_EXCHANGE, "coffee_labs.events");
    assert_eq!(DEFAULT_QUEUE, "coffee_labs.async_tasks");
    assert_eq!(DEFAULT_ROUTING_KEY, "events.#");
    assert_eq!(CONSUMER_PREFETCH_COUNT, 5);
}

#[test]
fn sanitize_amqp_url_masks_credentials_without_panic() {
    let raw = "amqps://lemur_user:s3cr3tP@ss@armadillo-01.rmq.cloudamqp.com/lemur_vhost";
    assert_eq!(
        sanitize_amqp_url(raw),
        "amqps://***:***@armadillo-01.rmq.cloudamqp.com/lemur_vhost"
    );
    assert_eq!(sanitize_amqp_url("invalid-without-scheme"), "[REDACTED_AMQP_URL]");
    assert_eq!(
        sanitize_amqp_url("amqps://localhost/vhost"),
        "amqps://localhost/vhost"
    );
}

#[test]
fn reconnect_backoff_stays_bounded_with_jitter() {
    for attempt in 0..=12 {
        let delay_ms = compute_reconnect_backoff_ms(attempt);
        assert!(
            delay_ms >= RECONNECT_BASE_MS / 2,
            "delay {delay_ms} below floor at attempt {attempt}"
        );
        assert!(
            delay_ms <= RECONNECT_MAX_MS,
            "delay {delay_ms} exceeded cap at attempt {attempt}"
        );
    }
}

#[test]
fn rabbitmq_event_envelope_roundtrips_json_cleanly() {
    let envelope = RabbitMqEventEnvelope::new(
        "BREW_SESSION_COMPLETED",
        "gamification",
        json!({ "xp_gained": 25, "current_streak": 4 }),
    );
    let encoded = serde_json::to_vec(&envelope).expect("envelope serializes");
    let decoded: RabbitMqEventEnvelope =
        serde_json::from_slice(&encoded).expect("envelope deserializes");

    assert_eq!(decoded.event_id, envelope.event_id);
    assert_eq!(decoded.event_type, "BREW_SESSION_COMPLETED");
    assert_eq!(decoded.source_module, "gamification");
    assert_eq!(decoded.payload["xp_gained"], 25);
}

#[tokio::test]
async fn rabbitmq_standby_client_handles_unset_url_without_panic() {
    let disabled = RabbitMqClient::disabled();
    assert!(!disabled.is_configured());
    assert!(!disabled.is_connected());

    let standby = RabbitMqClient::connect_or_lazy(None).await;
    assert!(!standby.is_configured());
    assert!(!standby.is_connected());

    let blank = RabbitMqClient::connect_or_lazy(Some(SecretString::from("   ".to_string()))).await;
    assert!(!blank.is_configured());
    assert!(!blank.is_connected());

    let envelope = RabbitMqEventEnvelope::new("PING", "tests", json!({ "ok": true }));
    standby.publish_json(DEFAULT_EXCHANGE, "events.test", &envelope);
    let confirmed_res = standby
        .publish_json_confirmed(DEFAULT_EXCHANGE, "events.test", &envelope)
        .await;
    assert!(confirmed_res.is_err());
}

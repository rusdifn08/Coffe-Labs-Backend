use std::time::Duration;

use futures_lite::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicQosOptions},
    types::FieldTable,
};
use tracing::{error, info, warn};

use crate::{
    core::{
        rabbitmq::{
            client::RabbitMqClient,
            topology::{
                compute_reconnect_backoff_ms, declare_topology, RabbitMqEventEnvelope,
                CONSUMER_PREFETCH_COUNT, DEFAULT_QUEUE,
            },
        },
        AppState,
    },
    modules::notifications::events::OmnichannelNotifEvent,
};

const CONSUMER_TAG: &str = "coffe-labs-render-worker";

/// Menjalankan consumer worker RabbitMQ di dalam proses Axum (`tokio::spawn`)
/// dengan batas `basic_qos` prefetch=5 dan auto-reconnect exponential backoff.
pub fn spawn_rabbitmq_worker(rabbitmq: RabbitMqClient, state: AppState) {
    tokio::spawn(async move {
        let mut backoff_attempt: u32 = 0;

        loop {
            if !rabbitmq.is_configured() {
                let delay = compute_reconnect_backoff_ms(backoff_attempt.max(4));
                if backoff_attempt == 0 {
                    warn!(
                        delay_ms = delay,
                        "🐇 Consumer RabbitMQ siaga: RABBITMQ_URL belum disetel; mode standby aktif."
                    );
                }
                backoff_attempt = backoff_attempt.saturating_add(1).min(5);
                tokio::time::sleep(Duration::from_millis(delay)).await;
                continue;
            }

            match consume_session(&rabbitmq, &state).await {
                Ok(()) => {
                    backoff_attempt = 0;
                    warn!("🐇 Stream consumer RabbitMQ tertutup; melakukan reconnect otomatis...");
                }
                Err(err) => {
                    let delay_ms = compute_reconnect_backoff_ms(backoff_attempt);
                    backoff_attempt = backoff_attempt.saturating_add(1).min(6);
                    warn!(
                        error = %err,
                        backoff_ms = delay_ms,
                        "🐇 Sesi RabbitMQ terputus; auto-reconnect setelah jeda backoff"
                    );
                    rabbitmq.mark_disconnected().await;
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    });
}

async fn consume_session(rabbitmq: &RabbitMqClient, state: &AppState) -> Result<(), String> {
    let conn = rabbitmq.ensure_shared_connection().await?;
    let channel = conn
        .create_channel()
        .await
        .map_err(|e| format!("Gagal membuka channel consumer: {e}"))?;

    declare_topology(&channel)
        .await
        .map_err(|e| format!("Gagal sinkronisasi topologi consumer: {e}"))?;

    channel
        .basic_qos(CONSUMER_PREFETCH_COUNT, BasicQosOptions::default())
        .await
        .map_err(|e| format!("Gagal set basic_qos prefetch={CONSUMER_PREFETCH_COUNT}: {e}"))?;

    let mut consumer = channel
        .basic_consume(
            DEFAULT_QUEUE,
            CONSUMER_TAG,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .map_err(|e| format!("Gagal memulai basic_consume: {e}"))?;

    info!(
        queue = DEFAULT_QUEUE,
        prefetch = CONSUMER_PREFETCH_COUNT,
        "🐇 Background RabbitMQ Consumer Worker aktif"
    );

    while let Some(delivery_res) = consumer.next().await {
        match delivery_res {
            Ok(delivery) => {
                match process_delivery(state, &delivery.data).await {
                    Ok(()) => {
                        if let Err(err) = delivery.ack(BasicAckOptions::default()).await {
                            warn!(error = %err, "Gagal mengirim basic_ack RabbitMQ");
                        }
                    }
                    Err(reason) => {
                        error!(
                            routing_key = %delivery.routing_key,
                            error = %reason,
                            "Payload RabbitMQ tidak valid / gagal diproses; mengirim basic_nack (requeue=false)"
                        );
                        if let Err(err) = delivery
                            .nack(BasicNackOptions {
                                multiple: false,
                                requeue: false,
                            })
                            .await
                        {
                            warn!(error = %err, "Gagal mengirim basic_nack RabbitMQ");
                        }
                    }
                }
            }
            Err(err) => {
                return Err(format!("Kesalahan stream delivery RabbitMQ: {err}"));
            }
        }
    }

    Ok(())
}

async fn process_delivery(state: &AppState, raw_bytes: &[u8]) -> Result<(), String> {
    let envelope = serde_json::from_slice::<RabbitMqEventEnvelope>(raw_bytes)
        .map_err(|e| format!("JSON envelope invalid: {e}"))?;

    info!(
        event_id = %envelope.event_id,
        event_type = %envelope.event_type,
        source_module = %envelope.source_module,
        "📥 Memproses event domain dari queue RabbitMQ"
    );

    if envelope.event_type == "NOTIFICATION_DISPATCH" {
        let notif = serde_json::from_value::<OmnichannelNotifEvent>(envelope.payload)
            .map_err(|e| format!("Payload OmnichannelNotifEvent invalid: {e}"))?;
        state.notifications.publish(notif);
    }

    Ok(())
}

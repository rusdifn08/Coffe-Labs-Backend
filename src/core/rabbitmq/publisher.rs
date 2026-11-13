use lapin::{options::BasicPublishOptions, BasicProperties, Channel};
use serde::Serialize;
use tracing::{debug, warn};

use crate::core::rabbitmq::{topology::DEFAULT_EXCHANGE, RabbitMqClient};

impl RabbitMqClient {
    async fn get_or_create_pub_channel(&self) -> Result<Channel, String> {
        {
            let guard = self.pub_channel.lock().await;
            if let Some(ch) = guard.as_ref() {
                if ch.status().connected() {
                    return Ok(ch.clone());
                }
            }
        }

        let conn = self.ensure_shared_connection().await?;
        let new_ch = conn
            .create_channel()
            .await
            .map_err(|e| format!("Gagal membuat publisher channel: {e}"))?;
        *self.pub_channel.lock().await = Some(new_ch.clone());
        Ok(new_ch)
    }

    /// Helper Publisher non-blocking yang dapat dipanggil langsung dari `src/modules/`
    /// tanpa membebani latensi HTTP request.
    pub fn publish_json<T: Serialize>(&self, exchange: &str, routing_key: &str, payload: &T) {
        if !self.is_configured() {
            return;
        }
        let body = match serde_json::to_vec(payload) {
            Ok(bytes) => bytes,
            Err(err) => {
                warn!(error = %err, routing_key = %routing_key, "Gagal serialisasi JSON untuk RabbitMQ");
                return;
            }
        };

        let this = self.clone();
        let exch = if exchange.trim().is_empty() {
            DEFAULT_EXCHANGE.to_string()
        } else {
            exchange.to_string()
        };
        let rkey = routing_key.to_string();

        tokio::spawn(async move {
            if let Err(err) = this.publish_raw_bytes(&exch, &rkey, body).await {
                warn!(
                    exchange = %exch,
                    routing_key = %rkey,
                    error = %err,
                    "Publish payload RabbitMQ gagal (non-blocking)"
                );
            }
        });
    }

    /// Varian async terkonfirmasi untuk kebutuhan publish dengan verifikasi broker confirm.
    #[allow(dead_code)]
    pub async fn publish_json_confirmed<T: Serialize + Sync>(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: &T,
    ) -> Result<(), String> {
        let body = serde_json::to_vec(payload)
            .map_err(|e| format!("Gagal serialisasi JSON RabbitMQ: {e}"))?;
        let exch = if exchange.trim().is_empty() {
            DEFAULT_EXCHANGE
        } else {
            exchange
        };
        self.publish_raw_bytes(exch, routing_key, body).await
    }

    async fn publish_raw_bytes(
        &self,
        exchange: &str,
        routing_key: &str,
        body: Vec<u8>,
    ) -> Result<(), String> {
        let channel = self.get_or_create_pub_channel().await?;
        let props = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_delivery_mode(2);

        let confirm = match channel
            .basic_publish(
                exchange,
                routing_key,
                BasicPublishOptions::default(),
                &body,
                props,
            )
            .await
        {
            Ok(pending) => pending,
            Err(err) => {
                self.mark_disconnected().await;
                return Err(format!("basic_publish gagal: {err}"));
            }
        };

        confirm
            .await
            .map(|_| {
                debug!(exchange = %exchange, routing_key = %routing_key, "Payload JSON terkirim ke RabbitMQ");
            })
            .map_err(|e| format!("Publisher confirm RabbitMQ gagal: {e}"))
    }
}

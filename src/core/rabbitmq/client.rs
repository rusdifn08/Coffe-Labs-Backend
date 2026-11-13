use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use lapin::{Channel, Connection, ConnectionProperties};
use secrecy::{ExposeSecret, SecretString};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::core::rabbitmq::topology::{declare_topology, sanitize_amqp_url};

const STARTUP_CONNECT_TIMEOUT_SECS: u64 = 6;

/// Klien RabbitMQ hemat koneksi (maks 1 koneksi TCP/TLS aktif yang di-share di AppState)
/// untuk mematuhi batas 20 koneksi CloudAMQP Little Lemur Free Plan.
#[derive(Clone)]
pub struct RabbitMqClient {
    url: Option<SecretString>,
    connection: Arc<Mutex<Option<Arc<Connection>>>>,
    pub(super) pub_channel: Arc<Mutex<Option<Channel>>>,
    connected: Arc<AtomicBool>,
}

impl RabbitMqClient {
    #[allow(dead_code)]
    pub fn disabled() -> Self {
        Self {
            url: None,
            connection: Arc::new(Mutex::new(None)),
            pub_channel: Arc::new(Mutex::new(None)),
            connected: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn connect_or_lazy(url: Option<SecretString>) -> Self {
        let client = Self {
            url: url.filter(|s| !s.expose_secret().trim().is_empty()),
            connection: Arc::new(Mutex::new(None)),
            pub_channel: Arc::new(Mutex::new(None)),
            connected: Arc::new(AtomicBool::new(false)),
        };

        let Some(secret) = &client.url else {
            info!("🐇 RABBITMQ_URL belum disetel; Publisher/Consumer RabbitMQ berjalan pada mode siaga non-blocking.");
            return client;
        };

        let safe_url = sanitize_amqp_url(secret.expose_secret());
        match tokio::time::timeout(
            Duration::from_secs(STARTUP_CONNECT_TIMEOUT_SECS),
            client.ensure_shared_connection(),
        )
        .await
        {
            Ok(Ok(_)) => {
                info!(broker = %safe_url, "🐇 Terhubung ke CloudAMQP RabbitMQ (TLS rustls, 1 shared connection).");
            }
            Ok(Err(err)) => {
                warn!(broker = %safe_url, error = %err, "⚠️ Gagal inisialisasi koneksi RabbitMQ saat startup; background worker akan melakukan auto-reconnect.");
            }
            Err(_) => {
                warn!(broker = %safe_url, "⚠️ Timeout koneksi RabbitMQ saat startup; server Axum tetap menyala & auto-reconnect aktif.");
            }
        }

        client
    }

    pub fn is_configured(&self) -> bool {
        self.url.is_some()
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub async fn mark_disconnected(&self) {
        self.connected.store(false, Ordering::Relaxed);
        *self.pub_channel.lock().await = None;
        *self.connection.lock().await = None;
    }

    /// Membuka atau menggunakan kembali tepat 1 koneksi AMQP bersama (max 1 TCP connection per instance).
    pub async fn ensure_shared_connection(&self) -> Result<Arc<Connection>, String> {
        let Some(secret) = &self.url else {
            return Err("RABBITMQ_URL belum dikonfigurasi".to_string());
        };

        let mut guard = self.connection.lock().await;
        if let Some(existing) = guard.as_ref() {
            if existing.status().connected() {
                self.connected.store(true, Ordering::Relaxed);
                return Ok(Arc::clone(existing));
            }
        }

        let raw_url = secret.expose_secret().trim();
        let conn = Connection::connect(raw_url, ConnectionProperties::default())
            .await
            .map_err(|e| format!("Koneksi CloudAMQP gagal: {e}"))?;

        let channel = conn
            .create_channel()
            .await
            .map_err(|e| format!("Gagal membuka channel deklarasi AMQP: {e}"))?;
        declare_topology(&channel)
            .await
            .map_err(|e| format!("Gagal deklarasi exchange/queue RabbitMQ: {e}"))?;

        let arc_conn = Arc::new(conn);
        *self.pub_channel.lock().await = Some(channel);
        *guard = Some(Arc::clone(&arc_conn));
        self.connected.store(true, Ordering::Relaxed);
        Ok(arc_conn)
    }
}

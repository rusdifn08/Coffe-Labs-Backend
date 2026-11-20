use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use rustls_pki_types::ServerName;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio_rustls::{rustls::ClientConfig, TlsConnector};
use tracing::{info, warn};

use crate::core::AppState;
use crate::modules::qris_billing::config::{LivinImapConfig, QrisBillingConfig};
use crate::modules::qris_billing::services::bridge_security_guard::parse_indonesian_rupiah_amount;
use crate::modules::qris_billing::services::reconcile_upgrade_service::{
    reconcile_incoming_payment, IncomingMutationInput,
};

/// Memverifikasi keaslian header email resmi Bank Mandiri (`dkim=pass`, `spf=pass`, & `@bankmandiri.co.id`).
pub fn is_verified_mandiri_email(raw_email: &str) -> bool {
    let lower = raw_email.to_ascii_lowercase();
    let has_mandiri_sender =
        lower.contains("@bankmandiri.co.id") || lower.contains("livin' merchant");
    let has_dkim_pass = lower.contains("dkim=pass");
    let has_spf_pass = lower.contains("spf=pass");
    has_mandiri_sender && has_dkim_pass && has_spf_pass
}

/// Menjalankan Layer 2 Failover Worker 24/7 (IMAP TLS Watcher untuk email notifikasi Livin' Merchant).
pub fn spawn_email_imap_watcher(state: AppState) {
    let cfg = QrisBillingConfig::from_app_config(&state.config);
    let Some(imap_cfg) = cfg.imap else {
        info!("📧 QRIS Layer-2 Email IMAP Watcher standby (set LIVIN_IMAP_HOST/USERNAME/PASSWORD untuk aktivasi 24/7).");
        return;
    };

    tokio::spawn(async move {
        info!(host = %imap_cfg.host, port = imap_cfg.port, "📧 Memulai QRIS Layer-2 Email IMAP TLS Watcher (Livin' Merchant)...");
        let mut backoff_secs = 5u64;
        loop {
            match run_imap_poll_cycle(&state, &imap_cfg).await {
                Ok(_) => {
                    backoff_secs = 5;
                    tokio::time::sleep(Duration::from_secs(12)).await;
                }
                Err(err) => {
                    warn!(error = %err, retry_secs = backoff_secs, "Koneksi IMAP Livin' Merchant terputus; mencoba ulang");
                    tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(120);
                }
            }
        }
    });
}

async fn run_imap_poll_cycle(state: &AppState, cfg: &LivinImapConfig) -> Result<(), String> {
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls_cfg = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(tls_cfg));

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let tcp = tokio::time::timeout(Duration::from_secs(15), TcpStream::connect(&addr))
        .await
        .map_err(|_| "Timeout koneksi TCP IMAP".to_string())?
        .map_err(|e| format!("TCP IMAP gagal: {e}"))?;

    let server_name = ServerName::try_from(cfg.host.clone())
        .map_err(|_| "Nama host TLS IMAP tidak valid".to_string())?;
    let tls_stream = connector
        .connect(server_name, tcp)
        .await
        .map_err(|e| format!("Handshake TLS IMAP gagal: {e}"))?;

    let mut reader = BufReader::new(tls_stream);
    let mut line = String::new();
    let _ = reader.read_line(&mut line).await;

    let user_esc = cfg.username.replace('"', "");
    let pass_esc = cfg.password.replace('"', "");
    send_and_expect_ok(&mut reader, "A1", &format!("LOGIN \"{user_esc}\" \"{pass_esc}\"")).await?;
    send_and_expect_ok(&mut reader, "A2", &format!("SELECT \"{}\"", cfg.mailbox)).await?;

    let search_lines = send_and_collect(&mut reader, "A3", "SEARCH UNSEEN FROM \"bankmandiri.co.id\"").await?;
    let uids = extract_search_ids(&search_lines);

    for uid in uids.into_iter().take(10) {
        let fetch_lines = send_and_collect(&mut reader, "A4", &format!("FETCH {uid} (BODY.PEEK[])")).await?;
        let raw_msg = fetch_lines.join("\n");
        if is_verified_mandiri_email(&raw_msg) {
            if let Some(amount_idr) = parse_indonesian_rupiah_amount(&raw_msg) {
                let _ = reconcile_incoming_payment(
                    state,
                    IncomingMutationInput {
                        source_channel: "LIVIN_EMAIL_IMAP".to_string(),
                        amount_idr,
                        payer_name: None,
                        bank_rrn_ref: Some(format!("IMAP-{uid}")),
                        raw_notification_text: raw_msg.chars().take(600).collect(),
                        received_at: Utc::now(),
                    },
                )
                .await;
                let _ = send_and_expect_ok(&mut reader, "A5", &format!("STORE {uid} +FLAGS (\\Seen)")).await;
            }
        }
    }

    let _ = send_and_collect(&mut reader, "A6", "LOGOUT").await;
    Ok(())
}

async fn send_and_expect_ok<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    reader: &mut BufReader<S>,
    tag: &str,
    cmd: &str,
) -> Result<(), String> {
    let lines = send_and_collect(reader, tag, cmd).await?;
    let ok_prefix = format!("{tag} OK");
    if lines.iter().any(|l| l.starts_with(&ok_prefix)) {
        Ok(())
    } else {
        Err(format!("Perintah IMAP {tag} ditolak server"))
    }
}

async fn send_and_collect<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    reader: &mut BufReader<S>,
    tag: &str,
    cmd: &str,
) -> Result<Vec<String>, String> {
    let wire = format!("{tag} {cmd}\r\n");
    reader
        .get_mut()
        .write_all(wire.as_bytes())
        .await
        .map_err(|e| format!("Gagal kirim IMAP {tag}: {e}"))?;

    let mut out = Vec::new();
    let tag_prefix = format!("{tag} ");
    for _ in 0..400 {
        let mut buf = String::new();
        let n = tokio::time::timeout(Duration::from_secs(12), reader.read_line(&mut buf))
            .await
            .map_err(|_| format!("Timeout baca balasan IMAP {tag}"))?
            .map_err(|e| format!("Gagal baca IMAP {tag}: {e}"))?;
        if n == 0 {
            break;
        }
        let done = buf.starts_with(&tag_prefix);
        out.push(buf);
        if done {
            break;
        }
    }
    Ok(out)
}

fn extract_search_ids(lines: &[String]) -> Vec<u64> {
    let mut ids = Vec::new();
    for l in lines {
        if let Some(rest) = l.strip_prefix("* SEARCH") {
            for tok in rest.split_whitespace() {
                if let Ok(id) = tok.parse::<u64>() {
                    ids.push(id);
                }
            }
        }
    }
    ids
}

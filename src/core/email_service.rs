use serde_json::json;
use std::{sync::OnceLock, time::Duration};
use tracing::{error, info, warn};

use crate::core::error::{AppError, AppResult};

static EMAIL_HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    EMAIL_HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn normalize_email(raw: &str) -> Option<String> {
    let trimmed = raw.trim().to_lowercase();
    if trimmed.is_empty() || trimmed.len() > 160 || trimmed.chars().any(char::is_whitespace) {
        return None;
    }
    let mut parts = trimmed.split('@');
    let (local, domain) = (parts.next()?, parts.next()?);
    if parts.next().is_some() || local.is_empty() || local.len() > 64 || domain.len() < 4 {
        return None;
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..")
        || domain.starts_with('.') || domain.ends_with('.') || domain.contains("..")
    {
        return None;
    }
    let tld = &domain[domain.rfind('.')? + 1..];
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let ok_local = local.chars().all(|c| c.is_ascii_alphanumeric() || "._+-".contains(c));
    let ok_domain = domain.chars().all(|c| c.is_ascii_alphanumeric() || ".-".contains(c));
    if ok_local && ok_domain { Some(trimmed) } else { None }
}

pub fn mask_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return "***".to_string();
    };
    let chars: Vec<char> = local.chars().collect();
    let masked_local = match chars.len() {
        0 => "***".to_string(),
        1 | 2 => format!("{}***", chars[0]),
        _ => format!("{}***{}", chars[0], chars[chars.len() - 1]),
    };
    format!("{}@{}", masked_local, domain)
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

async fn send_transactional_email(
    to_email: &str,
    to_name: &str,
    subject: &str,
    html_body: &str,
    text_body: &str,
) -> AppResult<()> {
    if let Some(resend_key) = non_empty_env("RESEND_API_KEY") {
        let from = non_empty_env("EMAIL_FROM").unwrap_or_else(|| "Coffee Labs <onboarding@resend.dev>".into());
        let res = http_client()
            .post("https://api.resend.com/emails")
            .bearer_auth(resend_key)
            .json(&json!({ "from": from, "to": [to_email], "subject": subject, "html": html_body, "text": text_body }))
            .send()
            .await
            .map_err(|e| {
                error!("Resend HTTP network error: {}", e);
                AppError::ServiceUnavailable("Gagal menghubungi server email. Silakan coba lagi.".into())
            })?;
        if res.status().is_success() {
            info!(provider = "resend", recipient = %mask_email(to_email), "Email terkirim");
            return Ok(());
        }
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        error!(provider = "resend", status = %status, body = %body, "Resend API rejected email");
        return Err(AppError::ServiceUnavailable(
            "Layanan email sedang menolak pengiriman. Pastikan EMAIL_FROM & RESEND_API_KEY valid.".into(),
        ));
    }

    if let Some(brevo_key) = non_empty_env("BREVO_API_KEY") {
        let from_email = non_empty_env("EMAIL_FROM").unwrap_or_else(|| "no-reply@coffeelabs.app".into());
        let from_name = non_empty_env("EMAIL_FROM_NAME").unwrap_or_else(|| "Coffee Labs".into());
        let res = http_client()
            .post("https://api.brevo.com/v3/smtp/email")
            .header("api-key", brevo_key)
            .json(&json!({
                "sender": { "name": from_name, "email": from_email },
                "to": [{ "email": to_email, "name": to_name }],
                "subject": subject,
                "htmlContent": html_body,
                "textContent": text_body
            }))
            .send()
            .await
            .map_err(|e| {
                error!("Brevo HTTP network error: {}", e);
                AppError::ServiceUnavailable("Gagal menghubungi server email. Silakan coba lagi.".into())
            })?;
        if res.status().is_success() {
            info!(provider = "brevo", recipient = %mask_email(to_email), "Email terkirim");
            return Ok(());
        }
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        error!(provider = "brevo", status = %status, body = %body, "Brevo API rejected email");
        return Err(AppError::ServiceUnavailable(
            "Layanan email Brevo menolak pengiriman. Periksa BREVO_API_KEY dan pengirim terverifikasi.".into(),
        ));
    }

    warn!("Email reset password diminta tetapi RESEND_API_KEY / BREVO_API_KEY belum diset.");
    Err(AppError::ServiceUnavailable(
        "Layanan pengiriman email reset password belum diaktifkan di server. Hubungi Admin Coffee Labs untuk menambahkan RESEND_API_KEY atau BREVO_API_KEY.".into(),
    ))
}

pub async fn send_password_reset_code_email(
    to_email: &str,
    full_name: &str,
    username: &str,
    code: &str,
    expires_minutes: i64,
) -> AppResult<()> {
    let safe_name = escape_html(full_name);
    let safe_user = escape_html(username);
    let subject = format!("[Coffee Labs] Kode Reset Password Anda: {}", code);
    let html = format!(
        r#"<div style="font-family:Arial,sans-serif;max-width:520px;margin:auto;padding:24px;background:#FAF6F1;border-radius:16px;color:#2C1E16">
        <h2 style="margin:0 0 8px;color:#5C3A21">Coffee Labs &bull; Reset Password</h2>
        <p style="font-size:14px;line-height:1.6">Halo <b>{safe_name}</b> (@{safe_user}),</p>
        <p style="font-size:14px;line-height:1.6">Masukkan kode verifikasi 6 digit berikut di aplikasi Coffee Labs untuk mengatur ulang password Anda:</p>
        <div style="margin:20px 0;padding:16px;background:#FFFFFF;border:2px dashed #8B5A2B;border-radius:12px;text-align:center;font-size:32px;font-weight:800;letter-spacing:8px;color:#3E2723">{code}</div>
        <p style="font-size:13px;color:#6D4C41">Berlaku <b>{expires_minutes} menit</b> &bull; hanya sekali pakai. Abaikan jika Anda tidak meminta reset password.</p>
        </div>"#
    );
    let text = format!(
        "Halo {full_name} (@{username}),\n\nKode reset password Coffee Labs Anda: {code}\nBerlaku {expires_minutes} menit.\n\nAbaikan email ini jika Anda tidak meminta reset password."
    );
    send_transactional_email(to_email, full_name, &subject, &html, &text).await
}

pub async fn send_password_changed_notice(to_email: &str, full_name: &str, username: &str) -> AppResult<()> {
    let safe_name = escape_html(full_name);
    let safe_user = escape_html(username);
    let subject = "[Coffee Labs] Password Akun Anda Berhasil Diubah";
    let html = format!(
        r#"<div style="font-family:Arial,sans-serif;max-width:520px;margin:auto;padding:24px;background:#FAF6F1;border-radius:16px;color:#2C1E16">
        <h2 style="margin:0 0 8px;color:#15803D">Password Berhasil Diperbarui</h2>
        <p style="font-size:14px;line-height:1.6">Halo <b>{safe_name}</b> (@{safe_user}),</p>
        <p style="font-size:14px;line-height:1.6">Password akun Coffee Labs Anda baru saja direset dan seluruh sesi perangkat lama telah dikeluarkan demi keamanan.</p>
        </div>"#
    );
    let text = format!("Halo {full_name} (@{username}), password akun Coffee Labs Anda baru saja direset.");
    send_transactional_email(to_email, full_name, subject, &html, &text).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_normalizes_emails() {
        assert_eq!(normalize_email("  Rusdi.Coffee+1@Gmail.COM "), Some("rusdi.coffee+1@gmail.com".into()));
        assert_eq!(normalize_email("invalid-email"), None);
        assert_eq!(normalize_email("a@b"), None);
        assert_eq!(normalize_email("a b@gmail.com"), None);
        assert_eq!(normalize_email(".abc@gmail.com"), None);
    }

    #[test]
    fn masks_emails_cleanly() {
        assert_eq!(mask_email("rusdi@gmail.com"), "r***i@gmail.com");
        assert_eq!(mask_email("ab@kopi.id"), "a***@kopi.id");
    }
}

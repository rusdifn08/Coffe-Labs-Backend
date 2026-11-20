use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    core::{jwt::JwtManager, AppState},
    modules::admin_control::{dto::FeatureControlDto, error::AdminControlError},
};

const BYPASS_PATH_PREFIXES: &[&str] = &[
    "/api/v1/admin",
    "/api/admin",
    "/api/v1/system",
    "/api/v1/auth",
    "/api/v1/health",
    "/api/health",
    "/health",
    "/admin-control",
    "/dashboard",
    "/api/v1/telemetry",
    "/api/v1/monitoring",
    "/api/v1/billing/qris",
];

fn is_exempt_control_path(path: &str) -> bool {
    if path == "/" {
        return true;
    }
    BYPASS_PATH_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn find_matching_locked_feature<'a>(
    features: &'a [FeatureControlDto],
    path: &str,
) -> Option<&'a FeatureControlDto> {
    for feature in features {
        if !feature.is_locked {
            continue;
        }
        for prefix in &feature.api_path_prefixes {
            let trimmed = prefix.trim();
            if !trimmed.is_empty() && path.starts_with(trimmed) {
                return Some(feature);
            }
        }
    }
    None
}

fn extract_bearer_or_query_token(req: &Request) -> Option<String> {
    if let Some(raw_header) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some((scheme, rest)) = raw_header.trim().split_once(' ') {
            if scheme.eq_ignore_ascii_case("bearer") && !rest.trim().is_empty() {
                return Some(rest.trim().to_string());
            }
        }
    }
    let query = req.uri().query()?;
    for pair in query.split('&') {
        if let Some((key, val)) = pair.split_once('=') {
            if key == "token" && !val.trim().is_empty() {
                return Some(val.trim().to_string());
            }
        }
    }
    None
}

fn is_valid_admin_request(req: &Request, jwt: &JwtManager) -> bool {
    let Some(token) = extract_bearer_or_query_token(req) else {
        return false;
    };
    match jwt.verify_token(&token) {
        Ok(claims) => {
            !claims.status.eq_ignore_ascii_case("declined")
                && claims.role.eq_ignore_ascii_case("admin")
        }
        Err(_) => false,
    }
}

fn default_lock_title(status: &str, display_name: &str) -> String {
    match status {
        "COMING_SOON" => format!("{display_name} Segera Hadir"),
        "UPDATED" => format!("Pembaruan {display_name} Sedang Diterapkan"),
        _ => format!("{display_name} Sedang Dalam Maintenance"),
    }
}

fn default_lock_message(status: &str) -> String {
    match status {
        "COMING_SOON" => {
            "Fitur baru ini sedang disiapkan tim Coffee Labs dan akan segera tersedia untuk Anda."
                .to_string()
        }
        "UPDATED" => {
            "Sistem sedang menerapkan pembaruan versi terbaru agar performa makin optimal."
                .to_string()
        }
        _ => "Fitur sedang dalam pemeliharaan terjadwal oleh Admin. Silakan coba kembali sesaat lagi."
            .to_string(),
    }
}

/// Layer 3 Enforcement (~15ns lock-free RAM lookup via `ArcSwap`, 0 DB query):
/// Mengembalikan `HTTP 423 Locked` untuk non-Admin bila endpoint termasuk fitur terkunci,
/// sembari memberi jalur QA Bypass untuk Admin ketika `allow_admin_bypass == true`.
pub async fn feature_gate_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    if is_exempt_control_path(path) {
        return next.run(req).await;
    }

    let snapshot = state.admin_control.load_snapshot();
    if let Some(locked) = find_matching_locked_feature(&snapshot.features, path) {
        if locked.allow_admin_bypass && is_valid_admin_request(&req, &state.jwt) {
            return next.run(req).await;
        }
        let title = if locked.lock_title.trim().is_empty() {
            default_lock_title(&locked.lock_status, &locked.display_name)
        } else {
            locked.lock_title.clone()
        };
        let message = if locked.lock_message.trim().is_empty() {
            default_lock_message(&locked.lock_status)
        } else {
            locked.lock_message.clone()
        };

        return AdminControlError::FeatureLocked {
            feature_key: locked.feature_key.clone(),
            display_name: locked.display_name.clone(),
            lock_status: locked.lock_status.clone(),
            title,
            message,
            estimated_unlock_at: locked.estimated_unlock_at,
        }
        .into_response();
    }

    next.run(req).await
}

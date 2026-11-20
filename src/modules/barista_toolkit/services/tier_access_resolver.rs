use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgPool};

use crate::{
    core::jwt::JwtClaims,
    modules::{barista_toolkit::models::has_tier_access, platform::repository::fetch_trial_policy},
};

#[derive(Debug, FromRow)]
struct LiveUserAccessRow {
    tier: String,
    role: String,
    trial_full_until: Option<DateTime<Utc>>,
    trial_policy_override: Option<String>,
}

/// Menentukan tier & role efektif pengguna langsung dari PostgreSQL (Ground Truth),
/// sehingga akun yang baru di-upgrade ke Pro/Business/Admin atau memiliki Trial 7 Hari Full
/// langsung mendapatkan akses penuh E-Book & LMS meski JWT di perangkat masih menyimpan claim lama.
pub async fn resolve_live_effective_tier(pool: &PgPool, claims: &JwtClaims) -> (String, String) {
    let row = sqlx::query_as::<_, LiveUserAccessRow>(
        r#"SELECT tier, role, trial_full_until, trial_policy_override
           FROM users
           WHERE id = $1"#,
    )
    .bind(claims.sub)
    .fetch_optional(pool)
    .await;

    let Ok(Some(live)) = row else {
        return (claims.tier.clone(), claims.role.clone());
    };

    if live.role.eq_ignore_ascii_case("admin") {
        return ("business".to_string(), live.role);
    }

    let normalized_tier = live.tier.trim().to_ascii_lowercase();
    if normalized_tier == "business" || normalized_tier == "pro" {
        return (normalized_tier, live.role);
    }

    let now = Utc::now();
    let has_user_full_trial = live
        .trial_policy_override
        .as_deref()
        .is_some_and(|m| m.eq_ignore_ascii_case("full_trial_7d_non_business"))
        && live.trial_full_until.is_some_and(|until| now < until);

    if has_user_full_trial {
        return ("pro".to_string(), live.role);
    }

    let global_policy = fetch_trial_policy(pool).await;
    let within_global_window = global_policy.policy_mode.eq_ignore_ascii_case("full_trial_7d_non_business")
        && global_policy.apply_to_registered_trial
        && now
            < global_policy.activated_at
                + Duration::days(global_policy.trial_duration_days.max(1) as i64);

    if within_global_window && (global_policy.unlock_learning_academy || global_policy.unlock_toolkit_pro) {
        return ("pro".to_string(), live.role);
    }

    ("trial".to_string(), live.role)
}

/// Memeriksa akses tier E-Book / LMS berbasis data PostgreSQL terbaru.
pub async fn user_has_tier_access(pool: &PgPool, claims: &JwtClaims, required_tier: &str) -> bool {
    let (live_tier, live_role) = resolve_live_effective_tier(pool, claims).await;
    has_tier_access(&live_tier, &live_role, required_tier)
}

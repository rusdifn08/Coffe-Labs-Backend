use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

use crate::modules::{
    gamification::services::xp_calculator::default_freezes_for_tier,
    rbac::models::AccountTier,
};

#[derive(Debug, Clone)]
pub struct StreakEvaluation {
    pub new_streak: i32,
    pub new_longest: i32,
    pub freezes_remaining: i32,
    pub streak_updated: bool,
    pub streak_status: &'static str,
    pub streak_bonus_xp: i32,
}

/// Menghitung tanggal lokal pengguna dari nama timezone (default Asia/Jakarta UTC+7)
pub fn resolve_local_date(tz: &str) -> NaiveDate {
    let offset_hours = match tz.trim() {
        "Asia/Makassar" | "Asia/Singapore" | "WITA" => 8,
        "Asia/Jayapura" | "Asia/Tokyo" | "WIT" => 9,
        "UTC" | "GMT" => 0,
        _ => 7, // Default WIB (Asia/Jakarta)
    };
    (Utc::now() + Duration::hours(offset_hours)).date_naive()
}

/// Evaluasi Daily Brewing Streak & Streak Freeze Shield (Eksklusif Pro & Business)
pub fn evaluate_daily_streak(
    current_streak: i32,
    longest_streak: i32,
    freezes_available: i32,
    last_brew_date: Option<NaiveDate>,
    today_local: NaiveDate,
    tier: AccountTier,
) -> StreakEvaluation {
    let mut freezes = freezes_available;
    if freezes == 0 && last_brew_date.is_none() {
        freezes = default_freezes_for_tier(tier);
    }

    match last_brew_date {
        None => {
            let new_streak = 1;
            let new_longest = longest_streak.max(1);
            StreakEvaluation {
                new_streak,
                new_longest,
                freezes_remaining: freezes,
                streak_updated: true,
                streak_status: "STREAK_STARTED",
                streak_bonus_xp: 25,
            }
        }
        Some(last_date) => {
            let days_diff = (today_local - last_date).num_days();
            if days_diff <= 0 {
                // Sudah menyeduh hari ini, streak tetap terjaga
                StreakEvaluation {
                    new_streak: current_streak.max(1),
                    new_longest: longest_streak.max(current_streak.max(1)),
                    freezes_remaining: freezes,
                    streak_updated: false,
                    streak_status: "STREAK_MAINTAINED_TODAY",
                    streak_bonus_xp: 0,
                }
            } else if days_diff == 1 {
                // Hari berturut-turut!
                let new_streak = current_streak + 1;
                let new_longest = longest_streak.max(new_streak);
                let milestone_bonus = if new_streak % 7 == 0 { 150 } else { 40 };
                StreakEvaluation {
                    new_streak,
                    new_longest,
                    freezes_remaining: freezes,
                    streak_updated: true,
                    streak_status: "STREAK_INCREMENTED",
                    streak_bonus_xp: milestone_bonus,
                }
            } else if days_diff == 2 && freezes > 0 && tier.is_at_least(AccountTier::Pro) {
                // Terlewat 1 hari, tetapi dilindungi Streak Freeze Shield (Pro / Business)
                let new_streak = current_streak + 1;
                let new_longest = longest_streak.max(new_streak);
                StreakEvaluation {
                    new_streak,
                    new_longest,
                    freezes_remaining: freezes - 1,
                    streak_updated: true,
                    streak_status: "STREAK_SAVED_BY_SHIELD",
                    streak_bonus_xp: 30,
                }
            } else {
                // Streak terputus dan mulai kembali dari 1
                StreakEvaluation {
                    new_streak: 1,
                    new_longest: longest_streak.max(1),
                    freezes_remaining: freezes,
                    streak_updated: true,
                    streak_status: "STREAK_RESET_TO_ONE",
                    streak_bonus_xp: 15,
                }
            }
        }
    }
}

/// Background Scheduler (`tokio-cron-scheduler`) untuk mereset/mengisi ulang kuota Streak Freeze
pub async fn spawn_gamification_cron_jobs(pool: PgPool) {
    if let Ok(sched) = JobScheduler::new().await {
        let pool_monthly = pool.clone();
        if let Ok(job) = Job::new_async("0 0 0 1 * *", move |_uuid, _l| {
            let db = pool_monthly.clone();
            Box::pin(async move {
                let _ = sqlx::query(
                    r#"UPDATE gamification_profiles gp
                       SET streak_freezes_available = CASE
                           WHEN u.tier = 'business' THEN 5
                           WHEN u.tier = 'pro' THEN 2
                           ELSE 0
                       END,
                       updated_at = NOW()
                       FROM users u WHERE gp.user_id = u.id"#,
                )
                .execute(&db)
                .await;
                info!("🛡️ [Cron] Monthly Streak Freeze Shield kuota diperbarui untuk akun Pro & Business.");
            })
        }) {
            let _ = sched.add(job).await;
        }
        let _ = sched.start().await;
    }
}

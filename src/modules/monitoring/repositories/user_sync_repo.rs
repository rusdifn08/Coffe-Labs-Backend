use std::collections::HashMap;

use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

/// Menyinkronkan seluruh akun eksisting di tabel `users` ke `account_monitoring_dossiers`
/// agar semua user lama (mis. rusdifn08, bisnis123, risaha15, dll) langsung muncul di dashboard.
pub async fn sync_existing_users_to_monitoring(pool: &PgPool) {
    let sql = r#"
        INSERT INTO account_monitoring_dossiers (
            user_id, bitmap_seq_id, username, full_name, role, account_tier, avatar_url,
            last_seen_at, total_brews_executed, total_recipes_created, total_social_posts,
            health_score, churn_risk_level, updated_at
        )
        SELECT
            u.id,
            COALESCE(u.bitmap_seq_id, nextval('user_bitmap_seq')),
            u.username,
            u.full_name,
            u.role,
            INITCAP(COALESCE(NULLIF(u.tier, ''), 'trial')),
            u.avatar_url,
            COALESCE(bl_agg.last_brew_at, u.updated_at, u.created_at),
            GREATEST(u.trial_brews_count, COALESCE(bl_agg.brew_count, 0)::INT),
            COALESCE(ur_agg.recipe_count, 0)::INT,
            COALESCE(st_agg.post_count, 0)::INT,
            CASE
                WHEN GREATEST(u.trial_brews_count, COALESCE(bl_agg.brew_count, 0)::INT) >= 3
                  OR COALESCE(st_agg.post_count, 0) >= 1 THEN 72
                WHEN u.status = 'approved' THEN 58
                ELSE 35
            END,
            CASE
                WHEN COALESCE(bl_agg.last_brew_at, u.updated_at, u.created_at) < NOW() - INTERVAL '14 days' THEN 'DORMANT'
                WHEN GREATEST(u.trial_brews_count, COALESCE(bl_agg.brew_count, 0)::INT) >= 3 THEN 'LOW'
                ELSE 'MEDIUM'
            END,
            NOW()
        FROM users u
        LEFT JOIN (
            SELECT user_id, COUNT(*) AS brew_count, MAX(created_at) AS last_brew_at
            FROM brew_logs WHERE user_id IS NOT NULL GROUP BY user_id
        ) bl_agg ON bl_agg.user_id = u.id
        LEFT JOIN (
            SELECT user_id, COUNT(*) AS recipe_count
            FROM user_recipes WHERE user_id IS NOT NULL GROUP BY user_id
        ) ur_agg ON ur_agg.user_id = u.id
        LEFT JOIN (
            SELECT author_id, COUNT(*) AS post_count
            FROM social_threads WHERE is_deleted = FALSE GROUP BY author_id
        ) st_agg ON st_agg.author_id = u.id
        ON CONFLICT (user_id) DO UPDATE SET
            username = EXCLUDED.username,
            full_name = EXCLUDED.full_name,
            role = EXCLUDED.role,
            account_tier = EXCLUDED.account_tier,
            avatar_url = COALESCE(EXCLUDED.avatar_url, account_monitoring_dossiers.avatar_url),
            total_brews_executed = GREATEST(
                account_monitoring_dossiers.total_brews_executed,
                EXCLUDED.total_brews_executed
            ),
            total_recipes_created = GREATEST(
                account_monitoring_dossiers.total_recipes_created,
                EXCLUDED.total_recipes_created
            ),
            total_social_posts = GREATEST(
                account_monitoring_dossiers.total_social_posts,
                EXCLUDED.total_social_posts
            )
    "#;

    match sqlx::query(sql).execute(pool).await {
        Ok(res) => {
            info!(
                synced_rows = res.rows_affected(),
                "📊 Sinkronisasi akun eksisting ke account_monitoring_dossiers selesai"
            );
        }
        Err(err) => {
            warn!(error = %err, "Gagal sinkronisasi awal account_monitoring_dossiers");
        }
    }
}

pub async fn lookup_bitmap_seq_and_tiers(
    pool: &PgPool,
    user_ids: &[Uuid],
) -> HashMap<Uuid, (u32, String)> {
    let mut map = HashMap::new();
    if user_ids.is_empty() {
        return map;
    }

    let res = sqlx::query_as::<_, (Uuid, Option<i64>, String)>(
        "SELECT id, bitmap_seq_id, tier FROM users WHERE id = ANY($1)",
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await;

    if let Ok(rows) = res {
        for (uid, seq_opt, tier) in rows {
            if let Some(seq) = seq_opt.and_then(|v| u32::try_from(v).ok()) {
                map.insert(uid, (seq, tier));
            }
        }
    }
    map
}

pub async fn mark_users_offline(pool: &PgPool, user_ids: &[Uuid]) {
    if user_ids.is_empty() {
        return;
    }
    let _ = sqlx::query(
        r#"UPDATE account_monitoring_dossiers
           SET is_online = FALSE, updated_at = NOW()
           WHERE user_id = ANY($1) AND is_online = TRUE"#,
    )
    .bind(user_ids)
    .execute(pool)
    .await;
}

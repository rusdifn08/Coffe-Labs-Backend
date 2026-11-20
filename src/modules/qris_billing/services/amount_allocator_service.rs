use std::collections::HashSet;
use std::time::Duration;

use fred::clients::RedisPool;
use fred::interfaces::{ClientLike, KeysInterface, LuaInterface};
use rand::Rng;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::core::AppState;
use crate::modules::qris_billing::dto::QrisWsFrame;
use crate::modules::qris_billing::error::QrisBillingError;
use crate::modules::qris_billing::repositories::invoice_repo;
use crate::modules::qris_billing::services::qris_ws_hub::global_qris_ws_hub;

const LOCK_TTL_SECS: i64 = 960;
const POST_PAID_COOLDOWN_SECS: i64 = 120;

/// Redis Atomic Lua Script: memilih kode unik 1..999 secara acak dan mengunci
/// `qris:lock_amount:{base + suffix}` secara atomik (`NX EX 960`) tanpa tabrakan.
const LUA_ALLOCATE_UNIQUE_AMOUNT: &str = r#"
local base = tonumber(ARGV[1])
local invoice_id = ARGV[2]
local ttl = tonumber(ARGV[3])
local start_offset = tonumber(ARGV[4])
for i = 0, 998 do
    local suffix = ((start_offset + i) % 999) + 1
    local total = base + suffix
    local key = 'qris:lock_amount:' .. tostring(total)
    if redis.call('SET', key, invoice_id, 'NX', 'EX', ttl) then
        return suffix
    end
end
return -1
"#;

/// Mengalokasikan nominal unik (`base_price_idr + 1..999`) secara bebas tabrakan via Redis Lua + DB fallback.
pub async fn allocate_unique_suffix(
    pool: &PgPool,
    redis_pool: Option<&RedisPool>,
    base_price_idr: i64,
    invoice_id: Uuid,
) -> AppResult<(i16, i64)> {
    let start_offset: i64 = rand::thread_rng().gen_range(0..999);

    if let Some(redis) = redis_pool.filter(|p| p.is_connected()) {
        let eval_res = tokio::time::timeout(
            Duration::from_millis(450),
            redis.eval::<i64, _, _, _>(
                LUA_ALLOCATE_UNIQUE_AMOUNT,
                Vec::<String>::new(),
                vec![
                    base_price_idr.to_string(),
                    invoice_id.to_string(),
                    LOCK_TTL_SECS.to_string(),
                    start_offset.to_string(),
                ],
            ),
        )
        .await;

        if let Ok(Ok(suffix)) = eval_res {
            if (1..=999).contains(&suffix) {
                let total = base_price_idr + suffix;
                let db_busy: bool = sqlx::query_scalar(
                    "SELECT EXISTS(
                        SELECT 1 FROM qris_subscription_invoices
                        WHERE total_payable_idr = $1 AND status = 'PENDING'::qris_invoice_status
                    )",
                )
                .bind(total)
                .fetch_one(pool)
                .await
                .unwrap_or(false);

                if !db_busy {
                    return Ok((suffix as i16, total));
                }
            }
        }
    }

    allocate_unique_suffix_db_fallback(pool, base_price_idr, start_offset).await
}

async fn allocate_unique_suffix_db_fallback(
    pool: &PgPool,
    base_price_idr: i64,
    start_offset: i64,
) -> AppResult<(i16, i64)> {
    let busy_amounts: Vec<i64> = sqlx::query_scalar(
        "SELECT total_payable_idr FROM qris_subscription_invoices
         WHERE status = 'PENDING'::qris_invoice_status
           AND total_payable_idr BETWEEN $1 AND $2",
    )
    .bind(base_price_idr + 1)
    .bind(base_price_idr + 999)
    .fetch_all(pool)
    .await?;

    let occupied: HashSet<i64> = busy_amounts.into_iter().collect();
    for step in 0..999 {
        let suffix = ((start_offset + step) % 999) + 1;
        let total = base_price_idr + suffix;
        if !occupied.contains(&total) {
            return Ok((suffix as i16, total));
        }
    }
    Err(QrisBillingError::AmountPoolExhausted.into())
}

/// Menetapkan cooldown 120 detik setelah invoice terbayar agar duplikat notifikasi telat tidak salah cocok.
pub async fn set_paid_amount_cooldown(redis_pool: Option<&RedisPool>, total_payable_idr: i64) {
    let Some(redis) = redis_pool.filter(|p| p.is_connected()) else {
        return;
    };
    let key = format!("qris:lock_amount:{total_payable_idr}");
    let _ = tokio::time::timeout(
        Duration::from_millis(250),
        redis.expire::<i64, _>(&key, POST_PAID_COOLDOWN_SECS),
    )
    .await;
}

pub async fn release_expired_amount_lock(redis_pool: Option<&RedisPool>, total_payable_idr: i64) {
    let Some(redis) = redis_pool.filter(|p| p.is_connected()) else {
        return;
    };
    let key = format!("qris:lock_amount:{total_payable_idr}");
    let _ = tokio::time::timeout(Duration::from_millis(250), redis.del::<i64, _>(&key)).await;
}

/// Background worker Tokio setiap 60 detik menandai invoice PENDING lewat 15 menit sebagai EXPIRED.
pub fn spawn_expiration_sweeper(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        info!("⏱️ QRIS Billing Expiration Sweeper aktif (interval 60 detik).");
        loop {
            interval.tick().await;
            match invoice_repo::expire_stale_pending_invoices(&state.pool).await {
                Ok(expired) if !expired.is_empty() => {
                    let hub = global_qris_ws_hub();
                    let redis_pool = state.redis.active_pool();
                    for (invoice_id, total_idr) in expired {
                        release_expired_amount_lock(redis_pool.as_ref(), total_idr).await;
                        hub.publish(
                            redis_pool.as_ref(),
                            invoice_id,
                            QrisWsFrame::Expired { invoice_id },
                        )
                        .await;
                        hub.cleanup_idle_channel(invoice_id);
                    }
                }
                Ok(_) => {}
                Err(err) => {
                    warn!(error = %err, "Gagal menjalankan sweeper kedaluwarsa invoice QRIS");
                }
            }
        }
    });
}

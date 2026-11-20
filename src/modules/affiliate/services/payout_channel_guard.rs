use fred::{
    interfaces::KeysInterface,
    types::{Expiration, SetOptions},
};
use uuid::Uuid;

use crate::{core::AppState, modules::affiliate::error::AffiliateError};

pub fn normalize_payout_channel(raw: &str) -> Option<(&'static str, &'static str)> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "BANK_MANDIRI" | "MANDIRI" => Some(("BANK_MANDIRI", "Bank Mandiri")),
        "BANK_BCA" | "BCA" => Some(("BANK_BCA", "Bank BCA")),
        "BANK_BRI" | "BRI" => Some(("BANK_BRI", "Bank BRI")),
        "BANK_BNI" | "BNI" => Some(("BANK_BNI", "Bank BNI")),
        "BANK_BSI" | "BSI" => Some(("BANK_BSI", "Bank BSI")),
        "BANK_SEABANK" | "SEABANK" => Some(("BANK_SEABANK", "SeaBank")),
        "BANK_JAGO" | "JAGO" => Some(("BANK_JAGO", "Bank Jago")),
        "EWALLET_GOPAY" | "GOPAY" => Some(("EWALLET_GOPAY", "GoPay")),
        "EWALLET_DANA" | "DANA" => Some(("EWALLET_DANA", "DANA")),
        "EWALLET_OVO" | "OVO" => Some(("EWALLET_OVO", "OVO")),
        "EWALLET_SHOPEEPAY" | "SHOPEEPAY" => Some(("EWALLET_SHOPEEPAY", "ShopeePay")),
        "OTHER_BANK" => Some(("OTHER_BANK", "Bank Lainnya")),
        _ => None,
    }
}

pub async fn acquire_wd_lock(
    state: &AppState,
    user_id: Uuid,
) -> Result<Option<String>, AffiliateError> {
    let Some(pool) = state.redis.active_pool() else {
        return Ok(None);
    };
    let key = format!("aff:wd_lock:{user_id}");
    if let Ok(Ok(acquired)) = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        pool.set::<Option<String>, _, _>(
            &key,
            "1",
            Some(Expiration::EX(15)),
            Some(SetOptions::NX),
            false,
        ),
    )
    .await
    {
        if acquired.is_none() {
            return Err(AffiliateError::ConcurrentWithdrawalLocked);
        }
        return Ok(Some(key));
    }
    Ok(None)
}

pub async fn release_wd_lock(state: &AppState, key: Option<String>) {
    if let (Some(pool), Some(k)) = (state.redis.active_pool(), key) {
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(180),
            pool.del::<(), _>(&k),
        )
        .await;
    }
}

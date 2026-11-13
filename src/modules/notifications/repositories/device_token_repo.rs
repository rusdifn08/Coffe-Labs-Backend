use sqlx::PgPool;
use uuid::Uuid;

pub use super::preference_repo::{get_or_default_preferences, upsert_preferences};
use crate::modules::notifications::{
    dto::RegisterDeviceTokenReq,
    error::{NotifError, NotifResult},
    models::DeviceTokenRow,
};

pub async fn upsert_device_token(
    pool: &PgPool,
    user_id: Uuid,
    req: &RegisterDeviceTokenReq,
) -> NotifResult<DeviceTokenRow> {
    let platform = req.platform.trim().to_ascii_uppercase();
    if !matches!(platform.as_str(), "ANDROID" | "IOS" | "WEB" | "DESKTOP") {
        return Err(NotifError::InvalidPayload(
            "Platform harus ANDROID, IOS, WEB, atau DESKTOP.".to_string(),
        ));
    }
    let app_version = req.app_version.as_deref().unwrap_or("2.2.0");
    let kiosk_active = req.is_kiosk_aod_active.unwrap_or(false);

    let row = sqlx::query_as::<_, DeviceTokenRow>(
        r#"
        INSERT INTO user_device_tokens (
            user_id, device_installation_id, platform, fcm_token,
            apns_voip_or_live_activity_token, app_version, device_model,
            is_active, is_kiosk_aod_active, failed_delivery_count, last_active_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE, $8, 0, NOW())
        ON CONFLICT (user_id, device_installation_id) DO UPDATE SET
            platform = EXCLUDED.platform,
            fcm_token = EXCLUDED.fcm_token,
            apns_voip_or_live_activity_token = EXCLUDED.apns_voip_or_live_activity_token,
            app_version = EXCLUDED.app_version,
            device_model = EXCLUDED.device_model,
            is_active = TRUE,
            is_kiosk_aod_active = EXCLUDED.is_kiosk_aod_active,
            failed_delivery_count = 0,
            last_active_at = NOW()
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(req.device_installation_id.trim())
    .bind(platform)
    .bind(req.fcm_token.trim())
    .bind(req.apns_voip_or_live_activity_token.as_deref())
    .bind(app_version)
    .bind(req.device_model.as_deref())
    .bind(kiosk_active)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn update_kiosk_aod_flag(
    pool: &PgPool,
    user_id: Uuid,
    device_installation_id: &str,
    is_kiosk_aod_active: bool,
) -> NotifResult<()> {
    sqlx::query(
        r#"
        UPDATE user_device_tokens
        SET is_kiosk_aod_active = $3, last_active_at = NOW()
        WHERE user_id = $1 AND device_installation_id = $2
        "#,
    )
    .bind(user_id)
    .bind(device_installation_id.trim())
    .bind(is_kiosk_aod_active)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn deactivate_by_installation(
    pool: &PgPool,
    user_id: Uuid,
    device_installation_id: &str,
) -> NotifResult<()> {
    sqlx::query(
        r#"
        UPDATE user_device_tokens
        SET is_active = FALSE, last_active_at = NOW()
        WHERE user_id = $1 AND device_installation_id = $2
        "#,
    )
    .bind(user_id)
    .bind(device_installation_id.trim())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn deactivate_stale_token(pool: &PgPool, fcm_token: &str) -> NotifResult<()> {
    sqlx::query(
        r#"
        UPDATE user_device_tokens
        SET is_active = FALSE, failed_delivery_count = failed_delivery_count + 1
        WHERE fcm_token = $1
        "#,
    )
    .bind(fcm_token)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn record_delivery_failure(pool: &PgPool, fcm_token: &str) -> NotifResult<()> {
    sqlx::query(
        r#"
        UPDATE user_device_tokens
        SET failed_delivery_count = failed_delivery_count + 1,
            is_active = CASE WHEN failed_delivery_count + 1 >= 10 THEN FALSE ELSE is_active END
        WHERE fcm_token = $1
        "#,
    )
    .bind(fcm_token)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_active_tokens(pool: &PgPool, user_id: Uuid) -> NotifResult<Vec<DeviceTokenRow>> {
    let rows = sqlx::query_as::<_, DeviceTokenRow>(
        r#"
        SELECT * FROM user_device_tokens
        WHERE user_id = $1 AND is_active = TRUE
        ORDER BY last_active_at DESC
        LIMIT 20
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

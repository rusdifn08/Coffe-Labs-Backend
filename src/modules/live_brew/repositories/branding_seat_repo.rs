use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::live_brew::{
    error::LiveBrewError,
    mirror_dto::UpsertBrandingRequest,
    mirror_models::{slugify_livebrew_segment, BusinessBrandingRow},
};

pub use crate::modules::live_brew::repositories::seat_repo::{
    create_or_upsert_seat, fetch_seat_by_fallback_code, fetch_seat_by_id,
    fetch_seat_by_store_and_slug, list_seats_for_store,
};

const BRANDING_COLS: &str = r#"
    store_id, owner_user_id, cafe_slug, cafe_display_name,
    logo_cloudinary_url, primary_color_hex, accent_color_hex,
    welcome_message, wifi_ssid, wifi_password, instagram_handle,
    show_pos_queue_position, show_live_bean_details
"#;

pub async fn ensure_store_id_for_owner(
    pool: &PgPool,
    owner_user_id: Uuid,
    default_store_name: &str,
) -> Result<Uuid, LiveBrewError> {
    if let Some(id) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM business_store_configs WHERE owner_user_id = $1",
    )
    .bind(owner_user_id)
    .fetch_optional(pool)
    .await?
    {
        return Ok(id);
    }

    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO business_store_configs (owner_user_id, store_name, cashier_pin_hash)
           VALUES ($1, $2, 'unconfigured_pin')
           ON CONFLICT (owner_user_id) DO UPDATE SET updated_at = NOW()
           RETURNING id"#,
    )
    .bind(owner_user_id)
    .bind(default_store_name)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn upsert_branding(
    pool: &PgPool,
    owner_user_id: Uuid,
    req: &UpsertBrandingRequest,
) -> Result<BusinessBrandingRow, LiveBrewError> {
    let store_id =
        ensure_store_id_for_owner(pool, owner_user_id, &req.cafe_display_name).await?;
    let clean_slug = slugify_livebrew_segment(&req.cafe_slug, 64);

    let conflict: Option<Uuid> = sqlx::query_scalar(
        "SELECT store_id FROM business_livebrew_branding WHERE cafe_slug = $1 AND store_id <> $2",
    )
    .bind(&clean_slug)
    .bind(store_id)
    .fetch_optional(pool)
    .await?;

    if conflict.is_some() {
        return Err(LiveBrewError::SeatSlugConflict(clean_slug));
    }

    let q = format!(
        r#"INSERT INTO business_livebrew_branding (
                store_id, owner_user_id, cafe_slug, cafe_display_name,
                logo_cloudinary_url, logo_cloudinary_public_id,
                primary_color_hex, accent_color_hex, welcome_message,
                wifi_ssid, wifi_password, instagram_handle,
                show_pos_queue_position, show_live_bean_details, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,NOW())
            ON CONFLICT (store_id) DO UPDATE SET
                cafe_slug = EXCLUDED.cafe_slug,
                cafe_display_name = EXCLUDED.cafe_display_name,
                logo_cloudinary_url = COALESCE(EXCLUDED.logo_cloudinary_url, business_livebrew_branding.logo_cloudinary_url),
                logo_cloudinary_public_id = COALESCE(EXCLUDED.logo_cloudinary_public_id, business_livebrew_branding.logo_cloudinary_public_id),
                primary_color_hex = EXCLUDED.primary_color_hex,
                accent_color_hex = EXCLUDED.accent_color_hex,
                welcome_message = EXCLUDED.welcome_message,
                wifi_ssid = EXCLUDED.wifi_ssid,
                wifi_password = EXCLUDED.wifi_password,
                instagram_handle = EXCLUDED.instagram_handle,
                show_pos_queue_position = EXCLUDED.show_pos_queue_position,
                show_live_bean_details = EXCLUDED.show_live_bean_details,
                updated_at = NOW()
            RETURNING {}"#,
        BRANDING_COLS
    );

    let row = sqlx::query_as::<_, BusinessBrandingRow>(&q)
        .bind(store_id)
        .bind(owner_user_id)
        .bind(&clean_slug)
        .bind(req.cafe_display_name.trim())
        .bind(&req.logo_cloudinary_url)
        .bind(&req.logo_cloudinary_public_id)
        .bind(req.primary_color_hex.as_deref().unwrap_or("#12100E"))
        .bind(req.accent_color_hex.as_deref().unwrap_or("#D4A373"))
        .bind(req.welcome_message.as_deref().unwrap_or(
            "Selamat menikmati seduhan segar kami! Pantau proses seduhan Anda secara live di sini.",
        ))
        .bind(&req.wifi_ssid)
        .bind(&req.wifi_password)
        .bind(&req.instagram_handle)
        .bind(req.show_pos_queue_position.unwrap_or(true))
        .bind(req.show_live_bean_details.unwrap_or(true))
        .fetch_one(pool)
        .await?;

    Ok(row)
}

pub async fn fetch_branding_by_owner(
    pool: &PgPool,
    owner_user_id: Uuid,
) -> Result<Option<BusinessBrandingRow>, LiveBrewError> {
    let q = format!("SELECT {} FROM business_livebrew_branding WHERE owner_user_id = $1", BRANDING_COLS);
    Ok(sqlx::query_as::<_, BusinessBrandingRow>(&q).bind(owner_user_id).fetch_optional(pool).await?)
}

pub async fn fetch_branding_by_slug(
    pool: &PgPool,
    cafe_slug: &str,
) -> Result<Option<BusinessBrandingRow>, LiveBrewError> {
    let q = format!("SELECT {} FROM business_livebrew_branding WHERE cafe_slug = $1", BRANDING_COLS);
    Ok(sqlx::query_as::<_, BusinessBrandingRow>(&q)
        .bind(cafe_slug.trim().to_ascii_lowercase())
        .fetch_optional(pool)
        .await?)
}

pub async fn fetch_branding_by_store(
    pool: &PgPool,
    store_id: Uuid,
) -> Result<Option<BusinessBrandingRow>, LiveBrewError> {
    let q = format!("SELECT {} FROM business_livebrew_branding WHERE store_id = $1", BRANDING_COLS);
    Ok(sqlx::query_as::<_, BusinessBrandingRow>(&q).bind(store_id).fetch_optional(pool).await?)
}

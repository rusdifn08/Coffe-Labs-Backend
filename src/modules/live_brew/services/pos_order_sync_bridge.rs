use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::{
    core::AppState,
    modules::live_brew::{
        error::LiveBrewError,
        mirror_dto::{BrandedSeatResolutionDto, LiveSessionMirrorSnapshot, PosOrderBindRequest},
        repositories::{branding_seat_repo, real_mirror_session_repo},
        services::seat_resolution_service,
    },
};

/// Menghubungkan pesanan POS Kasir Bisnis ke Meja/Kursi dan langsung mendorong update real-time.
pub async fn bind_pos_order_to_seat(
    state: &AppState,
    owner_user_id: Uuid,
    req: &PosOrderBindRequest,
) -> Result<BrandedSeatResolutionDto, LiveBrewError> {
    let branding = branding_seat_repo::fetch_branding_by_owner(&state.pool, owner_user_id)
        .await?
        .ok_or_else(|| LiveBrewError::SeatNotFound("Branding kafe belum diatur".to_string()))?;
    let seat = branding_seat_repo::fetch_seat_by_id(&state.pool, req.seat_id)
        .await?
        .ok_or_else(|| LiveBrewError::SeatNotFound(req.seat_id.to_string()))?;
    if seat.store_id != branding.store_id {
        return Err(LiveBrewError::UnauthorizedBarista);
    }

    let status = req.pos_order_status.as_deref().unwrap_or("QUEUED");
    let customer = req.customer_name.as_deref().unwrap_or("Tamu Coffee Labs");
    let items = req
        .items_summary
        .as_deref()
        .unwrap_or("Specialty Manual Brew");

    real_mirror_session_repo::update_seat_pos_binding(
        &state.pool,
        seat.id,
        req.pos_order_id,
        &req.pos_order_number,
        status,
        customer,
        items,
    )
    .await?;

    let updated_seat = branding_seat_repo::fetch_seat_by_id(&state.pool, seat.id)
        .await?
        .unwrap_or(seat);
    let resolution = seat_resolution_service::build_seat_resolution(
        &state.pool,
        &state.live_brew_hub.mirror_hub,
        &branding,
        &updated_seat,
    )
    .await?;

    state
        .live_brew_hub
        .mirror_hub
        .publish_seat_update(updated_seat.id, resolution.clone());
    Ok(resolution)
}

/// Dipanggil saat Brewer menyelesaikan seduhan (`CompleteAndServe`):
/// memperbarui status pesanan POS & memangkas stok biji kopi di Bean Vault toko bila tersedia.
pub async fn sync_pos_and_inventory_on_complete(
    pool: &PgPool,
    snap: &LiveSessionMirrorSnapshot,
) {
    if let Some(seat_id) = snap.seat_id {
        let _ = sqlx::query(
            r#"UPDATE business_cafe_seats
               SET active_pos_order_status = 'READY_TO_DELIVER',
                   updated_at = NOW()
               WHERE id = $1"#,
        )
        .bind(seat_id)
        .execute(pool)
        .await;
    }

    if let Some(pos_order_id) = snap.pos_order_id {
        let _ = sqlx::query("UPDATE pos_orders SET status = 'ready_to_deliver' WHERE id = $1")
            .bind(pos_order_id)
            .execute(pool)
            .await;
    }

    if let Some(store_id) = snap.store_id {
        let dose_g = snap
            .recipe_snapshot_json
            .get("coffeeWeightGrams")
            .and_then(|v| v.as_f64())
            .unwrap_or(15.0)
            .clamp(5.0, 200.0);

        let _ = sqlx::query(
            r#"UPDATE business_inventory_items
               SET stock_quantity = GREATEST(0.0, stock_quantity - $2),
                   updated_at = NOW()
               WHERE id = (
                   SELECT id FROM business_inventory_items
                   WHERE store_id = $1 AND LOWER(category) LIKE '%bean%'
                   ORDER BY stock_quantity DESC
                   LIMIT 1
               )"#,
        )
        .bind(store_id)
        .bind(dose_g)
        .execute(pool)
        .await;

        info!(
            store_id = %store_id,
            session_id = %snap.session_id,
            dose_g,
            "☕ Live Brew selesai — POS status READY_TO_DELIVER & inventori biji disinkronkan."
        );
    }
}

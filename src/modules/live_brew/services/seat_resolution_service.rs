use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::live_brew::{
    error::LiveBrewError,
    mirror_dto::{BrandedSeatResolutionDto, PosOrderProgressDto},
    mirror_models::{BusinessBrandingRow, CafeSeatRow},
    repositories::{branding_seat_repo, real_mirror_session_repo},
    services::{
        mirror_state_hub::MirrorStateHub,
        mirror_state_machine::{row_to_branding_dto, row_to_mirror_snapshot, row_to_seat_dto},
    },
};

pub async fn build_seat_resolution(
    pool: &PgPool,
    hub: &MirrorStateHub,
    branding: &BusinessBrandingRow,
    seat: &CafeSeatRow,
) -> Result<BrandedSeatResolutionDto, LiveBrewError> {
    let active_brew_session = if let Some(sid) = seat.active_live_session_id {
        if let Some(cached) = hub.get_cached_snapshot(sid) {
            Some(cached)
        } else if let Some(row) = real_mirror_session_repo::fetch_by_id(pool, sid).await? {
            let snap = row_to_mirror_snapshot(
                &row,
                Some((&branding.cafe_slug, &seat.seat_slug)),
                hub.active_viewers(row.id),
            );
            Some(snap)
        } else {
            None
        }
    } else {
        None
    };

    let pos_order_progress = seat.active_pos_order_number.as_ref().map(|ord_no| {
        PosOrderProgressDto {
            pos_order_id: seat.active_pos_order_id,
            pos_order_number: ord_no.clone(),
            pos_order_status: seat
                .active_pos_order_status
                .clone()
                .unwrap_or_else(|| "QUEUED".to_string()),
            queue_position_ahead: 0,
            items_summary: seat
                .active_pos_items_summary
                .clone()
                .unwrap_or_else(|| "Handcrafted Specialty Coffee".to_string()),
            customer_name: seat
                .active_pos_customer_name
                .clone()
                .unwrap_or_else(|| "Pelanggan Coffee Labs".to_string()),
            updated_at: seat.updated_at,
        }
    });

    let mut pos_progress = pos_order_progress;
    if let Some(ref mut prog) = pos_progress {
        if branding.show_pos_queue_position {
            prog.queue_position_ahead =
                real_mirror_session_repo::count_pos_queue_ahead(pool, branding.store_id, seat.id)
                    .await
                    .unwrap_or(0);
        }
    }

    let resolution_kind = if active_brew_session.is_some() {
        "ACTIVE_MIRROR_SESSION"
    } else if pos_progress.is_some() {
        "SEAT_POS_ORDER_QUEUED"
    } else {
        "SEAT_IDLE_BRANDED"
    };

    Ok(BrandedSeatResolutionDto {
        resolution_kind: resolution_kind.to_string(),
        branding: Some(row_to_branding_dto(branding)),
        seat: Some(row_to_seat_dto(seat, &branding.cafe_slug)),
        pos_order_progress: pos_progress,
        active_brew_session,
        server_epoch_ms: Utc::now().timestamp_millis(),
    })
}

pub async fn resolve_by_cafe_and_seat(
    pool: &PgPool,
    hub: &MirrorStateHub,
    cafe_slug: &str,
    seat_slug: &str,
) -> Result<BrandedSeatResolutionDto, LiveBrewError> {
    let branding = branding_seat_repo::fetch_branding_by_slug(pool, cafe_slug)
        .await?
        .ok_or_else(|| LiveBrewError::SeatNotFound(format!("{cafe_slug}/{seat_slug}")))?;
    let seat = branding_seat_repo::fetch_seat_by_store_and_slug(pool, branding.store_id, seat_slug)
        .await?
        .ok_or_else(|| LiveBrewError::SeatNotFound(format!("{cafe_slug}/{seat_slug}")))?;
    let _ = sqlx::query("UPDATE business_cafe_seats SET qr_scan_count = qr_scan_count + 1 WHERE id = $1")
        .bind(seat.id)
        .execute(pool)
        .await;
    build_seat_resolution(pool, hub, &branding, &seat).await
}

pub async fn resolve_by_share_code(
    pool: &PgPool,
    hub: &MirrorStateHub,
    code: &str,
) -> Result<BrandedSeatResolutionDto, LiveBrewError> {
    let trimmed = code.trim();
    if let Ok(uuid) = Uuid::parse_str(trimmed) {
        if let Some(cached) = hub.get_cached_snapshot(uuid) {
            return Ok(BrandedSeatResolutionDto {
                resolution_kind: "ACTIVE_MIRROR_SESSION".to_string(),
                branding: None,
                seat: None,
                pos_order_progress: None,
                active_brew_session: Some(cached),
                server_epoch_ms: Utc::now().timestamp_millis(),
            });
        }
        if let Some(row) = real_mirror_session_repo::fetch_by_id(pool, uuid).await? {
            let snap = row_to_mirror_snapshot(&row, None, hub.active_viewers(row.id));
            return Ok(BrandedSeatResolutionDto {
                resolution_kind: "ACTIVE_MIRROR_SESSION".to_string(),
                branding: None,
                seat: None,
                pos_order_progress: None,
                active_brew_session: Some(snap),
                server_epoch_ms: Utc::now().timestamp_millis(),
            });
        }
    }

    if let Some(row) = real_mirror_session_repo::fetch_by_share_code(pool, trimmed).await? {
        let snap = hub
            .get_cached_snapshot(row.id)
            .unwrap_or_else(|| row_to_mirror_snapshot(&row, None, hub.active_viewers(row.id)));
        let (branding_dto, seat_dto) = match (row.store_id, row.seat_id) {
            (Some(st_id), Some(s_id)) => {
                let b = branding_seat_repo::fetch_branding_by_store(pool, st_id).await?;
                let s = branding_seat_repo::fetch_seat_by_id(pool, s_id).await?;
                match (b, s) {
                    (Some(br), Some(sr)) => (
                        Some(row_to_branding_dto(&br)),
                        Some(row_to_seat_dto(&sr, &br.cafe_slug)),
                    ),
                    _ => (None, None),
                }
            }
            _ => (None, None),
        };
        return Ok(BrandedSeatResolutionDto {
            resolution_kind: "ACTIVE_MIRROR_SESSION".to_string(),
            branding: branding_dto,
            seat: seat_dto,
            pos_order_progress: None,
            active_brew_session: Some(snap),
            server_epoch_ms: Utc::now().timestamp_millis(),
        });
    }

    if let Some(seat) = branding_seat_repo::fetch_seat_by_fallback_code(pool, trimmed).await? {
        if let Some(branding) = branding_seat_repo::fetch_branding_by_store(pool, seat.store_id).await? {
            return build_seat_resolution(pool, hub, &branding, &seat).await;
        }
    }

    Err(LiveBrewError::SessionNotFound(trimmed.to_string()))
}

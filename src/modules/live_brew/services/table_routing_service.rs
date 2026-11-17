use std::{sync::Arc, time::Duration};

use fred::{interfaces::KeysInterface, types::Expiration};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::live_brew::{
    address_cipher::{landing_livebrew_url, verify_and_classify_code, AddressCodeKind},
    dto::{LiveBrewSnapshotDto, ResolveAddressResponse, TableWaitingInfoDto},
    error::LiveBrewError,
    protocol::ViewerDownstreamEvent,
    repositories::{brew_table_repo, live_session_repo},
    services::{live_telemetry_hub::LiveTelemetryHub, passport_archiver_service::session_row_to_snapshot},
};

const TABLE_PTR_PREFIX: &str = "livebrew:table_ptr:";

pub async fn set_table_pointer(
    hub: &LiveTelemetryHub,
    static_code: &str,
    session_id: Uuid,
) {
    let Some(pool) = &hub.redis_pool else {
        return;
    };
    let key = format!("{TABLE_PTR_PREFIX}{static_code}");
    let _ = tokio::time::timeout(
        Duration::from_millis(80),
        pool.set::<(), _, _>(key, session_id.to_string(), Some(Expiration::EX(3600)), None, false),
    )
    .await;
}

pub async fn get_table_pointer(hub: &LiveTelemetryHub, static_code: &str) -> Option<Uuid> {
    let pool = hub.redis_pool.as_ref()?;
    let key = format!("{TABLE_PTR_PREFIX}{static_code}");
    let Ok(Ok(Some(raw))) = tokio::time::timeout(
        Duration::from_millis(80),
        pool.get::<Option<String>, _>(key),
    )
    .await
    else {
        return None;
    };
    Uuid::parse_str(&raw).ok()
}

pub async fn bind_table_and_notify(
    pool: &PgPool,
    hub: &LiveTelemetryHub,
    table_id: Uuid,
    session_id: Uuid,
) -> Result<(), LiveBrewError> {
    let table = brew_table_repo::find_table_by_id(pool, table_id)
        .await?
        .ok_or_else(|| LiveBrewError::TableNotFound(table_id.to_string()))?;

    brew_table_repo::bind_active_session_on_table(pool, table_id, Some(session_id)).await?;
    set_table_pointer(hub, &table.static_address_code, session_id).await;

    if let Some(session_row) = live_session_repo::fetch_by_id(pool, session_id).await? {
        let (_, store_name) = brew_table_repo::fetch_user_store_info(pool, session_row.barista_user_id).await;
        let catchup = hub.get_catchup_curve(session_id).await;
        let snap = session_row_to_snapshot(
            &session_row,
            &store_name,
            Some(table.table_number.clone()),
            Some(table.static_address_code.clone()),
            catchup,
        );
        let static_code = table.static_address_code.clone();
        hub.broadcast_table_bound(
            &static_code,
            Arc::new(ViewerDownstreamEvent::TableSessionBound {
                table_number: table.table_number,
                static_address_code: table.static_address_code,
                snapshot: snap,
            }),
        );
    }
    Ok(())
}

pub async fn resolve_address_code(
    pool: &PgPool,
    hub: &LiveTelemetryHub,
    code: &str,
    cipher_secret: &str,
) -> Result<ResolveAddressResponse, LiveBrewError> {
    let kind = verify_and_classify_code(code, cipher_secret)?;
    let trimmed = code.trim().to_string();

    match kind {
        AddressCodeKind::DynamicSession { .. } => {
            let row = live_session_repo::fetch_by_dynamic_code(pool, &trimmed)
                .await?
                .ok_or_else(|| LiveBrewError::SessionNotFound(trimmed.clone()))?;
            let (_, store_name) = brew_table_repo::fetch_user_store_info(pool, row.barista_user_id).await;
            let (tbl_num, tbl_code) = match row.table_id {
                Some(tid) => match brew_table_repo::find_table_by_id(pool, tid).await? {
                    Some(t) => (Some(t.table_number), Some(t.static_address_code)),
                    None => (None, None),
                },
                None => (None, None),
            };
            let catchup = hub.get_catchup_curve(row.id).await;
            let snap: LiveBrewSnapshotDto = session_row_to_snapshot(&row, &store_name, tbl_num, tbl_code, catchup);
            Ok(ResolveAddressResponse {
                resolution_kind: "SESSION".to_string(),
                address_code: trimmed,
                session: Some(snap),
                waiting: None,
            })
        }
        AddressCodeKind::StaticTable { .. } => {
            let (table, store_name) = brew_table_repo::find_table_by_static_code(pool, &trimmed)
                .await?
                .ok_or_else(|| LiveBrewError::TableNotFound(trimmed.clone()))?;
            let _ = brew_table_repo::increment_table_scan(pool, table.id).await;

            let bound_session_id = get_table_pointer(hub, &trimmed)
                .await
                .or(table.active_brew_session_id);

            if let Some(sid) = bound_session_id {
                if let Some(row) = live_session_repo::fetch_by_id(pool, sid).await? {
                    let catchup = hub.get_catchup_curve(row.id).await;
                    let snap = session_row_to_snapshot(
                        &row,
                        &store_name,
                        Some(table.table_number.clone()),
                        Some(table.static_address_code.clone()),
                        catchup,
                    );
                    return Ok(ResolveAddressResponse {
                        resolution_kind: "SESSION".to_string(),
                        address_code: trimmed,
                        session: Some(snap),
                        waiting: None,
                    });
                }
            }

            Ok(ResolveAddressResponse {
                resolution_kind: "TABLE_WAITING".to_string(),
                address_code: trimmed.clone(),
                session: None,
                waiting: Some(TableWaitingInfoDto {
                    table_id: table.id,
                    table_number: table.table_number.clone(),
                    static_address_code: trimmed.clone(),
                    store_name,
                    livebrew_url: landing_livebrew_url(&trimmed),
                    message: format!(
                        "{} terhubung! Menunggu barista memulai seduhan Anda...",
                        table.table_number
                    ),
                }),
            })
        }
    }
}

use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State},
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse},
};
use futures_util::stream::Stream;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    core::{error::{ApiResponse, AppError, AppResult}, extractors::ValidatedJson, AppState},
    modules::live_brew::{
        dto::{ResolveAddressResponse, SendCheerRequest},
        error::LiveBrewError,
        protocol::ViewerDownstreamEvent,
        repositories::live_session_repo,
        services::{live_telemetry_hub::LiveTelemetryHub, table_routing_service},
    },
};

async fn relay_stream_events<F, Fut>(
    hub: Arc<LiveTelemetryHub>,
    session_id: Option<Uuid>,
    waiting_code: Option<String>,
    mut emit: F,
) where
    F: FnMut(Arc<ViewerDownstreamEvent>) -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let mut sid = session_id;
    if sid.is_none() {
        if let Some(code) = waiting_code {
            let mut tbl_rx = hub.subscribe_table_waiting(&code);
            while let Ok(ev) = tbl_rx.recv().await {
                if let ViewerDownstreamEvent::TableSessionBound { ref snapshot, .. } = *ev {
                    sid = Some(snapshot.session_id);
                }
                if !emit(ev).await || sid.is_some() {
                    break;
                }
            }
        }
    }
    if let Some(active_id) = sid {
        let mut sub = hub.subscribe_session(active_id);
        while let Ok(ev) = sub.recv().await {
            if !emit(ev).await {
                break;
            }
        }
    }
}

fn build_waiting_ev(w: &crate::modules::live_brew::dto::TableWaitingInfoDto) -> ViewerDownstreamEvent {
    ViewerDownstreamEvent::TableWaitingForBrew {
        table_number: w.table_number.clone(),
        static_address_code: w.static_address_code.clone(),
        store_name: w.store_name.clone(),
        livebrew_url: w.livebrew_url.clone(),
    }
}

pub async fn viewer_sse_stream(
    State(state): State<AppState>,
    Path(address_code): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let resolved = table_routing_service::resolve_address_code(
        &state.pool, &state.live_brew_hub, &address_code, state.config.jwt_secret_str(),
    )
    .await
    .map_err(AppError::from)?;

    let (tx, rx) = mpsc::channel::<Arc<ViewerDownstreamEvent>>(64);
    let hub = Arc::clone(&state.live_brew_hub);

    tokio::spawn(async move {
        let (sid, wait_code) = if let Some(snap) = resolved.session {
            let id = snap.session_id;
            let _ = tx.send(Arc::new(ViewerDownstreamEvent::InitialSessionSnapshot(snap))).await;
            (Some(id), None)
        } else if let Some(wait) = resolved.waiting {
            let code = wait.static_address_code.clone();
            let _ = tx.send(Arc::new(build_waiting_ev(&wait))).await;
            (None, Some(code))
        } else {
            (None, None)
        };

        relay_stream_events(hub, sid, wait_code, |ev| {
            let tx_c = tx.clone();
            async move { tx_c.send(ev).await.is_ok() }
        })
        .await;
    });

    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        let ev = rx.recv().await?;
        let item = serde_json::to_string(ev.as_ref())
            .map(|j| Event::default().data(j))
            .unwrap_or_else(|_| Event::default().comment("skip"));
        Some((Ok(item), rx))
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("ping")))
}

pub async fn viewer_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(address_code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let resolved = table_routing_service::resolve_address_code(
        &state.pool, &state.live_brew_hub, &address_code, state.config.jwt_secret_str(),
    )
    .await
    .map_err(AppError::from)?;
    Ok(ws.on_upgrade(move |socket| handle_viewer_socket(socket, state, resolved)))
}

async fn handle_viewer_socket(mut socket: WebSocket, state: AppState, resolved: ResolveAddressResponse) {
    let (sid, wait_code) = if let Some(snap) = resolved.session {
        let id = snap.session_id;
        let init = serde_json::to_string(&ViewerDownstreamEvent::InitialSessionSnapshot(snap));
        if let Ok(json) = init {
            if socket.send(Message::Text(json.into())).await.is_err() {
                return;
            }
        }
        (Some(id), None)
    } else if let Some(wait) = resolved.waiting {
        let code = wait.static_address_code.clone();
        if let Ok(json) = serde_json::to_string(&build_waiting_ev(&wait)) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                return;
            }
        }
        (None, Some(code))
    } else {
        (None, None)
    };

    let (tx, mut rx) = mpsc::channel::<Arc<ViewerDownstreamEvent>>(64);
    let hub = Arc::clone(&state.live_brew_hub);
    let forward = tokio::spawn(async move {
        relay_stream_events(hub, sid, wait_code, |ev| {
            let tx_c = tx.clone();
            async move { tx_c.send(ev).await.is_ok() }
        })
        .await;
    });

    while let Some(ev) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(ev.as_ref()) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    }
    forward.abort();
}

pub async fn send_cheer_reaction(
    State(state): State<AppState>,
    Path(address_code): Path<String>,
    ValidatedJson(req): ValidatedJson<SendCheerRequest>,
) -> AppResult<impl IntoResponse> {
    if !state.live_brew_hub.check_cheer_rate_limit(&address_code) {
        return Err(AppError::from(LiveBrewError::CheerRateLimited));
    }
    let resolved = table_routing_service::resolve_address_code(
        &state.pool, &state.live_brew_hub, &address_code, state.config.jwt_secret_str(),
    )
    .await
    .map_err(AppError::from)?;

    let Some(snap) = resolved.session else {
        return Err(AppError::from(LiveBrewError::SessionNotFound(address_code)));
    };
    let cheer: String = req.cheer_kind.chars().take(16).collect();
    let total = live_session_repo::increment_cheers(&state.pool, snap.session_id)
        .await
        .map_err(AppError::from)?;
    let ev = ViewerDownstreamEvent::CustomerCheer {
        cheer_kind: cheer,
        total_cheers: total.max(0) as u32,
    };
    if let Ok(json) = serde_json::to_string(&ev) {
        state.live_brew_hub.notify_barista_cheer(snap.session_id, json);
    }
    state.live_brew_hub.broadcast_to_session(snap.session_id, Arc::new(ev));
    Ok(ApiResponse::ok("Reaksi semangat terkirim ke Barista!", total))
}

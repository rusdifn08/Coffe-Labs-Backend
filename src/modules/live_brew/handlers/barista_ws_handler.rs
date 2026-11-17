use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    core::{error::AppError, extractors::AuthUser, AppState},
    modules::live_brew::{
        dto::CatchupCurvePoint,
        error::LiveBrewError,
        protocol::{BaristaUpstreamFrame, LiveBrewStage, ViewerDownstreamEvent},
        repositories::{brew_table_repo, live_session_repo},
        services::passport_archiver_service::build_passport_dto,
    },
};

pub async fn barista_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let row = live_session_repo::fetch_by_id(&state.pool, session_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::from(LiveBrewError::SessionNotFound(session_id.to_string())))?;

    if row.barista_user_id != claims.sub && !claims.role.eq_ignore_ascii_case("admin") {
        return Err(AppError::from(LiveBrewError::UnauthorizedBarista));
    }

    Ok(ws.on_upgrade(move |socket| handle_barista_socket(socket, state, session_id)))
}

async fn handle_barista_socket(mut socket: WebSocket, state: AppState, session_id: Uuid) {
    let mut cheer_rx = state.live_brew_hub.subscribe_barista_cheers(session_id);
    let mut last_db_persist = std::time::Instant::now();

    loop {
        tokio::select! {
            inbound = socket.recv() => {
                let Some(Ok(msg)) = inbound else {
                    break;
                };
                match msg {
                    Message::Text(txt) => {
                        if let Ok(frame) = serde_json::from_str::<BaristaUpstreamFrame>(&txt) {
                            process_barista_frame(&state, session_id, frame, &mut last_db_persist).await;
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            cheer = cheer_rx.recv() => {
                if let Ok(json) = cheer {
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

async fn process_barista_frame(
    state: &AppState,
    session_id: Uuid,
    frame: BaristaUpstreamFrame,
    last_db_persist: &mut std::time::Instant,
) {
    match frame {
        BaristaUpstreamFrame::StageTransition { stage, step_index, message, temp_c } => {
            let _ = live_session_repo::update_live_stage_snapshot(
                &state.pool,
                session_id,
                stage,
                step_index as i16,
                temp_c,
            )
            .await;
            let ev = ViewerDownstreamEvent::StageChanged {
                session_id,
                stage,
                step_index,
                stage_label_id: stage.stage_label_id().to_string(),
                message,
                elapsed_ms: 0,
                temp_c,
            };
            state.live_brew_hub.broadcast_to_session(session_id, Arc::new(ev));
        }
        BaristaUpstreamFrame::ScaleTelemetryTick { elapsed_ms, weight_g, flow_rate_ml_s, temp_c } => {
            let pt = CatchupCurvePoint {
                t_ms: elapsed_ms,
                w_g: weight_g,
                flow: flow_rate_ml_s,
                temp_c,
            };
            state.live_brew_hub.append_curve_point(session_id, pt.clone());
            state.live_brew_hub.broadcast_to_session(
                session_id,
                Arc::new(ViewerDownstreamEvent::TelemetryTick(pt)),
            );
            if last_db_persist.elapsed().as_secs() >= 2 {
                *last_db_persist = std::time::Instant::now();
                let pool = state.pool.clone();
                tokio::spawn(async move {
                    let _ = live_session_repo::update_telemetry_checkpoint(
                        &pool,
                        session_id,
                        (elapsed_ms / 1000) as i32,
                        weight_g,
                        flow_rate_ml_s,
                        temp_c,
                    )
                    .await;
                });
            }
        }
        BaristaUpstreamFrame::VideoStreamStatus { is_active, whep_url } => {
            let _ = live_session_repo::set_video_stream_flag(
                &state.pool,
                session_id,
                is_active,
                whep_url.as_deref(),
            )
            .await;
            state.live_brew_hub.broadcast_to_session(
                session_id,
                Arc::new(ViewerDownstreamEvent::VideoStatusChanged { is_active, whep_url }),
            );
        }
        BaristaUpstreamFrame::VideoKeyframe { jpeg_b64, ts_ms } => {
            if jpeg_b64.len() <= 160_000 {
                state.live_brew_hub.broadcast_to_session(
                    session_id,
                    Arc::new(ViewerDownstreamEvent::VideoKeyframeRelayed { jpeg_b64, ts_ms }),
                );
            }
        }
        BaristaUpstreamFrame::FinishAndServe {
            final_weight_g,
            total_duration_sec,
            accuracy_score_pct,
        } => {
            let curve = state.live_brew_hub.get_catchup_curve(session_id).await;
            let curve_json = serde_json::to_value(&curve).unwrap_or_default();
            let _ = live_session_repo::archive_completed_session(
                &state.pool,
                session_id,
                final_weight_g,
                total_duration_sec as i32,
                accuracy_score_pct,
                &curve_json,
            )
            .await;

            if let Ok(Some(row)) = live_session_repo::fetch_by_id(&state.pool, session_id).await {
                if let Some(tid) = row.table_id {
                    let _ = brew_table_repo::bind_active_session_on_table(&state.pool, tid, None).await;
                }
                let passport = build_passport_dto(
                    &row,
                    final_weight_g,
                    total_duration_sec as i32,
                    accuracy_score_pct,
                    row.cheers_reactions_count,
                );
                state.live_brew_hub.broadcast_to_session(
                    session_id,
                    Arc::new(ViewerDownstreamEvent::BrewCompletedPassport(passport)),
                );
                state.live_brew_hub.broadcast_to_session(
                    session_id,
                    Arc::new(ViewerDownstreamEvent::StageChanged {
                        session_id,
                        stage: LiveBrewStage::ReadyToServe,
                        step_index: 6,
                        stage_label_id: LiveBrewStage::ReadyToServe.stage_label_id().to_string(),
                        message: None,
                        elapsed_ms: total_duration_sec.saturating_mul(1000),
                        temp_c: row.current_temp_c,
                    }),
                );
            }
        }
    }
}

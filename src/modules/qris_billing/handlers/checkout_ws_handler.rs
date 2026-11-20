use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use chrono::{Duration, Utc};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;

use crate::core::{
    auth_extractor::AuthenticatedUser,
    error::{AppError, AppResult},
    AppState,
};
use crate::modules::qris_billing::{
    dto::QrisWsFrame,
    error::QrisBillingError,
    models::QrisInvoiceRow,
    repositories::{invoice_repo, subscription_repo},
    services::qris_ws_hub::global_qris_ws_hub,
};

pub async fn invoice_status_ws(
    auth: AuthenticatedUser,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(invoice_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let row = invoice_repo::find_invoice_by_id(&state.pool, invoice_id)
        .await?
        .ok_or(QrisBillingError::InvoiceNotFound)?;
    if row.user_id != auth.user_id && !auth.claims.role.eq_ignore_ascii_case("admin") {
        return Err(AppError::Forbidden("Akses WebSocket invoice ditolak.".into()));
    }

    Ok(ws.on_upgrade(move |socket| handle_invoice_ws(socket, state, invoice_id, row)))
}

async fn handle_invoice_ws(
    socket: WebSocket,
    state: AppState,
    invoice_id: Uuid,
    initial_row: QrisInvoiceRow,
) {
    let (mut sender, mut receiver) = socket.split();
    let hub = global_qris_ws_hub();
    let mut rx = hub.subscribe(invoice_id);

    let initial_frame = if initial_row.status.eq_ignore_ascii_case("PAID") {
        let user_opt = subscription_repo::fetch_user_for_token(&state.pool, initial_row.user_id)
            .await
            .ok()
            .flatten();
        let fresh_token = user_opt
            .as_ref()
            .and_then(|u| {
                state
                    .jwt
                    .generate_user_token(u.id, &u.username, &u.role, &u.tier, &u.status)
                    .ok()
            })
            .unwrap_or_default();
        let expires_at = user_opt
            .and_then(|u| u.subscription_expires_at)
            .unwrap_or_else(|| Utc::now() + Duration::days(i64::from(initial_row.duration_days)));
        QrisWsFrame::PaymentConfirmed {
            invoice_id,
            invoice_number: initial_row.invoice_number,
            new_tier: initial_row.target_tier,
            plan_code: initial_row.plan_code,
            subscription_expires_at: expires_at,
            fresh_access_token: fresh_token,
        }
    } else {
        QrisWsFrame::InvoiceStatus {
            invoice_id,
            status: initial_row.status,
            expires_at: initial_row.expires_at,
        }
    };

    if let Ok(json_txt) = serde_json::to_string(&initial_frame) {
        let _ = sender.send(Message::Text(json_txt.into())).await;
    }

    loop {
        tokio::select! {
            Ok(frame) = rx.recv() => {
                if let Ok(txt) = serde_json::to_string(&frame) {
                    if sender.send(Message::Text(txt.into())).await.is_err() {
                        break;
                    }
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Ping(p))) => { let _ = sender.send(Message::Pong(p)).await; }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
    hub.cleanup_idle_channel(invoice_id);
}

use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::stream::Stream;
use tokio::sync::mpsc;

use crate::{
    core::{error::AppError, AppState},
    modules::live_brew::{
        handlers::viewer_resolve_handler::resolve_stream_target,
        mirror_protocol::MirrorDownstreamFrame,
    },
};

pub async fn viewer_v2_sse_stream(
    State(state): State<AppState>,
    Path(target): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let resolved = resolve_stream_target(&state, &target).await?;
    let hub = Arc::clone(&state.live_brew_hub.mirror_hub);
    let (tx, rx) = mpsc::channel::<Arc<MirrorDownstreamFrame>>(64);

    tokio::spawn(async move {
        let init_frame = if let Some(ref snap) = resolved.active_brew_session {
            Arc::new(MirrorDownstreamFrame::MirrorSnapshot(snap.clone()))
        } else {
            Arc::new(MirrorDownstreamFrame::SeatPosUpdate(resolved.clone()))
        };
        let _ = tx.send(init_frame).await;

        let mut sub = if let Some(ref s) = resolved.active_brew_session {
            hub.subscribe_session(s.session_id)
        } else if let Some(ref seat) = resolved.seat {
            hub.subscribe_seat(seat.id)
        } else {
            return;
        };

        while let Ok(frame) = sub.recv().await {
            if tx.send(frame).await.is_err() {
                break;
            }
        }
    });

    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        let frame = rx.recv().await?;
        let item = serde_json::to_string(frame.as_ref())
            .map(|j| Event::default().data(j))
            .unwrap_or_else(|_| Event::default().comment("skip"));
        Some((Ok(item), rx))
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("ping")))
}

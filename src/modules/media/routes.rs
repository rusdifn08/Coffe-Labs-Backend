use axum::{extract::FromRef, routing::post, Router};

use crate::{
    core::{security::JwtKeyRing, storage::CloudinaryStorage, AppState},
    modules::media::handlers,
};

pub fn media_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    JwtKeyRing: FromRef<S>,
    CloudinaryStorage: FromRef<S>,
{
    Router::new().route(
        "/api/v1/media/upload-signature",
        post(handlers::issue_upload_signature),
    )
}

pub fn router() -> Router<AppState> {
    media_router::<AppState>()
}

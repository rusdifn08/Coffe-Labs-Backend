use axum::{
    routing::{get, post, put},
    Router,
};

use crate::core::AppState;
use crate::modules::roasteries::handlers::{business_studio_handler, public_catalog_handler};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/roasteries",
            get(public_catalog_handler::list_public_roasteries)
                .post(business_studio_handler::create_my_roastery),
        )
        .route(
            "/api/v1/roasteries/me",
            get(business_studio_handler::get_my_roastery)
                .put(business_studio_handler::update_my_roastery),
        )
        .route(
            "/api/v1/roasteries/me/lineups",
            post(business_studio_handler::add_lineup),
        )
        .route(
            "/api/v1/roasteries/me/lineups/{id}",
            put(business_studio_handler::update_lineup)
                .delete(business_studio_handler::delete_lineup),
        )
        .route(
            "/api/v1/roasteries/media/upload-signature",
            post(business_studio_handler::issue_roastery_upload_signature),
        )
        .route(
            "/api/v1/roasteries/{id_or_slug}",
            get(public_catalog_handler::get_public_roastery_detail),
        )
}

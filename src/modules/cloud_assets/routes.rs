use axum::{routing::get, Router};

use crate::{core::AppState, modules::cloud_assets::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/assets/storage-config", get(handlers::get_storage_config))
        .route("/api/v1/assets/datasets", get(handlers::list_datasets))
        .route(
            "/api/v1/assets/datasets/{key}",
            get(handlers::get_dataset).put(handlers::upsert_dataset),
        )
        .route(
            "/api/v1/assets/media",
            get(handlers::list_media_assets).put(handlers::upsert_media_assets),
        )
}

#[cfg(test)]
mod tests {
    use crate::modules::cloud_assets::repository::{validate_asset_path, validate_dataset_key};

    #[test]
    fn dataset_key_validation_accepts_clean_names() {
        assert_eq!(
            validate_dataset_key("Indonesia_Radar_Top_Places").ok().as_deref(),
            Some("indonesia_radar_top_places")
        );
        assert!(validate_dataset_key("../secret").is_err());
        assert!(validate_dataset_key("").is_err());
    }

    #[test]
    fn asset_path_validation_blocks_traversal() {
        assert_eq!(
            validate_asset_path("/images/beans/aceh_gayo.jpg").ok().as_deref(),
            Some("images/beans/aceh_gayo.jpg")
        );
        assert!(validate_asset_path("../../etc/passwd").is_err());
    }
}

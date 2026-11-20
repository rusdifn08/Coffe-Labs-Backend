use std::collections::BTreeMap;

use chrono::Utc;
use secrecy::ExposeSecret;
use sha1::{Digest, Sha1};
use tracing::warn;
use uuid::Uuid;

use crate::core::{
    error::{AppError, AppResult},
    storage::{CloudinaryStorage, CloudinaryUploadSignature},
};

/// Menghasilkan folder & signature upload Cloudinary ter-scope per `user_id` Business.
pub fn sign_roastery_asset_upload(
    storage: &CloudinaryStorage,
    user_id: Uuid,
    asset_kind: &str,
) -> AppResult<CloudinaryUploadSignature> {
    let kind_seg = match asset_kind.trim().to_ascii_lowercase().as_str() {
        "logo" => "logo",
        "banner" => "banner",
        "lineup" => "lineups",
        _ => {
            return Err(AppError::Validation(
                "asset_kind harus berupa 'logo', 'banner', atau 'lineup'.".to_string(),
            ));
        }
    };
    let folder = format!("coffeelabs/roasteries/{user_id}/{kind_seg}");
    storage.generate_upload_signature(&folder, None)
}

/// Menghapus aset gambar lama di Cloudinary secara asinkron (best-effort) tanpa memblokir response.
pub fn spawn_cleanup_old_asset(storage: CloudinaryStorage, public_id: Option<String>) {
    let Some(pid) = public_id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) else {
        return;
    };
    let Some(creds) = storage.credentials().cloned() else {
        return;
    };
    tokio::spawn(async move {
        let timestamp = Utc::now().timestamp();
        let params = BTreeMap::from([("public_id", pid.clone()), ("timestamp", timestamp.to_string())]);
        let joined: String = params.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
        let to_sign = format!("{joined}{}", creds.api_secret.expose_secret());
        let signature = hex::encode(Sha1::digest(to_sign.as_bytes()));
        let destroy_url = format!("https://api.cloudinary.com/v1_1/{}/image/destroy", creds.cloud_name);

        let form = [
            ("public_id", pid.as_str()),
            ("timestamp", &timestamp.to_string()),
            ("api_key", creds.api_key.as_str()),
            ("signature", signature.as_str()),
        ];

        let client = reqwest::Client::new();
        if let Err(err) = client.post(destroy_url).form(&form).send().await {
            warn!(public_id = %pid, error = %err, "Gagal membersihkan aset Cloudinary roastery lama.");
        }
    });
}

use axum::{extract::State, Json};

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{AuthUser, ValidatedJson},
        storage::{CloudinaryStorage, CloudinaryUploadSignature},
    },
    modules::media::dto::UploadSignatureRequest,
};

/// Menghasilkan signature SHA-1 Direct Upload untuk klien Flutter tanpa membebani RAM Render.
pub async fn issue_upload_signature(
    AuthUser(_claims): AuthUser,
    State(storage): State<CloudinaryStorage>,
    ValidatedJson(req): ValidatedJson<UploadSignatureRequest>,
) -> AppResult<Json<ApiResponse<CloudinaryUploadSignature>>> {
    let signed = storage.generate_upload_signature(&req.folder, req.public_id.as_deref())?;
    Ok(ApiResponse::ok(
        "Signature Direct Upload Cloudinary berhasil dibuat.",
        signed,
    ))
}

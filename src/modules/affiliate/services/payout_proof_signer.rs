use uuid::Uuid;

use crate::core::{
    error::AppResult,
    storage::{CloudinaryStorage, CloudinaryUploadSignature},
};

/// Menghasilkan signature direct-upload ke Cloudinary folder `coffeelabs/payout_proofs`.
pub fn sign_payout_proof_upload(
    storage: &CloudinaryStorage,
    withdrawal_id: Uuid,
) -> AppResult<CloudinaryUploadSignature> {
    let public_id = format!("payout_{}", withdrawal_id.simple());
    storage.generate_upload_signature("coffeelabs/payout_proofs", Some(&public_id))
}

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::modules::barista_toolkit::error::BaristaToolkitError;

pub const DEFAULT_DRM_ROOT_SECRET: &str = "coffee-labs-drm-root-key-2026-prod";
#[allow(dead_code)]
const CLBK_HEADER: &[u8; 6] = b"CLBK\x00\x01";

pub fn resolved_drm_secret() -> String {
    std::env::var("TOOLKIT_DRM_SECRET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_DRM_ROOT_SECRET.to_string())
}

fn server_master_wrap_key(drm_secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(format!("CLBK_MASTER_CEK_KEY_V1|{drm_secret}").as_bytes());
    hasher.finalize().into()
}

pub fn unwrap_master_cek(
    wrapped_blob: &[u8],
    book_code: &str,
    drm_secret: &str,
) -> Result<[u8; 32], BaristaToolkitError> {
    if wrapped_blob.len() < 12 + 32 + 16 {
        return Err(BaristaToolkitError::InvalidBookLicense(
            "Format wrapped_master_cek tidak lengkap.".to_string(),
        ));
    }
    let key_bytes = server_master_wrap_key(drm_secret);
    let cipher = Aes256Gcm::new(&key_bytes.into());
    let nonce = Nonce::from_slice(&wrapped_blob[..12]);
    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: &wrapped_blob[12..],
                aad: book_code.as_bytes(),
            },
        )
        .map_err(|_| {
            BaristaToolkitError::InvalidBookLicense("Verifikasi master CEK E-Book gagal.".to_string())
        })?;

    if plaintext.len() != 32 {
        return Err(BaristaToolkitError::InvalidBookLicense(
            "Panjang CEK hasil unwrap tidak valid.".to_string(),
        ));
    }
    let mut cek = [0u8; 32];
    cek.copy_from_slice(&plaintext);
    Ok(cek)
}

pub fn derive_device_wrapping_key(
    client_ikm: &[u8],
    device_id: &str,
    user_id: Uuid,
    ebook_id: Uuid,
) -> Result<([u8; 32], String), BaristaToolkitError> {
    let info = format!("CLBK-WRAP|{user_id}|{ebook_id}");
    let hk = Hkdf::<Sha256>::new(Some(device_id.as_bytes()), client_ikm);
    let mut okm = [0u8; 32];
    hk.expand(info.as_bytes(), &mut okm).map_err(|_| {
        BaristaToolkitError::InvalidBookLicense("HKDF key derivation gagal.".to_string())
    })?;
    Ok((okm, info))
}

pub fn wrap_cek_for_device(
    cek: &[u8; 32],
    device_id: &str,
    user_id: Uuid,
    ebook_id: Uuid,
    client_ikm: &[u8],
) -> Result<(String, String, String), BaristaToolkitError> {
    let (k_wrap, aad_info) = derive_device_wrapping_key(client_ikm, device_id, user_id, ebook_id)?;
    let cipher = Aes256Gcm::new(&k_wrap.into());
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: cek,
                aad: aad_info.as_bytes(),
            },
        )
        .map_err(|_| BaristaToolkitError::InvalidBookLicense("Enkripsi wrap CEK gagal.".to_string()))?;

    Ok((B64.encode(ciphertext), B64.encode(nonce_bytes), aad_info))
}

#[allow(dead_code)]
pub fn encrypt_clbook_package(cek: &[u8; 32], plaintext_pdf: &[u8]) -> Result<Vec<u8>, BaristaToolkitError> {
    let cipher = Aes256Gcm::new(cek.into());
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext_pdf,
                aad: CLBK_HEADER,
            },
        )
        .map_err(|_| BaristaToolkitError::InvalidBookLicense("Enkripsi paket .clbook gagal.".to_string()))?;

    let mut out = Vec::with_capacity(CLBK_HEADER.len() + 12 + ct.len());
    out.extend_from_slice(CLBK_HEADER);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

#[allow(dead_code)]
pub fn decrypt_clbook_package(cek: &[u8; 32], clbook_blob: &[u8]) -> Result<Vec<u8>, BaristaToolkitError> {
    if clbook_blob.len() < 6 + 12 + 16 || &clbook_blob[..6] != CLBK_HEADER {
        return Err(BaristaToolkitError::InvalidBookLicense(
            "Header biner .clbook tidak valid.".to_string(),
        ));
    }
    let cipher = Aes256Gcm::new(cek.into());
    let nonce = Nonce::from_slice(&clbook_blob[6..18]);
    cipher
        .decrypt(
            nonce,
            Payload {
                msg: &clbook_blob[18..],
                aad: CLBK_HEADER,
            },
        )
        .map_err(|_| {
            BaristaToolkitError::InvalidBookLicense("Tag autentikasi GCM .clbook tidak cocok.".to_string())
        })
}

use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum LiveBrewError {
    #[error("Kode alamat Live Brew 13-digit tidak valid atau gagal verifikasi checksum.")]
    InvalidBrewAddressCode,
    #[error("Sesi seduhan Live Brew '{0}' tidak ditemukan.")]
    SessionNotFound(String),
    #[error("Meja seduh statis '{0}' tidak ditemukan.")]
    TableNotFound(String),
    #[error("Meja atau kursi kustom '{0}' tidak ditemukan.")]
    SeatNotFound(String),
    #[error("Slug kafe atau meja '{0}' sudah digunakan. Pilih slug unik lainnya.")]
    SeatSlugConflict(String),
    #[error("Fitur Studio Barcode Meja & POS Sync hanya tersedia untuk akun Business.")]
    BusinessTierRequired,
    #[error("Versi state kadaluwarsa ({0}); sinkronisasi ulang diperlukan.")]
    StaleVersion(i64),
    #[error("Hanya barista pemilik sesi yang boleh mengirim telemetri seduhan ini.")]
    UnauthorizedBarista,
    #[error("Terlalu banyak reaksi dikirim dalam waktu singkat.")]
    CheerRateLimited,
    #[error("Kesalahan database Live Brew: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<LiveBrewError> for AppError {
    fn from(err: LiveBrewError) -> Self {
        match err {
            LiveBrewError::InvalidBrewAddressCode => AppError::BadRequest(err.to_string()),
            LiveBrewError::SessionNotFound(_)
            | LiveBrewError::TableNotFound(_)
            | LiveBrewError::SeatNotFound(_) => AppError::NotFound(err.to_string()),
            LiveBrewError::SeatSlugConflict(_) | LiveBrewError::StaleVersion(_) => {
                AppError::Conflict(err.to_string())
            }
            LiveBrewError::UnauthorizedBarista | LiveBrewError::BusinessTierRequired => {
                AppError::Forbidden(err.to_string())
            }
            LiveBrewError::CheerRateLimited => AppError::RateLimited(err.to_string()),
            LiveBrewError::Database(e) => AppError::Database(e),
        }
    }
}

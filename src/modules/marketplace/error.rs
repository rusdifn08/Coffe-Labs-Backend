use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum MarketplaceError {
    #[error("Kode voucher '{0}' tidak ditemukan atau sudah tidak berlaku.")]
    InvalidVoucher(String),

    #[error("Pembayaran marketplace saat ini hanya tersedia via QRIS.")]
    QrisOnlyPaymentRequired,

    #[error("Produk kopi tidak ditemukan atau belum terverifikasi.")]
    ProductNotFound,

    #[error("Stok varian kopi tidak mencukupi untuk kuantitas yang diminta.")]
    InsufficientStock,

    #[error("Pesanan marketplace '{0}' tidak ditemukan.")]
    OrderNotFound(String),
}

impl From<MarketplaceError> for AppError {
    fn from(err: MarketplaceError) -> Self {
        match err {
            MarketplaceError::InvalidVoucher(_)
            | MarketplaceError::QrisOnlyPaymentRequired
            | MarketplaceError::InsufficientStock => AppError::BadRequest(err.to_string()),
            MarketplaceError::ProductNotFound | MarketplaceError::OrderNotFound(_) => {
                AppError::NotFound(err.to_string())
            }
        }
    }
}

use crate::core::error::AppError;

#[derive(Debug)]
pub enum AffiliateError {
    ProgramInactive,
    ProAccountRequired,
    InvalidCodeFormat(String),
    CodeNotFound,
    CodeAlreadyTaken,
    CustomCodeAlreadyClaimed,
    SelfReferralForbidden,
    SameDeviceFraudBlocked,
    CircularReferralBlocked,
    RateLimited,
    BelowMinWithdrawal(i64),
    InsufficientBalance { available: i64, requested: i64 },
    ConcurrentWithdrawalLocked,
    WithdrawalNotFound,
    InvalidWithdrawalTransition(String),
    InvalidPayoutDetails(String),
}

impl From<AffiliateError> for AppError {
    fn from(err: AffiliateError) -> Self {
        match err {
            AffiliateError::ProgramInactive => {
                AppError::BadRequest("Program Afiliasi Coffee Labs sedang dinonaktifkan sementara.".into())
            }
            AffiliateError::ProAccountRequired => AppError::Forbidden(
                "Program Afiliasi 20% eksklusif untuk akun Coffee Labs Pro & Business. Upgrade akun Anda ke Pro untuk mulai mendapatkan komisi 20%!".into(),
            ),
            AffiliateError::InvalidCodeFormat(msg) => AppError::Validation(msg),
            AffiliateError::CodeNotFound => {
                AppError::NotFound("Kode referral tidak ditemukan atau pemilik kode belum aktif sebagai afiliator Pro.".into())
            }
            AffiliateError::CodeAlreadyTaken => {
                AppError::Conflict("Kode referral kustom tersebut sudah digunakan pengguna lain.".into())
            }
            AffiliateError::CustomCodeAlreadyClaimed => {
                AppError::BadRequest("Anda sudah pernah mengklaim perubahan kode referral kustom 1x.".into())
            }
            AffiliateError::SelfReferralForbidden => {
                AppError::BadRequest("Tidak dapat menggunakan kode referral milik akun Anda sendiri.".into())
            }
            AffiliateError::SameDeviceFraudBlocked => AppError::Forbidden(
                "Kode referral ditolak oleh Anti-Fraud Guard karena terdeteksi berasal dari perangkat fisik yang sama.".into(),
            ),
            AffiliateError::CircularReferralBlocked => {
                AppError::BadRequest("Kode referral hanya berlaku untuk pembelian pertama atau tidak boleh saling tukar kode.".into())
            }
            AffiliateError::RateLimited => {
                AppError::RateLimited("Terlalu banyak percobaan validasi kode. Coba lagi dalam 1 menit.".into())
            }
            AffiliateError::BelowMinWithdrawal(min_idr) => AppError::Validation(format!(
                "Minimal penarikan saldo afiliasi adalah Rp{min_idr}."
            )),
            AffiliateError::InsufficientBalance { available, requested } => {
                AppError::BadRequest(format!(
                    "Saldo tersedia Rp{available} tidak mencukupi untuk penarikan Rp{requested}."
                ))
            }
            AffiliateError::ConcurrentWithdrawalLocked => AppError::Conflict(
                "Permintaan penarikan Anda sedang diproses. Mohon tunggu beberapa detik.".into(),
            ),
            AffiliateError::WithdrawalNotFound => {
                AppError::NotFound("Tiket penarikan saldo tidak ditemukan.".into())
            }
            AffiliateError::InvalidWithdrawalTransition(msg) => AppError::BadRequest(msg),
            AffiliateError::InvalidPayoutDetails(msg) => AppError::Validation(msg),
        }
    }
}

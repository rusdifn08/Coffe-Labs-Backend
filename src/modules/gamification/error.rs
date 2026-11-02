use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum GamificationError {
    #[error("Sesi seduh tidak ditemukan atau bukan milik akun Anda.")]
    SessionNotFound,

    #[error("Sesi seduh ini sudah diklaim sebelumnya (Idempotent Guard).")]
    SessionAlreadyCompleted,

    #[error("Anti-Cheat Guard: Durasi seduh ({actual}s) terlalu singkat dibanding target resep ({min_required}s minimal 80%). Klaim XP ditolak.")]
    AntiCheatDurationTooShort { actual: i64, min_required: i64 },

    #[error("Batas maksimal XP harian ({cap} XP) telah tercapai untuk hari ini. Lanjutkan seduhan esok hari!")]
    DailyXpCapReached { cap: i32 },

    #[error("Kunci idempotensi '{0}' sudah pernah diproses pada buku besar XP.")]
    DuplicateIdempotencyKey(String),
}

impl From<GamificationError> for AppError {
    fn from(err: GamificationError) -> Self {
        match err {
            GamificationError::SessionNotFound => AppError::NotFound(err.to_string()),
            GamificationError::SessionAlreadyCompleted
            | GamificationError::DuplicateIdempotencyKey(_) => AppError::Conflict(err.to_string()),
            GamificationError::AntiCheatDurationTooShort { .. } => {
                AppError::Validation(err.to_string())
            }
            GamificationError::DailyXpCapReached { .. } => AppError::Forbidden(err.to_string()),
        }
    }
}

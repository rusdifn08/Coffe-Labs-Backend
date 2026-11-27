use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("Koordinat asal atau tujuan tidak valid: {0}")]
    InvalidCoordinates(String),

    #[error("Rute jalan tidak ditemukan untuk moda ini: {0}")]
    RouteNotFound(String),

    #[error("Tujuan tidak dapat dijangkau oleh moda transportasi yang dipilih")]
    UnreachableDestination,

    #[error("Semua mesin rute eksternal gagal merespons: {0}")]
    UpstreamFailed(String),
}

impl From<RoutingError> for AppError {
    fn from(err: RoutingError) -> Self {
        match err {
            RoutingError::InvalidCoordinates(msg) => AppError::Validation(msg),
            RoutingError::RouteNotFound(msg) => AppError::NotFound(msg),
            RoutingError::UnreachableDestination => {
                AppError::BadRequest(err.to_string())
            }
            RoutingError::UpstreamFailed(msg) => AppError::ServiceUnavailable(msg),
        }
    }
}

pub fn validate_coord_pair(lat: f64, lng: f64) -> Result<(f64, f64), RoutingError> {
    if !lat.is_finite() || !lng.is_finite() {
        return Err(RoutingError::InvalidCoordinates(
            "Latitude/longitude harus berupa angka berhingga.".to_string(),
        ));
    }
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
        return Err(RoutingError::InvalidCoordinates(
            "Koordinat berada di luar rentang bumi WGS-84.".to_string(),
        ));
    }
    if lat.abs() < 0.01 && lng.abs() < 0.01 {
        return Err(RoutingError::InvalidCoordinates(
            "Koordinat (0, 0) bukan lokasi valid.".to_string(),
        ));
    }
    Ok((lat, lng))
}

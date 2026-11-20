use crate::core::error::AppError;

#[derive(Debug)]
pub enum BeansRadarError {
    InvalidCoordinates(String),
    RadiusOutOfBounds(f64),
    PlaceNotFound,
}

impl From<BeansRadarError> for AppError {
    fn from(err: BeansRadarError) -> Self {
        match err {
            BeansRadarError::InvalidCoordinates(msg) => AppError::Validation(msg),
            BeansRadarError::RadiusOutOfBounds(r) => AppError::Validation(format!(
                "Radius {} km di luar batas yang didukung (0.5 km – 500.0 km).",
                r
            )),
            BeansRadarError::PlaceNotFound => {
                AppError::NotFound("Lokasi kedai kopi atau roastery tidak ditemukan.".to_string())
            }
        }
    }
}

pub const EARTH_RADIUS_METERS: f64 = 6_371_008.8;
const ROAD_TORTUOSITY_FACTOR: f64 = 1.32;
const WALK_SPEED_MPS: f64 = 1.35;
const RIDE_SPEED_MPS: f64 = 6.8;
const BAYESIAN_PRIOR_C: f64 = 4.45;
const BAYESIAN_WEIGHT_M: f64 = 25.0;

/// Memvalidasi bahwa koordinat WGS-84 berada di rentang valid & bukan (0, 0).
pub fn validate_coordinates(lat: f64, lng: f64) -> Result<(), String> {
    if !lat.is_finite() || !lng.is_finite() {
        return Err("Koordinat harus berupa angka valid.".to_string());
    }
    if lat < -90.0 || lat > 90.0 || lng < -180.0 || lng > 180.0 {
        return Err("Koordinat latitude/longitude berada di luar jangkauan WGS-84.".to_string());
    }
    if lat.abs() < 0.01 && lng.abs() < 0.01 {
        return Err("Koordinat (0, 0) tidak valid untuk pencarian lokasi.".to_string());
    }
    Ok(())
}

/// Memproyeksikan (lat, lng) derajat ke vektor satuan 3D pada bola bumi R^3.
pub fn to_unit_sphere(lat: f64, lng: f64) -> [f64; 3] {
    let phi = lat.to_radians();
    let lambda = lng.to_radians();
    let cos_phi = phi.cos();
    [cos_phi * lambda.cos(), cos_phi * lambda.sin(), phi.sin()]
}

/// Mengonversi jarak permukaan bumi (meter) menjadi kuadrat jarak tali busur R^3 untuk `rstar::locate_within_distance`.
pub fn chord_dist_sq_for_meters(meters: f64) -> f64 {
    let safe_m = meters.max(0.0).min(std::f64::consts::PI * EARTH_RADIUS_METERS);
    let half_angle = safe_m / (2.0 * EARTH_RADIUS_METERS);
    let s = half_angle.sin();
    4.0 * s * s
}

/// Menghitung jarak permukaan bumi sferoidal (Haversine) dalam satuan meter.
pub fn haversine_meters(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let d_phi = (lat2 - lat1).to_radians();
    let d_lambda = (lng2 - lng1).to_radians();

    let sin_dphi = (d_phi * 0.5).sin();
    let sin_dlmb = (d_lambda * 0.5).sin();
    let a = (sin_dphi * sin_dphi) + phi1.cos() * phi2.cos() * (sin_dlmb * sin_dlmb);
    let clamped = a.clamp(0.0, 1.0);
    let c = 2.0 * clamped.sqrt().atan2((1.0 - clamped).sqrt());
    EARTH_RADIUS_METERS * c
}

/// Skor rating Bayesian batas bawah untuk mencegah toko berulasan minim menggeser roastery mapan.
pub fn compute_bayesian_score(rating: f32, reviews_count: i32) -> f32 {
    let r = (rating as f64).clamp(0.0, 5.0);
    if r <= 0.0 {
        return BAYESIAN_PRIOR_C as f32;
    }
    let n = (reviews_count.max(0)) as f64;
    ((n * r + BAYESIAN_WEIGHT_M * BAYESIAN_PRIOR_C) / (n + BAYESIAN_WEIGHT_M)) as f32
}

/// Skor Smart Discovery Score untuk menyeimbangkan rating, kedekatan, status buka, dan partner.
pub fn compute_smart_discovery_score(
    bayesian_score: f32,
    distance_meters: f64,
    is_open_now: bool,
    is_verified_partner: bool,
) -> f64 {
    let dist_km = (distance_meters / 1000.0).max(0.0);
    let rating_part = (bayesian_score as f64 / 5.0) * 0.45;
    let proximity_part = (-dist_km / 4.0).exp() * 0.35;
    let open_part = if is_open_now { 0.12 } else { 0.0 };
    let partner_part = if is_verified_partner { 0.08 } else { 0.0 };
    rating_part + proximity_part + open_part + partner_part
}

/// Menghitung estimasi waktu tempuh realistis perkotaan (jalan kaki <= 1.2 km, motor/mobil).
pub fn estimate_eta_minutes(distance_meters: f64) -> (Option<i32>, i32) {
    let effective_road_m = distance_meters.max(10.0) * ROAD_TORTUOSITY_FACTOR;
    let walk = if distance_meters <= 1200.0 {
        Some(((effective_road_m / (WALK_SPEED_MPS * 60.0)).ceil() as i32).max(1))
    } else {
        None
    };
    let ride = ((effective_road_m / (RIDE_SPEED_MPS * 60.0)).ceil() as i32).max(1);
    (walk, ride)
}

/// Memformat jarak ke label UI ringkas ("340 m" atau "1.8 km").
pub fn format_distance_label(distance_meters: f64) -> String {
    if distance_meters < 1000.0 {
        format!("{} m", distance_meters.round() as i64)
    } else {
        format!("{:.1} km", distance_meters / 1000.0)
    }
}

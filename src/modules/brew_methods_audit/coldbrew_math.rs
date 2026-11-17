use crate::modules::brew_methods_audit::dto::{ColdBrewCheckpointDto, ColdBrewPlanResponse};

const BASE_IMMERSION_SECS_AT_20C: f64 = 43_200.0; // 12 jam referensi suhu ruang sejuk 20C
const ARRHENIUS_Q10: f64 = 1.48;
const DROP_VOLUME_ML: f64 = 0.05;

/// Menghitung durasi ekstraksi Cold Brew berbasis kinetika Arrhenius Q10 = 1.48.
/// Suhu kulkas (4C) memperpanjang durasi optimal ke ~19–22 jam; suhu ruang (22C) ~11–12 jam.
pub fn compute_cold_brew_plan(
    brew_mode: &str,
    coffee_dose_g: f32,
    water_ml: f32,
    ambient_temp_c: f32,
    drip_rate_bpm: Option<i16>,
) -> ColdBrewPlanResponse {
    let safe_dose = coffee_dose_g.clamp(10.0, 1000.0);
    let safe_water = water_ml.clamp(80.0, 5000.0);
    let normalized_mode = brew_mode.trim().to_ascii_uppercase();

    let (recommended_sec, drip_interval_ms) = if normalized_mode == "KYOTO_DRIP" {
        let bpm = f64::from(drip_rate_bpm.unwrap_or(40).clamp(15, 90));
        let ml_per_sec = (bpm * DROP_VOLUME_ML) / 60.0;
        let secs = (f64::from(safe_water) / ml_per_sec).round() as i32;
        let clamped_sec = secs.clamp(14_400, 43_200);
        let interval = (60_000.0 / bpm).round() as i32;
        (clamped_sec, Some(interval))
    } else {
        let temp = f64::from(ambient_temp_c.clamp(2.0, 28.0));
        let factor = ARRHENIUS_Q10.powf((20.0 - temp) / 10.0);
        let secs = (BASE_IMMERSION_SECS_AT_20C * factor).round() as i32;
        (secs.clamp(28_800, 86_400), None)
    };

    let ratio = safe_water / safe_dose;
    let absorbed_water = safe_dose * 2.0;
    let concentrate_yield = (safe_water - absorbed_water).max(safe_water * 0.55);
    // Target kekuatan siap minum 1:15 dari konsentrat pekat
    let ideal_total_drink = safe_dose * 15.0;
    let dilution = (ideal_total_drink - concentrate_yield).max(0.0);

    let mut checkpoints = vec![ColdBrewCheckpointDto {
        offset_sec: 3_600,
        label: "01:00 Agitasi Kerak & Degassing".to_string(),
        instruction: "Tekan perlahan crust bubuk kopi mengapung agar ekstraksi merata."
            .to_string(),
    }];
    if recommended_sec >= 28_800 {
        checkpoints.push(ColdBrewCheckpointDto {
            offset_sec: recommended_sec / 2,
            label: "50% Checkpoint Ekstraksi".to_string(),
            instruction: if normalized_mode == "KYOTO_DRIP" {
                "Periksa kelancaran katup jarum (40–45 BPM stabil) agar tidak macet.".to_string()
            } else {
                "Pastikan wadah tetap tertutup rapat dan stabil suhunya.".to_string()
            },
        });
    }

    ColdBrewPlanResponse {
        brew_mode: normalized_mode,
        recommended_duration_sec: recommended_sec,
        recommended_hours: ((recommended_sec as f32) / 3600.0 * 10.0).round() / 10.0,
        drip_interval_ms,
        ratio_label: format!("1:{:.1}", ratio),
        concentrate_yield_ml: concentrate_yield.round(),
        suggested_dilution_water_or_ice_ml: dilution.round(),
        checkpoints,
    }
}

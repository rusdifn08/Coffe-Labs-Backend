use crate::modules::barista_toolkit::dto::{BrewDoctorDiagnoseRequest, BrewDoctorDiagnosisResponse};

pub fn diagnose_extraction(req: &BrewDoctorDiagnoseRequest) -> BrewDoctorDiagnosisResponse {
    let mut x_acc = 0.0_f64;
    let mut y_acc = 0.0_f64;
    let mut sour_hits = 0_i32;
    let mut bitter_hits = 0_i32;
    let mut channeling_flag = false;

    for raw in &req.symptom_tags {
        match raw.trim().to_ascii_lowercase().as_str() {
            "sour" | "salty" | "grassy" | "acidic" | "quick_finish" => {
                x_acc -= 0.55;
                sour_hits += 1;
            }
            "bitter" | "astringent" | "dry" | "ashy" | "hollow" => {
                x_acc += 0.55;
                bitter_hits += 1;
            }
            "channeling" | "uneven" => {
                channeling_flag = true;
            }
            "weak" | "watery" | "thin" => y_acc -= 0.5,
            "heavy" | "muddy" | "intense" => y_acc += 0.5,
            _ => {}
        }
    }

    let time_delta = (req.actual_sec - req.target_sec) as f64;
    x_acc += (time_delta / 60.0).clamp(-0.4, 0.4);

    let ratio = (req.water_ml / req.dose_g).max(1.0);
    if ratio >= 16.5 {
        y_acc -= 0.3;
    } else if ratio <= 14.2 {
        y_acc += 0.3;
    }

    let x_ext = x_acc.clamp(-1.0, 1.0);
    let y_str = y_acc.clamp(-1.0, 1.0);

    let estimated_ey = (20.0 + x_ext * 3.2).clamp(15.0, 25.0);
    let estimated_tds = (1.38 + y_str * 0.25 + (15.5 - ratio) * 0.04).clamp(0.95, 1.85);

    if channeling_flag || (sour_hits > 0 && bitter_hits > 0) {
        return BrewDoctorDiagnosisResponse {
            diagnosis_code: "UNEVEN_CHANNELING".to_string(),
            diagnosis_title: "Channeling & Ekstraksi Tidak Merata".to_string(),
            x_extraction: (x_ext * 100.0).round() / 100.0,
            y_strength: (y_str * 100.0).round() / 100.0,
            estimated_ey_pct: (estimated_ey * 10.0).round() / 10.0,
            estimated_tds_pct: (estimated_tds * 100.0).round() / 100.0,
            grind_clicks_delta: -1,
            temp_c_delta: 0,
            recommended_ratio: "1:15.5".to_string(),
            explanation: "Rasa asam dan pahit sepat muncul bersamaan karena sebagian air menerobos celah bed kopi.".to_string(),
            pour_technique_advice: "Lakukan Rao Spin lembut saat blooming 35 detik dan tuang melingkar pelan di pusat tanpa menembak tepi kertas filter.".to_string(),
        };
    }

    if x_ext < -0.25 {
        BrewDoctorDiagnosisResponse {
            diagnosis_code: "UNDER_EXTRACTED".to_string(),
            diagnosis_title: "Under-Extracted (Kurang Terekstrak)".to_string(),
            x_extraction: (x_ext * 100.0).round() / 100.0,
            y_strength: (y_str * 100.0).round() / 100.0,
            estimated_ey_pct: (estimated_ey * 10.0).round() / 10.0,
            estimated_tds_pct: (estimated_tds * 100.0).round() / 100.0,
            grind_clicks_delta: -2,
            temp_c_delta: 2,
            recommended_ratio: "1:15".to_string(),
            explanation: "Senyawa gula manis belum larut sempurna sehingga asam tajam dan asin mendominasi.".to_string(),
            pour_technique_advice: "Haluskan gilingan -2 klik, naikkan suhu +2°C, dan bagi fase akhir menjadi 2 tuangan terukur.".to_string(),
        }
    } else if x_ext > 0.25 {
        BrewDoctorDiagnosisResponse {
            diagnosis_code: "OVER_EXTRACTED".to_string(),
            diagnosis_title: "Over-Extracted (Terlalu Terekstrak)".to_string(),
            x_extraction: (x_ext * 100.0).round() / 100.0,
            y_strength: (y_str * 100.0).round() / 100.0,
            estimated_ey_pct: (estimated_ey * 10.0).round() / 10.0,
            estimated_tds_pct: (estimated_tds * 100.0).round() / 100.0,
            grind_clicks_delta: 2,
            temp_c_delta: -2,
            recommended_ratio: "1:15.5".to_string(),
            explanation: "Kontak air terlalu lama atau terlalu halus sehingga tanin pahit dan rasa kering ikut larut.".to_string(),
            pour_technique_advice: "Kasarkan gilingan +2 klik, turunkan suhu -2°C, atau pakai Thermal Drop 70°C di tuangan terakhir.".to_string(),
        }
    } else if y_str < -0.25 {
        BrewDoctorDiagnosisResponse {
            diagnosis_code: "WEAK_RATIO".to_string(),
            diagnosis_title: "Body Terlalu Ringan (Weak / Watery)".to_string(),
            x_extraction: (x_ext * 100.0).round() / 100.0,
            y_strength: (y_str * 100.0).round() / 100.0,
            estimated_ey_pct: (estimated_ey * 10.0).round() / 10.0,
            estimated_tds_pct: (estimated_tds * 100.0).round() / 100.0,
            grind_clicks_delta: -1,
            temp_c_delta: 0,
            recommended_ratio: "1:14.5".to_string(),
            explanation: "Konsentrasi padatan terlarut (TDS) di bawah rentang manis Golden Cup.".to_string(),
            pour_technique_advice: "Padatkan rasio ke 1:14.5 atau bagi 60% air akhir menjadi 3x tuangan bertahap (4:6 Body).".to_string(),
        }
    } else if y_str > 0.25 {
        BrewDoctorDiagnosisResponse {
            diagnosis_code: "HEAVY_RATIO".to_string(),
            diagnosis_title: "Body Terlalu Pekat (Heavy / Muddy)".to_string(),
            x_extraction: (x_ext * 100.0).round() / 100.0,
            y_strength: (y_str * 100.0).round() / 100.0,
            estimated_ey_pct: (estimated_ey * 10.0).round() / 10.0,
            estimated_tds_pct: (estimated_tds * 100.0).round() / 100.0,
            grind_clicks_delta: 1,
            temp_c_delta: -1,
            recommended_ratio: "1:16".to_string(),
            explanation: "Kadar TDS tinggi menutupi kejernihan aroma buah dan bunga.".to_string(),
            pour_technique_advice: "Longgarkan rasio ke 1:16 atau tambahkan 15ml air bypass hangat pasca drawdown.".to_string(),
        }
    } else {
        BrewDoctorDiagnosisResponse {
            diagnosis_code: "BALANCED_GOLDEN_CUP".to_string(),
            diagnosis_title: "Golden Cup Seimbang (Sweet & Clean)".to_string(),
            x_extraction: (x_ext * 100.0).round() / 100.0,
            y_strength: (y_str * 100.0).round() / 100.0,
            estimated_ey_pct: (estimated_ey * 10.0).round() / 10.0,
            estimated_tds_pct: (estimated_tds * 100.0).round() / 100.0,
            grind_clicks_delta: 0,
            temp_c_delta: 0,
            recommended_ratio: format!("1:{:.1}", ratio),
            explanation: "Ekstraksi dan kekuatan berada di zona manis ideal SCA Golden Cup!".to_string(),
            pour_technique_advice: "Pertahankan ukuran gilingan, suhu, dan tempo tuang untuk konsistensi harian.".to_string(),
        }
    }
}

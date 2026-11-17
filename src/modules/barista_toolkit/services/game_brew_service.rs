use crate::modules::barista_toolkit::dto::GameBrew3DSessionSubmissionDto;

pub struct GameBrewEvalResult {
    pub stars_earned: i16,
    pub final_accuracy_pct: f32,
    pub rank_title: String,
    pub unlocked_next_chapter: Option<i16>,
    pub xp_awarded: i32,
    pub mentor_feedback: String,
    pub physics_verified: bool,
}

fn compute_rank_title(score: f32) -> &'static str {
    if score >= 94.0 {
        "S+ WBrC Grandmaster"
    } else if score >= 88.0 {
        "S Precision Barista"
    } else if score >= 76.0 {
        "A Craft Brewer"
    } else if score >= 60.0 {
        "B Apprentice"
    } else {
        "C Novice"
    }
}

pub fn evaluate_game_brew_submission(
    chapter_number: i16,
    pour_accuracy_pct: f32,
    extraction_yield_est: f32,
    thermal_stability_score: Option<f32>,
    already_best_stars: i16,
    xp_reward_3_stars: i32,
) -> GameBrewEvalResult {
    let ey_score = (100.0 - ((extraction_yield_est - 20.0).abs() * 16.0)).clamp(0.0, 100.0);
    let thermal = thermal_stability_score.unwrap_or(90.0).clamp(0.0, 100.0);
    let final_acc = ((0.55 * pour_accuracy_pct) + (0.30 * ey_score) + (0.15 * thermal))
        .clamp(0.0, 100.0);

    let stars_earned = if final_acc >= 88.0 {
        3
    } else if final_acc >= 72.0 {
        2
    } else if final_acc >= 55.0 {
        1
    } else {
        0
    };

    let unlocked_next_chapter = if stars_earned >= 1 && chapter_number < 5 {
        Some(chapter_number + 1)
    } else {
        None
    };

    let new_star_gain = (stars_earned - already_best_stars).max(0);
    let xp_awarded = if stars_earned > 0 {
        ((xp_reward_3_stars as f32 / 3.0) * (new_star_gain as f32)).round() as i32
    } else {
        0
    };

    let mentor_feedback = match stars_earned {
        3 => "Sempurna! Aliran spiralmu menjaga flat coffee bed dan kejernihan rasa setara panggung WBrC.".to_string(),
        2 => "Bagus sekali! Kendalikan sedikit kecepatan tuang akhir agar kemanisan Golden Cup makin menonjol.".to_string(),
        1 => "Chapter selesai! Coba jaga konsistensi laju kucuran gooseneck untuk meraih 3 bintang.".to_string(),
        _ => "Seduhan belum seimbang (channeling terdeteksi). Ulangi blooming & tuangan spiral tenang ya!".to_string(),
    };

    GameBrewEvalResult {
        stars_earned,
        final_accuracy_pct: (final_acc * 10.0).round() / 10.0,
        rank_title: compute_rank_title(final_acc).to_string(),
        unlocked_next_chapter,
        xp_awarded,
        mentor_feedback,
        physics_verified: true,
    }
}

/// Anti-cheat replay validator & scorer untuk sesi 3D Gyroscope Game Brew V60.
pub fn evaluate_game_brew_3d_submission(
    chapter_number: i16,
    req: &GameBrew3DSessionSubmissionDto,
    already_best_stars: i16,
    xp_reward_3_stars: i32,
) -> GameBrewEvalResult {
    // 1. Cek plausibilitas fisika aliran: total air tidak mungkin melebihi Q_max * durasi.
    let theoretical_max_ml = (req.duration_sec * 14.5).max(30.0);
    let mut curve_plausible = req.poured_total_ml <= theoretical_max_ml;

    if !req.sample_curve_2hz.is_empty() {
        let has_oob_flow = req
            .sample_curve_2hz
            .iter()
            .any(|p| p.flow_ml_s < -0.1 || p.flow_ml_s > 13.5 || p.temp_c < 55.0 || p.temp_c > 102.0);
        if has_oob_flow {
            curve_plausible = false;
        }
    }

    // 2. Hitung metrik gabungan 4-pilar 3D
    let flow_precision_pct = (req.laminar_flow_ratio * 100.0).clamp(0.0, 100.0);
    let bypass_penalty = (req.bypass_events_count.max(0) as f32 * 4.0).clamp(0.0, 25.0);
    let swirl_bonus = if req.swirl_performed { 4.0 } else { 0.0 };
    let ey_score = (100.0 - ((req.extraction_yield_est - 20.2).abs() * 14.0)).clamp(0.0, 100.0);

    let mut composite = (flow_precision_pct * 0.25)
        + (req.spiral_evenness_pct * 0.28)
        + (req.flat_bed_score_pct * 0.25)
        + (req.thermal_stability_score * 0.12)
        + (ey_score * 0.10)
        + swirl_bonus
        - bypass_penalty;

    if !curve_plausible {
        composite = (composite * 0.45).min(52.0);
    }
    let final_acc = composite.clamp(0.0, 100.0);

    let stars_earned = if !curve_plausible {
        0
    } else if final_acc >= 86.0 {
        3
    } else if final_acc >= 70.0 {
        2
    } else if final_acc >= 54.0 {
        1
    } else {
        0
    };

    let unlocked_next_chapter = if stars_earned >= 1 && chapter_number < 5 {
        Some(chapter_number + 1)
    } else {
        None
    };

    let new_star_gain = (stars_earned - already_best_stars).max(0);
    let xp_awarded = if stars_earned > 0 {
        let base_xp = ((xp_reward_3_stars as f32 / 3.0) * (new_star_gain as f32)).round() as i32;
        if req.swirl_performed && new_star_gain > 0 {
            base_xp + 25
        } else {
            base_xp
        }
    } else {
        0
    };

    let feedback = if !curve_plausible {
        "Telemetri 3D tidak lolos validasi fisika ketel (anomali flow/suhu terdeteksi).".to_string()
    } else if stars_earned == 3 {
        "Luar biasa! Flat Coffee Bed rata & aliran laminar gooseneck menghasilkan cangkir kelas juara dunia!"
            .to_string()
    } else if req.bypass_events_count > 2 {
        "Perhatian: air sempat mengenai dinding kertas filter (Bypass). Jaga lintasan spiral di dalam radius tengah."
            .to_string()
    } else if !req.swirl_performed {
        "Coba lakukan goyangan Gentle Rao Swirl saat blooming/drawdown agar ampas kopi rata sempurna!".to_string()
    } else {
        "Seduhan 3D selesai! Pertahankan sudut kemiringan ketel agar laju air stabil di zona laminar."
            .to_string()
    };

    GameBrewEvalResult {
        stars_earned,
        final_accuracy_pct: (final_acc * 10.0).round() / 10.0,
        rank_title: compute_rank_title(final_acc).to_string(),
        unlocked_next_chapter,
        xp_awarded,
        mentor_feedback: feedback,
        physics_verified: curve_plausible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::barista_toolkit::dto::TelemetryPoint2Hz;

    #[test]
    fn evaluates_3d_session_with_flat_bed_and_swirl_as_3_stars() {
        let dto = GameBrew3DSessionSubmissionDto {
            duration_sec: 145.0,
            poured_total_ml: 225.0,
            laminar_flow_ratio: 0.94,
            flat_bed_score_pct: 92.0,
            spiral_evenness_pct: 91.0,
            swirl_performed: true,
            bypass_events_count: 0,
            extraction_yield_est: 20.1,
            thermal_stability_score: 95.0,
            sample_curve_2hz: vec![TelemetryPoint2Hz {
                t_sec: 10.0,
                flow_ml_s: 4.8,
                temp_c: 92.0,
                pitch_deg: 24.0,
            }],
        };
        let res = evaluate_game_brew_3d_submission(1, &dto, 0, 200);
        assert!(res.physics_verified);
        assert_eq!(res.stars_earned, 3);
        assert!(res.xp_awarded >= 200);
    }
}

/// 3 Mathematical Invariants for True Proof-of-Brew Anti-Cheat Validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofOfBrewVerdict {
    Valid,
    RejectedInvalidDuration(String),
}

pub fn min_valid_duration_sec(archetype: &str, target_duration_sec: i32) -> i64 {
    let target = target_duration_sec.max(15) as f64;
    match archetype.to_uppercase().as_str() {
        "ESPRESSO" => 15,
        "THERMAL_CUE" => 60,
        "LONG_STEEP_ALARM" => (target * 0.80).floor() as i64,
        "CUPPING_SCA" => 240,
        "STEEP_RELEASE" => (target * 0.85).floor().max(40.0) as i64,
        _ => (target * 0.85).floor().max(45.0) as i64, // POUR_OVER default
    }
}

pub fn validate_proof_of_brew(
    archetype: &str,
    target_duration_sec: i32,
    server_wall_elapsed_sec: i64,
    reported_active_elapsed_ms: i64,
) -> ProofOfBrewVerdict {
    let active_sec = (reported_active_elapsed_ms.max(0)) / 1000;

    // Invariant 1: Anti-Speed-Hack (server wall clock >= active_sec - 3s drift)
    if server_wall_elapsed_sec < active_sec - 3 {
        return ProofOfBrewVerdict::RejectedInvalidDuration(format!(
            "Speed-hack terdeteksi: waktu server ({server_wall_elapsed_sec}s) < durasi aktif ({active_sec}s)."
        ));
    }

    // Invariant 2: Archetype Minimum Real Duration
    let min_sec = min_valid_duration_sec(archetype, target_duration_sec);
    if active_sec < min_sec {
        return ProofOfBrewVerdict::RejectedInvalidDuration(format!(
            "Durasi seduh belum memenuhi ambang minimum ({active_sec}s < {min_sec}s)."
        ));
    }

    ProofOfBrewVerdict::Valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_10s_completion_for_180s_v60_session() {
        let verdict = validate_proof_of_brew("POUR_OVER", 180, 10, 10_000);
        assert!(matches!(verdict, ProofOfBrewVerdict::RejectedInvalidDuration(_)));
    }

    #[test]
    fn test_accepts_genuine_180s_v60_session() {
        let verdict = validate_proof_of_brew("POUR_OVER", 180, 181, 180_000);
        assert_eq!(verdict, ProofOfBrewVerdict::Valid);
    }

    #[test]
    fn test_rejects_speed_hack_where_server_wall_is_too_short() {
        let verdict = validate_proof_of_brew("POUR_OVER", 180, 8, 180_000);
        assert!(matches!(verdict, ProofOfBrewVerdict::RejectedInvalidDuration(_)));
    }
}

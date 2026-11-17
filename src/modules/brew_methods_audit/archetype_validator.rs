use crate::modules::brew_methods_audit::{
    dto::{AuditRecipeRequest, AuditRecipeResponse},
    models::BrewExecutionArchetype,
};

pub fn classify_method_archetype(
    method: &str,
    category: &str,
    title: &str,
    water_temp_c: Option<f32>,
    ice_weight_g: Option<f32>,
) -> (BrewExecutionArchetype, bool) {
    let joined = format!(
        "{} {} {}",
        method.to_ascii_lowercase(),
        category.to_ascii_lowercase(),
        title.to_ascii_lowercase()
    );
    let m_lower = method.to_ascii_lowercase();
    let is_v60 = m_lower.contains("v60") || category.to_ascii_lowercase().contains("v60");

    let is_espresso_method =
        m_lower.contains("espresso") || category.to_ascii_lowercase().contains("espresso");
    let is_kyoto_cold_drip = joined.contains("kyoto")
        && (joined.contains("drip") || joined.contains("cold") || joined.contains("oji"))
        && !is_v60
        && !is_espresso_method
        && !joined.contains("japanese");

    if joined.contains("cold brew")
        || joined.contains("cold drip")
        || joined.contains("mizudashi")
        || is_kyoto_cold_drip
        || (water_temp_c.unwrap_or(92.0) <= 10.0 && joined.contains("cold"))
    {
        return (BrewExecutionArchetype::LongHorizonPersistentAlarm, is_v60);
    }

    if joined.contains("japanese")
        || joined.contains("es di server")
        || (ice_weight_g.unwrap_or(0.0) > 0.0
            && !joined.contains("espresso")
            && !joined.contains("cold brew"))
    {
        return (BrewExecutionArchetype::FlashChillIcedFilter, is_v60);
    }

    if joined.contains("espresso") && !joined.contains("moka") {
        return (BrewExecutionArchetype::SubSecondPressureEspresso, is_v60);
    }

    if joined.contains("moka")
        || joined.contains("turkish")
        || joined.contains("cezve")
        || joined.contains("ibrik")
        || joined.contains("syphon")
        || joined.contains("siphon")
        || joined.contains("vacuum")
    {
        return (BrewExecutionArchetype::ThermalCuePhaseBrewer, is_v60);
    }

    if joined.contains("aeropress")
        || joined.contains("clever")
        || joined.contains("french press")
        || joined.contains("cupping")
        || joined.contains("vietnam")
        || joined.contains("phin")
        || joined.contains("switch")
    {
        return (BrewExecutionArchetype::SteepAndReleaseImmersion, is_v60);
    }

    (BrewExecutionArchetype::PercolationPourOver, is_v60)
}

pub fn validate_recipe_audit(req: &AuditRecipeRequest) -> AuditRecipeResponse {
    let (archetype, is_v60) = classify_method_archetype(
        &req.method,
        &req.category,
        &req.title,
        req.water_temp_celsius,
        req.ice_weight_grams,
    );

    let step_water: f32 = req.steps.iter().map(|s| s.water_added_grams).sum();
    let step_dur: i32 = req.steps.iter().map(|s| s.duration_seconds).sum();
    let mut warnings = Vec::new();

    if is_v60 {
        return AuditRecipeResponse {
            archetype: archetype.as_str().to_string(),
            v60_protected: true,
            is_cold_brew_alarm_only: false,
            is_valid: true,
            step_water_sum_grams: step_water,
            step_duration_sum_seconds: step_dur,
            warnings,
        };
    }

    if (step_water - req.total_water_weight_grams).abs() > 1.0 {
        warnings.push(format!(
            "Total air langkah ({step_water}g) tidak sama dengan totalWaterWeightGrams ({}g).",
            req.total_water_weight_grams
        ));
    }
    if (step_dur - req.total_time_seconds).abs() > 2 {
        warnings.push(format!(
            "Total durasi langkah ({step_dur}s) tidak sama dengan totalTimeSeconds ({}s).",
            req.total_time_seconds
        ));
    }
    if archetype == BrewExecutionArchetype::LongHorizonPersistentAlarm
        && req.total_time_seconds < 21_600
    {
        warnings.push(
            "Cold Brew/Drip tidak memakai timer seduh instan dan durasinya minimal 6 jam (21.600s)."
                .to_string(),
        );
    }
    if archetype == BrewExecutionArchetype::FlashChillIcedFilter
        && req.ice_weight_grams.unwrap_or(0.0) <= 0.0
    {
        warnings.push("Japanese Iced wajib mencatat berat es server (iceWeightGrams > 0).".to_string());
    }

    let is_cold = archetype == BrewExecutionArchetype::LongHorizonPersistentAlarm;
    let is_valid = warnings.is_empty();
    AuditRecipeResponse {
        archetype: archetype.as_str().to_string(),
        v60_protected: false,
        is_cold_brew_alarm_only: is_cold,
        is_valid,
        step_water_sum_grams: step_water,
        step_duration_sum_seconds: step_dur,
        warnings,
    }
}

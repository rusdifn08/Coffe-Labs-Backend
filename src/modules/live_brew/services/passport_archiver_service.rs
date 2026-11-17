use crate::modules::live_brew::{
    address_cipher::landing_livebrew_url,
    dto::{CatchupCurvePoint, DigitalCupPassportDto, LiveBrewSnapshotDto},
    models::LiveBrewSessionRow,
};

const APK_RELEASE_URL: &str =
    "https://github.com/rusdifn08/coffe-labs-mobile/releases/download/v2.4.0/CoffeeLabs-arm64-v8a.apk";

pub fn parse_tasting_notes(raw: &serde_json::Value) -> Vec<String> {
    raw.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_stored_curve(raw: &serde_json::Value) -> Vec<CatchupCurvePoint> {
    serde_json::from_value::<Vec<CatchupCurvePoint>>(raw.clone()).unwrap_or_default()
}

pub fn session_row_to_snapshot(
    row: &LiveBrewSessionRow,
    store_name: &str,
    table_number: Option<String>,
    static_address_code: Option<String>,
    live_catchup: Vec<CatchupCurvePoint>,
) -> LiveBrewSnapshotDto {
    let catchup = if live_catchup.is_empty() {
        parse_stored_curve(&row.recorded_telemetry_curve)
    } else {
        live_catchup
    };

    LiveBrewSnapshotDto {
        session_id: row.id,
        dynamic_address_code: row.dynamic_address_code.clone(),
        livebrew_url: landing_livebrew_url(&row.dynamic_address_code),
        deep_link_url: format!("coffeelabs://livebrew/{}", row.dynamic_address_code),
        table_number,
        static_address_code,
        store_name: store_name.to_string(),
        barista_display_name: row.barista_display_name.clone(),
        customer_display_name: row.customer_display_name.clone(),
        recipe_id: row.recipe_id,
        recipe_title: row.recipe_title.clone(),
        brew_method: row.brew_method.clone(),
        bean_name: row.bean_name.clone(),
        roastery_name: row.roastery_name.clone(),
        bean_origin: row.bean_origin.clone(),
        bean_process: row.bean_process.clone(),
        tasting_notes: parse_tasting_notes(&row.tasting_notes),
        target_dose_g: row.target_dose_g,
        target_water_ml: row.target_water_ml,
        target_temp_c: row.target_temp_c,
        secondary_temp_c: row.secondary_temp_c,
        target_duration_sec: row.target_duration_sec,
        recipe_steps: row.recipe_steps_json.clone(),
        current_stage: row.current_stage,
        stage_label_id: row.current_stage.stage_label_id().to_string(),
        current_step_index: row.current_step_index,
        elapsed_sec: row.elapsed_sec,
        current_weight_g: row.current_weight_g,
        current_flow_rate_ml_s: row.current_flow_rate_ml_s,
        current_temp_c: row.current_temp_c,
        is_video_stream_active: row.is_video_stream_active,
        webrtc_whep_endpoint: row.webrtc_whep_endpoint.clone(),
        cheers_reactions_count: row.cheers_reactions_count,
        final_accuracy_score_pct: row.final_accuracy_score_pct,
        catchup_curve_points: catchup,
    }
}

pub fn build_passport_dto(
    row: &LiveBrewSessionRow,
    final_weight_g: f32,
    total_duration_sec: i32,
    accuracy_score_pct: f32,
    cheers_count: i32,
) -> DigitalCupPassportDto {
    DigitalCupPassportDto {
        session_id: row.id,
        dynamic_address_code: row.dynamic_address_code.clone(),
        recipe_title: row.recipe_title.clone(),
        bean_name: row.bean_name.clone(),
        roastery_name: row.roastery_name.clone(),
        brew_method: row.brew_method.clone(),
        tasting_notes: parse_tasting_notes(&row.tasting_notes),
        final_weight_g,
        total_duration_sec,
        accuracy_score_pct,
        cheers_reactions_count: cheers_count,
        livebrew_url: landing_livebrew_url(&row.dynamic_address_code),
        deep_link_url: format!("coffeelabs://livebrew/{}", row.dynamic_address_code),
        apk_download_url: APK_RELEASE_URL.to_string(),
    }
}

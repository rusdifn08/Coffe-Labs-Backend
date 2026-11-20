use chrono::Utc;

use crate::modules::live_brew::{
    mirror_dto::{BusinessBrandingDto, CafeSeatDto, LiveSessionMirrorSnapshot},
    mirror_models::{
        build_branded_seat_deeplink, build_branded_seat_url, build_personal_share_url,
        BusinessBrandingRow, CafeSeatRow, LiveBrewRealSessionRow,
    },
    mirror_protocol::{
        BrewerMirrorCommand, CafeSeatKind, LiveBrewMirrorPhase, LiveTimerStatus,
    },
};

pub fn row_to_branding_dto(r: &BusinessBrandingRow) -> BusinessBrandingDto {
    BusinessBrandingDto {
        store_id: r.store_id,
        cafe_slug: r.cafe_slug.clone(),
        cafe_display_name: r.cafe_display_name.clone(),
        logo_cloudinary_url: r.logo_cloudinary_url.clone(),
        primary_color_hex: r.primary_color_hex.clone(),
        accent_color_hex: r.accent_color_hex.clone(),
        welcome_message: r.welcome_message.clone(),
        wifi_ssid: r.wifi_ssid.clone(),
        wifi_password: r.wifi_password.clone(),
        instagram_handle: r.instagram_handle.clone(),
        show_pos_queue_position: r.show_pos_queue_position,
        show_live_bean_details: r.show_live_bean_details,
    }
}

pub fn row_to_seat_dto(r: &CafeSeatRow, cafe_slug: &str) -> CafeSeatDto {
    CafeSeatDto {
        id: r.id,
        store_id: r.store_id,
        seat_kind: CafeSeatKind::from_str_lossy(&r.seat_kind),
        seat_label: r.seat_label.clone(),
        seat_slug: r.seat_slug.clone(),
        zone_area: r.zone_area.clone(),
        capacity_persons: r.capacity_persons,
        numeric_fallback_code: r.numeric_fallback_code.clone(),
        branded_url: build_branded_seat_url(cafe_slug, &r.seat_slug),
        deep_link_url: build_branded_seat_deeplink(cafe_slug, &r.seat_slug),
        active_live_session_id: r.active_live_session_id,
        active_pos_order_id: r.active_pos_order_id,
        active_pos_order_number: r.active_pos_order_number.clone(),
        active_pos_order_status: r.active_pos_order_status.clone(),
        qr_scan_count: r.qr_scan_count,
        is_active: r.is_active,
    }
}

pub fn row_to_mirror_snapshot(
    r: &LiveBrewRealSessionRow,
    branded_override: Option<(&str, &str)>,
    active_viewers: i32,
) -> LiveSessionMirrorSnapshot {
    let phase = LiveBrewMirrorPhase::from_str_lossy(&r.phase);
    let (share_url, deep_link_url) = match branded_override {
        Some((c_slug, s_slug)) => (build_branded_seat_url(c_slug, s_slug), build_branded_seat_deeplink(c_slug, s_slug)),
        None => (build_personal_share_url(&r.share_code), format!("coffeelabs://livebrew/{}", r.share_code)),
    };
    LiveSessionMirrorSnapshot {
        session_id: r.id,
        share_code: r.share_code.clone(),
        share_url,
        deep_link_url,
        is_business_session: r.is_business_session,
        store_id: r.store_id,
        seat_id: r.seat_id,
        pos_order_id: r.pos_order_id,
        pos_order_number: r.pos_order_number.clone(),
        pos_order_status: r.pos_order_status.clone(),
        brewer_display_name: r.brewer_display_name.clone(),
        brewer_avatar_url: r.brewer_avatar_url.clone(),
        customer_name: r.customer_name.clone(),
        phase,
        phase_label_id: phase.label_id().to_string(),
        state_version: r.state_version,
        selected_method_id: r.selected_method_id.clone(),
        selected_method_name: r.selected_method_name.clone(),
        selected_recipe_id: r.selected_recipe_id,
        selected_recipe_title: r.selected_recipe_title.clone(),
        recipe_snapshot_json: r.recipe_snapshot_json.clone(),
        selected_bean_id: r.selected_bean_id,
        bean_snapshot_json: r.bean_snapshot_json.clone(),
        timer_status: LiveTimerStatus::from_str_lossy(&r.timer_status),
        timer_accumulated_ms: r.timer_accumulated_ms,
        timer_anchor_server_ms: r.timer_anchor_server_ms,
        current_step_index: r.current_step_index,
        current_weight_g: r.current_weight_g,
        current_flow_rate_ml_s: r.current_flow_rate_ml_s,
        current_temp_c: r.current_temp_c,
        telemetry_curve_json: r.telemetry_curve_json.clone(),
        cheers_count: r.cheers_count,
        active_viewers_count: active_viewers.max(r.active_viewers_count),
        server_epoch_ms: Utc::now().timestamp_millis(),
    }
}

/// Mesin transisi deterministik Server-Authoritative untuk state 1:1 Live Brew v2.0.
pub fn apply_mirror_command(
    snap: &mut LiveSessionMirrorSnapshot,
    cmd: &BrewerMirrorCommand,
    now_ms: i64,
) -> bool {
    match cmd {
        BrewerMirrorCommand::NtpPing { .. } => return false,
        BrewerMirrorCommand::SelectRecipePreview {
            method_id,
            method_name,
            recipe_id,
            recipe_title,
            recipe_snapshot_json,
        } => {
            snap.phase = LiveBrewMirrorPhase::BrewerSelectingRecipe;
            snap.selected_method_id = Some(method_id.clone());
            snap.selected_method_name = Some(method_name.clone());
            snap.selected_recipe_id = *recipe_id;
            snap.selected_recipe_title = Some(recipe_title.clone());
            snap.recipe_snapshot_json = recipe_snapshot_json.clone();
        }
        BrewerMirrorCommand::SelectBeanPreview {
            bean_id,
            bean_snapshot_json,
        } => {
            snap.phase = LiveBrewMirrorPhase::BrewerSelectingBean;
            snap.selected_bean_id = *bean_id;
            snap.bean_snapshot_json = bean_snapshot_json.clone();
        }
        BrewerMirrorCommand::ConfirmReadyToBrew => {
            snap.phase = LiveBrewMirrorPhase::PreBrewCalibrated;
            snap.timer_status = LiveTimerStatus::Idle;
            snap.timer_accumulated_ms = 0;
            snap.timer_anchor_server_ms = None;
            snap.current_step_index = 0;
        }
        BrewerMirrorCommand::TimerStartOrResume => {
            if snap.timer_status != LiveTimerStatus::Running {
                snap.timer_anchor_server_ms = Some(now_ms);
                snap.timer_status = LiveTimerStatus::Running;
                snap.phase = LiveBrewMirrorPhase::BrewingActive;
                if snap.pos_order_number.is_some() {
                    snap.pos_order_status = Some("BREWING".to_string());
                }
            }
        }
        BrewerMirrorCommand::TimerPause => {
            if snap.timer_status == LiveTimerStatus::Running {
                if let Some(anchor) = snap.timer_anchor_server_ms {
                    snap.timer_accumulated_ms += (now_ms - anchor).max(0);
                }
                snap.timer_anchor_server_ms = None;
                snap.timer_status = LiveTimerStatus::Paused;
                snap.phase = LiveBrewMirrorPhase::BrewingPaused;
            }
        }
        BrewerMirrorCommand::TimerReset => {
            snap.timer_accumulated_ms = 0;
            snap.timer_anchor_server_ms = None;
            snap.timer_status = LiveTimerStatus::Idle;
            snap.current_step_index = 0;
            snap.phase = LiveBrewMirrorPhase::PreBrewCalibrated;
        }
        BrewerMirrorCommand::AdvanceStepOrTelemetry {
            current_step_index,
            weight_g,
            flow_rate_ml_s,
            temp_c,
        } => {
            snap.current_step_index = *current_step_index;
            snap.current_weight_g = *weight_g;
            snap.current_flow_rate_ml_s = *flow_rate_ml_s;
            snap.current_temp_c = *temp_c;
        }
        BrewerMirrorCommand::CompleteAndServe { final_weight_g, .. } => {
            if snap.timer_status == LiveTimerStatus::Running {
                if let Some(anchor) = snap.timer_anchor_server_ms {
                    snap.timer_accumulated_ms += (now_ms - anchor).max(0);
                }
            }
            snap.timer_anchor_server_ms = None;
            snap.timer_status = LiveTimerStatus::Finished;
            snap.current_weight_g = *final_weight_g;
            snap.phase = LiveBrewMirrorPhase::ServingReadyPassport;
            if snap.pos_order_number.is_some() {
                snap.pos_order_status = Some("READY_TO_DELIVER".to_string());
            }
        }
    }

    snap.state_version += 1;
    snap.phase_label_id = snap.phase.label_id().to_string();
    snap.server_epoch_ms = now_ms;
    true
}

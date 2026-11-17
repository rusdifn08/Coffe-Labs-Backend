use std::sync::Arc;

use chrono::Utc;

use crate::{
    core::AppState,
    modules::monitoring::{
        error::MonitoringError,
        models::{MonitoringMemoryGuardStats, PlatformOverviewResponse, PublicMonitoringSummary},
        repositories::overview_query_repo,
        services::batch_flusher_worker::MPSC_QUEUE_CAPACITY,
    },
};

fn collect_ram_stats(state: &AppState) -> MonitoringMemoryGuardStats {
    MonitoringMemoryGuardStats {
        mpsc_queue_capacity: MPSC_QUEUE_CAPACITY,
        mpsc_pending_batches: state.monitoring.ingest_queue.pending_count(),
        dropped_batches_total: state.monitoring.ingest_queue.dropped_total(),
        bitmap_days_in_ram: state.monitoring.bitmaps.entry_count(),
        live_mem_entries: state.monitoring.live_presence.mem_entry_count(),
    }
}

pub async fn get_cached_overview(state: &AppState) -> Result<Arc<PlatformOverviewResponse>, MonitoringError> {
    let cache_key = "global_overview".to_string();
    if let Some(hit) = state.monitoring.l1_caches.overview.get(&cache_key).await {
        return Ok(hit);
    }

    let today = Utc::now().date_naive();
    let (agg, top_recipes, feat_res, live_map) = tokio::join!(
        overview_query_repo::query_overview_aggregates(&state.pool),
        overview_query_repo::query_top_brew_recipes(&state.pool, 8),
        overview_query_repo::query_feature_minutes_7d(&state.pool),
        state.monitoring.live_presence.live_snapshot_map(),
    );

    let agg = agg?;
    let top_brew_recipes = top_recipes.unwrap_or_default();
    let (mins_by_feat, brews_7d) = feat_res.unwrap_or_default();

    let (dau_today, wau_7d, mau_30d, stickiness_pct) =
        state.monitoring.bitmaps.compute_dau_wau_mau(today);
    let feature_adoption = state
        .monitoring
        .bitmaps
        .compute_feature_adoption_7d(today, &mins_by_feat);
    let cohorts = state.monitoring.bitmaps.compute_retention_cohorts(today);

    let overview = Arc::new(PlatformOverviewResponse {
        online_now: live_map.len(),
        total_accounts: agg.total_accounts,
        trial_accounts: agg.trial_accounts,
        pro_accounts: agg.pro_accounts,
        business_accounts: agg.business_accounts,
        dau_today,
        wau_7d,
        mau_30d,
        stickiness_pct,
        avg_today_screen_time_sec: agg.avg_today_screen_time_sec,
        total_brews_7d: brews_7d,
        at_risk_high_count: agg.at_risk_high_count,
        dormant_count: agg.dormant_count,
        feature_adoption,
        cohorts,
        top_brew_recipes,
        ram_guard: collect_ram_stats(state),
        generated_at: Utc::now(),
    });

    state
        .monitoring
        .l1_caches
        .overview
        .insert(cache_key, Arc::clone(&overview))
        .await;

    Ok(overview)
}

pub async fn get_public_summary(state: &AppState) -> PublicMonitoringSummary {
    match get_cached_overview(state).await {
        Ok(ov) => PublicMonitoringSummary {
            online_now: ov.online_now,
            total_accounts: ov.total_accounts,
            dau_today: ov.dau_today,
            wau_7d: ov.wau_7d,
            mau_30d: ov.mau_30d,
            stickiness_pct: ov.stickiness_pct,
            feature_adoption: ov.feature_adoption.clone(),
            top_brew_recipes: ov.top_brew_recipes.clone(),
            ram_guard: ov.ram_guard.clone(),
        },
        Err(_) => PublicMonitoringSummary {
            online_now: state.monitoring.live_presence.count_live_now(),
            total_accounts: 0,
            dau_today: 0,
            wau_7d: 0,
            mau_30d: 0,
            stickiness_pct: 0.0,
            feature_adoption: Vec::new(),
            top_brew_recipes: Vec::new(),
            ram_guard: collect_ram_stats(state),
        },
    }
}

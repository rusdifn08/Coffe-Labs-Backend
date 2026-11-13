use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Json,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use utoipa::OpenApi;

use crate::{
    core::{
        cache::{
            AppCaches, CHAT_MEMBER_CACHE_MAX_CAPACITY, FCM_TOKEN_CACHE_MAX_CAPACITY,
            PROFILE_CACHE_MAX_CAPACITY,
        },
        error::{ApiResponse, AppResult},
        rabbitmq::{
            topology::DEFAULT_ROUTING_KEY, CONSUMER_PREFETCH_COUNT, DEFAULT_EXCHANGE,
            DEFAULT_QUEUE,
        },
        redis::{REDIS_POOL_SIZE, REDIS_STREAM_MAXLEN},
        telemetry::{EndpointMetricSummary, RequestTrace, TelemetryRegistry},
        AppState,
    },
    modules::{
        auth::{
            dto::{
                AuthSessionPayload, ChangePasswordRequest, LoginRequest, RefreshTokenRequest,
                RegisterRequest, UnifiedAuthResponse,
            },
            models::{RefreshTokenSession, UserSummary},
        },
        beans::{
            dto::{BeanCatalogQuery, DialInCalculationRequest, DialInRecommendationResponse},
            models::{CuratedBeanRow, DialInPourStep},
        },
        brew_logs::{dto::BrewAnalyticsResponse, models::MethodFrequency},
        marketplace::{
            dto::{PlaceOrderRequest, ValidateVoucherRequest, VoucherValidationResponse},
            models::MarketplaceOrderRow,
        },
        platform::{
            dto::{CreateFeedbackRequest, VersionCheckResponse},
            models::{AppVersionConfig, Feedback, FeedbackStats, UpdateAction},
        },
        rbac::{
            dto::{QuotaStatusResponse, SubscriptionPlanDto, UpgradeTierRequest},
            models::{AccountTier, PermissionScope},
        },
        social::{
            dto::{
                CopyRecipeResponse, CreateCommentRequest, CreateSocialPostRequest,
                FollowStatsResponse, LikePostResponse, SocialPostResponse,
            },
            models::SocialComment,
        },
        telemetry::{
            dashboard_ui::render_dashboard_html,
            dto::DashboardTelemetrySnapshot,
            models::{DatabasePoolMetrics, DomainEntityCounts, ServiceModuleInfo},
            service,
        },
        users::models::{AdminUserStats, BaristaProfile},
    },
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Coffee Labs Enterprise Rust Axum API",
        version = "2.0.0",
        description = "High-performance, low-memory, zero-trust Rust Axum backend powered by NeonDB Serverless PostgreSQL, mimalloc, Ed25519 JWT, and real-time APM telemetry."
    ),
    components(schemas(
        UserSummary, RefreshTokenSession, RegisterRequest, LoginRequest, RefreshTokenRequest,
        ChangePasswordRequest, AuthSessionPayload, UnifiedAuthResponse,
        AccountTier, PermissionScope, SubscriptionPlanDto, QuotaStatusResponse, UpgradeTierRequest,
        SocialPostResponse, CreateSocialPostRequest, CreateCommentRequest, SocialComment,
        LikePostResponse, CopyRecipeResponse, FollowStatsResponse,
        BaristaProfile, AdminUserStats,
        AppVersionConfig, UpdateAction, VersionCheckResponse, CreateFeedbackRequest, Feedback, FeedbackStats,
        CuratedBeanRow, DialInPourStep, BeanCatalogQuery, DialInCalculationRequest, DialInRecommendationResponse,
        BrewAnalyticsResponse, MethodFrequency,
        ValidateVoucherRequest, VoucherValidationResponse, PlaceOrderRequest, MarketplaceOrderRow,
        DashboardTelemetrySnapshot, DatabasePoolMetrics, DomainEntityCounts, ServiceModuleInfo,
        EndpointMetricSummary, RequestTrace
    ))
)]
pub struct ApiDoc;

fn redis_summary_json(state: &AppState) -> Value {
    json!({
        "configured": state.redis.is_configured(),
        "connected": state.redis.is_connected(),
        "pool_size": REDIS_POOL_SIZE,
        "stream_maxlen": REDIS_STREAM_MAXLEN
    })
}

fn caches_summary_json(caches: &AppCaches) -> Value {
    let (profile_entries, chat_member_entries, fcm_entries) = caches.entry_counts();
    json!({
        "profile_l1_entries": profile_entries,
        "profile_l1_max": PROFILE_CACHE_MAX_CAPACITY,
        "chat_member_l1_entries": chat_member_entries,
        "chat_member_l1_max": CHAT_MEMBER_CACHE_MAX_CAPACITY,
        "fcm_token_l1_entries": fcm_entries,
        "fcm_token_l1_max": FCM_TOKEN_CACHE_MAX_CAPACITY
    })
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = tokio::time::timeout(
        std::time::Duration::from_millis(800),
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.pool),
    )
    .await
    .is_ok_and(|res| res.is_ok());
    let redis_ok = state.redis.is_connected();

    Json(json!({
        "status": "ok",
        "db_ok": db_ok,
        "redis_ok": redis_ok,
        "fcm_configured": state.fcm_oauth.is_configured(),
        "cloudinary_configured": state.cloudinary.is_configured(),
        "app": "Coffee Labs Enterprise API",
        "version": "2.0.0",
        "allocator": "mimalloc",
        "security": "Ed25519 + Argon2id + Zero-Trust RBAC",
        "rabbitmq": {
            "configured": state.rabbitmq.is_configured(),
            "connected": state.rabbitmq.is_connected(),
            "exchange": DEFAULT_EXCHANGE,
            "queue": DEFAULT_QUEUE,
            "prefetch_count": CONSUMER_PREFETCH_COUNT
        },
        "redis": redis_summary_json(&state),
        "caches": caches_summary_json(&state.caches),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn rabbitmq_health(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "configured": state.rabbitmq.is_configured(),
        "connected": state.rabbitmq.is_connected(),
        "exchange": DEFAULT_EXCHANGE,
        "queue": DEFAULT_QUEUE,
        "routing_key": DEFAULT_ROUTING_KEY,
        "prefetch_count": CONSUMER_PREFETCH_COUNT,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn redis_health(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "redis": redis_summary_json(&state),
        "caches": caches_summary_json(&state.caches),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn serve_dashboard() -> impl IntoResponse {
    Html(render_dashboard_html())
}

pub async fn get_telemetry_metrics(
    State(pool): State<PgPool>,
    State(telemetry): State<TelemetryRegistry>,
) -> AppResult<impl IntoResponse> {
    let snapshot = service::build_snapshot(&pool, &telemetry).await;
    Ok(ApiResponse::ok(
        "Real-time backend telemetry snapshot berhasil dimuat.",
        snapshot,
    ))
}

pub async fn reset_telemetry_metrics(
    State(telemetry): State<TelemetryRegistry>,
) -> AppResult<impl IntoResponse> {
    telemetry.reset();
    Ok(Json(json!({
        "success": true,
        "message": "Metrik telemetri API berhasil di-reset."
    })))
}

pub async fn serve_openapi() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}

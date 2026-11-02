use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Json,
};
use serde_json::json;
use sqlx::PgPool;
use utoipa::OpenApi;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        telemetry::{EndpointMetricSummary, RequestTrace, TelemetryRegistry},
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

pub async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "app": "Coffee Labs Enterprise API",
        "version": "2.0.0",
        "allocator": "mimalloc",
        "security": "Ed25519 + Argon2id + Zero-Trust RBAC",
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

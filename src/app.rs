use axum::{middleware as axum_mw, routing::get, Router};
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

pub use crate::core::AppState;

use crate::{
    core::middleware::{
        attach_governor_layer, build_cors_layer, telemetry_and_security_middleware,
    },
    modules::{
        auth, bean_scanner, beans, brew_logs, business_pos, chat, communities, gamification,
        marketplace, media, notifications, platform, rbac, recipes, recommendation, social,
        social_feed, social_graph, telemetry, users, vision_dataset,
    },
};

pub fn build_app(state: AppState) -> Router {
    let mw_state = (state.telemetry.clone(), state.rate_limiter.clone());
    tracing::info!(
        rabbitmq_configured = state.rabbitmq.is_configured(),
        rabbitmq_connected = state.rabbitmq.is_connected(),
        redis_configured = state.redis.is_configured(),
        redis_connected = state.redis.is_connected(),
        fcm_configured = state.fcm_oauth.is_configured(),
        cloudinary_configured = state.cloudinary.is_configured(),
        "🔌 Registrasi state PgPool, RedisManager, AppCaches, RabbitMqClient, FcmOAuthManager & CloudinaryStorage pada Axum Router"
    );

    let router = Router::new()
        .merge(telemetry::routes::router())
        .route("/api/v1/health/rabbitmq", get(telemetry::handlers::rabbitmq_health))
        .route("/api/v1/health/redis", get(telemetry::handlers::redis_health))
        .merge(auth::routes::router())
        .merge(rbac::routes::router())
        .merge(social::routes::router())
        .merge(social_feed::routes::router())
        .merge(social_graph::routes::router())
        .merge(users::routes::router())
        .merge(platform::routes::router())
        .merge(recipes::routes::router())
        .merge(brew_logs::routes::router())
        .merge(beans::routes::router())
        .merge(marketplace::routes::router())
        .merge(media::routes::router())
        .merge(gamification::routes::router())
        .merge(business_pos::routes::router())
        .merge(bean_scanner::routes::router())
        .merge(vision_dataset::routes::router())
        .merge(chat::routes::router())
        .merge(communities::routes::router())
        .merge(recommendation::routes::router())
        .merge(notifications::routes::router())
        .layer(axum_mw::from_fn_with_state(
            mw_state,
            telemetry_and_security_middleware,
        ))
        .layer(CompressionLayer::new().br(true).gzip(true))
        .layer(RequestBodyLimitLayer::new(4 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .layer(CatchPanicLayer::new())
        .layer(build_cors_layer());

    let router = attach_governor_layer(
        router,
        state.config.rate_limit_per_second,
        state.config.rate_limit_burst,
    );

    router.with_state(state)
}

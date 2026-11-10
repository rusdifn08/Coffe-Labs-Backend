use axum::{middleware as axum_mw, Router};
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

use crate::{
    core::{
        middleware::{
            attach_governor_layer, build_cors_layer, telemetry_and_security_middleware,
        },
        AppState,
    },
    modules::{
        auth, bean_scanner, beans, brew_logs, business_pos, gamification, marketplace, platform, rbac, recipes, social,
        telemetry, users, vision_dataset,
    },
};

pub fn build_app(state: AppState) -> Router {
    let mw_state = (state.telemetry.clone(), state.rate_limiter.clone());

    let router = Router::new()
        .merge(telemetry::routes::router())
        .merge(auth::routes::router())
        .merge(rbac::routes::router())
        .merge(social::routes::router())
        .merge(users::routes::router())
        .merge(platform::routes::router())
        .merge(recipes::routes::router())
        .merge(brew_logs::routes::router())
        .merge(beans::routes::router())
        .merge(marketplace::routes::router())
        .merge(gamification::routes::router())
        .merge(business_pos::routes::router())
        .merge(bean_scanner::routes::router())
        .merge(vision_dataset::routes::router())
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

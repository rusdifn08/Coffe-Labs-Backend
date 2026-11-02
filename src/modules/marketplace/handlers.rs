use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use sqlx::PgPool;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{OptionalAuthUser, ValidatedJson},
    },
    modules::{
        beans::{dto::BeanCatalogQuery, service as bean_service},
        marketplace::{
            dto::{PlaceOrderRequest, ValidateVoucherRequest},
            service,
        },
    },
};

pub async fn get_products(
    Query(query): Query<BeanCatalogQuery>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let products = bean_service::get_catalog(&pool, query).await?;
    Ok(ApiResponse::ok(
        "Produk Marketplace Specialty Coffee berhasil dimuat.",
        products,
    ))
}

pub async fn validate_voucher(
    ValidatedJson(payload): ValidatedJson<ValidateVoucherRequest>,
) -> AppResult<impl IntoResponse> {
    let res = service::evaluate_voucher(payload)?;
    Ok(ApiResponse::ok("Voucher berhasil divalidasi.", res))
}

pub async fn place_order(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(pool): State<PgPool>,
    ValidatedJson(payload): ValidatedJson<PlaceOrderRequest>,
) -> AppResult<impl IntoResponse> {
    let order = service::create_order(&pool, auth, payload).await?;
    Ok(ApiResponse::created(
        "Pesanan kopi berhasil dikonfirmasi dan masuk antrean sangrai!",
        order,
    ))
}

pub async fn get_orders(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let orders = service::get_user_orders(&pool, auth).await?;
    Ok(ApiResponse::ok("Daftar pesanan marketplace berhasil dimuat.", orders))
}

use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::live_brew::{address_cipher::encode_static_table_code, models::BusinessBrewTableRow};

pub async fn fetch_user_store_info(pool: &PgPool, owner_id: Uuid) -> (Option<Uuid>, String) {
    let row = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, store_name FROM business_store_configs WHERE owner_user_id = $1 LIMIT 1",
    )
    .bind(owner_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match row {
        Some((id, name)) => (Some(id), name),
        None => (None, "Coffee Labs Slow Bar".to_string()),
    }
}

pub async fn list_owner_tables(
    pool: &PgPool,
    owner_id: Uuid,
) -> Result<Vec<BusinessBrewTableRow>, sqlx::Error> {
    sqlx::query_as::<_, BusinessBrewTableRow>(
        r#"
        SELECT id, store_id, owner_user_id, table_number, static_address_code,
               active_brew_session_id, qr_scans_count, is_active, created_at, updated_at
        FROM business_brew_tables
        WHERE owner_user_id = $1 AND is_active = TRUE
        ORDER BY table_number ASC
        "#,
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await
}

pub async fn create_or_get_table(
    pool: &PgPool,
    owner_id: Uuid,
    table_number: &str,
    cipher_secret: &str,
) -> Result<BusinessBrewTableRow, sqlx::Error> {
    if let Some(existing) = sqlx::query_as::<_, BusinessBrewTableRow>(
        r#"
        SELECT id, store_id, owner_user_id, table_number, static_address_code,
               active_brew_session_id, qr_scans_count, is_active, created_at, updated_at
        FROM business_brew_tables
        WHERE owner_user_id = $1 AND table_number = $2
        "#,
    )
    .bind(owner_id)
    .bind(table_number.trim())
    .fetch_optional(pool)
    .await?
    {
        return Ok(existing);
    }

    let (store_id, _) = fetch_user_store_info(pool, owner_id).await;
    let id = Uuid::now_v7();
    let seed_num = (id.as_u128() & 0x3FFFFFFFF) as u64 + 1001;
    let static_code = encode_static_table_code(seed_num, cipher_secret);

    sqlx::query_as::<_, BusinessBrewTableRow>(
        r#"
        INSERT INTO business_brew_tables (id, store_id, owner_user_id, table_number, static_address_code)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (owner_user_id, table_number) DO UPDATE SET updated_at = NOW()
        RETURNING id, store_id, owner_user_id, table_number, static_address_code,
                  active_brew_session_id, qr_scans_count, is_active, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(store_id)
    .bind(owner_id)
    .bind(table_number.trim())
    .bind(static_code)
    .fetch_one(pool)
    .await
}

pub async fn find_table_by_static_code(
    pool: &PgPool,
    static_code: &str,
) -> Result<Option<(BusinessBrewTableRow, String)>, sqlx::Error> {
    let table = sqlx::query_as::<_, BusinessBrewTableRow>(
        r#"
        SELECT id, store_id, owner_user_id, table_number, static_address_code,
               active_brew_session_id, qr_scans_count, is_active, created_at, updated_at
        FROM business_brew_tables
        WHERE static_address_code = $1 AND is_active = TRUE
        "#,
    )
    .bind(static_code)
    .fetch_optional(pool)
    .await?;

    let Some(tbl) = table else {
        return Ok(None);
    };
    let (_, store_name) = fetch_user_store_info(pool, tbl.owner_user_id).await;
    Ok(Some((tbl, store_name)))
}

pub async fn find_table_by_id(
    pool: &PgPool,
    table_id: Uuid,
) -> Result<Option<BusinessBrewTableRow>, sqlx::Error> {
    sqlx::query_as::<_, BusinessBrewTableRow>(
        r#"
        SELECT id, store_id, owner_user_id, table_number, static_address_code,
               active_brew_session_id, qr_scans_count, is_active, created_at, updated_at
        FROM business_brew_tables
        WHERE id = $1
        "#,
    )
    .bind(table_id)
    .fetch_optional(pool)
    .await
}

pub async fn bind_active_session_on_table(
    pool: &PgPool,
    table_id: Uuid,
    session_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE business_brew_tables SET active_brew_session_id = $2, updated_at = NOW() WHERE id = $1",
    )
    .bind(table_id)
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn increment_table_scan(pool: &PgPool, table_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE business_brew_tables SET qr_scans_count = qr_scans_count + 1 WHERE id = $1")
        .bind(table_id)
        .execute(pool)
        .await?;
    Ok(())
}

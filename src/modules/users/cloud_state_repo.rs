use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    core::{error::AppResult, storage::auto_optimize_url},
    modules::users::{
        cloud_state_dto::{
            compute_coffee_level, extract_id_list_from_json, sanitize_id_list, sanitize_json_array,
            UpsertUserCloudStateRequest, UserCloudStateResponse,
        },
        cloud_state_projections::sync_gamification_and_projections,
        error::UsersError,
    },
};

const DEFAULT_AVATAR: &str = "assets/images/characters/avatar_1.png";

#[derive(Debug, FromRow)]
struct CloudStateJoinedRow {
    user_id: Uuid,
    username: String,
    full_name: String,
    avatar_url: Option<String>,
    has_cloud_record: bool,
    total_exp: i32,
    brews_count: i32,
    toolkit_count: i32,
    cupping_count: i32,
    streak_days: i32,
    last_activity_date: Option<String>,
    brew_history_json: Value,
    brew_journal_json: Value,
    favorite_recipe_ids: Value,
    custom_recipes_json: Value,
    deleted_default_ids: Value,
    state_version: i64,
    updated_at: DateTime<Utc>,
    gp_total_xp: Option<i64>,
    gp_current_streak: Option<i32>,
}

fn to_response(row: CloudStateJoinedRow) -> UserCloudStateResponse {
    let gp_xp = row.gp_total_xp.unwrap_or(0).clamp(0, i32::MAX as i64) as i32;
    let total_exp = row.total_exp.max(gp_xp).max(0);
    let streak_days = row.streak_days.max(row.gp_current_streak.unwrap_or(1)).max(1);
    let (current_level, level_title) = compute_coffee_level(total_exp);
    let avatar = row
        .avatar_url
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_AVATAR.to_string());

    UserCloudStateResponse {
        user_id: row.user_id,
        username: row.username,
        full_name: row.full_name,
        avatar_url: avatar,
        total_exp,
        current_level,
        level_title: level_title.to_string(),
        brews_count: row.brews_count.max(0),
        toolkit_count: row.toolkit_count.max(0),
        cupping_count: row.cupping_count.max(0),
        streak_days,
        last_activity_date: row.last_activity_date,
        brew_history: row.brew_history_json,
        brew_journal: row.brew_journal_json,
        favorite_recipe_ids: extract_id_list_from_json(&row.favorite_recipe_ids),
        custom_recipes: row.custom_recipes_json,
        deleted_default_ids: extract_id_list_from_json(&row.deleted_default_ids),
        state_version: row.state_version.max(1),
        updated_at: row.updated_at,
        has_cloud_record: row.has_cloud_record,
    }
}

pub async fn fetch_cloud_state(pool: &PgPool, user_id: Uuid) -> AppResult<UserCloudStateResponse> {
    let row = sqlx::query_as::<_, CloudStateJoinedRow>(
        r#"SELECT u.id AS user_id, u.username, u.full_name, u.avatar_url,
                  (ucs.user_id IS NOT NULL) AS has_cloud_record,
                  COALESCE(ucs.total_exp, 0) AS total_exp,
                  COALESCE(ucs.brews_count, 0) AS brews_count,
                  COALESCE(ucs.toolkit_count, 0) AS toolkit_count,
                  COALESCE(ucs.cupping_count, 0) AS cupping_count,
                  COALESCE(ucs.streak_days, 1) AS streak_days,
                  ucs.last_activity_date,
                  COALESCE(ucs.brew_history_json, '[]'::jsonb) AS brew_history_json,
                  COALESCE(ucs.brew_journal_json, '[]'::jsonb) AS brew_journal_json,
                  COALESCE(ucs.favorite_recipe_ids, '[]'::jsonb) AS favorite_recipe_ids,
                  COALESCE(ucs.custom_recipes_json, '[]'::jsonb) AS custom_recipes_json,
                  COALESCE(ucs.deleted_default_ids, '[]'::jsonb) AS deleted_default_ids,
                  COALESCE(ucs.state_version, 1) AS state_version,
                  COALESCE(ucs.updated_at, u.updated_at, NOW()) AS updated_at,
                  gp.total_xp AS gp_total_xp, gp.current_streak AS gp_current_streak
           FROM users u
           LEFT JOIN user_cloud_state ucs ON ucs.user_id = u.id
           LEFT JOIN gamification_profiles gp ON gp.user_id = u.id
           WHERE u.id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(UsersError::UserNotFoundOrProtected)?;

    Ok(to_response(row))
}

pub async fn upsert_cloud_state(
    pool: &PgPool,
    user_id: Uuid,
    req: &UpsertUserCloudStateRequest,
) -> AppResult<UserCloudStateResponse> {
    let brew_history = sanitize_json_array(req.brew_history.as_ref(), 50);
    let brew_journal = sanitize_json_array(req.brew_journal.as_ref(), 100);
    let custom_recipes = sanitize_json_array(req.custom_recipes.as_ref(), 100);
    let favorites_json = json!(sanitize_id_list(req.favorite_recipe_ids.as_deref(), 500));
    let deleted_json = json!(sanitize_id_list(req.deleted_default_ids.as_deref(), 500));
    let clean_last_date = req.last_activity_date.as_deref().map(str::trim).filter(|s| !s.is_empty());

    sqlx::query(
        r#"INSERT INTO user_cloud_state (
               user_id, total_exp, brews_count, toolkit_count, cupping_count,
               streak_days, last_activity_date, brew_history_json, brew_journal_json,
               favorite_recipe_ids, custom_recipes_json, deleted_default_ids, state_version, updated_at
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 1, NOW())
           ON CONFLICT (user_id) DO UPDATE SET
               total_exp = GREATEST(user_cloud_state.total_exp, EXCLUDED.total_exp),
               brews_count = GREATEST(user_cloud_state.brews_count, EXCLUDED.brews_count),
               toolkit_count = GREATEST(user_cloud_state.toolkit_count, EXCLUDED.toolkit_count),
               cupping_count = GREATEST(user_cloud_state.cupping_count, EXCLUDED.cupping_count),
               streak_days = GREATEST(user_cloud_state.streak_days, EXCLUDED.streak_days),
               last_activity_date = COALESCE(EXCLUDED.last_activity_date, user_cloud_state.last_activity_date),
               brew_history_json = CASE WHEN $13 THEN EXCLUDED.brew_history_json ELSE user_cloud_state.brew_history_json END,
               brew_journal_json = CASE WHEN $14 THEN EXCLUDED.brew_journal_json ELSE user_cloud_state.brew_journal_json END,
               favorite_recipe_ids = CASE WHEN $15 THEN EXCLUDED.favorite_recipe_ids ELSE user_cloud_state.favorite_recipe_ids END,
               custom_recipes_json = CASE WHEN $16 THEN EXCLUDED.custom_recipes_json ELSE user_cloud_state.custom_recipes_json END,
               deleted_default_ids = CASE WHEN $17 THEN EXCLUDED.deleted_default_ids ELSE user_cloud_state.deleted_default_ids END,
               state_version = user_cloud_state.state_version + 1,
               updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(req.total_exp.max(0))
    .bind(req.brews_count.max(0))
    .bind(req.toolkit_count.max(0))
    .bind(req.cupping_count.max(0))
    .bind(req.streak_days.max(1))
    .bind(clean_last_date)
    .bind(&brew_history)
    .bind(&brew_journal)
    .bind(&favorites_json)
    .bind(&custom_recipes)
    .bind(&deleted_json)
    .bind(req.brew_history.is_some())
    .bind(req.brew_journal.is_some())
    .bind(req.favorite_recipe_ids.is_some())
    .bind(req.custom_recipes.is_some())
    .bind(req.deleted_default_ids.is_some())
    .execute(pool)
    .await?;

    let clean_name = req.full_name.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let clean_avatar = req.avatar_url.as_deref().map(auto_optimize_url).filter(|s| !s.is_empty());

    if clean_name.is_some() || clean_avatar.is_some() {
        let _ = sqlx::query(
            r#"UPDATE users
               SET full_name = COALESCE($1, full_name),
                   avatar_url = COALESCE($2, avatar_url),
                   updated_at = NOW()
               WHERE id = $3"#,
        )
        .bind(clean_name)
        .bind(clean_avatar.as_deref())
        .bind(user_id)
        .execute(pool)
        .await;
        crate::modules::auth::repository::invalidate_user_cache_all();
    }

    let persisted = fetch_cloud_state(pool, user_id).await?;
    sync_gamification_and_projections(pool, &persisted).await;
    Ok(persisted)
}

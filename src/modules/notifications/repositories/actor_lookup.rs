use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::notifications::events::ActorSummary;

pub async fn lookup_actor_summary(pool: &PgPool, user_id: Uuid) -> Option<ActorSummary> {
    #[derive(sqlx::FromRow)]
    struct Row {
        username: String,
        display_name: String,
        avatar_url: Option<String>,
    }

    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            COALESCE(NULLIF(p.username, ''), u.username) AS username,
            COALESCE(NULLIF(p.display_name, ''), NULLIF(u.full_name, ''), u.username) AS display_name,
            COALESCE(p.avatar_url, u.avatar_url) AS avatar_url
        FROM users u
        LEFT JOIN user_profile_projections p ON p.user_id = u.id
        WHERE u.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()??;

    Some(ActorSummary {
        user_id,
        username: row.username,
        display_name: row.display_name,
        avatar_url: row.avatar_url,
    })
}

pub async fn lookup_thread_author(pool: &PgPool, thread_id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT author_id FROM social_threads WHERE id = $1 AND is_deleted = FALSE",
    )
    .bind(thread_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

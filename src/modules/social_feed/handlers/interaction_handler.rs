use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, OptionalAuthUser},
        AppState,
    },
    modules::{
        notifications::{
            builders::{build_thread_like_coalesce_seed, build_thread_reply_event},
            events::ActorSummary,
            repositories::actor_lookup,
        },
        social_feed::{
            dto::CreateThreadRequest,
            services::{thread_command_service, thread_create_service},
        },
    },
};

fn parse_uid(raw: &str) -> AppResult<Uuid> {
    Uuid::parse_str(raw).map_err(|_| AppError::BadRequest("ID user tidak valid.".to_string()))
}

pub async fn create_thread(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateThreadRequest>,
) -> AppResult<impl IntoResponse> {
    let uid = parse_uid(&claims.sub)?;
    let item = thread_create_service::create_thread(
        &state.pool,
        &state.social_profile_cqrs,
        &state.chat_moderation,
        uid,
        req,
    )
    .await?;

    if let Some(target_thread_id) = item.parent_thread_id.or(item.quoted_thread_id) {
        let pool = state.pool.clone();
        let notifications = state.notifications.clone();
        let actor = ActorSummary {
            user_id: uid,
            username: item.author_username.clone(),
            display_name: item.author_display_name.clone(),
            avatar_url: item.author_avatar_url.clone(),
        };
        let preview = item.body_text.chars().take(180).collect::<String>();
        let thread_id = item.id;
        tokio::spawn(async move {
            if let Some(target_author) = actor_lookup::lookup_thread_author(&pool, target_thread_id).await {
                if target_author != uid {
                    notifications.publish(build_thread_reply_event(
                        target_author,
                        thread_id,
                        actor,
                        &preview,
                    ));
                }
            }
        });
    }

    Ok(ApiResponse::created("Thread berhasil dipublikasikan.", item))
}

pub async fn toggle_like(
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let uid = parse_uid(&claims.sub)?;
    let data = thread_command_service::toggle_like(&state.pool, id, uid).await?;
    if data.is_liked_by_me {
        let pool = state.pool.clone();
        let notifications = state.notifications.clone();
        tokio::spawn(async move {
            let Some(author_id) = actor_lookup::lookup_thread_author(&pool, id).await else {
                return;
            };
            if author_id == uid {
                return;
            }
            if let Some(actor) = actor_lookup::lookup_actor_summary(&pool, uid).await {
                notifications.publish_coalesced(build_thread_like_coalesce_seed(
                    author_id, id, actor,
                ));
            }
        });
    }
    Ok(ApiResponse::ok("Status suka berhasil diperbarui.", data))
}

pub async fn toggle_repost(
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let uid = parse_uid(&claims.sub)?;
    let data = thread_command_service::toggle_repost(&state.pool, id, uid).await?;
    Ok(ApiResponse::ok("Status repost berhasil diperbarui.", data))
}

pub async fn try_recipe(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let data = thread_command_service::record_recipe_try(
        &state.pool,
        &state.social_profile_cqrs,
        id,
        viewer_id,
    )
    .await?;
    Ok(ApiResponse::ok(
        "Resep siap diseduh! Pembuat menerima +15 Community XP.",
        data,
    ))
}

pub async fn delete_thread(
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let uid = parse_uid(&claims.sub)?;
    thread_command_service::delete_thread(&state.pool, &state.social_profile_cqrs, id, uid)
        .await?;
    Ok(ApiResponse::ok("Thread berhasil dihapus.", serde_json::json!({ "id": id })))
}

use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::{
    communities::{
        dto::{
            CommunityChannelDto, CommunityWorkspaceDto, CreateCommunityRequest,
            JoinClubResponseDto,
        },
        error::CommunitiesError,
        models::{is_privileged_role, normalize_privacy_level},
        repositories::{
            community_query_repo, community_repo, invite_claim_repo, invite_secret_repo,
        },
        secret_id_crypto::{generate_secret_id, mask_secret_preview},
        services::{
            club_cache_service::ClubCacheService,
            club_discovery_service::summary_from_row,
        },
    },
    recommendation::scoring_math,
};

fn sanitize_slug(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').chars().take(45).collect()
}

pub async fn create_community(
    pg: &PgPool,
    hmac_key: &[u8; 32],
    cache: &ClubCacheService,
    owner_id: Uuid,
    req: CreateCommunityRequest,
) -> Result<CommunityWorkspaceDto, CommunitiesError> {
    let privacy = normalize_privacy_level(&req.privacy_level).ok_or_else(|| {
        CommunitiesError::BadRequest(
            "privacy_level harus PUBLIC, RESTRICTED, atau SECRET_VAULT.".into(),
        )
    })?;

    let derived_slug = match req.handle_slug.as_deref() {
        Some(custom) => sanitize_slug(custom),
        None => sanitize_slug(&req.name),
    };
    if derived_slug.len() < 3 {
        return Err(CommunitiesError::Validation(
            "Slug atau nama club minimal 3 karakter alfanumerik.".into(),
        ));
    }

    let clean_tags: Vec<String> = req
        .focus_tags
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .take(8)
        .collect();
    let combined_text = format!("{} {} {}", req.name, req.description, clean_tags.join(" "));
    let taste_vec = scoring_math::extract_16d_vector_from_text(&combined_text);

    let avatar_opt = req.avatar_url.map(|u| crate::core::storage::auto_optimize_url(&u));
    let banner_opt = req.banner_url.map(|u| crate::core::storage::auto_optimize_url(&u));
    let secret = generate_secret_id(hmac_key)?;
    let club_id = Uuid::now_v7();
    let tags_json = serde_json::to_value(&clean_tags).unwrap_or_default();

    let created = community_repo::create_club_with_defaults(
        pg,
        community_repo::CreateClubTxParams {
            community_id: club_id,
            handle_slug: &derived_slug,
            name: req.name.trim(),
            description: req.description.trim(),
            avatar_url: avatar_opt.as_deref(),
            banner_url: banner_opt.as_deref(),
            privacy_level: privacy,
            owner_id,
            focus_tags_json: tags_json,
            taste_vector: &taste_vec,
            initial_secret: &secret,
        },
    )
    .await
    .map_err(|e| match &e {
        CommunitiesError::Database(sqlx::Error::Database(db_err))
            if db_err.code().as_deref() == Some("23505") =>
        {
            CommunitiesError::Conflict("Slug/handle club ini sudah terdaftar.".into())
        }
        _ => e,
    })?;

    cache.put_club(created).await;
    cache.invalidate_club(club_id).await;
    get_workspace(pg, cache, Some(owner_id), club_id).await
}

pub async fn get_workspace(
    pg: &PgPool,
    cache: &ClubCacheService,
    viewer_id: Option<Uuid>,
    community_id: Uuid,
) -> Result<CommunityWorkspaceDto, CommunitiesError> {
    let club = cache.get_club(pg, community_id).await?;
    let my_role = match viewer_id {
        Some(uid) => community_query_repo::fetch_viewer_role(pg, community_id, uid).await?,
        None => None,
    };

    if club.privacy_level == "SECRET_VAULT" && my_role.is_none() {
        return Err(CommunitiesError::Forbidden);
    }

    let is_member = my_role.is_some();
    let can_expose_channels = club.privacy_level == "PUBLIC" || is_member;
    let channels = if can_expose_channels {
        community_query_repo::fetch_channels(pg, community_id)
            .await?
            .into_iter()
            .map(|c| CommunityChannelDto {
                id: c.id,
                community_id: c.community_id,
                name: c.name,
                description: c.description,
                kind: c.kind,
                linked_conversation_id: c.linked_conversation_id,
                display_order: c.display_order,
                is_readonly_announcement: c.is_readonly_announcement,
            })
            .collect()
    } else {
        Vec::new()
    };

    let can_manage_invites = my_role.as_deref().map(is_privileged_role).unwrap_or(false);
    let active_secret_code_preview = if is_member {
        let secrets = invite_secret_repo::list_active_secrets(pg, community_id).await?;
        secrets.first().map(|s| {
            if can_manage_invites {
                s.secret_code_display.clone()
            } else {
                mask_secret_preview(&s.secret_code_display)
            }
        })
    } else {
        None
    };

    let summary = summary_from_row(&club, my_role.clone());
    Ok(CommunityWorkspaceDto {
        community: summary,
        channels,
        my_role,
        is_member,
        can_post: is_member,
        can_manage_invites,
        active_secret_code_preview,
    })
}

pub async fn join_public_club(
    pg: &PgPool,
    cache: &ClubCacheService,
    user_id: Uuid,
    community_id: Uuid,
) -> Result<JoinClubResponseDto, CommunitiesError> {
    let club = cache.get_club(pg, community_id).await?;
    if club.privacy_level != "PUBLIC" {
        return Err(CommunitiesError::Forbidden);
    }
    let (already_member, assigned_role, default_conversation_id) =
        invite_claim_repo::join_public_club_atomic(pg, community_id, user_id).await?;
    cache.invalidate_club(community_id).await;
    let updated = cache.get_club(pg, community_id).await?;
    Ok(JoinClubResponseDto {
        joined: true,
        already_member,
        assigned_role: assigned_role.clone(),
        club: summary_from_row(&updated, Some(assigned_role)),
        default_conversation_id,
    })
}

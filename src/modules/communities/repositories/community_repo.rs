use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::communities::{
    error::CommunitiesError, models::CommunityRow, secret_id_crypto::GeneratedSecretId,
};

pub struct CreateClubTxParams<'a> {
    pub community_id: Uuid,
    pub handle_slug: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub avatar_url: Option<&'a str>,
    pub banner_url: Option<&'a str>,
    pub privacy_level: &'a str,
    pub owner_id: Uuid,
    pub focus_tags_json: serde_json::Value,
    pub taste_vector: &'a [f32],
    pub initial_secret: &'a GeneratedSecretId,
}

const SELECT_COMMUNITY_FIELDS: &str = r#"
SELECT id, handle_slug, name, description, avatar_url, banner_url,
       privacy_level::text AS privacy_level, owner_id, focus_tags,
       taste_feature_vector, members_count, threads_count,
       shared_recipes_count, total_club_xp, created_at, updated_at
FROM coffee_communities
"#;

pub async fn create_club_with_defaults(
    pg: &PgPool,
    params: CreateClubTxParams<'_>,
) -> Result<CommunityRow, CommunitiesError> {
    let mut tx = pg.begin().await?;

    sqlx::query(
        r#"INSERT INTO coffee_communities (
            id, handle_slug, name, description, avatar_url, banner_url,
            privacy_level, owner_id, focus_tags, taste_feature_vector,
            members_count, total_club_xp
        ) VALUES ($1, $2, $3, $4, $5, $6, $7::community_privacy_kind, $8, $9, $10, 1, 50)"#,
    )
    .bind(params.community_id)
    .bind(params.handle_slug)
    .bind(params.name)
    .bind(params.description)
    .bind(params.avatar_url)
    .bind(params.banner_url)
    .bind(params.privacy_level)
    .bind(params.owner_id)
    .bind(params.focus_tags_json)
    .bind(params.taste_vector)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO community_members (community_id, user_id, role, club_xp_contributed)
           VALUES ($1, $2, 'OWNER'::community_member_role, 50)"#,
    )
    .bind(params.community_id)
    .bind(params.owner_id)
    .execute(&mut *tx)
    .await?;

    let default_chats = [
        ("#general-lounge", "Obrolan santai seputar kopi, alat, dan kalibrasi harian.", 1i16),
        ("#klinik-seduh-v60", "Diskusi resep V60, ukuran gilingan, dan kurva pouring.", 2i16),
    ];

    for (ch_name, ch_desc, order) in default_chats {
        let conv_id = Uuid::now_v7();
        let conv_title = format!("{} • {}", params.name, ch_name);
        sqlx::query(
            r#"INSERT INTO conversations (id, kind, title, avatar_url)
               VALUES ($1, 'GROUP'::conversation_kind, $2, $3)"#,
        )
        .bind(conv_id)
        .bind(&conv_title)
        .bind(params.avatar_url)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"INSERT INTO conversation_participants (conversation_id, user_id, role)
               VALUES ($1, $2, 'ADMIN') ON CONFLICT DO NOTHING"#,
        )
        .bind(conv_id)
        .bind(params.owner_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"INSERT INTO community_channels (
                id, community_id, name, description, kind, linked_conversation_id, display_order
            ) VALUES ($1, $2, $3, $4, 'REALTIME_CHAT'::community_channel_kind, $5, $6)"#,
        )
        .bind(Uuid::now_v7())
        .bind(params.community_id)
        .bind(ch_name)
        .bind(ch_desc)
        .bind(conv_id)
        .bind(order)
        .execute(&mut *tx)
        .await?;
    }

    for (name, desc, kind, order) in [
        ("Recipe Vault", "Gudang resep rasio tetap komunitas untuk 1-Tap Brew.", "RECIPE_VAULT", 3i16),
        ("Brew Feed", "Log seduh & thread eksperimen anggota club.", "BREW_FEED", 4i16),
    ] {
        sqlx::query(
            r#"INSERT INTO community_channels (
                id, community_id, name, description, kind, display_order
            ) VALUES ($1, $2, $3, $4, $5::community_channel_kind, $6)"#,
        )
        .bind(Uuid::now_v7())
        .bind(params.community_id)
        .bind(name)
        .bind(desc)
        .bind(kind)
        .bind(order)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        r#"INSERT INTO community_invite_secrets (
            id, community_id, created_by, secret_code_display, secret_code_sha256,
            invite_url_token, role_granted
        ) VALUES ($1, $2, $3, $4, $5, $6, 'BARISTA_MEMBER'::community_member_role)"#,
    )
    .bind(Uuid::now_v7())
    .bind(params.community_id)
    .bind(params.owner_id)
    .bind(&params.initial_secret.display_code)
    .bind(&params.initial_secret.secret_sha256)
    .bind(&params.initial_secret.invite_url_token)
    .execute(&mut *tx)
    .await?;

    let club = sqlx::query_as::<_, CommunityRow>(&format!(
        "{SELECT_COMMUNITY_FIELDS} WHERE id = $1"
    ))
    .bind(params.community_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(club)
}

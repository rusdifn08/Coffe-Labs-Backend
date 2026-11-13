pub mod follow_repo;
pub mod interaction_repo;
pub mod profile_projection_repo;
pub mod thread_repo;
pub mod thread_write_repo;
pub mod timeline_repo;

pub const SELECT_THREAD_WITH_AUTHOR: &str = r#"
SELECT t.id, t.author_id,
       p.username AS author_username,
       p.display_name AS author_display_name,
       p.avatar_url AS author_avatar_url,
       p.account_tier AS author_tier,
       p.barista_level AS author_barista_level,
       p.barista_rank_title AS author_barista_rank_title,
       t.kind::text AS kind,
       t.parent_thread_id, t.root_thread_id, t.quoted_thread_id,
       t.depth_level, t.body_text, t.has_media, t.media_urls,
       t.attachment_payload,
       t.likes_count, t.replies_count, t.reposts_count, t.try_recipe_count,
       t.is_pinned_on_profile, t.created_at
FROM social_threads t
INNER JOIN user_profile_projections p ON p.user_id = t.author_id
"#;

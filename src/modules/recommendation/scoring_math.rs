pub use crate::modules::recommendation::taste_vectors::{
    cosine_similarity, extract_16d_vector_from_text, neutral_taste_vector, update_ema_l2,
    vec_from_slice, TASTE_DIM, TASTE_DIMENSION_LABELS, TASTE_EMA_ALPHA,
};

const GRAVITY_GAMMA: f64 = 1.65;

pub fn engagement_score(
    likes: i32,
    replies: i32,
    reposts: i32,
    try_recipe_count: i32,
    has_media: bool,
    has_attachment: bool,
) -> f64 {
    let base = 1.0 * (likes.max(0) as f64)
        + 2.5 * (replies.max(0) as f64)
        + 3.5 * (reposts.max(0) as f64)
        + 5.0 * (try_recipe_count.max(0) as f64);
    let media_bonus = if has_media { 1.2 } else { 0.0 };
    let attach_bonus = if has_attachment { 2.0 } else { 0.0 };
    base + media_bonus + attach_bonus
}

pub fn compute_trending_gravity_score(
    likes: i32,
    replies: i32,
    reposts: i32,
    try_recipe_count: i32,
    has_media: bool,
    has_attachment: bool,
    age_hours: f64,
) -> f64 {
    let eng = engagement_score(
        likes,
        replies,
        reposts,
        try_recipe_count,
        has_media,
        has_attachment,
    );
    let quality_lift = if has_attachment { 0.35 } else { 0.0 };
    let numerator = (1.0 + eng).ln() + quality_lift;
    let denom = (age_hours.max(0.0) + 2.0).powf(GRAVITY_GAMMA);
    numerator / denom
}

pub struct ForYouScoreInput {
    pub trending_gravity: f64,
    pub taste_cosine: f32,
    pub is_following_author: bool,
    pub is_shared_club: bool,
    pub is_own_post: bool,
    pub is_seen: bool,
    pub has_recipe_card: bool,
}

pub fn compute_for_you_score(input: &ForYouScoreInput) -> (f64, String) {
    let norm_trend = (input.trending_gravity * 2.4).clamp(0.0, 1.0);
    let social_boost = if input.is_following_author {
        0.24
    } else if input.is_shared_club {
        0.20
    } else {
        0.0
    };
    let taste_part = 0.42 * (input.taste_cosine as f64);
    let trend_part = 0.32 * norm_trend;
    let recipe_part = if input.has_recipe_card { 0.08 } else { 0.0 };
    let own_penalty = if input.is_own_post { -0.25 } else { 0.0 };
    let seen_penalty = if input.is_seen { -0.65 } else { 0.0 };

    let final_score =
        taste_part + trend_part + social_boost + recipe_part + own_penalty + seen_penalty;

    let pct = (input.taste_cosine * 100.0).round() as u8;
    let badge = if input.is_following_author {
        "Dari Barista Favoritmu".to_string()
    } else if input.is_shared_club {
        "Aktif di Clubmu".to_string()
    } else if pct >= 55 {
        format!("Cocok {pct}% • Selera Kopimu")
    } else if norm_trend >= 0.45 {
        "Sedang Ramai di Coffee Labs".to_string()
    } else if input.has_recipe_card {
        "Resep Siap Seduh".to_string()
    } else {
        "Rekomendasi Seduh".to_string()
    };

    (final_score, badge)
}

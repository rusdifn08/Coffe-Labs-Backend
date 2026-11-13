use coffe_labs_backend::{
    core::migrations::communities_schema,
    modules::{
        communities::{
            error::CommunitiesError,
            models::{is_privileged_role, normalize_club_role, normalize_privacy_level},
            secret_id_crypto::{
                derive_hmac_key, generate_secret_id, mask_secret_preview,
                normalize_and_verify_secret_id, CROCKFORD_ALPHABET,
            },
            services::{
                club_cache_service::{CachedClubMembers, ClubCacheService},
                secret_join_service::SecretJoinService,
            },
        },
        recommendation::{
            scoring_math::{
                compute_for_you_score, compute_trending_gravity_score, cosine_similarity,
                extract_16d_vector_from_text, neutral_taste_vector, update_ema_l2,
                ForYouScoreInput, TASTE_EMA_ALPHA,
            },
            taste_profiler_service::TasteProfilerService,
            trending_worker::FEED_TRENDING_ZSET_KEY,
        },
    },
};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn crockford_secret_id_generation_typo_normalization_and_edge_tamper_rejection() {
    let key = derive_hmac_key("unit-test-hmac-seed");
    let generated = generate_secret_id(&key).expect("generate secret id");

    assert!(generated.display_code.starts_with("CLB-"));
    assert_eq!(generated.display_code.len(), 15);
    assert_eq!(generated.normalized_code.len(), 10);
    assert!(generated.invite_url_token.len() >= 24);
    assert_eq!(CROCKFORD_ALPHABET.len(), 32);

    // Pastikan setiap karakter normalized berada dalam Crockford Base32 (tanpa I, L, O, U)
    assert!(generated
        .normalized_code
        .chars()
        .all(|c| !matches!(c, 'I' | 'L' | 'O' | 'U')));

    // Normalisasi format lower-case, spasi, dan typo O->0, I/L->1
    let mut typo_variant = generated.display_code.to_lowercase();
    typo_variant = typo_variant.replace('0', "o").replace('1', "l");
    let with_spaces = format!("  {typo_variant}  ");
    let (norm, sha) = normalize_and_verify_secret_id(&with_spaces, &key)
        .expect("valid normalized secret must verify checksum");
    assert_eq!(norm, generated.normalized_code);
    assert_eq!(sha, Sha256::digest(norm.as_bytes()).to_vec());

    // Normalisasi tanpa prefix CLB (10 karakter langsung dengan typo O/I/L)
    let raw_typo = generated.normalized_code.replace('0', "o").replace('1', "i");
    let (norm_raw, _) = normalize_and_verify_secret_id(&raw_typo, &key)
        .expect("raw 10-char normalized secret must verify checksum");
    assert_eq!(norm_raw, generated.normalized_code);

    // Tamper 1 karakter di payload -> harus ditolak HMAC checksum di edge RAM!
    let mut chars: Vec<char> = generated.normalized_code.chars().collect();
    chars[0] = if chars[0] == 'A' { 'B' } else { 'A' };
    let tampered: String = chars.into_iter().collect();
    assert!(normalize_and_verify_secret_id(&tampered, &key).is_none());

    // Preview masking tidak membocorkan kode utuh
    let masked = mask_secret_preview(&generated.display_code);
    assert!(masked.starts_with("CLB-****-***"));
    assert!(!masked.contains(&generated.normalized_code[0..4]));
}

#[tokio::test]
async fn secret_join_service_enforces_five_attempts_per_minute_limiter() {
    let lazy_pg = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgresql://user:pass@localhost:5432/coffe_labs_test")
        .expect("lazy pg pool");
    let cache = Arc::new(ClubCacheService::new(None));
    let key = derive_hmac_key("limiter-test-seed");
    let svc = SecretJoinService::new(lazy_pg, key, cache);
    let user_id = Uuid::new_v4();

    // 5 percobaan pertama dengan kode invalid ditolak secara kriptografis di RAM (tanpa query DB)
    for attempt in 1..=5 {
        let err = svc
            .join_by_secret_code(user_id, "CLB-ZZZZ-ZZZZZZ")
            .await
            .expect_err("invalid checksum must error");
        assert!(
            matches!(err, CommunitiesError::InvalidOrExpiredSecret),
            "attempt {attempt} expected InvalidOrExpiredSecret, got {err:?}"
        );
    }

    // Percobaan ke-6 dalam 1 menit wajib terkena governor 429 rate-limit!
    let sixth = svc
        .join_by_secret_code(user_id, "CLB-ZZZZ-ZZZZZZ")
        .await
        .expect_err("6th attempt within minute must trigger rate limit");
    assert!(matches!(sixth, CommunitiesError::SecretRateLimited));
}

#[test]
fn roaring_membership_bitmap_and_privacy_role_helpers_behave_accurately() {
    let member_a = Uuid::new_v4();
    let member_b = Uuid::new_v4();
    let outsider = Uuid::new_v4();
    let cache_entry = CachedClubMembers::from_ids(vec![member_a, member_b]);

    assert!(cache_entry.contains_user(member_a));
    assert!(cache_entry.contains_user(member_b));
    assert!(!cache_entry.contains_user(outsider));

    assert_eq!(normalize_privacy_level("secret_vault"), Some("SECRET_VAULT"));
    assert_eq!(normalize_privacy_level("unknown"), None);
    assert_eq!(normalize_club_role("member"), Some("BARISTA_MEMBER"));
    assert!(is_privileged_role("OWNER"));
    assert!(is_privileged_role("MODERATOR"));
    assert!(!is_privileged_role("BARISTA_MEMBER"));
}

#[test]
fn trending_gravity_and_for_you_math_favor_matching_fresh_recipes() {
    assert_eq!(FEED_TRENDING_ZSET_KEY, "feed:trending:global");

    let fresh_hot = compute_trending_gravity_score(12, 6, 4, 8, true, true, 1.0);
    let stale_hot = compute_trending_gravity_score(12, 6, 4, 8, true, true, 48.0);
    assert!(fresh_hot > stale_hot * 5.0);

    let v60_ethiopia =
        extract_16d_vector_from_text("Ethiopia Yirgacheffe Washed V60 citrus bergamot jasmine floral");
    let chocolate_espresso =
        extract_16d_vector_from_text("Dark Chocolate caramel nutty espresso shot moka");

    let mut user_profile = neutral_taste_vector();
    for _ in 0..4 {
        user_profile = update_ema_l2(&user_profile, &v60_ethiopia, TASTE_EMA_ALPHA);
    }

    let cos_match = cosine_similarity(&user_profile, &v60_ethiopia);
    let cos_mismatch = cosine_similarity(&user_profile, &chocolate_espresso);
    assert!(cos_match > cos_mismatch + 0.35);

    let top_labels = TasteProfilerService::top_taste_labels(&user_profile, 3);
    assert!(!top_labels.is_empty());

    let (score_fresh_match, badge_match) = compute_for_you_score(&ForYouScoreInput {
        trending_gravity: 0.25,
        taste_cosine: cos_match,
        is_following_author: false,
        is_shared_club: true,
        is_own_post: false,
        is_seen: false,
        has_recipe_card: true,
    });
    let (score_seen_mismatch, _) = compute_for_you_score(&ForYouScoreInput {
        trending_gravity: 0.25,
        taste_cosine: cos_mismatch,
        is_following_author: false,
        is_shared_club: false,
        is_own_post: false,
        is_seen: true,
        has_recipe_card: false,
    });
    assert!(score_fresh_match > score_seen_mismatch);
    assert_eq!(badge_match, "Aktif di Clubmu");
}

#[test]
fn communities_ddl_registers_all_core_tables_and_privacy_filter_index() {
    let joined = communities_schema::statements().join("\n");
    assert!(joined.contains("CREATE TABLE IF NOT EXISTS coffee_communities"));
    assert!(joined.contains("CREATE TABLE IF NOT EXISTS community_invite_secrets"));
    assert!(joined.contains("CREATE TABLE IF NOT EXISTS community_members"));
    assert!(joined.contains("CREATE TABLE IF NOT EXISTS community_channels"));
    assert!(joined.contains("CREATE TABLE IF NOT EXISTS user_taste_affinity_profiles"));
    assert!(joined.contains("WHERE privacy_level <> 'SECRET_VAULT'"));
}

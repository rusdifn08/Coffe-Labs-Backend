use secrecy::SecretString;
use serde_json::json;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::core::{
    error::AppResult,
    security::{cache_verified_password, hash_password, needs_fast_rehash},
};

pub async fn run_seeds(pool: &PgPool) -> AppResult<()> {
    seed_admin(pool).await?;
    seed_business_demo_account(pool).await?;
    seed_version_config(pool).await?;
    seed_social_posts(pool).await?;
    seed_curated_beans(pool).await?;
    Ok(())
}

async fn seed_admin(pool: &PgPool) -> AppResult<()> {
    let admin_username = "rusdifn08";
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(admin_username)
        .fetch_one(pool)
        .await?;

    let pwd = SecretString::from("121234aa".to_string());
    if !exists {
        let password_hash = hash_password(&pwd)?;
        cache_verified_password(&pwd, &password_hash);
        sqlx::query(
            r#"INSERT INTO users (username, password_hash, full_name, role, status, tier, avatar_url)
               VALUES ($1, $2, 'Admin Rusdi', 'admin', 'approved', 'business', 'assets/images/characters/avatar_1.png')"#,
        )
        .bind(admin_username)
        .bind(&password_hash)
        .execute(pool)
        .await?;
        info!("🔐 Akun Admin default dibuat: {}", admin_username);
    } else {
        let existing_hash: Option<String> = sqlx::query_scalar("SELECT password_hash FROM users WHERE username = $1")
            .bind(admin_username)
            .fetch_optional(pool)
            .await?;
        if let Some(h) = existing_hash {
            if needs_fast_rehash(&h) {
                if let Ok(fast_hash) = hash_password(&pwd) {
                    cache_verified_password(&pwd, &fast_hash);
                    let _ = sqlx::query("UPDATE users SET password_hash = $1, role = 'admin', status = 'approved', tier = 'business' WHERE username = $2")
                        .bind(&fast_hash)
                        .bind(admin_username)
                        .execute(pool)
                        .await;
                    return Ok(());
                }
            } else {
                cache_verified_password(&pwd, &h);
            }
        }
        sqlx::query("UPDATE users SET role = 'admin', status = 'approved', tier = 'business' WHERE username = $1")
            .bind(admin_username)
            .execute(pool)
            .await?;
    }
    Ok(())
}

async fn seed_business_demo_account(pool: &PgPool) -> AppResult<()> {
    let username = "bisnis123";
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(username)
        .fetch_one(pool)
        .await?;

    let pwd = SecretString::from("bisnis123".to_string());
    if !exists {
        let password_hash = hash_password(&pwd)?;
        cache_verified_password(&pwd, &password_hash);
        let user_id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO users (username, password_hash, full_name, role, status, tier, subscription_plan_id, avatar_url)
               VALUES ($1, $2, 'Coffee Labs Business Demo', 'user', 'approved', 'business', 'plan_business', 'assets/images/characters/avatar_2.png')
               RETURNING id"#,
        )
        .bind(username)
        .bind(&password_hash)
        .fetch_one(pool)
        .await?;

        // Seed default store configuration
        let _ = sqlx::query(
            r#"INSERT INTO business_store_configs 
                (owner_user_id, store_name, cashier_pin_hash, aod_grid_columns, aod_grid_rows)
               VALUES ($1, 'Coffee Labs Bar & Kitchen', '$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$6x+r14227nKzJkU/s01r5lI3W2i+aRz/J+4q57mJj+E', 3, 2)
               ON CONFLICT (owner_user_id) DO NOTHING"#
        )
        .bind(user_id)
        .execute(pool)
        .await;

        info!("🏢 Akun Demo Business berhasil dibuat: {} (Password: {})", username, "bisnis123");
    } else {
        let existing_hash: Option<String> = sqlx::query_scalar("SELECT password_hash FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(pool)
            .await?;
        if let Some(h) = existing_hash {
            if needs_fast_rehash(&h) {
                if let Ok(fast_hash) = hash_password(&pwd) {
                    cache_verified_password(&pwd, &fast_hash);
                    let _ = sqlx::query("UPDATE users SET password_hash = $1, status = 'approved', tier = 'business', subscription_plan_id = 'plan_business' WHERE username = $2")
                        .bind(&fast_hash)
                        .bind(username)
                        .execute(pool)
                        .await;
                }
            } else {
                cache_verified_password(&pwd, &h);
            }
        }
        sqlx::query("UPDATE users SET status = 'approved', tier = 'business', subscription_plan_id = 'plan_business' WHERE username = $1")
            .bind(username)
            .execute(pool)
            .await?;
    }
    Ok(())
}

async fn seed_version_config(pool: &PgPool) -> AppResult<()> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM app_version_configs WHERE id = 'default')")
        .fetch_one(pool)
        .await?;

    if !exists {
        sqlx::query(
            r#"INSERT INTO app_version_configs (
                id, platform, min_base_version, latest_base_version,
                min_build_number, latest_build_number, latest_patch_number,
                is_force_update, is_patch_required, release_notes, download_url
            ) VALUES (
                'default', 'all', '1.0.0', '1.0.0', 1, 1, 0, FALSE, FALSE,
                'Pembaruan stabilitas sistem dan optimasi fitur Coffee Labs.',
                'https://github.com/rusdi-labs/coffee-hub-mobile/releases/latest'
            )"#,
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn seed_social_posts(pool: &PgPool) -> AppResult<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM social_posts")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    if count > 0 {
        return Ok(());
    }

    let recipe_1 = json!({
        "title": "Sweetness Boost Bali Kintamani Anaerobic",
        "author": "Mikael Jasin 🇮🇩",
        "method": "Hario V60 02",
        "category": "V60",
        "waterTempCelsius": 91,
        "grindSize": "Medium (22 Klik Comandante)",
        "coffeeWeightGrams": 15,
        "totalWaterWeightGrams": 225,
        "totalTimeSeconds": 150,
        "tastingNotes": "Ripe Peach, Jackfruit, Wild Honey, Sparkling Acidity",
        "description": "Tuangan perlahan terkontrol untuk menonjolkan sweetness alami tanpa rasa over-fermented.",
        "steps": [
            {"stepOrder": 1, "title": "Blooming", "durationSeconds": 40, "waterAddedGrams": 45, "instruction": "Tuang 45g melingkar basahi kopi merata."},
            {"stepOrder": 2, "title": "Second Pour", "durationSeconds": 40, "waterAddedGrams": 65, "instruction": "Tuang spiral halus hingga total 110g."},
            {"stepOrder": 3, "title": "Third Pour", "durationSeconds": 35, "waterAddedGrams": 60, "instruction": "Tuang stabil hingga 170g."},
            {"stepOrder": 4, "title": "Final Pour", "durationSeconds": 35, "waterAddedGrams": 55, "instruction": "Tuang pelan di tengah hingga 225g tuntas."}
        ]
    });

    let post_1_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO social_posts (
            author_name, author_avatar, author_role, content, recipe_id, recipe_snapshot,
            likes_count, comments_count, copies_count, tags, created_at
        ) VALUES (
            'Mikael Jasin 🇮🇩', 'assets/images/characters/avatar_1.png', 'World Barista Champion',
            'Teman-teman barista! Buat yang sering ngerasa kopi natural gampang over-ferment atau muddy, coba teknik 4-pour dengan suhu 91°C ini di dripper V60. Di tuangan kedua kita tahan flow-rate nya agak lambat. Hasil cup di Kintamani Anaerobic keluar peach dan jackfruit yang super sparkling! Silakan copy resepnya & coba di bar kalian ☕🔥',
            102, $1, 48, 2, 29, '["V60", "Kintamani", "WBC2024", "Sweetness"]'::jsonb, NOW() - INTERVAL '3 hours'
        ) RETURNING id"#,
    )
    .bind(recipe_1)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        r#"INSERT INTO social_comments (post_id, author_name, author_avatar, content, created_at) VALUES
        ($1, 'Ryan Wibawa 🇮🇩', 'assets/images/characters/avatar_2.png', 'Mantap Bang Miki! Flow rate lambat di tuangan kedua emang ampuh banget ningkatin clarity.', NOW() - INTERVAL '2 hours'),
        ($1, 'Barista Dimas', 'assets/images/characters/avatar_3.png', 'Langsung saya copy resepnya ke Resep Saya, pas banget ada beans Kintamani di bar!', NOW() - INTERVAL '1 hour')"#,
    )
    .bind(post_1_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn seed_curated_beans(pool: &PgPool) -> AppResult<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM curated_beans")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    if count > 0 {
        return Ok(());
    }

    let seeds = [
        ("common_grounds_bali_kintamani", "Bali Kintamani Anaerobic Natural", "Common Grounds", "Jakarta", "Bali Kintamani", "Kartika & Kopyol", "Anaerobic Natural", "Light-Medium", "Ripe Peach, Jackfruit, Wild Honey, Sparkling Acidity", 95000, 175000, 4.95),
        ("space_roastery_strawberry_fields", "Strawberry Fields Halu Pink", "Space Roastery", "Yogyakarta", "Gunung Halu, Jawa Barat", "Sigararutang", "Honey Carbonic Maceration", "Light", "Strawberry Jam, Berry Candy, Vanilla, Sweet Finish", 88000, 165000, 4.92),
        ("anomali_aceh_gayo_wine", "Aceh Gayo Wine Process", "Anomali Coffee", "Jakarta", "Bener Meriah, Aceh", "Timtim & Bourbon", "Extended Wine Natural", "Medium-Light", "Sangria, Dried Fig, Concord Grape, Cocoa Nibs", 90000, 170000, 4.88),
        ("instinct_flores_bajawa", "Flores Bajawa Inerie Washed", "Instinct Roastery", "Denpasar", "Ngada, Flores", "Typica & Linea S795", "Full Washed", "Medium", "Dark Chocolate, Brown Sugar, Sweet Orange, Clean", 78000, 145000, 4.89),
    ];

    for (id, name, roastery, city, origin, variety, process, roast, notes, p100, p200, rating) in seeds {
        sqlx::query(
            r#"INSERT INTO curated_beans (
                id, name, roastery_name, roastery_city, origin, variety, process,
                roast_level, tasting_notes, price_100g, price_200g, rating
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(id)
        .bind(name)
        .bind(roastery)
        .bind(city)
        .bind(origin)
        .bind(variety)
        .bind(process)
        .bind(roast)
        .bind(notes)
        .bind(p100)
        .bind(p200)
        .bind(rating)
        .execute(pool)
        .await?;
    }
    Ok(())
}

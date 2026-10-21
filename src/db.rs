use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

use crate::auth::hash_password;

pub fn sanitize_db_url(raw: &str) -> String {
    let mut cleaned = raw.to_string();
    cleaned = cleaned.replace("&channel_binding=require", "");
    cleaned = cleaned.replace("channel_binding=require&", "");
    cleaned = cleaned.replace("?channel_binding=require", "");
    if !cleaned.contains("sslmode=") {
        if cleaned.contains('?') {
            cleaned.push_str("&sslmode=require");
        } else {
            cleaned.push_str("?sslmode=require");
        }
    }
    cleaned
}

pub async fn init_db(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let clean_url = sanitize_db_url(database_url);
    info!("Menghubungkan ke PostgreSQL Neon...");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&clean_url)
        .await?;

    info!("Koneksi PostgreSQL berhasil! Menjalankan inisialisasi tabel...");

    // 1. Inisialisasi tabel users
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            username VARCHAR(50) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            full_name VARCHAR(100) NOT NULL,
            role VARCHAR(20) NOT NULL DEFAULT 'user',
            status VARCHAR(20) NOT NULL DEFAULT 'pending',
            avatar_url TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // 2. Inisialisasi tabel feedbacks
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS feedbacks (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID REFERENCES users(id) ON DELETE SET NULL,
            username VARCHAR(100) NOT NULL,
            user_role VARCHAR(30) NOT NULL DEFAULT 'user',
            category VARCHAR(50) NOT NULL,
            feature_affected VARCHAR(100) NOT NULL,
            title VARCHAR(250) NOT NULL,
            description TEXT NOT NULL,
            rating INT DEFAULT 5,
            app_version VARCHAR(50) DEFAULT '1.0.0',
            device_info VARCHAR(100) DEFAULT 'Mobile',
            status VARCHAR(30) NOT NULL DEFAULT 'pending',
            admin_notes TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&pool)
    .await?;

    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_feedbacks_created_at ON feedbacks(created_at DESC)")
        .execute(&pool)
        .await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_feedbacks_status ON feedbacks(status)")
        .execute(&pool)
        .await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_feedbacks_category ON feedbacks(category)")
        .execute(&pool)
        .await;

    // 3. Inisialisasi akun Admin default: rusdifn08 / 121234aa
    let admin_username = "rusdifn08";
    let admin_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)",
    )
    .bind(admin_username)
    .fetch_one(&pool)
    .await?;

    if !admin_exists {
        info!("Akun admin '{}' belum ada. Membuat akun admin default...", admin_username);
        let password_hash = hash_password("121234aa").expect("Failed to hash admin password");
        sqlx::query(
            r#"
            INSERT INTO users (username, password_hash, full_name, role, status, avatar_url)
            VALUES ($1, $2, $3, 'admin', 'approved', 'assets/images/characters/avatar_1.png')
            "#,
        )
        .bind(admin_username)
        .bind(&password_hash)
        .bind("Admin Rusdi")
        .execute(&pool)
        .await?;
        info!("Akun admin default berhasil dibuat: {} (Status: approved)", admin_username);
    } else {
        // Pastikan status akun admin selalu approved dan role selalu admin
        sqlx::query(
            "UPDATE users SET role = 'admin', status = 'approved' WHERE username = $1",
        )
        .bind(admin_username)
        .execute(&pool)
        .await?;
        info!("Akun admin '{}' terverifikasi aktif.", admin_username);
    }

    // 4. Inisialisasi tabel app_version_configs (The Version Gatekeeper)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS app_version_configs (
            id VARCHAR(50) PRIMARY KEY DEFAULT 'default',
            platform VARCHAR(30) NOT NULL DEFAULT 'all',
            min_base_version VARCHAR(20) NOT NULL DEFAULT '1.0.0',
            latest_base_version VARCHAR(20) NOT NULL DEFAULT '1.0.0',
            min_build_number INT NOT NULL DEFAULT 1,
            latest_build_number INT NOT NULL DEFAULT 1,
            latest_patch_number INT NOT NULL DEFAULT 0,
            is_force_update BOOLEAN NOT NULL DEFAULT FALSE,
            is_patch_required BOOLEAN NOT NULL DEFAULT FALSE,
            release_notes TEXT NOT NULL DEFAULT 'Pembaruan stabilitas sistem, fitur feedback, dan optimasi performa Coffee Labs.',
            download_url TEXT,
            checksum_sha256 TEXT,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Seed default config jika belum ada
    let config_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM app_version_configs WHERE id = 'default')",
    )
    .fetch_one(&pool)
    .await?;

    if !config_exists {
        sqlx::query(
            r#"
            INSERT INTO app_version_configs (
                id, platform, min_base_version, latest_base_version,
                min_build_number, latest_build_number, latest_patch_number,
                is_force_update, is_patch_required, release_notes, download_url
            )
            VALUES (
                'default', 'all', '1.0.0', '1.0.0',
                1, 1, 0,
                FALSE, FALSE,
                'Pembaruan stabilitas sistem dan optimasi fitur Coffee Labs.',
                'https://github.com/rusdi-labs/coffee-hub-mobile/releases/latest'
            )
            "#,
        )
        .execute(&pool)
        .await?;
        info!("Konfigurasi Version Gatekeeper default berhasil diinisialisasi.");
    }

    // 5. Inisialisasi tabel Social Coffee (Posts, Likes, Comments)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS social_posts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID REFERENCES users(id) ON DELETE SET NULL,
            author_name VARCHAR(100) NOT NULL,
            author_avatar TEXT,
            author_role VARCHAR(30) NOT NULL DEFAULT 'Barista',
            content TEXT NOT NULL,
            recipe_id INT,
            recipe_snapshot JSONB,
            likes_count INT NOT NULL DEFAULT 0,
            comments_count INT NOT NULL DEFAULT 0,
            copies_count INT NOT NULL DEFAULT 0,
            tags JSONB DEFAULT '[]'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS social_post_likes (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            post_id UUID NOT NULL REFERENCES social_posts(id) ON DELETE CASCADE,
            username VARCHAR(100) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT unique_post_user_like UNIQUE (post_id, username)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS social_comments (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            post_id UUID NOT NULL REFERENCES social_posts(id) ON DELETE CASCADE,
            user_id UUID REFERENCES users(id) ON DELETE SET NULL,
            author_name VARCHAR(100) NOT NULL,
            author_avatar TEXT,
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(&pool)
    .await?;

    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_social_posts_created ON social_posts(created_at DESC)")
        .execute(&pool)
        .await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_social_post_likes_post ON social_post_likes(post_id)")
        .execute(&pool)
        .await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_social_comments_post ON social_comments(post_id, created_at ASC)")
        .execute(&pool)
        .await;

    // Seed initial Social Coffee posts jika masih kosong
    let posts_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM social_posts")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    if posts_count == 0 {
        info!("Seeding initial community posts for Social Coffee...");

        // Post 1: Mikael Jasin
        let recipe_1 = serde_json::json!({
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

        let post_1_id: uuid::Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO social_posts (
                author_name, author_avatar, author_role, content, recipe_id, recipe_snapshot,
                likes_count, comments_count, copies_count, tags, created_at
            )
            VALUES (
                'Mikael Jasin 🇮🇩', 'assets/images/characters/avatar_1.png', 'World Barista Champion',
                'Teman-teman barista! Buat yang sering ngerasa kopi natural gampang over-ferment atau muddy, coba teknik 4-pour dengan suhu 91°C ini di dripper V60. Di tuangan kedua kita tahan flow-rate nya agak lambat. Hasil cup di Kintamani Anaerobic keluar peach dan jackfruit yang super sparkling! Silakan copy resepnya & coba di bar kalian ☕🔥',
                102, $1, 48, 2, 29, '["V60", "Kintamani", "WBC2024", "Sweetness"]'::jsonb, NOW() - INTERVAL '3 hours'
            )
            RETURNING id
            "#
        )
        .bind(recipe_1)
        .fetch_one(&pool)
        .await?;

        // Seed 2 komentar untuk Post 1
        sqlx::query(
            r#"
            INSERT INTO social_comments (post_id, author_name, author_avatar, content, created_at)
            VALUES
            ($1, 'Ryan Wibawa 🇮🇩', 'assets/images/characters/avatar_2.png', 'Mantap Bang Miki! Flow rate lambat di tuangan kedua emang ampuh banget ningkatin clarity.', NOW() - INTERVAL '2 hours'),
            ($1, 'Barista Dimas', 'assets/images/characters/avatar_3.png', 'Langsung saya copy resepnya ke Resep Saya, pas banget ada beans Kintamani di bar!', NOW() - INTERVAL '1 hour')
            "#
        )
        .bind(post_1_id)
        .execute(&pool)
        .await?;

        // Post 2: Yoshua Tanu
        let recipe_2 = serde_json::json!({
            "title": "V60 High-Sweetness Gayo Wine Process",
            "author": "Yoshua Tanu 🇮🇩",
            "method": "Hario V60 02",
            "category": "V60",
            "waterTempCelsius": 89,
            "grindSize": "Medium-Coarse (26 Klik Comandante)",
            "coffeeWeightGrams": 16,
            "totalWaterWeightGrams": 240,
            "totalTimeSeconds": 150,
            "tastingNotes": "Sangria, Dried Fig, Concord Grape, Cocoa Nibs",
            "description": "Potong tetesan di menit 02:30 untuk meredam rasa kelat dan mengangkat aroma anggur manis.",
            "steps": [
                {"stepOrder": 1, "title": "Degassing Bloom", "durationSeconds": 45, "waterAddedGrams": 50, "instruction": "Tuang 50g air bersuhu 89°C."},
                {"stepOrder": 2, "title": "Sweet Extraction", "durationSeconds": 45, "waterAddedGrams": 90, "instruction": "Tuang memutar seukuran koin hingga 140g."},
                {"stepOrder": 3, "title": "Clean Finish", "durationSeconds": 60, "waterAddedGrams": 100, "instruction": "Tuang 100g air hingga 240g, angkat di 02:30."}
            ]
        });

        let post_2_id: uuid::Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO social_posts (
                author_name, author_avatar, author_role, content, recipe_id, recipe_snapshot,
                likes_count, comments_count, copies_count, tags, created_at
            )
            VALUES (
                'Yoshua Tanu 🇮🇩', 'assets/images/characters/avatar_2.png', 'Indonesian Barista Champion',
                'Banyak yang tanya settingan terbaik buat Aceh Gayo Wine Process biar ga terlalu asem menusuk. Kuncinya ada di rasio 1:15 dan potong tetesan akhir tepat di 02:30. Jangan biarkan drawdown menetes sampai tetes terakhir. Karakter anggur sangria dan buah tin keringnya jadi manis elegan!',
                111, $1, 42, 1, 34, '["WineProcess", "AcehGayo", "CommonGrounds", "HighSweetness"]'::jsonb, NOW() - INTERVAL '5 hours'
            )
            RETURNING id
            "#
        )
        .bind(recipe_2)
        .fetch_one(&pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO social_comments (post_id, author_name, author_avatar, content, created_at)
            VALUES
            ($1, 'Andra Roastery', 'assets/images/characters/avatar_4.png', 'Teknik cut di 02:30 beneran ngilangin dry aftertaste-nya mas Yoshua, gokil!', NOW() - INTERVAL '4 hours')
            "#
        )
        .bind(post_2_id)
        .execute(&pool)
        .await?;

        // Post 3: Space Roastery
        let recipe_3 = serde_json::json!({
            "title": "Strawberry Fields Circle-and-Center V60",
            "author": "Space Roastery 🇮🇩",
            "method": "Hario V60 02",
            "category": "V60",
            "waterTempCelsius": 90,
            "grindSize": "Medium-Coarse (24 Klik Comandante)",
            "coffeeWeightGrams": 15,
            "totalWaterWeightGrams": 225,
            "totalTimeSeconds": 140,
            "tastingNotes": "Selai Stroberi, Permen Buah Beri, Vanila, Manis Segar",
            "description": "Penuangan circle-and-center pour untuk aroma stroberi intens tanpa rasa pahit fermentasi.",
            "steps": [
                {"stepOrder": 1, "title": "Bloom Melingkar & Tengah", "durationSeconds": 35, "waterAddedGrams": 50, "instruction": "30g melingkar + 20g tengah."},
                {"stepOrder": 2, "title": "Second Pour", "durationSeconds": 30, "waterAddedGrams": 60, "instruction": "Tuang melingkar hingga 110g."},
                {"stepOrder": 3, "title": "Third Pour", "durationSeconds": 30, "waterAddedGrams": 60, "instruction": "Tuang tengah hingga 170g."},
                {"stepOrder": 4, "title": "Finish Pour", "durationSeconds": 45, "waterAddedGrams": 55, "instruction": "Tuang perlahan hingga 225g tuntas."}
            ]
        });

        sqlx::query(
            r#"
            INSERT INTO social_posts (
                author_name, author_avatar, author_role, content, recipe_id, recipe_snapshot,
                likes_count, comments_count, copies_count, tags, created_at
            )
            VALUES (
                'Space Roastery 🇮🇩', 'assets/images/characters/avatar_5.png', 'Specialty Roastery Team',
                'Buat pecinta karakter fruity berry yang aromatik banget, ini resep "Strawberry Fields" Space Roastery pake teknik hybrid circle-and-center pour! Rasa selai stroberinya keluar intens banget. Copy resepnya dan cobain seduh ya teman-teman!',
                176, $1, 55, 0, 41, '["SpaceRoastery", "Strawberry", "CirclePour", "ManualBrew"]'::jsonb, NOW() - INTERVAL '1 day'
            )
            "#
        )
        .bind(recipe_3)
        .execute(&pool)
        .await?;

        info!("Seeded 3 community discussion posts with full recipe snapshots.");
    }

    Ok(pool)
}

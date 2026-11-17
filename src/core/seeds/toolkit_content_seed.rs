use std::collections::HashMap;

use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::AppResult;

const EBOOKS_SEED_JSON: &str = include_str!("../../../storage/ebooks_catalog_seed.json");
const EBOOK_AST_SEED_JSON: &str = include_str!("../../../storage/ebooks_chapters_ast_seed.json");
const DICT_SEED_JSON: &str = include_str!("../../../storage/coffee_dictionary_seed.json");

#[derive(Deserialize)]
struct EbookSeedEntry {
    id: Uuid,
    book_code: String,
    title: String,
    original_title: Option<String>,
    author: String,
    isbn: Option<String>,
    synopsis: String,
    cover_image_url: Option<String>,
    cover_cloudinary_public_id: Option<String>,
    pdf_cloudinary_public_id: Option<String>,
    epub_cloudinary_public_id: Option<String>,
    clbook_ast_cloudinary_public_id: Option<String>,
    category: String,
    required_tier: String,
    total_pages: i32,
    total_chapters_count: Option<i16>,
    total_figures_count: Option<i16>,
    encrypted_asset_url: String,
    file_size_bytes: i64,
    pdf_file_size_bytes: Option<i64>,
    epub_file_size_bytes: Option<i64>,
    sha256_ciphertext_hash: String,
    pdf_sha256_hash: Option<String>,
    epub_sha256_hash: Option<String>,
    wrapped_master_cek_hex: String,
    object_storage_provider: Option<String>,
    toc_chapters_json: serde_json::Value,
    xp_reward_on_finish: i32,
}

#[derive(Deserialize)]
struct ChapterAstSeed {
    id: Uuid,
    chapter_number: i16,
    title_id: String,
    title_en: Option<String>,
    estimated_read_time_min: i16,
    start_pdf_page: i32,
    end_pdf_page: i32,
    mobile_blocks_ast: serde_json::Value,
    key_summary_points: serde_json::Value,
    chapter_quiz_json: serde_json::Value,
}

#[derive(Deserialize)]
struct DictionarySeed {
    id: Uuid,
    term_slug: String,
    alphabet_letter: String,
    term_title: String,
    category: String,
    short_definition_id: String,
    deep_explanation_id: String,
    related_formula_latex: Option<String>,
    related_book_code: String,
    related_chapter_num: i16,
}

pub async fn seed_ebooks_and_game_chapters(pool: &PgPool) -> AppResult<()> {
    let ast_map: HashMap<String, Vec<ChapterAstSeed>> =
        serde_json::from_str(EBOOK_AST_SEED_JSON).unwrap_or_default();

    if let Ok(books) = serde_json::from_str::<Vec<EbookSeedEntry>>(EBOOKS_SEED_JSON) {
        for b in books {
            let wrapped = hex::decode(&b.wrapped_master_cek_hex).unwrap_or_default();
            let actual_id: Option<Uuid> = sqlx::query_scalar(
                r#"INSERT INTO toolkit_ebooks (
                       id, book_code, title, original_title, author, isbn, synopsis,
                       cover_image_url, cover_cloudinary_public_id, pdf_cloudinary_public_id,
                       epub_cloudinary_public_id, clbook_ast_cloudinary_public_id, category,
                       required_tier, total_pages, total_chapters_count, total_figures_count,
                       encrypted_asset_url, file_size_bytes, pdf_file_size_bytes, epub_file_size_bytes,
                       sha256_ciphertext_hash, pdf_sha256_hash, epub_sha256_hash,
                       wrapped_master_cek, object_storage_provider, toc_chapters_json, xp_reward_on_finish
                   ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28)
                   ON CONFLICT (book_code) DO UPDATE SET
                       title = EXCLUDED.title, synopsis = EXCLUDED.synopsis,
                       total_pages = EXCLUDED.total_pages, total_chapters_count = EXCLUDED.total_chapters_count,
                       file_size_bytes = EXCLUDED.file_size_bytes,
                       pdf_file_size_bytes = EXCLUDED.pdf_file_size_bytes,
                       epub_file_size_bytes = EXCLUDED.epub_file_size_bytes,
                       wrapped_master_cek = EXCLUDED.wrapped_master_cek,
                       toc_chapters_json = EXCLUDED.toc_chapters_json
                   RETURNING id"#,
            )
            .bind(b.id).bind(&b.book_code).bind(&b.title).bind(&b.original_title)
            .bind(&b.author).bind(&b.isbn).bind(&b.synopsis)
            .bind(b.cover_image_url.unwrap_or_else(|| format!("/api/v1/toolkit/object-storage/coffeelabs/ebooks/{}/cover.webp", b.book_code)))
            .bind(&b.cover_cloudinary_public_id).bind(&b.pdf_cloudinary_public_id)
            .bind(&b.epub_cloudinary_public_id).bind(&b.clbook_ast_cloudinary_public_id)
            .bind(&b.category).bind(&b.required_tier).bind(b.total_pages)
            .bind(b.total_chapters_count.unwrap_or(1)).bind(b.total_figures_count.unwrap_or(1))
            .bind(&b.encrypted_asset_url).bind(b.file_size_bytes)
            .bind(b.pdf_file_size_bytes.unwrap_or(0)).bind(b.epub_file_size_bytes.unwrap_or(0))
            .bind(&b.sha256_ciphertext_hash).bind(&b.pdf_sha256_hash).bind(&b.epub_sha256_hash)
            .bind(&wrapped).bind(b.object_storage_provider.as_deref().unwrap_or("OBJECT_STORAGE"))
            .bind(&b.toc_chapters_json).bind(b.xp_reward_on_finish)
            .fetch_optional(pool)
            .await?;

            if let (Some(eb_id), Some(chapters)) = (actual_id, ast_map.get(&b.book_code)) {
                for ch in chapters {
                    sqlx::query(
                        r#"INSERT INTO ebook_chapters_ast (
                               id, ebook_id, chapter_number, title_id, title_en,
                               estimated_read_time_min, start_pdf_page, end_pdf_page,
                               mobile_blocks_ast, key_summary_points, chapter_quiz_json
                           ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                           ON CONFLICT (ebook_id, chapter_number) DO NOTHING"#,
                    )
                    .bind(ch.id).bind(eb_id).bind(ch.chapter_number)
                    .bind(&ch.title_id).bind(&ch.title_en).bind(ch.estimated_read_time_min)
                    .bind(ch.start_pdf_page).bind(ch.end_pdf_page)
                    .bind(&ch.mobile_blocks_ast).bind(&ch.key_summary_points).bind(&ch.chapter_quiz_json)
                    .execute(pool)
                    .await?;
                }
            }
        }
    }

    if let Ok(dicts) = serde_json::from_str::<Vec<DictionarySeed>>(DICT_SEED_JSON) {
        for d in dicts {
            sqlx::query(
                r#"INSERT INTO coffee_dictionary_entries (
                       id, term_slug, alphabet_letter, term_title, category,
                       short_definition_id, deep_explanation_id, related_formula_latex,
                       related_book_code, related_chapter_num
                   ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                   ON CONFLICT (term_slug) DO NOTHING"#,
            )
            .bind(d.id).bind(&d.term_slug).bind(&d.alphabet_letter).bind(&d.term_title)
            .bind(&d.category).bind(&d.short_definition_id).bind(&d.deep_explanation_id)
            .bind(&d.related_formula_latex).bind(&d.related_book_code).bind(d.related_chapter_num)
            .execute(pool)
            .await?;
        }
    }

    seed_game_chapters(pool).await
}

async fn seed_game_chapters(pool: &PgPool) -> AppResult<()> {
    let chapters = [
        (1_i16, "chapter-1-first-drip-blooming", "Tetes Pertama di Kedai Senja", "Kedai Senja Malabar", "Master Kaito", "v60-fundamental-bloom", serde_json::json!({"dose_g": 15, "water_ml": 225, "bloom_ml": 30, "bloom_sec": 35, "target_temp_c": 92, "target_time_sec": 135, "ideal_flow_ml_s": 5.0, "tolerance_pct": 15}), serde_json::json!([{"speaker": "Master Kaito", "line": "Selamat datang di Kedai Senja! Bilas filter lalu tuang tepat 30ml air untuk melepaskan CO2 selama 35 detik."}]), 200_i32),
        (2, "chapter-2-pusaran-air-flat-bed", "Pusaran Air & Flat Coffee Bed", "Slow Bar Bandung", "Master Kaito", "smoking-barrels-v60", serde_json::json!({"dose_g": 15, "water_ml": 225, "bloom_ml": 45, "bloom_sec": 35, "target_temp_c": 89, "target_time_sec": 145, "ideal_flow_ml_s": 5.5, "tolerance_pct": 10}), serde_json::json!([{"speaker": "Master Kaito", "line": "Selesaikan 225ml di 02:25 dengan deviasi <5% dan akhiri Rao Spin lembut."}]), 250),
        (3, "chapter-3-teka-teki-kasuya-4-6", "Teka-Teki Matematika 4:6 Juara Dunia", "Kyoto Heritage Corner", "Master Kaito", "tetsu-kasuya-4-6", serde_json::json!({"dose_g": 20, "water_ml": 300, "bloom_ml": 50, "bloom_sec": 45, "target_temp_c": 90, "target_time_sec": 210, "ideal_flow_ml_s": 6.0, "tolerance_pct": 10}), serde_json::json!([{"speaker": "Master Kaito", "line": "Atur porsi 40% awal (50ml lalu 70ml) untuk kemanisan optimal!"}]), 300),
        (4, "chapter-4-rahasia-dua-suhu-air", "Rahasia Dua Suhu Air (Thermal Drop)", "Malabar Reserve Lab", "Master Kaito", "sherry-hsu-dual-temp", serde_json::json!({"dose_g": 15, "water_ml": 300, "bloom_ml": 45, "bloom_sec": 35, "target_temp_c": 93, "drop_temp_c": 70, "drop_at_ml": 180, "target_time_sec": 165, "ideal_flow_ml_s": 5.2, "tolerance_pct": 8}), serde_json::json!([{"speaker": "Master Kaito", "line": "Ekstrak di 93C hingga 180ml, lalu aktifkan Air 70C agar akhir tidak sepat!"}]), 350),
        (5, "chapter-5-final-wbrc-sandwich", "Panggung Final World Brewers Cup", "WBrC Main Stage", "Master Kaito", "wbrc-sandwich-champion", serde_json::json!({"dose_g": 16, "water_ml": 250, "bloom_ml": 50, "bloom_sec": 35, "target_temp_c": 91, "target_time_sec": 155, "ideal_flow_ml_s": 5.0, "tolerance_pct": 6}), serde_json::json!([{"speaker": "Master Kaito", "line": "Capai Extraction Yield 20.5% dengan bed rata tanpa channeling!"}]), 500),
    ];
    for (num, slug, title, loc, npc, rec_slug, phys, dial, xp) in chapters {
        sqlx::query(
            r#"INSERT INTO game_brew_chapters (chapter_number, slug, title, story_location, npc_mentor_name, target_recipe_slug, physics_config_json, story_dialogue_json, xp_reward_3_stars)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
               ON CONFLICT (chapter_number) DO UPDATE SET title = EXCLUDED.title, physics_config_json = EXCLUDED.physics_config_json"#,
        )
        .bind(num).bind(slug).bind(title).bind(loc).bind(npc).bind(rec_slug).bind(phys).bind(dial).bind(xp)
        .execute(pool)
        .await?;
    }
    Ok(())
}

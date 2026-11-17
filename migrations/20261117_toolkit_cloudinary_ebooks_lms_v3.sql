-- =============================================================================
-- COFFEE LABS MODUL 2.1 UPGRADE v3.0:
-- Cloudinary / Object Storage E-Book Tri-Mode (PDF + EPUB3 + Reflowable AST),
-- Kamus Kopi A-Z, & Kurikulum Manual Brew LMS 9 Bab (30 Sub-bab)
-- =============================================================================

ALTER TABLE toolkit_ebooks
    ADD COLUMN IF NOT EXISTS original_title VARCHAR(200),
    ADD COLUMN IF NOT EXISTS isbn VARCHAR(32),
    ADD COLUMN IF NOT EXISTS cover_cloudinary_public_id TEXT,
    ADD COLUMN IF NOT EXISTS pdf_cloudinary_public_id TEXT,
    ADD COLUMN IF NOT EXISTS epub_cloudinary_public_id TEXT,
    ADD COLUMN IF NOT EXISTS clbook_ast_cloudinary_public_id TEXT,
    ADD COLUMN IF NOT EXISTS pdf_file_size_bytes BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS epub_file_size_bytes BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pdf_sha256_hash VARCHAR(64),
    ADD COLUMN IF NOT EXISTS epub_sha256_hash VARCHAR(64),
    ADD COLUMN IF NOT EXISTS total_chapters_count SMALLINT NOT NULL DEFAULT 4,
    ADD COLUMN IF NOT EXISTS total_figures_count SMALLINT NOT NULL DEFAULT 6,
    ADD COLUMN IF NOT EXISTS supports_bilingual_toggle BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS object_storage_provider VARCHAR(30) NOT NULL DEFAULT 'OBJECT_STORAGE';

ALTER TABLE user_ebook_progress
    ADD COLUMN IF NOT EXISTS preferred_reader_mode VARCHAR(20) NOT NULL DEFAULT 'MOBILE_AST',
    ADD COLUMN IF NOT EXISTS current_chapter_number SMALLINT NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS current_block_index INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS epub_cfi_locator TEXT;

CREATE TABLE IF NOT EXISTS ebook_chapters_ast (
    id UUID PRIMARY KEY,
    ebook_id UUID NOT NULL REFERENCES toolkit_ebooks(id) ON DELETE CASCADE,
    chapter_number SMALLINT NOT NULL,
    title_id VARCHAR(200) NOT NULL,
    title_en VARCHAR(200),
    estimated_read_time_min SMALLINT NOT NULL DEFAULT 12,
    start_pdf_page INT NOT NULL DEFAULT 1,
    end_pdf_page INT NOT NULL DEFAULT 25,
    mobile_blocks_ast JSONB NOT NULL DEFAULT '[]'::jsonb,
    key_summary_points JSONB NOT NULL DEFAULT '[]'::jsonb,
    chapter_quiz_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_ebook_chapter_v3 UNIQUE (ebook_id, chapter_number)
);

CREATE INDEX IF NOT EXISTS idx_ebook_chapters_v3_lookup
    ON ebook_chapters_ast (ebook_id, chapter_number ASC);

CREATE TABLE IF NOT EXISTS coffee_dictionary_entries (
    id UUID PRIMARY KEY,
    term_slug VARCHAR(80) NOT NULL UNIQUE,
    alphabet_letter CHAR(1) NOT NULL,
    term_title VARCHAR(120) NOT NULL,
    category VARCHAR(50) NOT NULL,
    short_definition_id VARCHAR(320) NOT NULL,
    deep_explanation_id TEXT NOT NULL,
    related_formula_latex TEXT,
    illustration_cloudinary_url TEXT,
    related_book_code VARCHAR(30) NOT NULL DEFAULT 'CL-BOOK-08',
    related_chapter_num SMALLINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_coffee_dict_v3_letter
    ON coffee_dictionary_entries (alphabet_letter ASC, term_title ASC);

ALTER TABLE lms_courses
    ADD COLUMN IF NOT EXISTS linked_ebook_id UUID REFERENCES toolkit_ebooks(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS competency_level VARCHAR(60),
    ADD COLUMN IF NOT EXISTS focus_summary TEXT,
    ADD COLUMN IF NOT EXISTS interactive_engine_type VARCHAR(140);

ALTER TABLE lms_lessons
    ADD COLUMN IF NOT EXISTS subchapter_code VARCHAR(16),
    ADD COLUMN IF NOT EXISTS chapter_markers_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS formulas_latex_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS storyboard_scenes_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS simulation_config_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS linked_ebook_code VARCHAR(30),
    ADD COLUMN IF NOT EXISTS linked_ebook_chapter SMALLINT;

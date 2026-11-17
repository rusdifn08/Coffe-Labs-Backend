use axum::{
    routing::{get, post, put},
    Router,
};

use crate::{
    core::AppState,
    modules::barista_toolkit::handlers::{
        dictionary_handler, ebook_handler, ebook_v3_handler, game_and_doctor_handler, lms_handler,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/toolkit/lms/courses", get(lms_handler::get_lms_courses))
        .route("/api/v1/toolkit/lms/lessons/{id}/stream", get(lms_handler::get_lesson_stream))
        .route("/api/v1/toolkit/lms/lessons/{id}/progress", post(lms_handler::sync_lesson_progress))
        .route("/api/v1/toolkit/ebooks", get(ebook_handler::list_ebooks))
        .route("/api/v1/toolkit/ebooks/{id}/chapters", get(ebook_v3_handler::get_ebook_chapters_ast))
        .route("/api/v1/toolkit/ebooks/{id}/license", post(ebook_handler::request_ebook_license))
        .route("/api/v1/toolkit/ebooks/{id}/asset", get(ebook_v3_handler::download_encrypted_clbook))
        .route("/api/v1/toolkit/ebooks/{id}/progress", put(ebook_handler::sync_ebook_progress))
        .route("/api/v1/toolkit/dictionary", get(dictionary_handler::list_dictionary_entries))
        .route("/api/v1/toolkit/dictionary/{slug}", get(dictionary_handler::get_dictionary_term))
        .route("/api/v1/toolkit/object-storage/{*key}", get(ebook_v3_handler::serve_object_storage_file))
        .route("/api/v1/toolkit/game-brew/chapters", get(game_and_doctor_handler::list_game_chapters))
        .route(
            "/api/v1/toolkit/game-brew/chapters/{chapter}/submit",
            post(game_and_doctor_handler::submit_game_chapter_score),
        )
        .route(
            "/api/v1/toolkit/game-brew/chapters/{chapter}/submit-3d",
            post(game_and_doctor_handler::submit_game_chapter_3d_score),
        )
        .route(
            "/api/v1/toolkit/brew-doctor/diagnose",
            post(game_and_doctor_handler::diagnose_brew_doctor),
        )
}

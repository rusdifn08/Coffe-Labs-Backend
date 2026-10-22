use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::{generate_jwt, hash_password, verify_password, AdminUser, AuthUser, OptionalAuthUser},
    models::{
        ApiResponse, AppVersionConfig, AuthResponse, ChangePasswordRequest, CopyRecipeResponse, CreateCommentRequest,
        CreateFeedbackRequest, CreateSocialPostRequest, Feedback, FeedbackQuery, FeedbackStats,
        LikePostRequest, LikePostResponse, LoginRequest, RegisterRequest, SocialComment,
        SocialFeedQuery, SocialPost, SocialPostResponse, UpdateAction,
        UpdateAppVersionConfigRequest, UpdateFeedbackStatusRequest, User, UserSummary,
        VersionCheckQuery, VersionCheckResponse,
    },
    AppState,
};

pub async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "app": "Coffee Labs API",
        "version": "1.0.0",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn register(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Response {
    let username = payload.username.trim().to_lowercase();
    let full_name = payload.full_name.trim();

    if username.len() < 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Username minimal harus 3 karakter."
            })),
        )
            .into_response();
    }

    if payload.password.len() < 6 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Password minimal harus 6 karakter."
            })),
        )
            .into_response();
    }

    if full_name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Nama lengkap tidak boleh kosong."
            })),
        )
            .into_response();
    }

    // Cek apakah username sudah terdaftar
    let user_exists: bool = match sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(&username)
        .fetch_one(&pool)
        .await
    {
        Ok(exists) => exists,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                })),
            )
                .into_response();
        }
    };

    if user_exists {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "success": false,
                "message": format!("Username '{}' sudah digunakan. Silakan pilih username lain.", username)
            })),
        )
            .into_response();
    }

    let password_hash = match hash_password(&payload.password) {
        Ok(h) => h,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Hashing error: {}", e)
                })),
            )
                .into_response();
        }
    };

    let avatar = payload.avatar_url.unwrap_or_else(|| "assets/images/characters/avatar_1.png".to_string());

    // Masukkan user baru dengan status default PENDING
    let new_user = match sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, password_hash, full_name, role, status, avatar_url)
        VALUES ($1, $2, $3, 'user', 'pending', $4)
        RETURNING *
        "#,
    )
    .bind(&username)
    .bind(&password_hash)
    .bind(full_name)
    .bind(&avatar)
    .fetch_one(&pool)
    .await
    {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal mendaftar: {}", e)
                })),
            )
                .into_response();
        }
    };

    let summary: UserSummary = new_user.into();

    (
        StatusCode::CREATED,
        Json(ApiResponse {
            success: true,
            message: "Pendaftaran berhasil! Akun Anda sedang menunggu persetujuan (Accept) dari Admin sebelum dapat digunakan sepenuhnya.".to_string(),
            data: Some(summary),
        }),
    )
        .into_response()
}

pub async fn login(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Response {
    let username = payload.username.trim().to_lowercase();

    let user_opt: Option<User> = match sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(&pool)
    .await
    {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                })),
            )
                .into_response();
        }
    };

    let user = match user_opt {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "message": "Username atau password salah."
                })),
            )
                .into_response();
        }
    };

    if !verify_password(&payload.password, &user.password_hash) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "success": false,
                "message": "Username atau password salah."
            })),
        )
            .into_response();
    }

    // Cek status persetujuan dari Admin (Kecuali jika role == 'admin')
    if user.role != "admin" {
        if user.status == "pending" {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "status": "pending",
                    "message": "Akun Anda sedang menunggu persetujuan dari admin. Silakan tunggu hingga admin meng-accept akun Anda."
                })),
            )
                .into_response();
        } else if user.status == "declined" {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "status": "declined",
                    "message": "Pendaftaran akun Anda telah ditolak (decline) oleh admin. Silakan hubungi admin."
                })),
            )
                .into_response();
        }
    }

    // Generate JWT token
    let token = match generate_jwt(&user.id.to_string(), &user.username, &user.role, &user.status) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal membuat token: {}", e)
                })),
            )
                .into_response();
        }
    };

    let summary: UserSummary = user.into();

    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: "Login berhasil!".to_string(),
            data: Some(AuthResponse {
                token,
                user: summary,
                message: "Selamat datang di Coffee Labs!".to_string(),
            }),
        }),
    )
        .into_response()
}

pub async fn get_me(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
) -> Response {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": "ID user tidak valid."
                })),
            )
                .into_response();
        }
    };

    let user_opt: Option<User> = match sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                })),
            )
                .into_response();
        }
    };

    match user_opt {
        Some(u) => {
            let summary: UserSummary = u.into();
            (
                StatusCode::OK,
                Json(ApiResponse {
                    success: true,
                    message: "Data profil berhasil diambil.".to_string(),
                    data: Some(summary),
                }),
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "message": "User tidak ditemukan."
            })),
        )
            .into_response(),
    }
}

pub async fn change_password(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> Response {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": "ID user tidak valid."
                })),
            )
                .into_response();
        }
    };

    let user_opt: Option<User> = match sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                })),
            )
                .into_response();
        }
    };

    let user = match user_opt {
        Some(u) => u,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "message": "User tidak ditemukan."
                })),
            )
                .into_response();
        }
    };

    if !verify_password(&payload.current_password, &user.password_hash) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Password saat ini tidak sesuai."
            })),
        )
            .into_response();
    }

    if payload.new_password.len() < 6 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Password baru minimal 6 karakter."
            })),
        )
            .into_response();
    }

    let new_hash = match hash_password(&payload.new_password) {
        Ok(h) => h,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal memproses password: {}", e)
                })),
            )
                .into_response();
        }
    };

    if let Err(e) = sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(&new_hash)
        .bind(user_id)
        .execute(&pool)
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Gagal mengupdate password di database: {}", e)
            })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Password berhasil diubah!"
        })),
    )
        .into_response()
}

pub async fn admin_get_users(
    State(pool): State<PgPool>,
    _admin: AdminUser,
) -> Response {
    let users = match sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    {
        Ok(list) => list,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                })),
            )
                .into_response();
        }
    };

    let summaries: Vec<UserSummary> = users.into_iter().map(Into::into).collect();

    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: "Daftar pengguna berhasil diambil.".to_string(),
            data: Some(summaries),
        }),
    )
        .into_response()
}

pub async fn admin_accept_user(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> Response {
    let updated_user = match sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET status = 'approved', updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "message": "User tidak ditemukan."
                })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                })),
            )
                .into_response();
        }
    };

    let summary: UserSummary = updated_user.into();

    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: format!("Akun '{}' berhasil disetujui (Approved).", summary.username),
            data: Some(summary),
        }),
    )
        .into_response()
}

pub async fn admin_decline_user(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> Response {
    let updated_user = match sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET status = 'declined', updated_at = NOW()
        WHERE id = $1 AND role != 'admin'
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "message": "User tidak ditemukan atau merupakan akun Admin."
                })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                })),
            )
                .into_response();
        }
    };

    let summary: UserSummary = updated_user.into();

    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: format!("Pendaftaran akun '{}' berhasil ditolak (Declined).", summary.username),
            data: Some(summary),
        }),
    )
        .into_response()
}

pub async fn admin_delete_user(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> Response {
    let result = sqlx::query("DELETE FROM users WHERE id = $1 AND role != 'admin'")
        .bind(user_id)
        .execute(&pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Pengguna berhasil dihapus."
            })),
        )
            .into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "message": "User tidak ditemukan atau tidak dapat dihapus."
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Database error: {}", e)
            })),
        )
            .into_response(),
    }
}

// ================= FEEDBACK HANDLERS =================

pub async fn submit_feedback(
    State(pool): State<PgPool>,
    OptionalAuthUser(claims_opt): OptionalAuthUser,
    Json(payload): Json<CreateFeedbackRequest>,
) -> Response {
    let title = payload.title.trim();
    let description = payload.description.trim();
    let category = payload.category.trim().to_lowercase();
    let feature = payload.feature_affected.trim();

    if title.is_empty() || description.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Judul dan deskripsi feedback tidak boleh kosong."
            })),
        )
            .into_response();
    }

    let (user_id, username, user_role) = match claims_opt {
        Some(claims) => (
            Uuid::parse_str(&claims.sub).ok(),
            claims.username,
            claims.role,
        ),
        None => (
            None,
            payload.guest_name.unwrap_or_else(|| "Tamu / Trial".to_string()),
            "guest".to_string(),
        ),
    };

    let rating = payload.rating.unwrap_or(5).clamp(1, 5);
    let app_version = payload.app_version.unwrap_or_else(|| "1.0.0".to_string());
    let device_info = payload.device_info.unwrap_or_else(|| "Mobile".to_string());

    let feedback = match sqlx::query_as::<_, Feedback>(
        r#"
        INSERT INTO feedbacks (
            user_id, username, user_role, category, feature_affected,
            title, description, rating, app_version, device_info, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'pending')
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&username)
    .bind(&user_role)
    .bind(&category)
    .bind(feature)
    .bind(title)
    .bind(description)
    .bind(rating)
    .bind(&app_version)
    .bind(&device_info)
    .fetch_one(&pool)
    .await
    {
        Ok(fb) => fb,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal menyimpan feedback: {}", e)
                })),
            )
                .into_response();
        }
    };

    (
        StatusCode::CREATED,
        Json(ApiResponse {
            success: true,
            message: "Terima kasih atas masukan Anda! Feedback berhasil dikirim dan akan segera ditinjau oleh Admin Rusdi.".to_string(),
            data: Some(feedback),
        }),
    )
        .into_response()
}

pub async fn admin_get_feedbacks(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Query(params): Query<FeedbackQuery>,
) -> Response {
    let mut query = "SELECT * FROM feedbacks WHERE 1=1".to_string();
    if let Some(ref st) = params.status {
        if !st.trim().is_empty() && st != "all" {
            query.push_str(&format!(" AND status = '{}'", st.replace('\'', "''")));
        }
    }
    if let Some(ref cat) = params.category {
        if !cat.trim().is_empty() && cat != "all" {
            query.push_str(&format!(" AND category = '{}'", cat.replace('\'', "''")));
        }
    }
    query.push_str(" ORDER BY created_at DESC");

    let feedbacks: Vec<Feedback> = match sqlx::query_as::<_, Feedback>(&query)
        .fetch_all(&pool)
        .await
    {
        Ok(list) => list,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                })),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: "Berhasil memuat daftar feedback.".to_string(),
            data: Some(feedbacks),
        }),
    )
        .into_response()
}

pub async fn admin_update_feedback_status(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateFeedbackStatusRequest>,
) -> Response {
    let updated = match sqlx::query_as::<_, Feedback>(
        r#"
        UPDATE feedbacks
        SET status = $1, admin_notes = COALESCE($2, admin_notes), updated_at = NOW()
        WHERE id = $3
        RETURNING *
        "#,
    )
    .bind(&payload.status)
    .bind(&payload.admin_notes)
    .bind(id)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(fb)) => fb,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "message": "Feedback tidak ditemukan."
                })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                })),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: format!("Status feedback berhasil diperbarui menjadi '{}'.", updated.status),
            data: Some(updated),
        }),
    )
        .into_response()
}

pub async fn admin_delete_feedback(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> Response {
    let result = sqlx::query("DELETE FROM feedbacks WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Feedback berhasil dihapus."
            })),
        )
            .into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "message": "Feedback tidak ditemukan."
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Database error: {}", e)
            })),
        )
            .into_response(),
    }
}

pub async fn admin_get_feedback_stats(
    State(pool): State<PgPool>,
    _admin: AdminUser,
) -> Response {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE status = 'pending'")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let in_progress: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE status = 'in_progress'")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let resolved: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE status = 'resolved'")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let bug_reports: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE category = 'bug_report'")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let feature_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE category = 'feature_request'")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let average_rating: f64 = sqlx::query_scalar("SELECT COALESCE(AVG(rating)::FLOAT8, 5.0) FROM feedbacks WHERE rating IS NOT NULL")
        .fetch_one(&pool)
        .await
        .unwrap_or(5.0);

    let stats = FeedbackStats {
        total,
        pending,
        in_progress,
        resolved,
        bug_reports,
        feature_requests,
        average_rating: (average_rating * 10.0).round() / 10.0,
    };

    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: "Statistik feedback berhasil dimuat.".to_string(),
            data: Some(stats),
        }),
    )
        .into_response()
}

// ================= VERSION GATEKEEPER HANDLERS =================

/// Fast in-memory cached version check endpoint
pub async fn version_check(
    State(cache): State<Arc<tokio::sync::RwLock<AppVersionConfig>>>,
    Query(params): Query<VersionCheckQuery>,
) -> Response {
    let config = cache.read().await;

    let client_build = params.build_number.unwrap_or(1);
    let client_patch = params.patch_number.unwrap_or(0);

    let action = if config.is_force_update || client_build < config.min_build_number {
        UpdateAction::ForceStoreUpdate
    } else if client_build < config.latest_build_number {
        UpdateAction::FlexibleStoreUpdate
    } else if client_build == config.latest_build_number {
        if client_patch < config.latest_patch_number {
            if config.is_patch_required {
                UpdateAction::ForceOtaPatch
            } else {
                UpdateAction::SilentOtaPatch
            }
        } else {
            UpdateAction::None
        }
    } else {
        UpdateAction::None
    };

    let action_code = match action {
        UpdateAction::None => "UP_TO_DATE",
        UpdateAction::SilentOtaPatch => "SILENT_OTA_PATCH",
        UpdateAction::ForceOtaPatch => "FORCE_OTA_PATCH",
        UpdateAction::FlexibleStoreUpdate => "FLEXIBLE_STORE_UPDATE",
        UpdateAction::ForceStoreUpdate => "FORCE_STORE_UPDATE",
    }
    .to_string();

    let target_patch = if config.latest_patch_number > 0 {
        Some(config.latest_patch_number)
    } else {
        None
    };

    let response = VersionCheckResponse {
        action,
        action_code,
        target_patch,
        latest_base_version: config.latest_base_version.clone(),
        latest_build_number: config.latest_build_number,
        is_force_update: config.is_force_update,
        release_notes: config.release_notes.clone(),
        download_url: config.download_url.clone(),
        checksum_sha256: config.checksum_sha256.clone(),
    };

    Json(response).into_response()
}

/// Admin endpoint: Mengambil konfigurasi gatekeeper saat ini
pub async fn admin_get_version_config(
    State(cache): State<Arc<tokio::sync::RwLock<AppVersionConfig>>>,
    _admin: AdminUser,
) -> Response {
    let config = cache.read().await.clone();
    Json(ApiResponse {
        success: true,
        message: "Konfigurasi Version Gatekeeper berhasil dimuat.".to_string(),
        data: Some(config),
    })
    .into_response()
}

/// Admin endpoint: Memperbarui aturan versi & memicu update in-memory cache instan
pub async fn admin_update_version_config(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(payload): Json<UpdateAppVersionConfigRequest>,
) -> Response {
    let pool = state.pool;
    let cache = state.version_cache;

    let updated_config = match sqlx::query_as::<_, AppVersionConfig>(
        r#"
        UPDATE app_version_configs
        SET min_base_version = COALESCE($1, min_base_version),
            latest_base_version = COALESCE($2, latest_base_version),
            min_build_number = COALESCE($3, min_build_number),
            latest_build_number = COALESCE($4, latest_build_number),
            latest_patch_number = COALESCE($5, latest_patch_number),
            is_force_update = COALESCE($6, is_force_update),
            is_patch_required = COALESCE($7, is_patch_required),
            release_notes = COALESCE($8, release_notes),
            download_url = COALESCE($9, download_url),
            checksum_sha256 = COALESCE($10, checksum_sha256),
            updated_at = NOW()
        WHERE id = 'default'
        RETURNING *
        "#,
    )
    .bind(&payload.min_base_version)
    .bind(&payload.latest_base_version)
    .bind(payload.min_build_number)
    .bind(payload.latest_build_number)
    .bind(payload.latest_patch_number)
    .bind(payload.is_force_update)
    .bind(payload.is_patch_required)
    .bind(&payload.release_notes)
    .bind(&payload.download_url)
    .bind(&payload.checksum_sha256)
    .fetch_one(&pool)
    .await
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal memperbarui konfigurasi: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Update in-memory cache instan
    {
        let mut w = cache.write().await;
        *w = updated_config.clone();
    }

    Json(ApiResponse {
        success: true,
        message: "Konfigurasi Version Gatekeeper berhasil diperbarui dan diterapkan ke cache instan.".to_string(),
        data: Some(updated_config),
    })
    .into_response()
}

// ================= SOCIAL COFFEE HANDLERS =================

/// Ambil daftar postingan komunitas (Feed)
pub async fn get_social_posts(
    Query(query): Query<SocialFeedQuery>,
    State(pool): State<PgPool>,
) -> Response {
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let sort = query.sort.unwrap_or_else(|| "latest".to_string());
    let username = query.username.map(|u| u.trim().to_lowercase()).unwrap_or_default();

    let posts = match sort.as_str() {
        "popular" => {
            sqlx::query_as::<_, SocialPost>(
                r#"
                SELECT * FROM social_posts
                ORDER BY (likes_count * 2 + comments_count * 3 + copies_count * 4) DESC, created_at DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
        }
        "recipes" => {
            sqlx::query_as::<_, SocialPost>(
                r#"
                SELECT * FROM social_posts
                WHERE recipe_snapshot IS NOT NULL
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
        }
        _ => {
            sqlx::query_as::<_, SocialPost>(
                r#"
                SELECT * FROM social_posts
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
        }
    };

    let posts = match posts {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal memuat postingan komunitas: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Ambil daftar like pengguna saat ini jika username tersedia
    let liked_post_ids: std::collections::HashSet<Uuid> = if !username.is_empty() && !posts.is_empty() {
        let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id).collect();
        let rows = sqlx::query_scalar::<_, Uuid>(
            "SELECT post_id FROM social_post_likes WHERE username = $1 AND post_id = ANY($2)",
        )
        .bind(&username)
        .bind(&post_ids)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();
        rows.into_iter().collect()
    } else {
        std::collections::HashSet::new()
    };

    let responses: Vec<SocialPostResponse> = posts
        .into_iter()
        .map(|p| {
            let is_liked = liked_post_ids.contains(&p.id);
            let tags: Vec<String> = match p.tags {
                Some(serde_json::Value::Array(arr)) => arr
                    .into_iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                _ => Vec::new(),
            };

            SocialPostResponse {
                id: p.id,
                user_id: p.user_id,
                author_name: p.author_name,
                author_avatar: p.author_avatar,
                author_role: p.author_role,
                content: p.content,
                recipe_id: p.recipe_id,
                recipe_snapshot: p.recipe_snapshot,
                likes_count: p.likes_count,
                comments_count: p.comments_count,
                copies_count: p.copies_count,
                tags,
                is_liked_by_me: is_liked,
                created_at: p.created_at,
                updated_at: p.updated_at,
            }
        })
        .collect();

    Json(ApiResponse {
        success: true,
        message: "Feed Social Coffee berhasil dimuat".to_string(),
        data: Some(responses),
    })
    .into_response()
}

/// Buat postingan diskusi atau bagikan resep baru
pub async fn create_social_post(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(pool): State<PgPool>,
    Json(payload): Json<CreateSocialPostRequest>,
) -> Response {
    let content = payload.content.trim();
    if content.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Konten postingan tidak boleh kosong"
            })),
        )
            .into_response();
    }

    let user_id = auth.as_ref().and_then(|u| Uuid::parse_str(&u.sub).ok());
    let author_name = if let Some(ref u) = auth {
        u.username.clone()
    } else if !payload.author_name.trim().is_empty() {
        payload.author_name.trim().to_string()
    } else {
        "Barista Komunitas".to_string()
    };

    let author_avatar = payload
        .author_avatar
        .or_else(|| Some("assets/images/characters/avatar_1.png".to_string()));

    let author_role = payload.author_role.unwrap_or_else(|| "Barista".to_string());
    let tags_json = serde_json::to_value(payload.tags.unwrap_or_default()).unwrap_or(json!([]));

    let post = match sqlx::query_as::<_, SocialPost>(
        r#"
        INSERT INTO social_posts (
            user_id, author_name, author_avatar, author_role, content,
            recipe_id, recipe_snapshot, tags
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&author_name)
    .bind(&author_avatar)
    .bind(&author_role)
    .bind(content)
    .bind(payload.recipe_id)
    .bind(payload.recipe_snapshot)
    .bind(tags_json)
    .fetch_one(&pool)
    .await
    {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal membuat postingan: {}", e)
                })),
            )
                .into_response();
        }
    };

    let tags: Vec<String> = match post.tags {
        Some(serde_json::Value::Array(arr)) => arr
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    };

    let response = SocialPostResponse {
        id: post.id,
        user_id: post.user_id,
        author_name: post.author_name,
        author_avatar: post.author_avatar,
        author_role: post.author_role,
        content: post.content,
        recipe_id: post.recipe_id,
        recipe_snapshot: post.recipe_snapshot,
        likes_count: post.likes_count,
        comments_count: post.comments_count,
        copies_count: post.copies_count,
        tags,
        is_liked_by_me: false,
        created_at: post.created_at,
        updated_at: post.updated_at,
    };

    (
        StatusCode::CREATED,
        Json(ApiResponse {
            success: true,
            message: "Postingan berhasil dipublikasikan ke Social Coffee".to_string(),
            data: Some(response),
        }),
    )
        .into_response()
}

/// Toggle Like pada postingan
pub async fn toggle_like_social_post(
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
    Json(payload): Json<LikePostRequest>,
) -> Response {
    let username = payload.username.trim().to_lowercase();
    if username.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Username diperlukan untuk memberikan like"
            })),
        )
            .into_response();
    }

    // Periksa apakah sudah di-like
    let already_liked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM social_post_likes WHERE post_id = $1 AND username = $2)",
    )
    .bind(id)
    .bind(&username)
    .fetch_one(&pool)
    .await
    .unwrap_or(false);

    let (is_liked, new_count): (bool, i32) = if already_liked {
        // Hapus like
        let _ = sqlx::query("DELETE FROM social_post_likes WHERE post_id = $1 AND username = $2")
            .bind(id)
            .bind(&username)
            .execute(&pool)
            .await;

        let count: i32 = sqlx::query_scalar(
            "UPDATE social_posts SET likes_count = GREATEST(likes_count - 1, 0) WHERE id = $1 RETURNING likes_count",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

        (false, count)
    } else {
        // Tambah like
        let _ = sqlx::query("INSERT INTO social_post_likes (post_id, username) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id)
            .bind(&username)
            .execute(&pool)
            .await;

        let count: i32 = sqlx::query_scalar(
            "UPDATE social_posts SET likes_count = likes_count + 1 WHERE id = $1 RETURNING likes_count",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap_or(1);

        (true, count)
    };

    Json(ApiResponse {
        success: true,
        message: if is_liked { "Postingan disukai" } else { "Batal menyukai postingan" }.to_string(),
        data: Some(LikePostResponse {
            is_liked,
            likes_count: new_count,
        }),
    })
    .into_response()
}

/// Ambil daftar komentar untuk suatu postingan
pub async fn get_social_post_comments(
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
) -> Response {
    let comments = match sqlx::query_as::<_, SocialComment>(
        "SELECT * FROM social_comments WHERE post_id = $1 ORDER BY created_at ASC",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal memuat komentar: {}", e)
                })),
            )
                .into_response();
        }
    };

    Json(ApiResponse {
        success: true,
        message: "Daftar komentar berhasil dimuat".to_string(),
        data: Some(comments),
    })
    .into_response()
}

/// Tambah komentar pada postingan
pub async fn add_social_post_comment(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
    Json(payload): Json<CreateCommentRequest>,
) -> Response {
    let content = payload.content.trim();
    if content.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Komentar tidak boleh kosong"
            })),
        )
            .into_response();
    }

    let user_id = auth.as_ref().and_then(|u| Uuid::parse_str(&u.sub).ok());
    let author_name = if let Some(ref u) = auth {
        u.username.clone()
    } else if !payload.author_name.trim().is_empty() {
        payload.author_name.trim().to_string()
    } else {
        "Barista".to_string()
    };

    let author_avatar = payload
        .author_avatar
        .or_else(|| Some("assets/images/characters/avatar_1.png".to_string()));

    let comment = match sqlx::query_as::<_, SocialComment>(
        r#"
        INSERT INTO social_comments (post_id, user_id, author_name, author_avatar, content)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(&author_name)
    .bind(&author_avatar)
    .bind(content)
    .fetch_one(&pool)
    .await
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal menambahkan komentar: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Update jumlah komentar di postingan
    let _ = sqlx::query("UPDATE social_posts SET comments_count = comments_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await;

    (
        StatusCode::CREATED,
        Json(ApiResponse {
            success: true,
            message: "Komentar berhasil dikirim".to_string(),
            data: Some(comment),
        }),
    )
        .into_response()
}

/// Copy resep dari postingan (Menambah counter copies_count dan mengembalikan data snapshot resep)
pub async fn copy_social_post_recipe(
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
) -> Response {
    let post = match sqlx::query_as::<_, SocialPost>(
        r#"
        UPDATE social_posts
        SET copies_count = copies_count + 1
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal menyalin resep: {}", e)
                })),
            )
                .into_response();
        }
    };

    Json(ApiResponse {
        success: true,
        message: "Resep berhasil disalin ke koleksi Anda!".to_string(),
        data: Some(CopyRecipeResponse {
            copies_count: post.copies_count,
            recipe_snapshot: post.recipe_snapshot,
        }),
    })
    .into_response()
}

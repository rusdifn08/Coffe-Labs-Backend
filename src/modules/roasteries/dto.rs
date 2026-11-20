use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateRoasteryDto {
    #[validate(length(min = 2, max = 120, message = "Nama roastery harus 2-120 karakter."))]
    pub name: String,
    #[validate(length(min = 2, max = 80, message = "Kota harus 2-80 karakter."))]
    pub city: String,
    #[validate(length(min = 5, max = 500, message = "Alamat lengkap minimal 5 karakter."))]
    pub address: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub cluster: Option<String>,
    #[validate(length(min = 10, max = 2000, message = "Deskripsi roastery minimal 10 karakter."))]
    pub description: String,
    pub specialties: Option<String>,
    pub sales_highlight: Option<String>,
    #[validate(length(min = 8, max = 1024, message = "URL logo roastery wajib valid."))]
    pub logo_url: String,
    pub logo_public_id: Option<String>,
    pub banner_url: Option<String>,
    pub banner_public_id: Option<String>,
    pub whatsapp_number: Option<String>,
    pub instagram_handle: Option<String>,
    pub website_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpdateRoasteryDto {
    #[validate(length(min = 2, max = 120, message = "Nama roastery harus 2-120 karakter."))]
    pub name: Option<String>,
    #[validate(length(min = 2, max = 80, message = "Kota harus 2-80 karakter."))]
    pub city: Option<String>,
    #[validate(length(min = 5, max = 500, message = "Alamat lengkap minimal 5 karakter."))]
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub cluster: Option<String>,
    #[validate(length(min = 10, max = 2000, message = "Deskripsi roastery minimal 10 karakter."))]
    pub description: Option<String>,
    pub specialties: Option<String>,
    pub sales_highlight: Option<String>,
    pub logo_url: Option<String>,
    pub logo_public_id: Option<String>,
    pub banner_url: Option<String>,
    pub banner_public_id: Option<String>,
    pub whatsapp_number: Option<String>,
    pub instagram_handle: Option<String>,
    pub website_url: Option<String>,
    pub is_published: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpsertLineupDto {
    #[validate(length(min = 2, max = 140, message = "Nama kopi harus 2-140 karakter."))]
    pub name: String,
    #[validate(length(min = 2, max = 120, message = "Asal/Origin harus 2-120 karakter."))]
    pub origin: String,
    #[validate(length(min = 2, max = 40, message = "Profil sangrai wajib diisi."))]
    pub roast_profile: String,
    #[validate(length(min = 2, max = 120, message = "Varietas wajib diisi."))]
    pub variety: String,
    #[validate(length(min = 2, max = 80, message = "Proses pasca panen wajib diisi."))]
    pub process: String,
    #[validate(length(min = 3, max = 500, message = "Tasting notes minimal 3 karakter."))]
    pub tasting_notes: String,
    #[validate(range(min = 50, max = 10000, message = "Berat bersih harus 50-10000 gram."))]
    pub net_weight_grams: i32,
    #[validate(range(min = 1000, max = 50_000_000, message = "Harga kemasan minimal Rp1.000."))]
    pub price_idr: i64,
    pub bag_image_url: Option<String>,
    pub bag_image_public_id: Option<String>,
    pub badge: Option<String>,
    pub is_available: Option<bool>,
    pub display_order: Option<i32>,
    pub recommended_brew_method: Option<String>,
    #[validate(range(min = 70, max = 100, message = "Suhu rekomendasi harus 70-100 C."))]
    pub recommended_temp_c: Option<i16>,
    pub recommended_ratio: Option<String>,
    pub recommended_grind_size: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct RoasteryUploadSigDto {
    #[validate(length(min = 3, max = 20, message = "asset_kind harus 'logo', 'banner', atau 'lineup'."))]
    pub asset_kind: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PublicRoasteryQuery {
    pub cluster: Option<String>,
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub fn format_idr_price(amount: i64) -> String {
    let clamped = amount.max(0);
    let digits = clamped.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 2);
    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    let formatted: String = out.chars().rev().collect();
    format!("Rp{formatted}")
}

pub fn compute_unit_price_labels(net_weight_grams: i32, price_idr: i64) -> (String, String, String) {
    let grams = f64::from(net_weight_grams.max(1));
    let per_gram = (price_idr.max(0) as f64) / grams;
    let p100 = (per_gram * 100.0).round() as i64;
    let p200 = (per_gram * 200.0).round() as i64;
    let orig = format!("Asli: {} / {}g", format_idr_price(price_idr), net_weight_grams);
    (format_idr_price(p100), format_idr_price(p200), orig)
}

pub fn slugify_roastery_name(raw: &str, unique_suffix: &str) -> String {
    let mut base = String::with_capacity(raw.len());
    let mut prev_dash = false;
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            base.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !base.is_empty() {
            base.push('-');
            prev_dash = true;
        }
    }
    let clean_base = base.trim_matches('-');
    let prefix = if clean_base.is_empty() { "roastery" } else { clean_base };
    let trimmed_prefix: String = prefix.chars().take(80).collect();
    format!("{trimmed_prefix}-{unique_suffix}")
}

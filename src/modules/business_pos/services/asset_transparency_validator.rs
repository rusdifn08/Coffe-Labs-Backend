use crate::core::error::{AppError, AppResult};
use image::{GenericImageView, Pixel};

pub struct AssetTransparencyValidator;

impl AssetTransparencyValidator {
    /// Validasi biner gambar: SVG atau PNG/WEBP transparan
    pub fn validate_asset_bytes(
        filename: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> AppResult<String> {
        let lower_fn = filename.to_lowercase();
        let is_svg = lower_fn.ends_with(".svg") || content_type.contains("svg");
        let is_png = lower_fn.ends_with(".png") || content_type.contains("png");
        let is_webp = lower_fn.ends_with(".webp") || content_type.contains("webp");

        if !is_svg && !is_png && !is_webp {
            return Err(AppError::BadRequest(
                "Format file ditolak. Hanya PNG, WebP dengan transparansi atau SVG yang diizinkan."
                    .to_string(),
            ));
        }

        if is_svg {
            Self::validate_svg_content(bytes)?;
            return Ok("SVG".to_string());
        }

        // Validasi Raster (PNG atau WebP)
        Self::validate_raster_transparency(bytes)?;
        let format = if is_png { "PNG" } else { "WEBP" };
        Ok(format.to_string())
    }

    fn validate_svg_content(bytes: &[u8]) -> AppResult<()> {
        let text = std::str::from_utf8(bytes).map_err(|_| {
            AppError::BadRequest("File SVG tidak berupa teks UTF-8 yang valid.".to_string())
        })?;

        if !text.contains("<svg") || !text.contains("</svg>") {
            return Err(AppError::BadRequest(
                "Struktur file SVG tidak lengkap atau rusak.".to_string(),
            ));
        }

        // Cek potensi background putih pekat full-canvas
        let lower = text.to_lowercase();
        if lower.contains("<rect")
            && (lower.contains("fill=\"#fff")
                || lower.contains("fill=\"white\"")
                || lower.contains("fill=\"rgb(255,255,255)"))
            && (lower.contains("width=\"100%\"") || lower.contains("height=\"100%\""))
        {
            return Err(AppError::BadRequest(
                "File SVG terdeteksi memiliki layer background putih solid. Hapus background sebelum diunggah."
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn validate_raster_transparency(bytes: &[u8]) -> AppResult<()> {
        let img = image::load_from_memory(bytes).map_err(|e| {
            AppError::BadRequest(format!("Gagal memproses byte gambar raster: {}", e))
        })?;

        if !img.color().has_alpha() {
            return Err(AppError::BadRequest(
                "Gambar tidak memiliki kanal Alpha (transparansi). Gunakan PNG atau WebP 32-bit RGBA."
                    .to_string(),
            ));
        }

        let (width, height) = img.dimensions();
        if width < 16 || height < 16 {
            return Err(AppError::BadRequest(
                "Resolusi gambar terlalu kecil (minimal 16x16 piksel).".to_string(),
            ));
        }

        // Periksa 4 sudut gambar: wajib transparan (alpha < 10)
        let corners = [
            (0, 0),
            (width - 1, 0),
            (0, height - 1),
            (width - 1, height - 1),
        ];

        let mut opaque_corners = 0;
        for (x, y) in corners {
            let pixel = img.get_pixel(x, y);
            let channels = pixel.channels();
            let alpha = if channels.len() >= 4 { channels[3] } else { 255 };
            if alpha > 15 {
                opaque_corners += 1;
            }
        }

        if opaque_corners >= 2 {
            return Err(AppError::BadRequest(
                "Sudut gambar terdeteksi tidak transparan. Pastikan aset menu adalah cut-out tanpa background."
                    .to_string(),
            ));
        }

        Ok(())
    }
}

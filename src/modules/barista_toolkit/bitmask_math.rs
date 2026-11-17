use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

pub const BITMAP_BYTES_LEN: usize = 13;
pub const SEGMENT_COUNT: usize = 100;

/// Mendekode Base64 bitmap 100-bit menjadi array tetap 13 byte dengan bit sisa dibersihkan.
pub fn decode_bitmap_13(raw_b64: &str) -> [u8; BITMAP_BYTES_LEN] {
    let mut out = [0u8; BITMAP_BYTES_LEN];
    if let Ok(decoded) = B64.decode(raw_b64.trim()) {
        let copy_len = decoded.len().min(BITMAP_BYTES_LEN);
        out[..copy_len].copy_from_slice(&decoded[..copy_len]);
    }
    mask_unused_bits(&mut out);
    out
}

/// Operasi bitwise OR idempoten untuk menggabungkan progres video lama & baru tanpa curang skip.
pub fn merge_100bit_segments(
    existing: &[u8],
    incoming: &[u8; BITMAP_BYTES_LEN],
) -> ([u8; BITMAP_BYTES_LEN], i16) {
    let mut out = *incoming;
    for (idx, byte) in out.iter_mut().enumerate() {
        if let Some(prev) = existing.get(idx) {
            *byte |= *prev;
        }
    }
    mask_unused_bits(&mut out);
    let pct = popcount_100bit(&out);
    (out, pct)
}

fn mask_unused_bits(buf: &mut [u8; BITMAP_BYTES_LEN]) {
    buf[12] &= 0b0000_1111;
}

/// Menghitung jumlah segmen 1..100 yang telah ditonton lewat popcount (`count_ones`).
pub fn popcount_100bit(bitmap: &[u8]) -> i16 {
    let mut count: u32 = 0;
    for (idx, &byte) in bitmap.iter().take(BITMAP_BYTES_LEN).enumerate() {
        let valid = if idx == 12 { byte & 0b0000_1111 } else { byte };
        count += valid.count_ones();
    }
    (count as i16).clamp(0, SEGMENT_COUNT as i16)
}

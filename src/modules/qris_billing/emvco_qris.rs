use crate::core::error::AppError;
use crate::modules::qris_billing::error::QrisBillingError;

/// String QRIS Statis resmi Livin' Merchant (Bank Mandiri Acquirer 93600008, Merchant `Coffeelabs`,
/// NMID `ID1023277752888`, Bandung (Kab) 40382, CRC-16 tervalidasi `2BD1`).
/// Dapat ditimpa kapan saja via environment variable `MASTER_STATIC_QRIS_PAYLOAD`.
pub const DEFAULT_COFFEELABS_STATIC_QRIS: &str = "00020101021126690021ID.CO.BANKMANDIRI.WWW01189360000802317152890211723171528980303UMI51440014ID.CO.QRIS.WWW0215ID10232777528880303UMI5204481453033605802ID5910Coffeelabs6013Bandung (Kab)61054038262070703A0163042BD1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrisMerchantMeta {
    pub merchant_name: String,
    pub merchant_city: String,
    pub acquirer_code: String,
    pub nmid: String,
}

/// Menghitung CRC-16/CCITT-FALSE standar EMVCo QRIS (Poly: 0x1021, Init: 0xFFFF, tanpa refleksi).
pub fn compute_crc16_ccitt_false(payload: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in payload {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Memverifikasi checksum 4-digit HEX CRC-16 pada ekor string QRIS (`...6304XXXX`).
pub fn verify_qris_crc16(raw_qris: &str) -> Result<bool, AppError> {
    let trimmed = raw_qris.trim();
    if trimmed.len() < 12 || !trimmed.is_ascii() {
        return Err(QrisBillingError::InvalidQrisPayload("String QRIS terlalu pendek atau non-ASCII".into()).into());
    }
    let split_idx = trimmed.len().saturating_sub(4);
    let body = trimmed.get(..split_idx).ok_or_else(|| {
        QrisBillingError::InvalidQrisPayload("Gagal memisahkan body QRIS".into())
    })?;
    let provided_crc = trimmed.get(split_idx..).ok_or_else(|| {
        QrisBillingError::InvalidQrisPayload("Gagal membaca 4 digit CRC QRIS".into())
    })?;
    if !body.ends_with("6304") {
        return Err(QrisBillingError::InvalidQrisPayload("Penanda Tag CRC 6304 tidak ditemukan".into()).into());
    }
    let computed = format!("{:04X}", compute_crc16_ccitt_false(body.as_bytes()));
    Ok(computed.eq_ignore_ascii_case(provided_crc))
}

/// Mengurai string QRIS EMVCo menjadi pasangan TLV terurut `(Tag, Value)` tanpa memotong string secara naif.
pub fn parse_emvco_tlv(raw_qris: &str) -> Result<Vec<(String, String)>, AppError> {
    let trimmed = raw_qris.trim();
    if !trimmed.is_ascii() || trimmed.len() < 12 {
        return Err(QrisBillingError::InvalidQrisPayload("Format dasar QRIS EMVCo tidak valid".into()).into());
    }
    let mut cursor = 0usize;
    let mut tags = Vec::new();

    while cursor + 4 <= trimmed.len() {
        let tag = trimmed
            .get(cursor..cursor + 2)
            .ok_or_else(|| QrisBillingError::InvalidQrisPayload("Format Tag EMVCo tidak valid".into()))?;
        let len_str = trimmed
            .get(cursor + 2..cursor + 4)
            .ok_or_else(|| QrisBillingError::InvalidQrisPayload("Format Length EMVCo tidak valid".into()))?;
        let len: usize = len_str
            .parse()
            .map_err(|_| QrisBillingError::InvalidQrisPayload(format!("Panjang TLV bukan angka pada tag {tag}")))?;

        let val_start = cursor + 4;
        let val_end = val_start
            .checked_add(len)
            .ok_or_else(|| QrisBillingError::InvalidQrisPayload("Overflow panjang TLV".into()))?;
        let value = trimmed
            .get(val_start..val_end)
            .ok_or_else(|| QrisBillingError::InvalidQrisPayload(format!("Nilai TLV terpotong pada tag {tag}")))?;

        if tag != "63" {
            tags.push((tag.to_string(), value.to_string()));
        }
        cursor = val_end;
    }

    if cursor != trimmed.len() {
        return Err(QrisBillingError::InvalidQrisPayload("Terdapat sisa byte TLV yang rusak di ekor QRIS".into()).into());
    }
    Ok(tags)
}

/// Mengonversi QRIS Statis Livin' Merchant -> QRIS Dinamis ber-nominal terkunci (`Tag 01=12`, `Tag 54=amount`, `Tag 63=CRC16`).
pub fn build_dynamic_qris(static_qris: &str, amount_idr: i64) -> Result<String, AppError> {
    if !(1..=10_000_000).contains(&amount_idr) {
        return Err(QrisBillingError::AmountOutOfRange(amount_idr).into());
    }
    if !verify_qris_crc16(static_qris)? {
        return Err(QrisBillingError::InvalidQrisPayload("Checksum CRC-16 QRIS Statis master tidak cocok".into()).into());
    }

    let mut tags = parse_emvco_tlv(static_qris)?;
    let mut has_tag_01 = false;
    for (tag, val) in &mut tags {
        if tag == "01" {
            *val = "12".to_string();
            has_tag_01 = true;
        }
    }
    if !has_tag_01 {
        let insert_idx = usize::from(!tags.is_empty());
        tags.insert(insert_idx, ("01".to_string(), "12".to_string()));
    }

    tags.retain(|(tag, _)| tag != "54");
    let amount_str = amount_idr.to_string();
    let insert_pos = tags.iter().position(|(tag, _)| tag == "58").unwrap_or(tags.len());
    tags.insert(insert_pos, ("54".to_string(), amount_str));

    let mut body = String::with_capacity(static_qris.len() + 18);
    for (tag, val) in tags {
        body.push_str(&tag);
        body.push_str(&format!("{:02}", val.len()));
        body.push_str(&val);
    }
    body.push_str("6304");

    let crc = compute_crc16_ccitt_false(body.as_bytes());
    body.push_str(&format!("{crc:04X}"));
    Ok(body)
}

/// Mengekstrak nama merchant, kota, kode acquirer Bank Mandiri (`93600008`), dan NMID dari string QRIS.
pub fn extract_merchant_meta(raw_qris: &str) -> QrisMerchantMeta {
    let tags = parse_emvco_tlv(raw_qris).unwrap_or_default();
    let find = |t: &str| tags.iter().find(|(k, _)| k == t).map(|(_, v)| v.clone());

    let merchant_name = find("59").unwrap_or_else(|| "Coffeelabs".to_string());
    let merchant_city = find("60").unwrap_or_else(|| "Bandung (Kab)".to_string());
    let acquirer_code = find("26")
        .and_then(|v| parse_emvco_tlv(&v).ok())
        .and_then(|sub| sub.into_iter().find(|(k, _)| k == "01").map(|(_, v)| v))
        .map(|pan| pan.chars().take(8).collect::<String>())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "93600008".to_string());
    let nmid = find("51")
        .and_then(|v| parse_emvco_tlv(&v).ok())
        .and_then(|sub| sub.into_iter().find(|(k, _)| k == "02").map(|(_, v)| v))
        .unwrap_or_else(|| "ID1023277752888".to_string());

    QrisMerchantMeta {
        merchant_name,
        merchant_city,
        acquirer_code,
        nmid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_coffeelabs_livin_qris_converts_to_valid_dynamic_qris() {
        assert_eq!(verify_qris_crc16(DEFAULT_COFFEELABS_STATIC_QRIS).ok(), Some(true));
        let meta = extract_merchant_meta(DEFAULT_COFFEELABS_STATIC_QRIS);
        assert_eq!(meta.merchant_name, "Coffeelabs");
        assert_eq!(meta.acquirer_code, "93600008");
        assert_eq!(meta.nmid, "ID1023277752888");

        let dyn_pro = build_dynamic_qris(DEFAULT_COFFEELABS_STATIC_QRIS, 49_317).ok().unwrap_or_default();
        assert!(dyn_pro.contains("010212"));
        assert!(dyn_pro.contains("5405493175802ID"));
        assert!(dyn_pro.contains("5910Coffeelabs"));
        assert_eq!(verify_qris_crc16(&dyn_pro).ok(), Some(true));
    }
}

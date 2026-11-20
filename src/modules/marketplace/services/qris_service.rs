use crate::core::error::AppResult;
use crate::modules::marketplace::error::MarketplaceError;

pub const COFFEE_LABS_QRIS_NMID: &str = "ID1026049882714";
pub const COFFEE_LABS_MERCHANT_NAME: &str = "COFFEE LABS ROASTERY";
pub const COFFEE_LABS_MERCHANT_CITY: &str = "JAKARTA";

/// Saat ini checkout Marketplace hanya mengizinkan pembayaran via QRIS.
pub fn enforce_qris_only_payment(payment_method: &str) -> AppResult<()> {
    let normalized = payment_method.trim().to_ascii_uppercase();
    if normalized != "QRIS" {
        return Err(MarketplaceError::QrisOnlyPaymentRequired.into());
    }
    Ok(())
}

fn tlv(tag: &str, value: &str) -> String {
    format!("{tag}{:02}{value}", value.len())
}

/// Menghitung checksum CRC-16/CCITT-FALSE standar EMVCo QRIS Indonesia (polinomial 0x1021).
pub fn compute_emvco_crc16(input: &str) -> String {
    let mut crc: u16 = 0xFFFF;
    for byte in input.as_bytes() {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    format!("{crc:04X}")
}

/// Membentuk string QRIS Dinamis EMVCo dengan nominal unik transaksi (`Tag 54`) & Referensi (`Tag 62`).
pub fn build_dynamic_qris_payload(order_code: &str, amount_idr: i32) -> String {
    let safe_amount = amount_idr.max(0).to_string();
    let merchant_acct = format!(
        "{}{}",
        tlv("00", "ID.CO.QRIS.WWW"),
        tlv("02", COFFEE_LABS_QRIS_NMID)
    );
    let additional_data = tlv("01", &order_code.chars().take(25).collect::<String>());

    let body_without_crc = format!(
        "{}{}{}{}{}{}{}{}{}{}{}",
        tlv("00", "01"),                         // Payload Format Indicator
        tlv("01", "12"),                         // Point of Initiation Method: Dynamic QR
        tlv("26", &merchant_acct),               // Merchant Account Info QRIS
        tlv("52", "5499"),                       // MCC Specialty Food & Coffee Stores
        tlv("53", "360"),                        // Currency: IDR (ISO-4217 360)
        tlv("54", &safe_amount),                 // Exact Transaction Amount
        tlv("58", "ID"),                         // Country Code
        tlv("59", COFFEE_LABS_MERCHANT_NAME),    // Merchant Name
        tlv("60", COFFEE_LABS_MERCHANT_CITY),    // Merchant City
        tlv("62", &additional_data),             // Bill Reference (Order Code)
        "6304"                                   // CRC Tag + Length before checksum
    );

    let crc = compute_emvco_crc16(&body_without_crc);
    format!("{body_without_crc}{crc}")
}

use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct OpenShiftDto {
    #[validate(length(min = 2, max = 50, message = "Nama shift minimal 2 karakter (contoh: Pagi, Malam)."))]
    pub shift_name: String,
    #[validate(range(min = 0, message = "Uang modal awal (cash float) tidak boleh negatif."))]
    pub opening_cash_cents: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CloseShiftDto {
    #[validate(range(min = 0, message = "Hitungan uang tunai fisik tidak boleh negatif."))]
    pub actual_cash_cents: i64,
    pub notes: Option<String>,
}

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, Default, Clone, ToSchema)]
pub struct CursorQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

impl CursorQuery {
    pub fn clamped_limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }

    pub fn decode_cursor(&self) -> Option<(DateTime<Utc>, Uuid)> {
        let raw = self.cursor.as_ref()?;
        let bytes = URL_SAFE_NO_PAD.decode(raw.as_bytes()).ok()?;
        let decoded_str = String::from_utf8(bytes).ok()?;
        let (ts_str, id_str) = decoded_str.split_once('|')?;
        let ts_micros = ts_str.parse::<i64>().ok()?;
        let dt = DateTime::<Utc>::from_timestamp_micros(ts_micros)?;
        let id = Uuid::parse_str(id_str).ok()?;
        Some((dt, id))
    }
}

pub fn encode_cursor(created_at: DateTime<Utc>, id: Uuid) -> String {
    let payload = format!("{}|{}", created_at.timestamp_micros(), id);
    URL_SAFE_NO_PAD.encode(payload.as_bytes())
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub count: usize,
}

impl<T> CursorPage<T> {
    pub fn new(items: Vec<T>, next_cursor: Option<String>, has_more: bool) -> Self {
        let count = items.len();
        Self {
            items,
            next_cursor,
            has_more,
            count,
        }
    }
}

use chrono::{Duration, NaiveDate, Utc};
use sqlx::{FromRow, PgPool};
use tracing::warn;

use crate::modules::monitoring::services::bitmap_analytics_service::BitmapAnalyticsService;

#[derive(FromRow)]
struct BitmapDbRow {
    metric_date: NaiveDate,
    metric_key: String,
    roaring_bitmap_bytes: Vec<u8>,
}

pub async fn load_recent_bitmaps_from_db(pool: &PgPool, service: &BitmapAnalyticsService) {
    let cutoff = Utc::now().date_naive() - Duration::days(32);
    let res = sqlx::query_as::<_, BitmapDbRow>(
        r#"SELECT metric_date, metric_key, roaring_bitmap_bytes
           FROM telemetry_daily_bitmaps
           WHERE metric_date >= $1
           ORDER BY metric_date DESC"#,
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await;

    match res {
        Ok(rows) => {
            for row in rows {
                service.load_bitmap_bytes(row.metric_date, row.metric_key, &row.roaring_bitmap_bytes);
            }
        }
        Err(err) => {
            warn!(error = %err, "Gagal memuat RoaringBitmap historis dari NeonDB");
        }
    }
}

pub async fn save_bitmap_snapshots(
    pool: &PgPool,
    snapshots: &[(NaiveDate, String, Vec<u8>, i32)],
) {
    if snapshots.is_empty() {
        return;
    }

    let mut dates = Vec::with_capacity(snapshots.len());
    let mut keys = Vec::with_capacity(snapshots.len());
    let mut blobs = Vec::with_capacity(snapshots.len());
    let mut cards = Vec::with_capacity(snapshots.len());

    for (d, k, bytes, card) in snapshots {
        dates.push(*d);
        keys.push(k.as_str());
        blobs.push(bytes.as_slice());
        cards.push(*card);
    }

    let res = sqlx::query(
        r#"INSERT INTO telemetry_daily_bitmaps (
               metric_date, metric_key, roaring_bitmap_bytes, cardinality, updated_at
           )
           SELECT v.d, v.k, v.b, v.c, NOW()
           FROM UNNEST($1::date[], $2::varchar[], $3::bytea[], $4::int[]) AS v(d, k, b, c)
           ON CONFLICT (metric_date, metric_key) DO UPDATE SET
               roaring_bitmap_bytes = EXCLUDED.roaring_bitmap_bytes,
               cardinality = EXCLUDED.cardinality,
               updated_at = NOW()"#,
    )
    .bind(&dates)
    .bind(&keys)
    .bind(&blobs)
    .bind(&cards)
    .execute(pool)
    .await;

    if let Err(err) = res {
        warn!(error = %err, "Gagal menyimpan RoaringBitmap harian ke NeonDB");
    }
}

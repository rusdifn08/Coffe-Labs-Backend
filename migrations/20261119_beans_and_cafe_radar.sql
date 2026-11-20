-- Migration: Beans Radar & Cafe Radar 3.0 — Unified 10.000+ PostGIS Dual-Scope Architecture
-- Supports CAFE, BEAN_SELLER, and ROASTERY_AND_CAFE (appears in BOTH Beans Radar & Cafe Radar)

CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

DO $$
BEGIN
    CREATE TYPE radar_place_kind AS ENUM (
        'CAFE',
        'BEAN_SELLER',
        'ROASTERY_AND_CAFE'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS radar_places (
    id UUID PRIMARY KEY,
    external_place_id VARCHAR(160) UNIQUE,
    linked_roastery_id UUID REFERENCES coffee_roasteries(id) ON DELETE SET NULL,
    name VARCHAR(160) NOT NULL,
    kind radar_place_kind NOT NULL DEFAULT 'CAFE',
    category_tag VARCHAR(40) NOT NULL DEFAULT 'COFFEE_SHOP',
    address TEXT NOT NULL DEFAULT '',
    district VARCHAR(100) NOT NULL DEFAULT '',
    city VARCHAR(100) NOT NULL DEFAULT '',
    province VARCHAR(80) NOT NULL DEFAULT '',
    timezone VARCHAR(40) NOT NULL DEFAULT 'Asia/Jakarta',
    latitude DOUBLE PRECISION NOT NULL CHECK (latitude BETWEEN -11.5 AND 6.5),
    longitude DOUBLE PRECISION NOT NULL CHECK (longitude BETWEEN 94.5 AND 141.5),
    location GEOGRAPHY(Point, 4326) NOT NULL,
    rating REAL NOT NULL DEFAULT 4.5 CHECK (rating >= 0.0 AND rating <= 5.0),
    reviews_count INT NOT NULL DEFAULT 0,
    featured_beans JSONB NOT NULL DEFAULT '[]'::jsonb,
    brewing_methods JSONB NOT NULL DEFAULT '[]'::jsonb,
    phone VARCHAR(40),
    whatsapp_number VARCHAR(40),
    instagram_handle VARCHAR(80),
    opening_hours JSONB NOT NULL DEFAULT '{}'::jsonb,
    image_url TEXT,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_radar_places_location_gist
    ON radar_places USING GIST (location)
    WHERE is_active = TRUE;

CREATE INDEX IF NOT EXISTS idx_radar_places_kind_city
    ON radar_places (kind, city)
    WHERE is_active = TRUE;

CREATE INDEX IF NOT EXISTS idx_radar_places_name_trgm_v3
    ON radar_places USING GIN (name gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_radar_places_city_trgm_v3
    ON radar_places USING GIN (city gin_trgm_ops);

-- Tambahkan kolom dual-scope pada coffee_radar_places untuk kompatibilitas mundur 100%
ALTER TABLE coffee_radar_places
    ADD COLUMN IF NOT EXISTS radar_kind radar_place_kind NOT NULL DEFAULT 'CAFE',
    ADD COLUMN IF NOT EXISTS serves_cafe BOOLEAN NOT NULL DEFAULT TRUE;

CREATE INDEX IF NOT EXISTS idx_coffee_radar_places_radar_kind
    ON coffee_radar_places (radar_kind, city_kabupaten)
    WHERE is_active = TRUE;

-- Backfill dual-scope pada coffee_radar_places yang sudah ada (6.732+ lokasi)
UPDATE coffee_radar_places
SET
    radar_kind = CASE
        WHEN category = 'BEAN_SELLER' THEN 'BEAN_SELLER'::radar_place_kind
        WHEN category = 'ROASTERY' OR sells_roast_beans = TRUE THEN 'ROASTERY_AND_CAFE'::radar_place_kind
        ELSE 'CAFE'::radar_place_kind
    END,
    serves_cafe = CASE
        WHEN category = 'BEAN_SELLER' THEN FALSE
        ELSE TRUE
    END;

-- Sinkronkan seluruh record aktif dari coffee_radar_places ke radar_places
INSERT INTO radar_places (
    id, external_place_id, linked_roastery_id, name, kind, category_tag,
    address, district, city, province, timezone,
    latitude, longitude, location, rating, reviews_count,
    featured_beans, brewing_methods, phone, whatsapp_number,
    instagram_handle, opening_hours, image_url, is_verified, is_active,
    created_at, updated_at
)
SELECT
    c.id,
    c.external_place_id,
    c.linked_roastery_id,
    c.name,
    c.radar_kind,
    c.category::text,
    c.full_address,
    c.district_kecamatan,
    c.city_kabupaten,
    c.province,
    c.timezone,
    c.latitude,
    c.longitude,
    c.location,
    CASE WHEN c.rating > 0 THEN c.rating ELSE 4.5 END,
    c.reviews_count,
    CASE
        WHEN c.featured_bean_summary IS NOT NULL AND c.featured_bean_summary <> '' THEN jsonb_build_array(c.featured_bean_summary)
        WHEN c.sells_roast_beans THEN '["Arabica Single Origin", "House Blend Roasted Beans"]'::jsonb
        ELSE '[]'::jsonb
    END,
    CASE
        WHEN c.has_manual_brew_v60 THEN '["V60", "Espresso", "Manual Brew"]'::jsonb
        ELSE '["Espresso", "Kopi Susu"]'::jsonb
    END,
    c.phone_number,
    c.whatsapp_number,
    c.instagram_handle,
    c.opening_hours_json,
    c.thumbnail_url,
    c.is_verified_partner,
    c.is_active,
    c.created_at,
    NOW()
FROM coffee_radar_places c
WHERE c.is_active = TRUE
ON CONFLICT (external_place_id) DO UPDATE SET
    name = EXCLUDED.name,
    kind = EXCLUDED.kind,
    category_tag = EXCLUDED.category_tag,
    address = COALESCE(NULLIF(EXCLUDED.address, ''), radar_places.address),
    city = COALESCE(NULLIF(EXCLUDED.city, ''), radar_places.city),
    province = COALESCE(NULLIF(EXCLUDED.province, ''), radar_places.province),
    latitude = EXCLUDED.latitude,
    longitude = EXCLUDED.longitude,
    location = EXCLUDED.location,
    rating = GREATEST(EXCLUDED.rating, radar_places.rating),
    reviews_count = GREATEST(EXCLUDED.reviews_count, radar_places.reviews_count),
    image_url = COALESCE(EXCLUDED.image_url, radar_places.image_url),
    updated_at = NOW();

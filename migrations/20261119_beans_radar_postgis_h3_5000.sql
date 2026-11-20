-- Migration: Beans Radar 2.0 — PostGIS GEOGRAPHY(Point, 4326) + Uber H3 + pg_trgm
-- Target: 5.000+ Coffee Shop, Roastery, Cafe & Penjual Kopi Terverifikasi se-Indonesia

CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

DO $$
BEGIN
    CREATE TYPE coffee_place_category AS ENUM (
        'ROASTERY',
        'SPECIALTY_CAFE',
        'COFFEE_SHOP',
        'BEAN_SELLER',
        'CAFE_RESTO'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS coffee_radar_places (
    id UUID PRIMARY KEY,
    external_source VARCHAR(30) NOT NULL DEFAULT 'GOOGLE_MAPS',
    external_place_id VARCHAR(160) UNIQUE,
    linked_roastery_id UUID REFERENCES coffee_roasteries(id) ON DELETE SET NULL,

    slug VARCHAR(180) NOT NULL UNIQUE,
    name VARCHAR(160) NOT NULL,
    category coffee_place_category NOT NULL DEFAULT 'COFFEE_SHOP',
    sub_categories JSONB NOT NULL DEFAULT '[]'::jsonb,

    latitude DOUBLE PRECISION NOT NULL CHECK (latitude BETWEEN -11.5 AND 6.5),
    longitude DOUBLE PRECISION NOT NULL CHECK (longitude BETWEEN 94.5 AND 141.5),
    location GEOGRAPHY(Point, 4326) NOT NULL,
    h3_res7 BIGINT NOT NULL,
    h3_res9 BIGINT NOT NULL,

    full_address TEXT NOT NULL DEFAULT '',
    district_kecamatan VARCHAR(100) NOT NULL DEFAULT '',
    city_kabupaten VARCHAR(100) NOT NULL DEFAULT '',
    province VARCHAR(80) NOT NULL DEFAULT '',
    timezone VARCHAR(40) NOT NULL DEFAULT 'Asia/Jakarta',

    rating REAL NOT NULL DEFAULT 0.0 CHECK (rating >= 0.0 AND rating <= 5.0),
    reviews_count INT NOT NULL DEFAULT 0,
    bayesian_score REAL NOT NULL DEFAULT 4.45,
    price_level SMALLINT DEFAULT 2,
    opening_hours_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_24_hours BOOLEAN NOT NULL DEFAULT FALSE,

    sells_roast_beans BOOLEAN NOT NULL DEFAULT FALSE,
    has_manual_brew_v60 BOOLEAN NOT NULL DEFAULT FALSE,
    featured_bean_summary VARCHAR(180),
    phone_number VARCHAR(40),
    whatsapp_number VARCHAR(40),
    instagram_handle VARCHAR(80),
    website_url TEXT,
    google_maps_url TEXT,

    thumbnail_url TEXT,
    cloudinary_public_id TEXT,

    is_verified_partner BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_verified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_radar_places_geog_gist
    ON coffee_radar_places USING GIST (location)
    WHERE is_active = TRUE;

CREATE INDEX IF NOT EXISTS idx_radar_places_h3_res7
    ON coffee_radar_places (h3_res7, bayesian_score DESC)
    WHERE is_active = TRUE;

CREATE INDEX IF NOT EXISTS idx_radar_places_city_category
    ON coffee_radar_places (city_kabupaten, category, bayesian_score DESC)
    WHERE is_active = TRUE;

CREATE INDEX IF NOT EXISTS idx_radar_places_name_trgm
    ON coffee_radar_places USING GIN (name gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_radar_places_address_trgm
    ON coffee_radar_places USING GIN (full_address gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_radar_places_linked_roastery
    ON coffee_radar_places (linked_roastery_id)
    WHERE linked_roastery_id IS NOT NULL AND is_active = TRUE;

CREATE TABLE IF NOT EXISTS user_favorite_coffee_places (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    place_id UUID NOT NULL REFERENCES coffee_radar_places(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, place_id)
);

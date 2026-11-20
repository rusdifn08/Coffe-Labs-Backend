-- Migration: Business Plan Roastery Studio & Public Coffee Lineup Discovery
-- Enforces 1 Business Account = 1 Roastery (UNIQUE owner_user_id)

CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE IF NOT EXISTS coffee_roasteries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    slug VARCHAR(120) NOT NULL UNIQUE,
    name VARCHAR(120) NOT NULL,
    city VARCHAR(80) NOT NULL,
    address TEXT NOT NULL,
    latitude DOUBLE PRECISION NOT NULL DEFAULT -6.2088,
    longitude DOUBLE PRECISION NOT NULL DEFAULT 106.8456,
    cluster VARCHAR(100) NOT NULL DEFAULT 'Roastery Bisnis',
    description TEXT NOT NULL,
    specialties TEXT NOT NULL DEFAULT '',
    sales_highlight VARCHAR(140) NOT NULL DEFAULT 'Roastery Bisnis Terverifikasi',
    logo_url TEXT NOT NULL,
    logo_public_id VARCHAR(255),
    banner_url TEXT,
    banner_public_id VARCHAR(255),
    whatsapp_number VARCHAR(30),
    instagram_handle VARCHAR(60),
    website_url TEXT,
    is_verified_business BOOLEAN NOT NULL DEFAULT TRUE,
    is_published BOOLEAN NOT NULL DEFAULT TRUE,
    rating DOUBLE PRECISION NOT NULL DEFAULT 4.9,
    total_lineups_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_roastery_published_cluster
    ON coffee_roasteries (is_published, cluster, rating DESC, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_roastery_name_trgm
    ON coffee_roasteries USING GIN (name gin_trgm_ops);

CREATE TABLE IF NOT EXISTS roastery_coffee_lineups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    roastery_id UUID NOT NULL REFERENCES coffee_roasteries(id) ON DELETE CASCADE,
    name VARCHAR(140) NOT NULL,
    origin VARCHAR(120) NOT NULL,
    roast_profile VARCHAR(40) NOT NULL,
    variety VARCHAR(120) NOT NULL,
    process VARCHAR(80) NOT NULL,
    tasting_notes TEXT NOT NULL,
    net_weight_grams INT NOT NULL CHECK (net_weight_grams BETWEEN 50 AND 10000),
    price_idr BIGINT NOT NULL CHECK (price_idr >= 1000),
    price_100g_label VARCHAR(40) NOT NULL,
    price_200g_label VARCHAR(40) NOT NULL,
    original_package_label VARCHAR(60) NOT NULL,
    bag_image_url TEXT,
    bag_image_public_id VARCHAR(255),
    badge VARCHAR(60),
    is_available BOOLEAN NOT NULL DEFAULT TRUE,
    display_order INT NOT NULL DEFAULT 0,
    recommended_brew_method VARCHAR(50),
    recommended_temp_c SMALLINT,
    recommended_ratio VARCHAR(20),
    recommended_grind_size VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_roastery_lineup_filter
    ON roastery_coffee_lineups (roastery_id, is_available, roast_profile, display_order ASC);

CREATE INDEX IF NOT EXISTS idx_lineup_name_origin_trgm
    ON roastery_coffee_lineups USING GIN (name gin_trgm_ops);

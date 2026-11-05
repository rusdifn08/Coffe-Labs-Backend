-- ==============================================================================
-- NeonDB PostgreSQL Migration: AI Smart Bean Scanner (Object Detection + Spatial OCR)
-- Includes pg_trgm GIN indexing for sub-5ms typo-resilient fuzzy catalog matching
-- ==============================================================================

CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- 1. Roasteries Catalog
CREATE TABLE IF NOT EXISTS roasteries_catalog (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(120) NOT NULL UNIQUE,
    normalized_aliases TEXT[] NOT NULL DEFAULT '{}',
    country VARCHAR(60) DEFAULT 'Indonesia',
    logo_url TEXT,
    is_verified BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Coffee Beans Catalog
CREATE TABLE IF NOT EXISTS coffee_beans_catalog (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    roastery_id UUID REFERENCES roasteries_catalog(id) ON DELETE SET NULL,
    roastery_name VARCHAR(120) NOT NULL,
    bean_name VARCHAR(160) NOT NULL,
    origin_country VARCHAR(80),
    origin_region VARCHAR(120),
    variety VARCHAR(120) NOT NULL DEFAULT 'Mixed Varieties',
    process VARCHAR(80) NOT NULL DEFAULT 'Washed',
    roast_level VARCHAR(40) NOT NULL DEFAULT 'Light to Medium',
    altitude_masl VARCHAR(50),
    tasting_notes TEXT[] NOT NULL DEFAULT '{}',
    recommended_water_temp_c INT DEFAULT 91,
    recommended_recipe_slugs TEXT[] NOT NULL DEFAULT '{}',
    packaging_image_url TEXT,
    search_vector_text TEXT GENERATED ALWAYS AS (
        LOWER(roastery_name || ' ' || bean_name || ' ' || COALESCE(origin_region, '') || ' ' || variety || ' ' || process)
    ) STORED,
    is_verified BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 3. GIN Trigram Index
CREATE INDEX IF NOT EXISTS idx_beans_search_trgm 
ON coffee_beans_catalog USING GIN (search_vector_text gin_trgm_ops);

-- 4. Seed Roasteries
INSERT INTO roasteries_catalog (name, normalized_aliases, country, is_verified)
VALUES
    ('Smoking Barrels', ARRAY['smoking barrels craft coffee', 'smoking barrels', 'smkng brrls'], 'Indonesia', true),
    ('Common Grounds', ARRAY['common grounds coffee roasters', 'cg roasters'], 'Indonesia', true),
    ('Instinct Roastery', ARRAY['instinct coffee', 'instinct specialty roaster'], 'Indonesia', true),
    ('Fuglen', ARRAY['fuglen oslo', 'fuglen tokyo', 'fuglen coffee roasters'], 'Norway', true),
    ('Onyx Coffee Lab', ARRAY['onyx', 'onyx coffee', 'onyx lab'], 'United States', true),
    ('Anomali Coffee', ARRAY['anomali', 'anomali coffee indonesia'], 'Indonesia', true)
ON CONFLICT (name) DO NOTHING;

-- 5. Seed 10 Realistic Specialty Coffee Beans
INSERT INTO coffee_beans_catalog (
    roastery_name, bean_name, origin_country, origin_region, variety, process, 
    roast_level, altitude_masl, tasting_notes, recommended_water_temp_c, recommended_recipe_slugs, is_verified
)
VALUES
    (
        'Smoking Barrels', 'Halmahera Spice Island', 'Indonesia', 'Halmahera',
        'Kartika & Abyssinia', 'Natural Anaerobic', 'Light', '1500 - 1700 MDPL',
        ARRAY['Strawberry Jam', 'Cacao Nibs', 'Rose Wine', 'Spices'], 90,
        ARRAY['dual-temperature-thermal-drop-v60', 'tetsu-kasuya-4-6'], true
    ),
    (
        'Smoking Barrels', 'Flores Bajawa', 'Indonesia', 'Bajawa',
        'Typica & S795', 'Full Washed', 'Light to Medium', '1400 - 1600 MDPL',
        ARRAY['Milk Chocolate', 'Caramel', 'Hazelnut', 'Green Apple'], 92,
        ARRAY['smoking-barrels-v60', 'james-hoffmann-better-1-cup'], true
    ),
    (
        'Common Grounds', 'Finca Deborah Nirvana', 'Panama', 'Boquete',
        'Geisha', 'Nitrogen Anaerobic', 'Light', '1900 - 2000 MASL',
        ARRAY['Jasmine', 'Bergamot', 'Peach', 'Orange Blossom'], 91,
        ARRAY['lance-hedrick-1-pour-v60', 'tetsu-kasuya-4-6'], true
    ),
    (
        'Common Grounds', 'Cloak & Dagger Blend', 'Indonesia', 'Aceh Gayo & Bali Kintamani',
        'Ateng & Bourbon', 'Washed', 'Medium', '1300 - 1500 MDPL',
        ARRAY['Dark Chocolate', 'Brown Sugar', 'Dried Plum', 'Nutty'], 92,
        ARRAY['tetsu-kasuya-4-6', 'smoking-barrels-v60'], true
    ),
    (
        'Instinct Roastery', 'Gayo Honey Reserve', 'Indonesia', 'Aceh Gayo',
        'Abyssinia & Typica', 'Black Honey', 'Light to Medium', '1600 MDPL',
        ARRAY['Tropical Fruit', 'Honey', 'Melon', 'Sweet Orange'], 91,
        ARRAY['dual-temperature-thermal-drop-v60', 'scott-rao-v60'], true
    ),
    (
        'Fuglen', 'Ethiopia Yirgacheffe Chelchele', 'Ethiopia', 'Yirgacheffe',
        'Heirloom', 'Full Washed', 'Ultra Light', '1950 - 2100 MASL',
        ARRAY['Floral', 'Earl Grey', 'Lemon Peel', 'White Peach'], 93,
        ARRAY['james-hoffmann-better-1-cup', 'lance-hedrick-1-pour-v60'], true
    ),
    (
        'Fuglen', 'Kenya Kiambu Handege', 'Kenya', 'Kiambu',
        'SL28 & SL34', 'Washed', 'Light', '1800 MASL',
        ARRAY['Blackcurrant', 'Grapefruit', 'Tomato Umami', 'Rhubarb'], 93,
        ARRAY['james-hoffmann-better-1-cup', 'tetsu-kasuya-4-6'], true
    ),
    (
        'Onyx Coffee Lab', 'Southern Weather Blend', 'Colombia', 'Huila',
        'Caturra & Colombia', 'Washed', 'Medium', '1650 - 1850 MASL',
        ARRAY['Milk Chocolate', 'Plum', 'Candied Walnut', 'Citrus'], 92,
        ARRAY['tetsu-kasuya-4-6', 'smoking-barrels-v60'], true
    ),
    (
        'Onyx Coffee Lab', 'Tropical Weather', 'Ethiopia', 'Guji',
        'Heirloom', 'Natural & Washed', 'Light', '1900 - 2200 MASL',
        ARRAY['Mango', 'Pineapple', 'Berries', 'Sweet Tea'], 91,
        ARRAY['dual-temperature-thermal-drop-v60', 'lance-hedrick-1-pour-v60'], true
    ),
    (
        'Anomali Coffee', 'Bali Kintamani Carbonic', 'Indonesia', 'Kintamani',
        'Kartika', 'Carbonic Maceration', 'Light to Medium', '1450 MDPL',
        ARRAY['Winey', 'Citrus Orange', 'Dark Cherry', 'Caramel'], 90,
        ARRAY['dual-temperature-thermal-drop-v60', 'scott-rao-v60'], true
    )
ON CONFLICT DO NOTHING;

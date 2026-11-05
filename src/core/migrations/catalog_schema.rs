pub fn statements() -> &'static [&'static str] {
    &[
        "CREATE EXTENSION IF NOT EXISTS pg_trgm",
        r#"CREATE TABLE IF NOT EXISTS roasteries_catalog (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(120) NOT NULL UNIQUE,
            normalized_aliases TEXT[] NOT NULL DEFAULT '{}',
            country VARCHAR(60) DEFAULT 'Indonesia',
            logo_url TEXT,
            is_verified BOOLEAN DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS coffee_beans_catalog (
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
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_beans_search_trgm ON coffee_beans_catalog USING GIN (search_vector_text gin_trgm_ops)",
    ]
}

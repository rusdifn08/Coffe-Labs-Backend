pub fn statements() -> &'static [&'static str] {
    &[
        r#"CREATE TABLE IF NOT EXISTS trained_coffee_models (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            class_slug VARCHAR(120) NOT NULL UNIQUE,
            roastery_name VARCHAR(120) NOT NULL,
            bean_name VARCHAR(160) NOT NULL,
            varietal VARCHAR(120) NOT NULL DEFAULT 'Mixed Varieties',
            process VARCHAR(80) NOT NULL DEFAULT 'Washed',
            tasting_notes TEXT[] NOT NULL DEFAULT '{}',
            altitude_masl VARCHAR(50),
            recommended_recipe_id VARCHAR(80),
            weights_vector FLOAT4[] NOT NULL DEFAULT '{}',
            sub_centroids JSONB NOT NULL DEFAULT '[]',
            cosine_threshold FLOAT4 NOT NULL DEFAULT 0.72,
            augmented_samples_count INT NOT NULL DEFAULT 45,
            validation_accuracy FLOAT4 NOT NULL DEFAULT 0.98,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_trained_models_slug ON trained_coffee_models (class_slug)",
    ]
}

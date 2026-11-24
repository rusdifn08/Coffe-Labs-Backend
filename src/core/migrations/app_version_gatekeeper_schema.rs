pub fn statements() -> &'static [&'static str] {
    &[
        "ALTER TABLE app_version_configs ADD COLUMN IF NOT EXISTS is_patch_required BOOLEAN NOT NULL DEFAULT FALSE",
        "ALTER TABLE app_version_configs ADD COLUMN IF NOT EXISTS release_notes TEXT NOT NULL DEFAULT 'Pembaruan stabilitas sistem dan optimasi fitur Coffee Labs.'",
        "ALTER TABLE app_version_configs ADD COLUMN IF NOT EXISTS checksum_sha256 VARCHAR(128)",
        "ALTER TABLE app_version_configs ADD COLUMN IF NOT EXISTS ios_download_url TEXT",
        "ALTER TABLE app_version_configs ADD COLUMN IF NOT EXISTS windows_download_url TEXT",
        "ALTER TABLE app_version_configs ADD COLUMN IF NOT EXISTS content_manifest_version INT NOT NULL DEFAULT 1",
        r#"INSERT INTO app_version_configs (
            id, platform, min_base_version, latest_base_version,
            min_build_number, latest_build_number, latest_patch_number,
            is_force_update, is_patch_required, release_notes,
            download_url, ios_download_url, windows_download_url, content_manifest_version
        ) VALUES (
            'default', 'all', '2.0.0', '2.5.1', 2, 9, 0, FALSE, FALSE,
            'Coffee Labs v2.5.1: Cafe Radar fokus rute ala Google Maps, Email & Lupa Password OTP, Cloud Sync per akun, Jurnal Seduh grafik sensori, & Moderasi Multibahasa.',
            'https://github.com/rusdifn08/coffe-labs-mobile/releases/download/v2.5.1/Coffe_Labs_25.apk',
            'https://github.com/rusdifn08/coffe-labs-mobile/releases/download/v2.5.1/CoffeLabs25.ipa',
            'https://github.com/rusdifn08/coffe-labs-mobile/releases/latest',
            1
        ) ON CONFLICT (id) DO NOTHING"#,
        r#"UPDATE app_version_configs
           SET min_base_version = '2.0.0',
               latest_base_version = '2.5.1',
               min_build_number = 2,
               latest_build_number = 9,
               download_url = 'https://github.com/rusdifn08/coffe-labs-mobile/releases/download/v2.5.1/Coffe_Labs_25.apk',
               ios_download_url = COALESCE(NULLIF(ios_download_url, ''), 'https://github.com/rusdifn08/coffe-labs-mobile/releases/download/v2.5.1/CoffeLabs25.ipa'),
               windows_download_url = COALESCE(NULLIF(windows_download_url, ''), 'https://github.com/rusdifn08/coffe-labs-mobile/releases/latest'),
               release_notes = 'Coffee Labs v2.5.1: Cafe Radar fokus rute ala Google Maps, Email & Lupa Password OTP, Cloud Sync per akun, Jurnal Seduh grafik sensori, & Moderasi Multibahasa.',
               updated_at = NOW()
           WHERE id = 'default'
             AND (latest_base_version = '1.0.0' OR download_url LIKE '%rusdi-labs/coffee-hub-mobile%')"#,
    ]
}

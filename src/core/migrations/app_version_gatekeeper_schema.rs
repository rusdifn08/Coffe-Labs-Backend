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
            download_url, ios_download_url, windows_download_url,
            checksum_sha256, content_manifest_version
        ) VALUES (
            'default', 'all', '2.0.0', '2.5.2', 2, 10, 0, FALSE, FALSE,
            'Coffee Labs v2.5.2 (Shorebird Delta-OTA Base): pasang sekali, seluruh update fitur & perbaikan berikutnya otomatis ringan ~1-5 MB di dalam aplikasi tanpa install ulang.',
            'https://github.com/rusdifn08/coffe-labs-mobile/releases/download/v2.5.2/Coffe_Labs_25.apk',
            'https://github.com/rusdifn08/coffe-labs-mobile/releases/download/v2.5.1/CoffeLabs25.ipa',
            'https://github.com/rusdifn08/coffe-labs-mobile/releases/latest',
            '6C29C51F39CD75B4F985C88733D62A9BC49FD6F0DD55E1D451A76A91FD6E954D',
            1
        ) ON CONFLICT (id) DO NOTHING"#,
        r#"UPDATE app_version_configs
           SET min_base_version = '2.0.0',
               latest_base_version = '2.5.2',
               min_build_number = 2,
               latest_build_number = 10,
               download_url = 'https://github.com/rusdifn08/coffe-labs-mobile/releases/download/v2.5.2/Coffe_Labs_25.apk',
               ios_download_url = COALESCE(NULLIF(ios_download_url, ''), 'https://github.com/rusdifn08/coffe-labs-mobile/releases/download/v2.5.1/CoffeLabs25.ipa'),
               windows_download_url = COALESCE(NULLIF(windows_download_url, ''), 'https://github.com/rusdifn08/coffe-labs-mobile/releases/latest'),
               checksum_sha256 = '6C29C51F39CD75B4F985C88733D62A9BC49FD6F0DD55E1D451A76A91FD6E954D',
               release_notes = 'Coffee Labs v2.5.2 (Shorebird Delta-OTA Base): pasang sekali, seluruh update fitur & perbaikan berikutnya otomatis ringan ~1-5 MB di dalam aplikasi tanpa install ulang.',
               updated_at = NOW()
           WHERE id = 'default'
             AND (latest_base_version IN ('1.0.0', '2.5.1') OR download_url LIKE '%rusdi-labs/coffee-hub-mobile%')"#,
    ]
}

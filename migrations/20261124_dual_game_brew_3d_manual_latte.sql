-- Game Brew Studio 3D v2.0: Dual Discipline (Manual Brew 3D + Latte Art 3D)
-- Matches Blueprint Arsitektur Game Brew 3D v2.0 (20 Progressive Levels + Progress)

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'game_brew_discipline') THEN
    CREATE TYPE game_brew_discipline AS ENUM ('MANUAL_BREW_3D', 'LATTE_ART_3D');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'game_brew_difficulty') THEN
    CREATE TYPE game_brew_difficulty AS ENUM ('BASIC', 'INTERMEDIATE', 'ADVANCE', 'MASTER');
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS game_brew_levels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    level_code VARCHAR(40) NOT NULL UNIQUE,
    discipline game_brew_discipline NOT NULL,
    level_number SMALLINT NOT NULL CHECK (level_number BETWEEN 1 AND 20),
    difficulty game_brew_difficulty NOT NULL,
    title VARCHAR(140) NOT NULL,
    subtitle VARCHAR(240) NOT NULL,
    mentor_intro_dialogue TEXT NOT NULL,
    target_parameters_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    xp_reward_base INT NOT NULL DEFAULT 150,
    xp_three_star_bonus INT NOT NULL DEFAULT 100,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_discipline_level_num UNIQUE (discipline, level_number)
);

CREATE TABLE IF NOT EXISTS user_game_brew_progress (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    level_id UUID NOT NULL REFERENCES game_brew_levels(id) ON DELETE CASCADE,
    discipline game_brew_discipline NOT NULL,
    stars_earned SMALLINT NOT NULL DEFAULT 0 CHECK (stars_earned BETWEEN 0 AND 3),
    best_overall_score_pct REAL NOT NULL DEFAULT 0.0,
    best_physics_metrics_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    best_latte_art_snapshot_url TEXT,
    attempts_count INT NOT NULL DEFAULT 1,
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, level_id)
);

CREATE INDEX IF NOT EXISTS idx_game_brew_user_discipline
    ON user_game_brew_progress (user_id, discipline, stars_earned DESC);

-- Seed 10 Manual Brew 3D Levels (MB-01..MB-10) & 10 Latte Art 3D Levels (LA-01..LA-10)
INSERT INTO game_brew_levels (level_code, discipline, level_number, difficulty, title, subtitle, mentor_intro_dialogue, target_parameters_json, xp_reward_base, xp_three_star_bonus)
VALUES
  ('MB-01', 'MANUAL_BREW_3D', 1, 'BASIC', 'Ritual Kertas & Air Bilasan', 'Basahi >= 95% kertas filter, panaskan server, & buang air bilasan.', 'Miringkan ketel kaca transparan memutar untuk membilas kertas filter dan memanaskan server.', '{"dose_g":15,"water_ml":180,"pattern":"circular_cw","require_ritual":true}'::jsonb, 150, 100),
  ('MB-02', 'MANUAL_BREW_3D', 2, 'BASIC', 'Seni RDT & Center Divot', 'Semprot 1-2x RDT, leveling, & buat lubang tengah 4-9 mm.', 'Semprotan RDT mengurangi fines retention hingga 22%, lalu tekan tengah bubuk membentuk divot.', '{"dose_g":15,"water_ml":210,"pattern":"circular_cw","rdt_optimal":2}'::jsonb, 150, 100),
  ('MB-03', 'MANUAL_BREW_3D', 3, 'BASIC', 'Napas Pertama: 100% Wet Blooming', 'Jaga laju 4.0 ml/s hingga 100% bubuk basah tanpa dry spots.', 'Ikuti Dry-Spot Heatmap sampai seluruh permukaan kopi terbasahi merata.', '{"dose_g":15,"water_ml":225,"bloom_ml":45,"ideal_flow":4.0}'::jsonb, 150, 100),
  ('MB-04', 'MANUAL_BREW_3D', 4, 'BASIC', 'Aliran Laminar (Re < 2300)', 'Jaga tinggi ketel rendah agar pancaran kaca tidak pecah & nol bypass.', 'Pertahankan tinggi tuang < 11 cm agar hukum Rayleigh-Plateau tidak memecah stream.', '{"dose_g":15,"water_ml":225,"pattern":"center_osmotic","max_height_cm":11}'::jsonb, 160, 100),
  ('MB-05', 'MANUAL_BREW_3D', 5, 'INTERMEDIATE', 'Tarian Spiral & Gentle Rao Swirl', 'Tuang spiral keluar-masuk lalu Rao Swirl hingga Flat Bed >= 90%.', 'Goyangkan lembut melingkar agar difusi Laplacian meratakan dinding ampas kopi.', '{"dose_g":15,"water_ml":240,"pattern":"spiral_swirl","flat_bed_min":90}'::jsonb, 180, 120),
  ('MB-06', 'MANUAL_BREW_3D', 6, 'INTERMEDIATE', 'Matematika 4:6 Tetsu Kasuya', '5x pulse pour presisi (50->120->180->240->300 ml) fokus sweetness.', 'Pantau skala ml di ketel kaca transparan agar tiap pulse berhenti tepat.', '{"dose_g":20,"water_ml":300,"pattern":"pulse_46","pulses":5}'::jsonb, 180, 120),
  ('MB-07', 'MANUAL_BREW_3D', 7, 'INTERMEDIATE', 'Flash-Chill Japanese Iced V60', 'Imbangi 40% es batu di server dengan tuangan lambat terkontrol.', 'Gunakan pola zigzag lembut dengan laju rendah agar ekstraksi kaya sebelum mengenai es.', '{"dose_g":18,"water_ml":180,"ice_g":120,"pattern":"zigzag"}'::jsonb, 190, 120),
  ('MB-08', 'MANUAL_BREW_3D', 8, 'ADVANCE', 'Dual-Temp Thermal Drop (93->70C)', 'Turunkan suhu di tuangan akhir ala Sherry Hsu WBrC pangkas pahit.', 'Awali ekstraksi manis di 93C lalu turunkan suhu untuk menutup cangkir bersih.', '{"dose_g":16,"water_ml":240,"temp_c":93,"drop_temp_c":70,"pattern":"reverse_circular_ccw"}'::jsonb, 220, 150),
  ('MB-09', 'MANUAL_BREW_3D', 9, 'ADVANCE', 'Penyelamatan Ethiopian Fines', 'Tinggi ketel ultra-rendah & agitasi minimal agar Darcy tak mampet.', 'Biji Ethiopia tinggi fines memerlukan RDT tepat dan aliran paling lembut.', '{"dose_g":15,"water_ml":240,"high_fines":true,"ideal_flow":3.5}'::jsonb, 240, 150),
  ('MB-10', 'MANUAL_BREW_3D', 10, 'MASTER', 'Panggung World Brewers Cup (Blind)', 'Seduh sempurna tanpa angka timbangan — murni skala ketel kaca!', 'Andalkan garis ml ketel borosilikat dan insting tangan untuk EY 19.5-20.8%.', '{"dose_g":15,"water_ml":240,"blind_scale":true,"target_ey_min":19.5,"target_ey_max":20.8}'::jsonb, 300, 200),

  ('LA-01', 'LATTE_ART_3D', 1, 'BASIC', 'Kanvas Emas (High Pour & Crema)', 'Tahan pitcher tinggi (> 4 cm) isi 60% cangkir tanpa bercak putih.', 'Biarkan jet susu menyelam bersih di bawah crema untuk membangun kanvas emas rata.', '{"pattern":"canvas_base","min_height_cm":4.0,"target_fill_pct":65}'::jsonb, 150, 100),
  ('LA-02', 'LATTE_ART_3D', 2, 'BASIC', 'Monk''s Head (Turunkan Pitcher)', 'Transisi High Pour -> Low Pour (< 1.2 cm) untuk bola foam simetris.', 'Turunkan pitcher rendah tepat di pusat saat cangkir mencapai 55%.', '{"pattern":"monks_head","float_height_cm":1.2}'::jsonb, 150, 100),
  ('LA-03', 'LATTE_ART_3D', 3, 'BASIC', 'Classic Heart (Pull-Through Cut)', 'Kembangkan bola foam lalu naikkan pitcher & sapu lurus ke depan.', 'Naikkan pitcher lalu tarik lurus membelah puncak bola foam membentuk Hati.', '{"pattern":"heart"}'::jsonb, 160, 100),
  ('LA-04', 'LATTE_ART_3D', 4, 'INTERMEDIATE', 'Stacked 2-Layer Tulip', 'Teknik Stop-Move-Push 2 lapisan lalu akhiri Pull-Through Cut.', 'Tuang lapisan pertama, jeda singkat, dorong lapisan kedua, lalu Cut.', '{"pattern":"two_stack_tulip","stacks":2}'::jsonb, 180, 120),
  ('LA-05', 'LATTE_ART_3D', 5, 'INTERMEDIATE', 'Classic 3-Stack Tulip', 'Presisi ritme 3 dorongan terpisah dengan kontrol microfoam pitcher.', 'Pantau sisa microfoam di pitcher kaca transparan agar 3 lapisan seimbang.', '{"pattern":"three_stack_tulip","stacks":3}'::jsonb, 190, 120),
  ('LA-06', 'LATTE_ART_3D', 6, 'INTERMEDIATE', 'Winged Base (Gyroscope Wiggle)', 'Goyangkan kiri-kanan 2.5 Hz agar lapisan dasar membungkus cangkir.', 'Gunakan osilasi wiggle stabil sehingga fluida melengkung memeluk dinding cangkir.', '{"pattern":"winged_base","wiggle_hz":2.5}'::jsonb, 200, 120),
  ('LA-07', 'LATTE_ART_3D', 7, 'ADVANCE', 'Classic Rosetta (Ripple & Pull)', 'Wiggle berirama sambil perlahan mundur lalu potong lurus ke depan.', 'Mundur perlahan menuju tepi belakang cangkir sambil menjaga ritme wiggle.', '{"pattern":"rosetta","wiggle_hz":2.4}'::jsonb, 220, 150),
  ('LA-08', 'LATTE_ART_3D', 8, 'ADVANCE', 'Winged 5-Stack Tulip', 'Gabungkan Winged Base lebar dengan 4 tumpukan push presisi tinggi.', 'Atur ruang sejak awal agar 5 lapisan muat sempurna di kapasitas 200 ml.', '{"pattern":"winged_five_tulip","stacks":5}'::jsonb, 240, 150),
  ('LA-09', 'LATTE_ART_3D', 9, 'ADVANCE', 'Slowsetta (Thick High-Contrast)', 'Wiggle lambat (1.4 Hz) amplitudo lebar untuk daun tebal berkontras.', 'Perlambat ritme goyangan agar helaian microfoam lebar dan tajam.', '{"pattern":"slowsetta","wiggle_hz":1.4}'::jsonb, 250, 150),
  ('LA-10', 'LATTE_ART_3D', 10, 'MASTER', 'The Graceful Swan (WLAC Master)', 'Sayap Rosetta miring, tarik lengkung leher S, & tutup Mini Heart.', 'Bentuk sayap Rosetta di kanan, tarik garis leher S halus, dan akhiri kepala Hati kecil.', '{"pattern":"swan","wiggle_hz":2.4}'::jsonb, 300, 200)
ON CONFLICT (level_code) DO NOTHING;

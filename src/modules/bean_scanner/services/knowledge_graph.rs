/// Stage 3: Origin Geography & Terroir Knowledge Graph
/// Maps geographic keywords to canonical origin, terroir details, and default varietals.
use strsim::jaro_winkler;

#[derive(Debug, Clone)]
pub struct TerroirProfile {
    pub canonical_name: &'static str,
    pub region_province: &'static str,
    pub default_altitude: &'static str,
    pub default_varietals: &'static str,
    pub keywords: &'static [&'static str],
}

pub const ORIGIN_PROFILES: &[TerroirProfile] = &[
    // NOTE: Urutan kritis — profil dengan keyword paling spesifik dievaluasi lebih dulu
    // agar "Gunung Halu" tidak tertelan oleh keyword longgar "halu"/"malabar".
    TerroirProfile {
        canonical_name: "Kamojang Garut",
        region_province: "Kamojang, Garut, Jawa Barat",
        default_altitude: "1400 - 1750 MDPL",
        default_varietals: "Typica, Sigararutang, Mixed Variety",
        keywords: &["kamojang", "garut", "amojang", "wanoja", "papandayan", "cikuray"],
    },
    TerroirProfile {
        canonical_name: "Gunung Halu",
        region_province: "Gunung Halu, Bandung Barat, Jawa Barat",
        default_altitude: "1350 - 1650 MDPL",
        default_varietals: "Sigararutang, Lini S795, Ateng Super",
        keywords: &[
            "gunung halu",
            "gununghalu",
            "gn halu",
            "haloe",
            "sindangkerta",
            "halu",
        ],
    },
    TerroirProfile {
        canonical_name: "Gunung Puntang",
        region_province: "Gunung Puntang, Bandung Selatan, Jawa Barat",
        default_altitude: "1200 - 1600 MDPL",
        default_varietals: "Java Typica, Sigararutang, Lini S795",
        keywords: &["gunung puntang", "gn puntang", "puntang", "puntan"],
    },
    TerroirProfile {
        canonical_name: "Malabar",
        region_province: "Gunung Malabar / Pangalengan, Jawa Barat",
        default_altitude: "1400 - 1800 MDPL",
        default_varietals: "Sigararutang, Lini S795, Kartika",
        keywords: &["malabar", "pangalengan", "ciwidey", "preanger"],
    },
    TerroirProfile {
        canonical_name: "Aceh Gayo",
        region_province: "Dataran Tinggi Gayo, Aceh",
        default_altitude: "1400 - 1750 MDPL",
        default_varietals: "Abyssinia, Timtim, Bourbon, Ateng Super",
        keywords: &["gayo", "aceh", "takengon", "bener meriah", "pantan musara", "kenawat"],
    },
    TerroirProfile {
        canonical_name: "Bali Kintamani",
        region_province: "Kintamani, Bali",
        default_altitude: "1200 - 1500 MDPL",
        default_varietals: "Kartika, Kopyol, USDA 762",
        keywords: &["kintamani", "bali", "ulian", "batur", "mengani"],
    },
    TerroirProfile {
        canonical_name: "Toraja",
        region_province: "Toraja, Sulawesi Selatan",
        default_altitude: "1500 - 1900 MDPL",
        default_varietals: "Lini S795 (Jember), Typica",
        keywords: &["toraja", "sapan", "pulu pulu", "enrekang", "mamasa"],
    },
    TerroirProfile {
        canonical_name: "Flores Bajawa",
        region_province: "Flores Bajawa, NTT",
        default_altitude: "1300 - 1600 MDPL",
        default_varietals: "Juria (Typica), Kartika, Lini S795",
        keywords: &["flores", "bajawa", "manggarai", "jurang"],
    },
    TerroirProfile {
        canonical_name: "Kerinci",
        region_province: "Gunung Kerinci, Sumatera",
        default_altitude: "1400 - 1700 MDPL",
        default_varietals: "Andungsari, Sigararutang",
        keywords: &["kerinci", "solok", "koerintji", "jambi", "kayu aro"],
    },
    TerroirProfile {
        canonical_name: "Temanggung",
        region_province: "Temanggung, Jawa Tengah",
        default_altitude: "1300 - 1600 MDPL",
        default_varietals: "Kartika, Lini S795",
        keywords: &["temanggung", "sindoro", "sumbing", "wonosobo"],
    },
    TerroirProfile {
        canonical_name: "Java Ijen",
        region_province: "Kawah Ijen, Jawa Timur",
        default_altitude: "1300 - 1600 MDPL",
        default_varietals: "USDA 762, Kartika, Blue Mountain",
        keywords: &["ijen", "raung", "argopuro", "wonosalam"],
    },
    TerroirProfile {
        canonical_name: "Papua Wamena",
        region_province: "Pegunungan Tengah / Wamena, Papua",
        default_altitude: "1500 - 2000 MDPL",
        default_varietals: "Typica, Bourbon",
        keywords: &["wamena", "bpiri", "jayawijaya", "papua", "moanemani", "halmahera"],
    },
    TerroirProfile {
        canonical_name: "Ethiopia",
        region_province: "Sidama / Yirgacheffe / Guji, Ethiopia",
        default_altitude: "1850 - 2200 MDPL",
        default_varietals: "Ethiopian Heirloom (74110 / 74158)",
        keywords: &["ethiopia", "yirgacheffe", "guji", "sidamo", "chelbesa"],
    },
    TerroirProfile {
        canonical_name: "Colombia",
        region_province: "Huila / Cauca / Tolima, Colombia",
        default_altitude: "1600 - 2000 MDPL",
        default_varietals: "Pink Bourbon, Caturra, Castillo",
        keywords: &["colombia", "huila", "cauca", "tolima"],
    },
    TerroirProfile {
        canonical_name: "Panama Boquete",
        region_province: "Chiriquí, Panama",
        default_altitude: "1650 - 1950 MDPL",
        default_varietals: "Panama Geisha, Catuai",
        keywords: &["panama", "boquete", "volcan"],
    },
    TerroirProfile {
        canonical_name: "Kenya",
        region_province: "Nyeri / Kiambu / Kirinyaga, Kenya",
        default_altitude: "1700 - 2000 MDPL",
        default_varietals: "SL28, SL34, Ruiru 11",
        keywords: &["kenya", "nyeri", "kiambu", "kirinyaga"],
    },
];

pub struct KnowledgeGraph;

impl KnowledgeGraph {
    /// Fuzzy lookup against Terroir Knowledge Graph using Jaro-Winkler.
    /// Strategi: keyword terpanjang yang cocok menang (longest-match-wins) supaya
    /// "gunung halu" tidak kalah oleh keyword longgar "halu", dan "gunung puntang"
    /// menang atas "puntang" bila keduanya ada di teks.
    pub fn find_origin(text: &str) -> Option<&'static TerroirProfile> {
        let clean = text.to_lowercase();
        let words: Vec<&str> = clean
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() >= 3)
            .collect();

        // 1. Direct substring matching — longest keyword wins.
        let mut best_substring: Option<(&'static TerroirProfile, usize)> = None;
        for profile in ORIGIN_PROFILES {
            for &kw in profile.keywords {
                if clean.contains(kw) {
                    let len = kw.len();
                    // Utamakan keyword terpanjang; jika sama panjang, profil yang lebih
                    // spesifik (dievaluasi lebih dulu) tetap dipertahankan.
                    if best_substring.map_or(true, |(_, best_len)| len > best_len) {
                        best_substring = Some((profile, len));
                    }
                }
            }
        }
        if let Some((profile, _)) = best_substring {
            return Some(profile);
        }

        // 2. Fuzzy match on word tokens (threshold >= 0.85) — longest keyword wins.
        let mut best_fuzzy: Option<(&'static TerroirProfile, usize, f64)> = None;
        for profile in ORIGIN_PROFILES {
            for &kw in profile.keywords {
                for &word in &words {
                    let score = jaro_winkler(word, kw);
                    if score >= 0.85 {
                        let len = kw.len();
                        let is_better = match best_fuzzy {
                            None => true,
                            Some((_, best_len, best_score)) => {
                                len > best_len || (len == best_len && score > best_score)
                            }
                        };
                        if is_better {
                            best_fuzzy = Some((profile, len, score));
                        }
                    }
                }
            }
        }
        if let Some((profile, _, _)) = best_fuzzy {
            return Some(profile);
        }

        None
    }
}

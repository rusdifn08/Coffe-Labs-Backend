use coffe_labs_backend::modules::bean_scanner::dto::OcrTextBlockDto;
use coffe_labs_backend::modules::bean_scanner::services::cognitive_resolver::CognitiveResolver;
use coffe_labs_backend::modules::bean_scanner::services::fuzzy_flavor_matcher::FuzzyFlavorMatcher;

#[test]
fn test_cognitive_scanner_bak_coffee_bean_malabar_wine() {
    let raw_text = r#"
BAK COFFEE BEAN
MALABAR
WINE
FRESH BEAN
BETTER BREW
ROASTING DATE
"#;

    let blocks = vec![
        OcrTextBlockDto {
            text: "BAK COFFEE BEAN".to_string(),
            normalized_height: 0.05,
            top_y: 0.36,
            left_x: 0.20,
            normalized_width: 0.60,
            confidence: 0.98,
        },
        OcrTextBlockDto {
            text: "MALABAR".to_string(),
            normalized_height: 0.14,
            top_y: 0.44,
            left_x: 0.25,
            normalized_width: 0.50,
            confidence: 0.99,
        },
        OcrTextBlockDto {
            text: "WINE".to_string(),
            normalized_height: 0.08,
            top_y: 0.52,
            left_x: 0.35,
            normalized_width: 0.30,
            confidence: 0.96,
        },
        OcrTextBlockDto {
            text: "FRESH BEAN".to_string(),
            normalized_height: 0.03,
            top_y: 0.78,
            left_x: 0.15,
            normalized_width: 0.30,
            confidence: 0.90,
        },
        OcrTextBlockDto {
            text: "BETTER BREW".to_string(),
            normalized_height: 0.03,
            top_y: 0.82,
            left_x: 0.15,
            normalized_width: 0.30,
            confidence: 0.90,
        },
        OcrTextBlockDto {
            text: "ROASTING DATE".to_string(),
            normalized_height: 0.03,
            top_y: 0.88,
            left_x: 0.15,
            normalized_width: 0.35,
            confidence: 0.88,
        },
    ];

    let result = CognitiveResolver::analyze(raw_text, &blocks);

    assert_eq!(result.roastery_name, "Bak Coffee Bean");
    assert_eq!(result.bean_name, "Malabar Wine");
    assert!(result.origin.contains("Malabar"), "Origin should reference Malabar, got {}", result.origin);
    assert_eq!(result.process, "Wine Process");
    assert_eq!(result.notes_source, "AI_SOMMELIER_INFERRED");
    assert!(result.is_ai_inferred_altitude);
    assert!(result.is_ai_inferred_variety);

    // AI Sommelier inferred notes for Wine Process
    assert!(result.tasting_notes.contains(&"Winey".to_string()));
    assert!(result.tasting_notes.contains(&"Red Grape".to_string()));
    assert!(result.tasting_notes.contains(&"Strawberry".to_string()));
    assert!(result.tasting_notes.contains(&"Blackcurrant".to_string()));
    assert!(result.tasting_notes.contains(&"Dark Cocoa".to_string()));

    // Recommended temperature for Wine process
    assert_eq!(result.recommended_water_temp_c, 89);
}

#[test]
fn test_cognitive_scanner_gunung_halu_checkbox_natural() {
    let raw_text = r#"
KOPI GUNUNG HALU
FRESH BEAN BETTER BREW
DARI GUNUNG, UNTUK CITA RASA TERBAIK
ROASTING DATE 26/10
PROSES
[x] NATURAL
[ ] WASHED
[ ] WINE
100% ARABICA PREMIUM QUALITY
"#;

    let blocks = vec![
        OcrTextBlockDto {
            text: "KOPI GUNUNG HALU".to_string(),
            normalized_height: 0.12,
            top_y: 0.35,
            left_x: 0.2,
            normalized_width: 0.6,
            confidence: 0.95,
        },
        OcrTextBlockDto {
            text: "[x] NATURAL".to_string(),
            normalized_height: 0.04,
            top_y: 0.70,
            left_x: 0.5,
            normalized_width: 0.25,
            confidence: 0.92,
        },
        OcrTextBlockDto {
            text: "[ ] WASHED".to_string(),
            normalized_height: 0.04,
            top_y: 0.75,
            left_x: 0.5,
            normalized_width: 0.25,
            confidence: 0.90,
        },
        OcrTextBlockDto {
            text: "[ ] WINE".to_string(),
            normalized_height: 0.04,
            top_y: 0.80,
            left_x: 0.5,
            normalized_width: 0.25,
            confidence: 0.90,
        },
    ];

    let result = CognitiveResolver::analyze(raw_text, &blocks);

    assert_eq!(result.process, "Natural");
    assert!(
        result.origin.contains("Gunung Halu"),
        "Origin should be Gunung Halu (not Malabar), got {}",
        result.origin
    );
    assert!(result.tasting_notes.contains(&"Blueberry".to_string()));
}

/// Regression: Gunung Puntang tidak boleh lagi salah dipetakan ke Malabar.
/// Sebelumnya keyword "puntang" ada di profil Malabar sehingga origin selalu
/// keluar "Gunung Malabar / Halu" walau kemasan menulis "Puntang".
#[test]
fn test_gunung_puntang_not_mismapped_to_malabar() {
    let raw_text = r#"
PUNTANG WINE
SPECIALTY ROASTERY
GUNUNG PUNTANG
WINE PROCESS
Winey
"#;

    let blocks = vec![
        OcrTextBlockDto {
            text: "PUNTANG WINE".to_string(),
            normalized_height: 0.14,
            top_y: 0.40,
            left_x: 0.2,
            normalized_width: 0.6,
            confidence: 0.97,
        },
        OcrTextBlockDto {
            text: "GUNUNG PUNTANG".to_string(),
            normalized_height: 0.05,
            top_y: 0.55,
            left_x: 0.2,
            normalized_width: 0.5,
            confidence: 0.94,
        },
        OcrTextBlockDto {
            text: "WINE PROCESS".to_string(),
            normalized_height: 0.04,
            top_y: 0.65,
            left_x: 0.2,
            normalized_width: 0.4,
            confidence: 0.93,
        },
    ];

    let result = CognitiveResolver::analyze(raw_text, &blocks);

    assert!(
        result.origin.contains("Puntang"),
        "Origin should be Gunung Puntang, got {}",
        result.origin
    );
    assert!(
        !result.origin.contains("Malabar"),
        "Origin must NOT contain Malabar when packaging says Puntang, got {}",
        result.origin
    );
    assert_eq!(result.process, "Wine Process");
}

/// Regression: Longest-match-wins — "GUNUNG HALU" harus menang atas keyword
/// longgar "halu", dan "GUNUNG PUNTANG" harus menang atas "puntang".
#[test]
fn test_origin_longest_match_wins() {
    use coffe_labs_backend::modules::bean_scanner::services::knowledge_graph::KnowledgeGraph;

    let halu = KnowledgeGraph::find_origin("ARABICA NATURAL HALU BANANA GUNUNG HALU").unwrap();
    assert_eq!(halu.canonical_name, "Gunung Halu");

    let puntang = KnowledgeGraph::find_origin("PUNTANG WINE GUNUNG PUNTANG").unwrap();
    assert_eq!(puntang.canonical_name, "Gunung Puntang");

    let malabar = KnowledgeGraph::find_origin("MALABAR WINE PROCESS").unwrap();
    assert_eq!(malabar.canonical_name, "Malabar");
}

/// Regression: kemasan "ARABICA NATURAL HALU BANANA" tidak boleh lagi
/// terdeteksi sebagai "Full Washed" hanya karena tidak ada checkbox proses.
#[test]
fn test_halu_banana_detected_as_natural_not_washed() {
    let raw_text = r#"
ARABICA NATURAL
HALU BANANA
BY TIGA IKAT COFFEE SUPPLY
Sweet Caramel, Banana, Low Body, Smooth
100g - 200g - 500g - 1000g
"#;

    let blocks = vec![
        OcrTextBlockDto {
            text: "HALU BANANA".to_string(),
            normalized_height: 0.16,
            top_y: 0.45,
            left_x: 0.2,
            normalized_width: 0.6,
            confidence: 0.97,
        },
        OcrTextBlockDto {
            text: "BY TIGA IKAT COFFEE SUPPLY".to_string(),
            normalized_height: 0.04,
            top_y: 0.60,
            left_x: 0.2,
            normalized_width: 0.6,
            confidence: 0.94,
        },
        OcrTextBlockDto {
            text: "Sweet Caramel, Banana, Low Body, Smooth".to_string(),
            normalized_height: 0.04,
            top_y: 0.70,
            left_x: 0.2,
            normalized_width: 0.6,
            confidence: 0.92,
        },
    ];

    let result = CognitiveResolver::analyze(raw_text, &blocks);

    assert_eq!(
        result.process, "Natural",
        "Packaging says ARABICA NATURAL — process must be Natural, not {}",
        result.process
    );
    assert!(
        result.origin.contains("Halu"),
        "Origin should be Gunung Halu, got {}",
        result.origin
    );
    assert!(
        result.bean_name.contains("Banana") || result.bean_name.contains("Halu"),
        "Bean name should preserve product identity 'Halu Banana', got {}",
        result.bean_name
    );
    assert_eq!(result.recommended_water_temp_c, 90);
}

#[test]
fn test_cognitive_scanner_otten_wamena_explicit_notes() {
    let raw_text = r#"
Otten COFFEE
WAMENA BPIRI washed
Palm Sugar • Red Fruit • Chestnut
Region: Distrik Bpiri, Jayawijaya/Wamena, Papua
Altitude: 2068 Masl
200g
"#;

    let blocks = vec![
        OcrTextBlockDto {
            text: "Otten COFFEE".to_string(),
            normalized_height: 0.10,
            top_y: 0.25,
            left_x: 0.2,
            normalized_width: 0.6,
            confidence: 0.97,
        },
        OcrTextBlockDto {
            text: "WAMENA BPIRI washed".to_string(),
            normalized_height: 0.08,
            top_y: 0.55,
            left_x: 0.2,
            normalized_width: 0.6,
            confidence: 0.95,
        },
        OcrTextBlockDto {
            text: "Palm Sugar • Red Fruit • Chestnut".to_string(),
            normalized_height: 0.04,
            top_y: 0.65,
            left_x: 0.2,
            normalized_width: 0.6,
            confidence: 0.92,
        },
        OcrTextBlockDto {
            text: "Altitude: 2068 Masl".to_string(),
            normalized_height: 0.03,
            top_y: 0.75,
            left_x: 0.2,
            normalized_width: 0.4,
            confidence: 0.91,
        },
    ];

    let result = CognitiveResolver::analyze(raw_text, &blocks);

    assert_eq!(result.roastery_name, "Otten Coffee");
    assert_eq!(result.process, "Full Washed");
    assert_eq!(result.notes_source, "OCR_EXPLICIT");
    assert!(!result.is_ai_inferred_altitude);
    assert_eq!(result.altitude_masl.as_deref(), Some("2068 MASL"));
    assert!(result.tasting_notes.contains(&"Brown Sugar".to_string()) || result.tasting_notes.contains(&"Red Fruit".to_string()) || result.tasting_notes.contains(&"Chestnut".to_string()));
}

#[test]
fn test_fuzzy_typo_tasting_notes() {
    let typos = "bluberry, starberry, jasmin, tea, choco";
    let matched = FuzzyFlavorMatcher::match_tasting_notes(typos);

    assert!(matched.contains(&"Blueberry".to_string()));
    assert!(matched.contains(&"Strawberry".to_string()));
    assert!(matched.contains(&"Jasmine".to_string()));
    assert!(matched.contains(&"Black Tea".to_string()));
    assert!(matched.contains(&"Milk Chocolate".to_string()));
}

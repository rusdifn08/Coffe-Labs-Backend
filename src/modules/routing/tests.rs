use crate::modules::{
    beans_radar::category_bitmask::{
        compute_category_flags, matches_strict_filter, StrictCategoryFilter, StrictRadarScope,
    },
    routing::{
        dto::TravelMode,
        indonesia_costing::{calibrate_indonesian_eta, tobler_speed_kmh, traffic_speed_factor},
        polyline6::{decode_polyline6, encode_polyline6, jaccard_overlap_ratio},
        services::{multimodal_router_service, rust_local_astar_solver},
    },
};

#[test]
fn multimodal_routes_diverge_on_bandung_coordinates() {
    let origin = [-6.9565, 107.6942]; // Summarecon Bandung
    let dest_tovi = [-6.9510, 107.6908]; // Tovi Incorporated Gedebage
    let foot = rust_local_astar_solver::solve_local_multimodal_astar(origin, dest_tovi, TravelMode::Foot, true);
    let moto = rust_local_astar_solver::solve_local_multimodal_astar(origin, dest_tovi, TravelMode::Motorcycle, true);
    let car_toll = rust_local_astar_solver::solve_local_multimodal_astar(origin, dest_tovi, TravelMode::Car, false);
    let car_notoll = rust_local_astar_solver::solve_local_multimodal_astar(origin, dest_tovi, TravelMode::Car, true);

    assert_ne!(foot.primary.coordinates, moto.primary.coordinates);
    assert_ne!(moto.primary.coordinates, car_notoll.primary.coordinates);
    assert!(foot.primary.uses_alleys);
    assert!(moto.primary.avoided_tolls);
    assert!(!moto.primary.has_toll);
    assert!(!car_notoll.primary.has_toll);
    assert!(car_toll.primary.has_toll);
    assert!(foot.primary.duration_seconds > moto.primary.duration_seconds);

    // 5 km corridor verification across Bandung
    let dest_5km = [-6.9480, 107.6350];
    let f5 = rust_local_astar_solver::solve_local_multimodal_astar(origin, dest_5km, TravelMode::Foot, true);
    let m5 = rust_local_astar_solver::solve_local_multimodal_astar(origin, dest_5km, TravelMode::Motorcycle, true);
    let c5 = rust_local_astar_solver::solve_local_multimodal_astar(origin, dest_5km, TravelMode::Car, true);
    assert_ne!(f5.primary.coordinates, m5.primary.coordinates);
    assert_ne!(m5.primary.coordinates, c5.primary.coordinates);
    for alt in &m5.alternatives {
        let ov = jaccard_overlap_ratio(&m5.primary.coordinates, &alt.coordinates);
        assert!(ov <= 0.72);
    }
}

#[test]
fn polyline6_tobler_and_indonesian_traffic_math() {
    let pts = [[-6.9565, 107.6942], [-6.9540, 107.6921], [-6.9510, 107.6908]];
    let enc = encode_polyline6(&pts);
    let dec = decode_polyline6(&enc);
    assert_eq!(dec.len(), pts.len());
    assert!((dec[0][0] - pts[0][0]).abs() < 1e-5);
    assert!((dec[2][1] - pts[2][1]).abs() < 1e-5);

    assert!(tobler_speed_kmh(0.0) > tobler_speed_kmh(0.18));
    assert!((traffic_speed_factor(TravelMode::Foot, true, true) - 1.0).abs() < 1e-6);
    assert!((traffic_speed_factor(TravelMode::Motorcycle, true, true) - 0.82).abs() < 1e-6);
    assert!((traffic_speed_factor(TravelMode::Car, true, true) - 0.55).abs() < 1e-6);

    let (moto_rush, _) = calibrate_indonesian_eta(TravelMode::Motorcycle, 600.0, 5000.0, false, true, true, 6);
    let (car_rush, _) = calibrate_indonesian_eta(TravelMode::Car, 600.0, 5000.0, false, true, true, 6);
    assert!(moto_rush < car_rush);
}

#[test]
fn reverse_geocode_summarecon_bandung_never_palembang() {
    let geo = multimodal_router_service::nearest_indonesia_city_fallback(-6.9565, 107.6942);
    assert_eq!(geo.city, "Bandung");
    assert_eq!(geo.district, "Gedebage");
    assert!(!geo.label.to_ascii_lowercase().contains("palembang"));
}

#[test]
fn strict_bitmask_zero_pin_leakage_across_all_modes() {
    let roastery = compute_category_flags("ROASTERY", true, true);
    let bean_shop = compute_category_flags("BEAN_SELLER", true, false);
    let pure_cafe = compute_category_flags("COFFEE_SHOP", false, false);
    let spec_cafe = compute_category_flags("SPECIALTY_CAFE", false, true);
    let resto = compute_category_flags("CAFE_RESTO", false, false);

    // Mode BEANS_RADAR: 0 CAFE_RESTO, 0 non-bean COFFEE_SHOP
    assert!(matches_strict_filter(roastery, StrictRadarScope::BeansRadar, StrictCategoryFilter::All));
    assert!(matches_strict_filter(bean_shop, StrictRadarScope::BeansRadar, StrictCategoryFilter::All));
    assert!(!matches_strict_filter(pure_cafe, StrictRadarScope::BeansRadar, StrictCategoryFilter::All));
    assert!(!matches_strict_filter(spec_cafe, StrictRadarScope::BeansRadar, StrictCategoryFilter::All));
    assert!(!matches_strict_filter(resto, StrictRadarScope::BeansRadar, StrictCategoryFilter::All));

    // Category CAFE: ONLY COFFEE_SHOP & SPECIALTY_CAFE (ZERO CAFE_RESTO, ZERO pure ROASTERY/BEAN_SELLER)
    assert!(matches_strict_filter(pure_cafe, StrictRadarScope::All, StrictCategoryFilter::Cafe));
    assert!(matches_strict_filter(spec_cafe, StrictRadarScope::All, StrictCategoryFilter::Cafe));
    assert!(!matches_strict_filter(roastery, StrictRadarScope::All, StrictCategoryFilter::Cafe));
    assert!(!matches_strict_filter(resto, StrictRadarScope::All, StrictCategoryFilter::Cafe));
    assert!(!matches_strict_filter(bean_shop, StrictRadarScope::All, StrictCategoryFilter::Cafe));

    // Category RESTO: ONLY CAFE_RESTO
    assert!(matches_strict_filter(resto, StrictRadarScope::All, StrictCategoryFilter::Resto));
    assert!(!matches_strict_filter(pure_cafe, StrictRadarScope::All, StrictCategoryFilter::Resto));
    assert!(!matches_strict_filter(roastery, StrictRadarScope::All, StrictCategoryFilter::Resto));

    // Category ROASTERY & SLOW_BAR
    assert!(matches_strict_filter(roastery, StrictRadarScope::All, StrictCategoryFilter::Roastery));
    assert!(!matches_strict_filter(bean_shop, StrictRadarScope::All, StrictCategoryFilter::Roastery));
    assert!(matches_strict_filter(spec_cafe, StrictRadarScope::All, StrictCategoryFilter::SlowBar));
    assert!(!matches_strict_filter(pure_cafe, StrictRadarScope::All, StrictCategoryFilter::SlowBar));
}

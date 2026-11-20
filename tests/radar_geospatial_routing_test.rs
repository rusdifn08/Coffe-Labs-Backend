use coffe_labs_backend::modules::radar::{
    dto::{normalize_route_mode, parse_origin_coords, validate_coords, validate_indonesia_coords, RouteQuery},
    models::{RadarPlaceKind, RadarScopeMode},
    repository::build_kind_filter_sql,
    route_parsers::{parse_geoapify_route, parse_osrm_route, simplify_stride},
    routing_service::{build_route_cache_key, round_grid_3},
};
use serde_json::json;

#[test]
fn test_coordinate_and_origin_validation() {
    assert!(validate_coords(-6.9175, 107.6191).is_ok());
    assert!(validate_coords(95.0, 107.6191).is_err());
    assert!(validate_coords(-6.9175, f64::NAN).is_err());

    assert!(validate_indonesia_coords(-6.9175, 107.6191)); // Bandung
    assert!(validate_indonesia_coords(-8.6705, 115.2126)); // Bali
    assert!(!validate_indonesia_coords(0.0, 0.0)); // Null island
    assert!(!validate_indonesia_coords(35.6762, 139.6503)); // Tokyo

    let q = RouteQuery {
        origin_lat: None,
        origin_lng: None,
        origin: Some("-6.9175, 107.6191".into()),
        dest_place_id: None,
        dest_id: None,
        dest_lat: Some(-6.9015),
        dest_lng: Some(107.6128),
        mode: Some("motorcycle".into()),
    };
    let (lat, lng) = parse_origin_coords(&q).expect("valid origin string");
    assert!((lat + 6.9175).abs() < 1e-6);
    assert!((lng - 107.6191).abs() < 1e-6);
    assert_eq!(normalize_route_mode(Some("driving")), "drive");
    assert_eq!(normalize_route_mode(Some("walk")), "walk");
    assert_eq!(normalize_route_mode(None), "motorcycle");
}

#[test]
fn test_redis_3_decimal_grid_cache_key_determinism() {
    // Dua pengguna dalam jarak ~30 meter dibulatkan ke kunci grid 100m yang sama
    assert_eq!(round_grid_3(-6.91742), -6.917);
    assert_eq!(round_grid_3(-6.91718), -6.917);
    let k1 = build_route_cache_key("motorcycle", -6.91742, 107.61921, "place-abc");
    let k2 = build_route_cache_key("motorcycle", -6.91718, 107.61944, "place-abc");
    assert_eq!(k1, k2);
    assert_eq!(k1, "radar:route:motorcycle:-6.917:107.619:place-abc");
}

#[test]
fn test_dual_radar_membership_roastery_and_cafe() {
    let cafe_kinds = build_kind_filter_sql(None, RadarScopeMode::Cafe);
    assert!(cafe_kinds.contains(&"CAFE"));
    assert!(cafe_kinds.contains(&"ROASTERY_AND_CAFE"));
    assert!(!cafe_kinds.contains(&"BEAN_SELLER"));

    let beans_kinds = build_kind_filter_sql(None, RadarScopeMode::Beans);
    assert!(beans_kinds.contains(&"BEAN_SELLER"));
    assert!(beans_kinds.contains(&"ROASTERY_AND_CAFE"));
    assert!(!beans_kinds.contains(&"CAFE"));

    let exact = build_kind_filter_sql(Some(RadarPlaceKind::RoasteryAndCafe), RadarScopeMode::All);
    assert_eq!(exact, vec!["ROASTERY_AND_CAFE"]);
}

#[test]
fn test_geoapify_and_osrm_polyline_extraction() {
    let geoapify_sample = json!({
        "features": [{
            "properties": {
                "distance": 1850.0,
                "time": 320.0,
                "legs": [{
                    "steps": [
                        { "distance": 900.0, "time": 160.0, "instruction": { "text": "Belok kanan ke Jl. Braga" } },
                        { "distance": 950.0, "time": 160.0, "instruction": { "text": "Tiba di tujuan" } }
                    ]
                }]
            },
            "geometry": {
                "type": "MultiLineString",
                "coordinates": [[[107.6191, -6.9175], [107.6150, -6.9140], [107.6105, -6.9110]]]
            }
        }]
    });

    let g = parse_geoapify_route(&geoapify_sample, "motorcycle", [-6.9175, 107.6191], [-6.9110, 107.6105])
        .expect("geoapify parsed");
    assert_eq!(g.provider, "geoapify");
    assert_eq!(g.coordinates.len(), 3);
    assert_eq!(g.coordinates[0], [-6.9175, 107.6191]);
    assert_eq!(g.steps.len(), 2);

    let osrm_sample = json!({
        "routes": [{
            "distance": 2100.0,
            "duration": 400.0,
            "geometry": {
                "type": "LineString",
                "coordinates": [[107.6191, -6.9175], [107.6160, -6.9150], [107.6105, -6.9110]]
            },
            "legs": [{
                "steps": [{
                    "name": "Jl. Asia Afrika",
                    "distance": 1200.0,
                    "duration": 220.0,
                    "maneuver": { "type": "turn" }
                }]
            }]
        }]
    });
    let o = parse_osrm_route(&osrm_sample, "motorcycle", [-6.9175, 107.6191], [-6.9110, 107.6105])
        .expect("osrm parsed");
    assert_eq!(o.provider, "osrm");
    assert_eq!(o.coordinates.len(), 3);
    assert!(o.duration_seconds < 400.0);

    let dense: Vec<[f64; 2]> = (0..500).map(|i| [-6.9 + i as f64 * 0.0001, 107.6]).collect();
    let simplified = simplify_stride(&dense, 100);
    assert!(simplified.len() <= 102);
    assert_eq!(simplified.last(), dense.last());
}

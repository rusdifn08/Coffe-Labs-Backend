use coffe_labs_backend::modules::social_feed::{
    attachments::{PourCurvePoint, RecipeStepMiniDto, ThreadAttachmentPayload},
    cursor::{DirectoryCursor, TimelineCursor},
    models::{
        barista_rank_for_level, compute_level_progress_ratio, decode_roaring_ids,
        level_from_total_xp, xp_req_for_level,
    },
    services::profile_cqrs_service::ProfileCqrsService,
};
use roaring::RoaringBitmap;
use uuid::Uuid;

#[test]
fn barista_rank_level_formula_matches_spec() {
    assert_eq!(xp_req_for_level(1), 0);
    assert_eq!(xp_req_for_level(2), 313);
    assert_eq!(xp_req_for_level(10), 4466);

    assert_eq!(level_from_total_xp(0), 1);
    assert_eq!(level_from_total_xp(313), 2);
    assert_eq!(level_from_total_xp(4466), 10);

    assert_eq!(barista_rank_for_level(1), "Home Brewer");
    assert_eq!(barista_rank_for_level(10), "Pour-Over Artisan");
    assert_eq!(barista_rank_for_level(25), "Sensory Alchemist");
    assert_eq!(barista_rank_for_level(50), "Head Roaster & Master");
    assert_eq!(barista_rank_for_level(80), "WBrC Grandmaster");

    let ratio = compute_level_progress_ratio(1, 156);
    assert!(ratio > 0.45 && ratio < 0.55);
}

#[test]
fn timeline_and_directory_cursors_roundtrip_and_validate() {
    let tid = Uuid::now_v7();
    let cursor = TimelineCursor::new(tid, false);
    let encoded = cursor.encode_base64_url().expect("encode cursor");
    let decoded = TimelineCursor::decode_base64_url(&encoded).expect("decode cursor");
    assert_eq!(decoded, cursor);

    assert!(TimelineCursor::decode_base64_url("not_valid!@#").is_err());

    let dc = DirectoryCursor {
        total_xp: 8500,
        user_id: Uuid::new_v4(),
    };
    let dc_enc = dc.encode_base64_url().expect("encode dir cursor");
    let dc_dec = DirectoryCursor::decode_base64_url(&dc_enc).expect("decode dir cursor");
    assert_eq!(dc_dec, dc);
}

#[test]
fn thread_attachments_serde_tagged_union_roundtrip() {
    let recipe = ThreadAttachmentPayload::InteractiveRecipeCard {
        recipe_id: "42".to_string(),
        title: "Dual-Temp V60 Sherry Hsu".to_string(),
        brewer_name: "Hario V60".to_string(),
        method: "V60".to_string(),
        coffee_dose_g: 15.0,
        total_water_ml: 240.0,
        primary_temp_c: 92,
        secondary_temp_c: Some(70),
        target_duration_sec: 150,
        steps_preview: vec![RecipeStepMiniDto {
            step_number: 1,
            title: "Bloom".to_string(),
            duration_sec: 35,
            water_added_ml: 45.0,
            target_temp_c: Some(92),
            instruction: Some("Tuang melingkar perlahan".to_string()),
        }],
    };
    let json = serde_json::to_value(&recipe).expect("serialize recipe");
    assert_eq!(json["type"], "interactive_recipe_card");
    assert_eq!(json["data"]["secondary_temp_c"], 70);
    let back: ThreadAttachmentPayload = serde_json::from_value(json).expect("deserialize recipe");
    assert_eq!(back, recipe);

    let telemetry = ThreadAttachmentPayload::PouringTelemetryCurve {
        session_id: Uuid::now_v7(),
        recipe_title: "Tetsu 4:6".to_string(),
        bean_name: "Malabar Wine".to_string(),
        accuracy_score_pct: 98.4,
        total_duration_sec: 210,
        curve_points: vec![PourCurvePoint {
            t_sec: 45,
            actual_ml: 60.5,
            target_ml: 60.0,
            temp_c: 91.5,
        }],
    };
    let tjson = serde_json::to_value(&telemetry).expect("serialize telemetry");
    assert_eq!(tjson["type"], "pouring_telemetry_curve");
    let tback: ThreadAttachmentPayload = serde_json::from_value(tjson).expect("deserialize telemetry");
    assert_eq!(tback, telemetry);
}

#[test]
fn roaring_bitmap_badges_decode_in_sub_microsecond_scale() {
    let mut bitmap = RoaringBitmap::new();
    bitmap.insert(1);
    bitmap.insert(5);
    bitmap.insert(12);
    bitmap.insert(20);

    let mut bytes = Vec::new();
    bitmap.serialize_into(&mut bytes).expect("serialize roaring");
    let ids = decode_roaring_ids(Some(&bytes));
    assert_eq!(ids, vec![1, 5, 12, 20]);
    assert!(decode_roaring_ids(None).is_empty());
}

#[tokio::test]
async fn profile_cqrs_service_initializes_without_redis() {
    let cqrs = ProfileCqrsService::new(None);
    cqrs.invalidate_profile(Uuid::new_v4()).await;
}

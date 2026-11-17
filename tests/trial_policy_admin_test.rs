use coffe_labs_backend::modules::platform::{
    dto::UpdateTrialPolicyRequest, models::TrialAccessPolicy,
};

#[test]
fn trial_policy_default_is_strict_and_locks_business() {
    let policy = TrialAccessPolicy::fallback_default();
    assert_eq!(policy.policy_mode, "strict_limited");
    assert!(!policy.unlock_all_recipes);
    assert!(!policy.unlock_smart_brew_dial_in);
    assert!(!policy.unlock_toolkit_pro);
    assert!(!policy.allow_business_suite);
    assert_eq!(policy.max_free_recipes, 10);
}

#[test]
fn update_trial_policy_dto_deserializes_7d_non_business_preset() {
    let raw = r#"{
        "policy_mode": "full_trial_7d_non_business",
        "trial_duration_days": 7,
        "unlock_all_recipes": true,
        "unlock_smart_brew_dial_in": true,
        "unlock_ai_scanner": true,
        "unlock_toolkit_pro": true,
        "unlock_learning_academy": true,
        "unlock_brew_doctor": true,
        "unlimited_brews": true,
        "allow_business_suite": false,
        "reset_activated_at": true
    }"#;
    let req: UpdateTrialPolicyRequest = serde_json::from_str(raw).expect("valid JSON");
    assert_eq!(
        req.policy_mode.as_deref(),
        Some("full_trial_7d_non_business")
    );
    assert_eq!(req.trial_duration_days, Some(7));
    assert_eq!(req.unlock_all_recipes, Some(true));
    assert_eq!(req.allow_business_suite, Some(false));
    assert_eq!(req.reset_activated_at, Some(true));
}

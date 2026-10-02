use iloader_lib::mobile_bootstrap::{MobileAccountSecret, MobileBootstrapPayload};

#[test]
fn account_secret_is_explicitly_versioned_and_contains_login_material() {
    let secret = MobileAccountSecret::new("user@example.test", "fixture-secret-value");
    let json = serde_json::to_value(&secret).expect("secret should serialize");

    assert_eq!(json["schema"], "ibridge.mobile-account-secret/1");
    assert_eq!(json["appleId"], "user@example.test");
    assert_eq!(json["password"], "fixture-secret-value");
}

#[test]
fn public_bootstrap_never_serializes_login_secret() {
    let payload = MobileBootstrapPayload::new(
        "TEST-UDID",
        "Test iPhone",
        "26.4",
        "user@example.test",
        "https://anisette.example.test",
    );
    let encoded = serde_json::to_string(&payload).expect("bootstrap should serialize");

    assert!(!encoded.contains("fixture-secret-value"));
    assert!(!encoded.contains("password"));
}

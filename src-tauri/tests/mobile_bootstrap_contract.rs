use iloader_lib::mobile_bootstrap::{
    MOBILE_BOOTSTRAP_PATH, MOBILE_PAIRING_PATH, MOBILE_RELEASE_URL, MOBILE_SECRET_PATH,
    MobileBootstrapPayload, is_ibridge_mobile_bundle_id,
};

#[test]
fn mobile_release_contract_points_to_ibridge_asset() {
    assert_eq!(
        MOBILE_RELEASE_URL,
        "https://github.com/DannyBaanks/iBridge/releases/latest/download/iBridge-Mobile.ipa"
    );
    assert_eq!(MOBILE_PAIRING_PATH, "iBridgeBootstrap/pairing.plist");
    assert_eq!(MOBILE_BOOTSTRAP_PATH, "iBridgeBootstrap/bootstrap.json");
    assert_eq!(MOBILE_SECRET_PATH, "iBridgeBootstrap/account-secret.json");
}

#[test]
fn mobile_bundle_match_survives_team_suffix_added_by_isideload() {
    assert!(is_ibridge_mobile_bundle_id(
        "com.dannybaanks.ibridge.mobile"
    ));
    assert!(is_ibridge_mobile_bundle_id(
        "com.dannybaanks.ibridge.mobile.A1B2C3D4E5"
    ));
    assert!(!is_ibridge_mobile_bundle_id("com.SideStore.SideStore"));
}

#[test]
fn bootstrap_payload_contains_only_non_secret_account_metadata() {
    let payload = MobileBootstrapPayload::new(
        "00008110-001234567890001E",
        "Danny's iPhone",
        "26.4",
        "danny@example.com",
        "https://ani.example.test",
    );

    let json = serde_json::to_value(&payload).expect("bootstrap should serialize");

    assert_eq!(json["schema"], "ibridge.mobile-bootstrap/1");
    assert_eq!(json["deviceUdid"], "00008110-001234567890001E");
    assert_eq!(json["deviceName"], "Danny's iPhone");
    assert_eq!(json["deviceVersion"], "26.4");
    assert_eq!(json["appleId"], "danny@example.com");
    assert_eq!(json["anisetteServer"], "https://ani.example.test");
    assert_eq!(json["pairingPath"], MOBILE_PAIRING_PATH);
    assert_eq!(json["secretPath"], MOBILE_SECRET_PATH);
    assert!(json.get("password").is_none());
}

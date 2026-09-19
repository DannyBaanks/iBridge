//! Integration tests for report binding — the seam between the physical
//! device evidence (device.report.v1, stored by AppleBridge's receiver) and
//! iBridge receipts. Fixtures are generated at runtime; nothing touches the
//! real evidence tree.

use ibridge::applebridge::bind_report;
use ibridge::core::{iso_from_epoch, sha256_hex};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn fixture_report(generated_at: &str, os_version: &str) -> (std::path::PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("ibridge-bind-{}-{}", std::process::id(), sha256_hex(&[generated_at])[..8].to_string()));
    std::fs::create_dir_all(&dir).unwrap();
    let body = serde_json::json!({
        "schema": "applebridge.device.report.v1",
        "protocol_version": "1",
        "app_build": {"name": "AppleBridgeProbe", "version": "0.1.0"},
        "environment": {"os_name": "iOS", "os_version": os_version, "arch": "arm64"},
        "probes": [{
            "probe_id": "env.runtime", "requirement": "r", "operation": "o",
            "expected_observation": "e", "observed_result": "ok",
            "native_error_domain": null, "native_error_code": null,
            "duration_ms": 1.0, "verdict": "PASS",
            "evidence_class": "VERIFIED_ON_IOS_DEVICE",
            "evidence_reference": "doctor://env.runtime"
        }],
        "summary": {"pass": 1, "fail": 0, "not_demonstrated": 0,
                    "requires_macos_xcode": 0, "verdict": "FULL_PASS"},
        "generated_at": generated_at
    });
    let path = dir.join("report.json");
    std::fs::write(&path, serde_json::to_string(&body).unwrap()).unwrap();
    let sha = ibridge::applebridge::sha256_bytes(std::fs::read(&path).unwrap().as_slice());
    (path, sha)
}

#[test]
fn binds_current_report() {
    let now = now_epoch();
    let (path, sha) = fixture_report(&iso_from_epoch(now - 30), "18.7.8");
    let b = bind_report(&path, 48, Some("18.7.8")).unwrap();
    assert_eq!(b.classification, "CURRENT");
    assert_eq!(b.sha256, sha);
    assert_eq!(b.consistency_ios_version_match, Some(true));
}

#[test]
fn binds_historical_report_with_consistency_check() {
    let now = now_epoch();
    let (path, _) = fixture_report(&iso_from_epoch(now - 3600), "18.7.8");
    let b = bind_report(&path, 48, Some("17.0.0")).unwrap();
    assert_eq!(b.classification, "HISTORICAL");
    assert_eq!(b.consistency_ios_version_match, Some(false));
}

#[test]
fn stale_report_detected() {
    let now = now_epoch();
    let (path, _) = fixture_report(&iso_from_epoch(now - 100 * 3600), "18.7.8");
    let b = bind_report(&path, 48, None).unwrap();
    assert_eq!(b.classification, "HISTORICAL_STALE");
}

#[test]
fn rejects_wrong_schema() {
    let (path, _) = fixture_report("2026-09-18T00:00:00Z", "18.7.8");
    let raw = std::fs::read_to_string(&path).unwrap().replace("applebridge.device.report.v1", "someone.else.v9");
    std::fs::write(&path, raw).unwrap();
    let err = bind_report(&path, 48, None).unwrap_err();
    assert!(err.contains("schema"));
}

#[test]
fn rejects_unparseable_generated_at() {
    let (path, _) = fixture_report("not-a-timestamp", "18.7.8");
    assert!(bind_report(&path, 48, None).is_err());
}

#[test]
fn malformed_pack_rejected() {
    assert!(ibridge::applebridge::AppleBridgeProvider::validate_pack_bytes(b"not json").is_err());
    let no_probes = br#"{"schema":"applebridge.probe-pack.v1","pack_version":"1","probes":[]}"#;
    assert!(ibridge::applebridge::AppleBridgeProvider::validate_pack_bytes(no_probes).is_err());
    let wrong_schema = br#"{"schema":"other","pack_version":"1","probes":[{"probe_id":"x","requirement":"r","operation":"o","expected_observation":"e"}]}"#;
    assert!(ibridge::applebridge::AppleBridgeProvider::validate_pack_bytes(wrong_schema).is_err());
    let good = br#"{"schema":"applebridge.probe-pack.v1","pack_version":"1","probes":[{"probe_id":"x","requirement":"r","operation":"o","expected_observation":"e"}]}"#;
    assert!(ibridge::applebridge::AppleBridgeProvider::validate_pack_bytes(good).is_ok());
}

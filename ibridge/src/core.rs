//! iBridge core: identity aliasing, authority, time, hashing.
//!
//! Invariants:
//! - Communication is not authority: pairing/transport grants nothing until
//!   an operation is explicitly allowed.
//! - UDIDs never appear in receipts or stdout output; only salted aliases.
//! - Operational result and evidence verdict are separate concepts.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub const IBRIDGE_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Authority
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityClass {
    /// Safe, passive, allowed by default.
    DefaultAllowed,
    /// Requires explicit invocation; NOT implemented in the MVP.
    /// (advertising the seam without granting it)
    ExplicitOnlyNotImplemented,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorityDecision {
    pub capability: String,
    pub class: CapabilityClass,
    pub allowed: bool,
    pub reason: String,
}

/// The frozen capability registry. Discovery NEVER triggers anything here.
/// `signing`/`certificates`/`appid` act on Danny's Apple account: they are
/// denied in the MVP even when explicitly requested (fail closed), because
/// they are not implemented — they are documented seams only.
pub fn capability_registry() -> Vec<(&'static str, CapabilityClass)> {
    vec![
        ("discovery", CapabilityClass::DefaultAllowed),
        ("read_only_diagnostic", CapabilityClass::DefaultAllowed),
        ("transport", CapabilityClass::DefaultAllowed),
        ("doctor_orchestration", CapabilityClass::DefaultAllowed),
        ("deployment", CapabilityClass::ExplicitOnlyNotImplemented),
        ("signing", CapabilityClass::ExplicitOnlyNotImplemented),
        ("certificates", CapabilityClass::ExplicitOnlyNotImplemented),
        ("appid", CapabilityClass::ExplicitOnlyNotImplemented),
    ]
}

pub fn authorize(requested: &str) -> AuthorityDecision {
    for (name, class) in capability_registry() {
        if name == requested {
            return match class {
                CapabilityClass::DefaultAllowed => AuthorityDecision {
                    capability: name.to_string(),
                    class,
                    allowed: true,
                    reason: "default-allowed read/passive capability".into(),
                },
                CapabilityClass::ExplicitOnlyNotImplemented => AuthorityDecision {
                    capability: name.to_string(),
                    class,
                    allowed: false,
                    reason: format!(
                        "{} is an explicit-only capability and is not implemented in the \
                         iBridge MVP; denied (fail closed). It would act on the linked \
                         Apple account and must never run as a side effect.",
                        name
                    ),
                },
            };
        }
    }
    AuthorityDecision {
        capability: requested.to_string(),
        class: CapabilityClass::ExplicitOnlyNotImplemented,
        allowed: false,
        reason: "unknown capability; deny".into(),
    }
}

/// Capabilities a discovered device may be routed to under MVP authority.
pub fn device_capability_surface() -> Vec<String> {
    capability_registry()
        .into_iter()
        .filter(|(_, c)| *c == CapabilityClass::DefaultAllowed)
        .map(|(n, _)| n.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Identity aliases (privacy: no UDID, no device name in evidence)
// ---------------------------------------------------------------------------

pub fn sha256_hex(parts: &[&str]) -> String {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p.as_bytes());
    }
    let out = h.finalize();
    out.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn host_alias(hostname: &str) -> String {
    sha256_hex(&["ibridge-host", hostname])[..16].to_string()
}

pub fn device_alias(host: &str, udid: &str) -> String {
    sha256_hex(&["ibridge-device", host, udid])[..16].to_string()
}

// ---------------------------------------------------------------------------
// Time (no chrono: stdlib civil-date conversion, unit-tested)
// ---------------------------------------------------------------------------

/// Epoch seconds -> (y, m, d, h, mi, s) UTC. Howard Hinnant's civil algorithm.
pub fn epoch_to_utc(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let h = (rem / 3600) as u32;
    let mi = ((rem % 3600) / 60) as u32;
    let s = (rem % 60) as u32;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, h, mi, s)
}

pub fn iso_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    iso_from_epoch(secs)
}

pub fn iso_from_epoch(secs: u64) -> String {
    let (y, mo, d, h, mi, s) = epoch_to_utc(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, mi, s)
}

/// Parse the Doctor's ISO8601 "YYYY-MM-DDTHH:MM:SSZ" into epoch seconds.
pub fn parse_iso(ts: &str) -> Option<u64> {
    let b = ts.as_bytes();
    if b.len() != 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':'
        || b[16] != b':' || b[19] != b'Z'
    {
        return None;
    }
    let num = |a: usize, z: usize| -> Option<u64> { ts.get(a..=z)?.parse::<u64>().ok() };
    let (y, mo, d, h, mi, s) = (
        num(0, 3)?,
        num(5, 6)?,
        num(8, 9)?,
        num(11, 12)?,
        num(14, 15)?,
        num(17, 18)?,
    );
    // inverse civil algorithm
    let y = y as i64 - if mo <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if mo > 2 { mo - 3 } else { mo + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some((days * 86_400 + h as i64 * 3600 + mi as i64 * 60 + s as i64) as u64)
}

/// Age classification for report binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReportClassification {
    Current,
    Historical,
    HistoricalStale,
}

pub fn classify_report_age(
    generated_at: &str,
    max_age_hours: u64,
) -> Result<ReportClassification, String> {
    let gen = parse_iso(generated_at).ok_or_else(|| format!("unparseable generated_at {generated_at:?}"))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let age = now.saturating_sub(gen);
    if age <= 120 {
        Ok(ReportClassification::Current)
    } else if age <= max_age_hours * 3600 {
        Ok(ReportClassification::Historical)
    } else {
        Ok(ReportClassification::HistoricalStale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zero_is_1970() {
        assert_eq!(iso_from_epoch(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn known_epoch_roundtrip() {
        // 2026-09-18T21:21:37Z
        let iso = "2026-09-18T21:21:37Z";
        assert_eq!(parse_iso(iso), Some(1789766497));
        assert_eq!(iso_from_epoch(1789766497), iso);
    }

    #[test]
    fn parse_rejects_garbage() {
        assert_eq!(parse_iso("nope"), None);
        assert_eq!(parse_iso("2026-09-18 21:21:37Z"), None);
    }

    #[test]
    fn alias_is_stable_and_short() {
        let a = device_alias("hostA", "UDID-X");
        let b = device_alias("hostA", "UDID-X");
        let c = device_alias("hostA", "UDID-Y");
        let d = device_alias("hostB", "UDID-X");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn discovery_authority_is_default_allowed() {
        let d = authorize("discovery");
        assert!(d.allowed);
        let d2 = authorize("doctor_orchestration");
        assert!(d2.allowed);
    }

    #[test]
    fn signing_is_denied_even_when_explicit() {
        let d = authorize("signing");
        assert!(!d.allowed);
        assert!(d.reason.contains("fail closed"));
        let d2 = authorize("deployment");
        assert!(!d2.allowed);
        let d3 = authorize("not_a_capability");
        assert!(!d3.allowed);
    }

    #[test]
    fn capability_surface_excludes_explicit_only() {
        let surface = device_capability_surface();
        assert!(surface.contains(&"discovery".to_string()));
        assert!(!surface.contains(&"signing".to_string()));
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(&["abc"]),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn classify_age_boundaries() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        assert_eq!(
            classify_report_age(&iso_from_epoch(now - 60), 24).unwrap(),
            ReportClassification::Current
        );
        assert_eq!(
            classify_report_age(&iso_from_epoch(now - 3600), 24).unwrap(),
            ReportClassification::Historical
        );
        assert_eq!(
            classify_report_age(&iso_from_epoch(now - 100 * 3600), 24).unwrap(),
            ReportClassification::HistoricalStale
        );
    }
}

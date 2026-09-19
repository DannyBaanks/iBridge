//! ibridge.receipt.v1 — one receipt per real operation.
//!
//! Operational result and evidence verdict are SEPARATE fields:
//!   execution_result  = what the host did (bound report, timeout, denied, ...)
//!   evidence_verdict  = what the bound evidence says + its classification
//! The overall `verdict` is honest and conservative:
//!   CURRENT report  -> PASS
//!   HISTORICAL      -> PARTIAL
//!   HISTORICAL_STALE-> PARTIAL
//!   timeout         -> NOT_DEMONSTRATED
//!   denied          -> DENIED
//!   transport error -> ERROR

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Pass,
    Partial,
    NotDemonstrated,
    Denied,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub role: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub schema: String,
    pub operation_id: String,
    pub ibridge_version: String,
    pub host_alias: String,
    pub device_alias: Option<String>,
    pub device: Option<DeviceInReceipt>,
    pub capability: String,
    pub authority: AuthorityInReceipt,
    pub input: InputInReceipt,
    pub transport: TransportInReceipt,
    pub started_at: String,
    pub finished_at: String,
    pub execution_result: String,
    pub evidence_verdict: Option<EvidenceVerdict>,
    pub evidence: Vec<EvidenceRef>,
    pub verdict: Verdict,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInReceipt {
    pub device_alias: String,
    pub model_class: Option<String>,
    pub ios_version: Option<String>,
    pub connection_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityInReceipt {
    pub requested: Vec<String>,
    pub allowed: Vec<String>,
    pub denied: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputInReceipt {
    pub pack_sha256: Option<String>,
    pub receiver_url: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportInReceipt {
    pub usbmuxd: bool,
    pub receiver: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceVerdict {
    pub classification: String,
    pub report_sha256: Option<String>,
    pub report_generated_at: Option<String>,
    pub consistency_ios_version_match: Option<bool>,
    pub report_summary: Option<serde_json::Value>,
}

impl Receipt {
    pub fn new(operation_id: &str, host_alias: &str, capability: &str, args: &[String]) -> Self {
        Receipt {
            schema: "ibridge.receipt.v1".into(),
            operation_id: operation_id.to_string(),
            ibridge_version: crate::core::IBRIDGE_VERSION.to_string(),
            host_alias: host_alias.to_string(),
            device_alias: None,
            device: None,
            capability: capability.to_string(),
            authority: AuthorityInReceipt {
                requested: vec![capability.to_string()],
                allowed: vec![],
                denied: vec![],
            },
            input: InputInReceipt {
                pack_sha256: None,
                receiver_url: String::new(),
                arguments: args.to_vec(),
            },
            transport: TransportInReceipt {
                usbmuxd: false,
                receiver: false,
                detail: String::new(),
            },
            started_at: crate::core::iso_now(),
            finished_at: String::new(),
            execution_result: String::new(),
            evidence_verdict: None,
            evidence: vec![],
            verdict: Verdict::Error,
            notes: vec![],
        }
    }

    pub fn finalize(&mut self) {
        if self.finished_at.is_empty() {
            self.finished_at = crate::core::iso_now();
        }
    }

    pub fn write(&self, evidence_dir: &Path) -> Result<String, String> {
        std::fs::create_dir_all(evidence_dir.join("receipts"))
            .map_err(|e| format!("mkdir receipts: {e}"))?;
        let name = format!(
            "{}-{}.json",
            self.started_at.replace(['-', ':', 'T', 'Z'], ""),
            self.operation_id
        );
        let path = evidence_dir.join("receipts").join(name);
        if path.exists() {
            return Err(format!("refusing to overwrite existing receipt {path:?}"));
        }
        let body = serde_json::to_string_pretty(self)
            .map_err(|e| format!("serialize receipt: {e}"))?;
        std::fs::write(&path, body).map_err(|e| format!("write receipt: {e}"))?;
        Ok(path.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_roundtrip_and_keys() {
        let mut r = Receipt::new("op-test-1", "hostA", "discovery", &["--json".into()]);
        r.execution_result = "devices_enumerated".into();
        r.verdict = Verdict::Pass;
        r.finalize();
        let json = serde_json::to_string(&r).unwrap();
        let back: Receipt = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema, "ibridge.receipt.v1");
        assert_eq!(back.operation_id, "op-test-1");
        assert_eq!(back.verdict, Verdict::Pass);
        assert!(!back.finished_at.is_empty());
        for key in ["schema", "operation_id", "host_alias", "capability",
                    "started_at", "finished_at", "execution_result", "verdict",
                    "evidence", "authority", "input", "transport"] {
            assert!(json.contains(&format!("\"{key}\"")), "missing key {key}");
        }
    }

    #[test]
    fn receipt_refuses_overwrite() {
        let dir = std::env::temp_dir().join(format!("ibridge-test-{}", std::process::id()));
        let mut r = Receipt::new("op-dup", "h", "discovery", &[]);
        r.finalize();
        let p1 = r.write(&dir).unwrap();
        let p2 = r.write(&dir).unwrap_err();
        assert!(p2.contains("refusing to overwrite"));
        let _ = p1;
        let _ = std::fs::remove_dir_all(&dir);
    }
}

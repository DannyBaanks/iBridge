//! provider.applebridge — the protocol/evidence authority boundary.
//!
//! AppleBridge (Python, untracked-tree at `C:\Development\ISyCo Git\AppleBridge`)
//! owns probe-pack.v1 emission, device.report.v1 validation, evidence storage
//! and receipts. iBridge never re-validates protocol semantics itself — it
//! checks health, fetches packs, READS stored reports, and binds references
//! (path + sha256). Verdicts come from AppleBridge-accepted reports; iBridge
//! fabricates none.

use crate::core::{parse_iso, sha256_hex, ReportClassification};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AppleBridgeProvider {
    pub receiver_url: String,
    pub applebridge_dir: PathBuf,
}

impl AppleBridgeProvider {
    pub fn new(receiver_url: &str, applebridge_dir: &Path) -> Self {
        Self {
            receiver_url: receiver_url.trim_end_matches('/').to_string(),
            applebridge_dir: applebridge_dir.to_path_buf(),
        }
    }

    fn host_port(&self) -> Result<(String, u16), String> {
        let rest = self
            .receiver_url
            .strip_prefix("http://")
            .ok_or_else(|| format!("only http:// receiver URLs are supported (got {:?})", self.receiver_url))?;
        let (host, port) = rest
            .split_once(':')
            .ok_or_else(|| "receiver URL must include :port".to_string())?;
        Ok((host.to_string(), port.parse::<u16>().map_err(|_| "bad port")?))
    }

    /// Minimal HTTP/1.0 GET over std TcpStream (no extra deps).
    fn http_get(&self, path: &str, timeout: Duration) -> Result<(u16, Vec<u8>), String> {
        let (host, port) = self.host_port()?;
        let mut stream = TcpStream::connect((host.as_str(), port))
            .map_err(|e| format!("connect {}: {} (receiver down?)", self.receiver_url, e))?;
        stream.set_read_timeout(Some(timeout)).ok();
        let req = format!(
            "GET {} HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n",
            path, host
        );
        stream
            .write_all(req.as_bytes())
            .map_err(|e| format!("send failed: {e}"))?;
        let mut buf = Vec::new();
        stream
            .read_to_end(&mut buf)
            .map_err(|e| format!("read failed: {e}"))?;
        parse_http_response(&buf)
    }

    pub fn health(&self) -> Result<Value, String> {
        let (code, body) = self.http_get("/applebridge/health", Duration::from_secs(5))?;
        if code != 200 {
            return Err(format!("receiver health returned HTTP {code}"));
        }
        serde_json::from_slice(&body).map_err(|e| format!("health body not JSON: {e}"))
    }

    /// Fetch the served pack; returns (bytes, sha256).
    pub fn fetch_pack(&self) -> Result<(Vec<u8>, String), String> {
        let (code, body) = self.http_get("/applebridge/probe-pack", Duration::from_secs(5))?;
        if code != 200 {
            return Err(format!("pack fetch returned HTTP {code}"));
        }
        Ok((body.clone(), sha256_bytes(&body)))
    }

    /// Canonical pack bytes from the AppleBridge tree (provenance reference).
    pub fn canonical_pack(&self) -> Option<(Vec<u8>, String)> {
        let p = self.applebridge_dir.join("examples").join("probe-pack.v1.json");
        let bytes = std::fs::read(&p).ok()?;
        let sha = sha256_bytes(&bytes);
        Some((bytes, sha))
    }

    pub fn validate_pack_bytes(bytes: &[u8]) -> Result<(), String> {
        let v: Value = serde_json::from_slice(bytes)
            .map_err(|e| format!("pack is not valid JSON: {e}"))?;
        if v.get("schema").and_then(|s| s.as_str()) != Some("applebridge.probe-pack.v1") {
            return Err("pack schema != applebridge.probe-pack.v1".into());
        }
        let probes = v
            .get("probes")
            .and_then(|p| p.as_array())
            .ok_or("pack has no probes array")?;
        if probes.is_empty() {
            return Err("pack probes array is empty".into());
        }
        Ok(())
    }

    pub fn evidence_dir(&self) -> PathBuf {
        self.applebridge_dir.join("device-evidence")
    }

    /// Package dirs look like `20260918T235012Z-8cd57985` (applebridge receiver layout).
    pub fn snapshot_report_packages(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(self.evidence_dir()) {
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.len() == 25
                    && name.ends_with(&format!("-{}", &name[17..25]))
                    && name.as_bytes()[8] == b'T'
                    && name.as_bytes()[16] == b'Z'
                {
                    out.push(entry.path());
                }
            }
        }
        out.sort();
        out
    }
}

fn parse_http_response(buf: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let sep = b"\r\n\r\n";
    let pos = buf
        .windows(4)
        .position(|w| w == sep)
        .ok_or("malformed HTTP response (no header terminator)")?;
    let head = String::from_utf8_lossy(&buf[..pos]);
    let status: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .ok_or("malformed HTTP status line")?;
    Ok((status, buf[pos + 4..].to_vec()))
}

pub fn sha256_bytes(data: &[u8]) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(data);
    let out = h.finalize();
    out.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {path:?}: {e}"))?;
    Ok(sha256_bytes(&bytes))
}

/// A bound Doctor report: read-only parse of what AppleBridge's receiver
/// already validated and stored. iBridge adds classification + references.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoundReport {
    pub path: String,
    pub sha256: String,
    pub generated_at: String,
    pub ios_version: Option<String>,
    pub summary: Value,
    pub classification: String,
    pub consistency_ios_version_match: Option<bool>,
}

pub fn bind_report(
    path: &Path,
    max_age_hours: u64,
    discovered_ios_version: Option<&str>,
) -> Result<BoundReport, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read report {path:?}: {e}"))?;
    let v: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("report not valid JSON: {e}"))?;
    if v.get("schema").and_then(|s| s.as_str()) != Some("applebridge.device.report.v1") {
        return Err("report schema != applebridge.device.report.v1".into());
    }
    let generated_at = v
        .get("generated_at")
        .and_then(|s| s.as_str())
        .ok_or("report lacks generated_at")?
        .to_string();
    parse_iso(&generated_at).ok_or_else(|| format!("unparseable generated_at {generated_at:?}"))?;
    let classification = crate::core::classify_report_age(&generated_at, max_age_hours)?;
    let report_ios = v
        .pointer("/environment/os_version")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let summary = v
        .get("summary")
        .cloned()
        .ok_or("report lacks summary")?;
    let consistency = match (discovered_ios_version, &report_ios) {
        (Some(d), Some(r)) => Some(d == r),
        _ => None,
    };
    Ok(BoundReport {
        path: path.to_string_lossy().to_string(),
        sha256: sha256_bytes(&bytes),
        generated_at,
        ios_version: report_ios,
        summary,
        classification: serde_json::to_value(classification)
            .unwrap_or(Value::String("UNKNOWN".into()))
            .as_str()
            .unwrap_or("UNKNOWN")
            .to_string(),
        consistency_ios_version_match: consistency,
    })
}

// Classification string mapping used by receipts.
pub fn classification_str(c: ReportClassification) -> &'static str {
    match c {
        ReportClassification::Current => "CURRENT",
        ReportClassification::Historical => "HISTORICAL",
        ReportClassification::HistoricalStale => "HISTORICAL_STALE",
    }
}

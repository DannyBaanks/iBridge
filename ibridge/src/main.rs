//! iBridge CLI — status | discover | doctor.
//!
//! Only implemented commands are exposed. Deny remains representable: an
//! operation without authority exits DENIED with a receipt, never silently.
//! Communication is not authority.

use ibridge::{applebridge, core, discovery, receipt};

use applebridge::{AppleBridgeProvider, BoundReport};
use receipt::{AuthorityInReceipt, EvidenceRef, EvidenceVerdict, Receipt, Verdict};
use std::path::{Path, PathBuf};
use std::process::exit;

const DEFAULT_RECEIVER: &str = "http://127.0.0.1:8377";
const DEFAULT_APPLEBRIDGE: &str = r"C:\Development\ISyCo Git\AppleBridge";

// exit codes
const EXIT_PASS: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_DENIED: i32 = 2;
const EXIT_PARTIAL: i32 = 3;
const EXIT_NOT_DEMONSTRATED: i32 = 4;

fn host_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".into())
}

fn default_evidence_dir() -> PathBuf {
    PathBuf::from("evidence/ibridge/mvp")
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn parse_args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

fn op_id(prefix: &str) -> String {
    let ts = core::iso_now().replace(['-', ':', 'T', 'Z'], "");
    let tail = &core::sha256_hex(&[&ts, &std::process::id().to_string()])[..8];
    format!("{prefix}-{ts}-{tail}")
}

fn main() {
    let args = parse_args();
    let cmd = args.first().cloned().unwrap_or_default();
    match cmd.as_str() {
        "status" => cmd_status(&args),
        "discover" => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(cmd_discover(&args))
        }
        "doctor" => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(cmd_doctor(&args))
        }
        "" => {
            eprintln!("iBridge {}", core::IBRIDGE_VERSION);
            eprintln!("usage: ibridge <status|discover|doctor> [options]");
            exit(EXIT_ERROR);
        }
        other => {
            // unknown command: fail closed, do not imply capabilities
            let out = serde_json::json!({
                "error": format!("unknown command {other:?}"),
                "implemented": ["status", "discover", "doctor"],
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
            exit(EXIT_ERROR);
        }
    }
}

fn provider_from(args: &[String]) -> AppleBridgeProvider {
    let receiver = flag_value(args, "--receiver").unwrap_or_else(|| DEFAULT_RECEIVER.into());
    let ab_dir = flag_value(args, "--applebridge-dir").unwrap_or_else(|| DEFAULT_APPLEBRIDGE.into());
    AppleBridgeProvider::new(&receiver, Path::new(&ab_dir))
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

fn cmd_status(args: &[String]) {
    let host_alias = core::host_alias(&host_name());
    let prov = provider_from(args);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut doc = serde_json::json!({
        "schema": "ibridge.status.v1",
        "ibridge_version": core::IBRIDGE_VERSION,
        "host_alias": host_alias,
    });

    // receiver health
    let receiver = prov.health().ok();
    doc["receiver"] = serde_json::json!({
        "url": prov.receiver_url,
        "healthy": receiver.is_some(),
    });

    // canonical pack availability (AppleBridge tree provenance)
    doc["applebridge_dir"] = serde_json::json!(prov.applebridge_dir.to_string_lossy());
    doc["pack_canonical_available"] = serde_json::json!(prov.canonical_pack().is_some());

    // devices (read-only)
    let contract = rt.block_on(discovery::discover(&host_alias));
    doc["usbmuxd_reachable"] = serde_json::json!(contract.transport.reachable);
    doc["transport_detail"] = serde_json::json!(contract.transport.detail);
    doc["device_count"] = serde_json::json!(contract.devices.len());
    doc["devices"] = serde_json::Value::Array(contract
        .devices
        .iter()
        .map(|d| {
            serde_json::json!({
                "alias": d.device_alias,
                "model": d.model,
                "iosVersion": d.ios_version,
                "connectionType": d.connection_type,
            })
        })
        .collect::<Vec<_>>());

    // last receipt
    let ev_dir = flag_value(args, "--evidence-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_evidence_dir);
    doc["last_receipt"] = last_receipt_summary(&ev_dir);

    println!("{}", serde_json::to_string_pretty(&doc).unwrap());
    exit(if contract.transport.reachable { EXIT_PASS } else { EXIT_ERROR });
}

fn last_receipt_summary(ev_dir: &Path) -> serde_json::Value {
    let mut receipts_dir = ev_dir.to_path_buf();
    receipts_dir.push("receipts");
    let mut latest: Option<(String, serde_json::Value)> = None;
    if let Ok(rd) = std::fs::read_dir(&receipts_dir) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".json") {
                continue;
            }
            if let Ok(body) = std::fs::read_to_string(entry.path()) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    let ts = v
                        .get("started_at")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    if latest.as_ref().map(|(t, _)| ts > *t).unwrap_or(true) {
                        latest = Some((
                            ts,
                            serde_json::json!({
                                "path": entry.path().to_string_lossy(),
                                "operation_id": v.get("operation_id"),
                                "capability": v.get("capability"),
                                "verdict": v.get("verdict"),
                                "execution_result": v.get("execution_result"),
                                "started_at": v.get("started_at"),
                            }),
                        ));
                    }
                }
            }
        }
    }
    latest.map(|(_, v)| v).unwrap_or(serde_json::Value::Null)
}

// ---------------------------------------------------------------------------
// discover
// ---------------------------------------------------------------------------

async fn cmd_discover(args: &[String]) {
    let host_alias = core::host_alias(&host_name());
    let capability = "discovery";
    let decision = core::authorize(capability);
    let mut r = Receipt::new(&op_id("discover"), &host_alias, capability, args);
    r.authority = AuthorityInReceipt {
        requested: vec![capability.into()],
        allowed: if decision.allowed { vec![capability.into()] } else { vec![] },
        denied: if decision.allowed { vec![] } else { vec![capability.into()] },
    };

    let contract = discovery::discover(&host_alias).await;
    r.transport.usbmuxd = contract.transport.reachable;
    r.transport.detail = contract.transport.detail.clone();

    // evidence dir + raw contract copy
    let ev_dir = flag_value(args, "--evidence-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_evidence_dir);
    let _ = std::fs::create_dir_all(&ev_dir);

    let contract_json = serde_json::to_string_pretty(&contract).unwrap();
    let raw_path = ev_dir.join(format!("discover-{}.json", contract.observed_at.replace(['-', ':', 'T', 'Z'], "")));
    let raw_sha = if std::fs::write(&raw_path, &contract_json).is_ok() {
        r.evidence.push(EvidenceRef {
            role: "discover_output".into(),
            path: raw_path.to_string_lossy().to_string(),
            sha256: applebridge::sha256_bytes(contract_json.as_bytes()),
        });
        Some(())
    } else {
        None
    };

    // structured vs human output (both; stdout gets JSON by default, --human for table)
    if has_flag(args, "--human") {
        println!("iBridge discover — {} device(s) [usbmuxd: {}]", contract.devices.len(),
            if contract.transport.reachable { "reachable" } else { "UNREACHABLE" });
        for (i, d) in contract.devices.iter().enumerate() {
            println!(
                "  [{}] alias={} model={} ios={} conn={} capabilities={:?}",
                i, d.device_alias,
                d.model.as_deref().unwrap_or("?"),
                d.ios_version.as_deref().unwrap_or("?"),
                d.connection_type,
                d.capabilities
            );
            if let Some(diag) = &d.diagnostic {
                println!("        diagnostic: {diag}");
            }
        }
    } else {
        println!("{contract_json}");
    }

    // fill receipt
    if let Some(first) = contract.devices.first() {
        r.device_alias = Some(first.device_alias.clone());
        r.device = Some(receipt::DeviceInReceipt {
            device_alias: first.device_alias.clone(),
            model_class: first.model.clone(),
            ios_version: first.ios_version.clone(),
            connection_type: Some(first.connection_type.clone()),
        });
    }
    r.execution_result = format!("devices_enumerated:{}", contract.devices.len());
    r.verdict = if contract.transport.reachable && !contract.devices.is_empty() {
        Verdict::Pass
    } else if contract.transport.reachable {
        Verdict::NotDemonstrated
    } else {
        Verdict::Error
    };
    if raw_sha.is_none() {
        r.notes.push("evidence dir not writable; raw discover output not preserved".into());
    }
    r.finalize();

    // --expect-alias: selector check (case: invalid/stale selector)
    if let Some(expect) = flag_value(args, "--expect-alias") {
        let found = contract.devices.iter().any(|d| d.device_alias == expect);
        r.notes.push(format!("--expect-alias {expect} matched={found}"));
        r.verdict = if found { r.verdict.clone() } else { Verdict::Error };
        r.execution_result = format!("{}; expect_alias_matched={}", r.execution_result, found);
    }

    let receipt_path = r.write(&ev_dir).ok();
    if let Some(p) = &receipt_path {
        eprintln!("receipt: {p}");
    }

    let code = match r.verdict {
        Verdict::Pass => EXIT_PASS,
        Verdict::NotDemonstrated => EXIT_NOT_DEMONSTRATED,
        _ => EXIT_ERROR,
    };
    exit(code);
}

// ---------------------------------------------------------------------------
// doctor (orchestration)
// ---------------------------------------------------------------------------

async fn cmd_doctor(args: &[String]) {
    let host_alias = core::host_alias(&host_name());
    let capability = "doctor_orchestration";
    let mut r = Receipt::new(&op_id("doctor"), &host_alias, capability, args);

    // authority: base capability + any --with <cap>
    let mut requested = vec![capability.to_string()];
    if let Some(extra) = flag_value(args, "--with") {
        requested.push(extra.clone());
    }
    let mut allowed = vec![];
    let mut denied = vec![];
    for cap in &requested {
        let d = core::authorize(cap);
        if d.allowed {
            allowed.push(cap.clone());
        } else {
            denied.push(cap.clone());
            r.notes.push(format!("authority: {cap} denied: {}", d.reason));
        }
    }
    r.authority = AuthorityInReceipt { requested, allowed, denied };
    if !r.authority.denied.is_empty() {
        r.execution_result = "denied_by_authority".into();
        r.verdict = Verdict::Denied;
        r.finalize();
        let ev_dir = flag_value(args, "--evidence-dir").map(PathBuf::from).unwrap_or_else(default_evidence_dir);
        print_and_exit(&r, &ev_dir, EXIT_DENIED);
    }

    let prov = provider_from(args);
    r.input.receiver_url = prov.receiver_url.clone();

    // 1) discover (read-only)
    let contract = discovery::discover(&host_alias).await;
    r.transport.usbmuxd = contract.transport.reachable;
    r.transport.detail.clone_from(&contract.transport.detail);

    let selected = if let Some(want) = flag_value(args, "--alias") {
        contract.devices.iter().find(|d| d.device_alias == want).cloned()
    } else {
        contract.devices.first().cloned()
    };

    let device = selected;
    if device.is_none() {
        r.notes.push(
            "no_device_discovered: receiver/pack checks continue; device identity is ABSENT \
             from this receipt (honest absence — nothing fabricated)"
                .into(),
        );
    }

    if let Some(dv) = &device {
        r.device_alias = Some(dv.device_alias.clone());
        r.device = Some(receipt::DeviceInReceipt {
            device_alias: dv.device_alias.clone(),
            model_class: dv.model.clone(),
            ios_version: dv.ios_version.clone(),
            connection_type: Some(dv.connection_type.clone()),
        });
        if let Some(diag) = &dv.diagnostic {
            r.notes.push(format!("lockdown enrichment diagnostic (observation only): {diag}"));
        }
    }

    // 2) receiver transport (start it if down — our own tool, reversible)
    let receiver_healthy = match prov.health() {
        Ok(_) => true,
        Err(e) => {
            r.notes.push(format!("receiver down ({e}); attempting to start applebridge receive"));
            start_receiver(&prov);
            let mut ok = false;
            for _ in 0..15 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if prov.health().is_ok() {
                    ok = true;
                    break;
                }
            }
            ok
        }
    };
    r.transport.receiver = receiver_healthy;
    if !receiver_healthy {
        r.execution_result = "receiver_unavailable".into();
        r.verdict = Verdict::Error;
        r.finalize();
        let ev_dir = flag_value(args, "--evidence-dir").map(PathBuf::from).unwrap_or_else(default_evidence_dir);
        print_and_exit(&r, &ev_dir, EXIT_ERROR);
    }

    // 3) pack check (fetch from receiver; optionally override with --pack-file for failure tests)
    let pack_bytes: Vec<u8>;
    let pack_sha: String;
    if let Some(pack_path) = flag_value(args, "--pack-file") {
        pack_bytes = std::fs::read(&pack_path)
            .unwrap_or_else(|e| {
                r.execution_result = format!("pack_file unreadable: {e}");
                r.verdict = Verdict::Error;
                r.finalize();
                let ev_dir = flag_value(args, "--evidence-dir").map(PathBuf::from).unwrap_or_else(default_evidence_dir);
                print_and_exit(&r, &ev_dir, EXIT_ERROR);
            });
        pack_sha = applebridge::sha256_bytes(&pack_bytes);
    } else {
        let (b, s) = prov.fetch_pack().unwrap_or_else(|e| {
            r.execution_result = format!("pack_fetch_failed: {e}");
            r.verdict = Verdict::Error;
            r.finalize();
            let ev_dir = flag_value(args, "--evidence-dir").map(PathBuf::from).unwrap_or_else(default_evidence_dir);
            print_and_exit(&r, &ev_dir, EXIT_ERROR);
        });
        pack_bytes = b;
        pack_sha = s;
    }

    if let Err(e) = AppleBridgeProvider::validate_pack_bytes(&pack_bytes) {
        // PHASE6 case: malformed pack -> ERROR, no partial evidence promotion
        r.execution_result = format!("malformed_pack_rejected: {e}");
        r.verdict = Verdict::Error;
        r.evidence.push(EvidenceRef {
            role: "malformed_pack_rejected".into(),
            path: flag_value(args, "--pack-file").unwrap_or_else(|| "receiver".into()),
            sha256: pack_sha.clone(),
        });
        r.finalize();
        let ev_dir = flag_value(args, "--evidence-dir").map(PathBuf::from).unwrap_or_else(default_evidence_dir);
        print_and_exit(&r, &ev_dir, EXIT_ERROR);
    }
    r.input.pack_sha256 = Some(pack_sha.clone());
    if let Some((_, canonical_sha)) = prov.canonical_pack() {
        if canonical_sha != pack_sha {
            r.notes.push(format!(
                "pack sha drift: receiver served {pack_sha}, AppleBridge canonical is {canonical_sha}"
            ));
        }
    }

    // 4) bind a report: --bind-report PATH | --wait SECONDS | nothing -> informative NOT_DEMONSTRATED
    let max_age_hours: u64 = flag_value(args, "--max-age-hours")
        .and_then(|s| s.parse().ok())
        .unwrap_or(48);

    let bound: Result<BoundReport, String> = if let Some(p) = flag_value(args, "--bind-report") {
        if device.is_none() {
            r.notes.push("degraded_bind: binding without a discovered device; device fields are null by design".into());
        }
        let path = PathBuf::from(&p);
        let path = if path.is_dir() { path.join("report.json") } else { path };
        applebridge::bind_report(
            &path,
            max_age_hours,
            device.as_ref().and_then(|d| d.ios_version.as_deref()),
        )
    } else if let Some(wait_s) = flag_value(args, "--wait").and_then(|s| s.parse::<u64>().ok()) {
        if device.is_none() {
            Err("wait mode requires a discovered device (the report would be device-unbound)".into())
        } else {
            let before = prov.snapshot_report_packages();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(wait_s);
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let now = prov.snapshot_report_packages();
                let new_pkg = now.iter().find(|p| !before.contains(p));
                if let Some(pkg) = new_pkg {
                    break applebridge::bind_report(
                        &pkg.join("report.json"),
                        max_age_hours,
                        device.as_ref().and_then(|d| d.ios_version.as_deref()),
                    );
                }
                if std::time::Instant::now() >= deadline {
                    break Err(format!("no_new_report_within_{}_s", wait_s));
                }
            }
        }
    } else {
        Err("no bind mode selected: use --bind-report <path> or --wait <seconds> (the Doctor app on the phone requires a manual tap; iBridge cannot remote-trigger it)".into())
    };

    match bound {
        Ok(b) => {
            r.evidence.push(EvidenceRef {
                role: "device_report".into(),
                path: b.path.clone(),
                sha256: b.sha256.clone(),
            });
            r.evidence_verdict = Some(EvidenceVerdict {
                classification: b.classification.clone(),
                report_sha256: Some(b.sha256.clone()),
                report_generated_at: Some(b.generated_at.clone()),
                consistency_ios_version_match: b.consistency_ios_version_match,
                report_summary: Some(b.summary.clone()),
            });
            r.execution_result = format!("bound_report:{}", b.classification);
            r.verdict = match b.classification.as_str() {
                "CURRENT" => Verdict::Pass,
                "HISTORICAL" | "HISTORICAL_STALE" => Verdict::Partial,
                other => {
                    r.notes.push(format!("unexpected classification {other}"));
                    Verdict::Error
                }
            };
            if b.consistency_ios_version_match == Some(false) {
                r.notes.push("discovered iOS version does NOT match report os_version — check device selection".into());
            }
            let code = if r.verdict == Verdict::Pass { EXIT_PASS } else { EXIT_PARTIAL };
            r.finalize();
            let ev_dir = flag_value(args, "--evidence-dir").map(PathBuf::from).unwrap_or_else(default_evidence_dir);
            print_and_exit(&r, &ev_dir, code);
        }
        Err(e) => {
            r.execution_result = format!("no_report_bound: {e}");
            r.verdict = Verdict::NotDemonstrated;
            r.notes.push("no partial evidence was promoted to PASS".into());
            r.finalize();
            let ev_dir = flag_value(args, "--evidence-dir").map(PathBuf::from).unwrap_or_else(default_evidence_dir);
            print_and_exit(&r, &ev_dir, EXIT_NOT_DEMONSTRATED);
        }
    }
}

fn start_receiver(prov: &AppleBridgeProvider) {
    use std::os::windows::process::CommandExt;
    let port = prov
        .receiver_url
        .rsplit(':')
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8377);
    let _ = std::process::Command::new("applebridge")
        .args([
            "receive",
            "--port",
            &port.to_string(),
            "--evidence-dir",
            prov.evidence_dir().to_string_lossy().as_ref(),
        ])
        .current_dir(&prov.applebridge_dir)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .spawn();
}

fn print_and_exit(r: &Receipt, ev_dir: &Path, code: i32) -> ! {
    match r.write(ev_dir) {
        Ok(p) => eprintln!("receipt: {p}"),
        Err(e) => eprintln!("receipt write failed: {e}"),
    }
    println!("{}", serde_json::to_string_pretty(r).unwrap());
    exit(code);
}

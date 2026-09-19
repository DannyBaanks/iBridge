# ibridge.receipt.v1

One receipt per real operation, written append-only under
`evidence/ibridge/mvp/receipts/<started_at>-<operation_id>.json`
(timestamped name; overwrites are refused).

```json
{
  "schema": "ibridge.receipt.v1",
  "operation_id": "doctor-<ts>-<rand8>",
  "ibridge_version": "0.1.0",
  "host_alias": "sha256(\"ibridge-host\"||hostname)[:16]",
  "device_alias": "ab12cd34ef567891",
  "device": {
    "device_alias": "…",
    "model_class": "iPhone13,2",
    "ios_version": "18.7.8",
    "connection_type": "usb"
  },
  "capability": "doctor_orchestration",
  "authority": { "requested": [], "allowed": [], "denied": [] },
  "input": { "pack_sha256": "…", "receiver_url": "http://…", "arguments": [] },
  "transport": { "usbmuxd": true, "receiver": true, "detail": "…" },
  "started_at": "2026-09-19T06:34:19Z",
  "finished_at": "2026-09-19T06:34:20Z",
  "execution_result": "bound_report:HISTORICAL",
  "evidence_verdict": {
    "classification": "CURRENT | HISTORICAL | HISTORICAL_STALE",
    "report_sha256": "…",
    "report_generated_at": "…",
    "consistency_ios_version_match": true,
    "report_summary": { "pass": 7, "fail": 0, "not_demonstrated": 1,
                        "requires_macos_xcode": 0, "verdict": "PARTIAL" }
  },
  "evidence": [ { "role": "device_report", "path": "…", "sha256": "…" } ],
  "verdict": "PASS | PARTIAL | NOT_DEMONSTRATED | DENIED | ERROR",
  "notes": []
}
```

## Rules

- **Operational result ≠ evidence verdict.** `execution_result` is what the
  host did; `evidence_verdict` is what the bound evidence says.
- **Overall verdict is conservative:**
  CURRENT report → PASS · HISTORICAL/HISTORICAL_STALE → PARTIAL ·
  no report (timeout/no device) → NOT_DEMONSTRATED · authority denial →
  DENIED · transport/pack failures → ERROR.
- **Privacy:** receipts contain NO UDID, no device name, no Apple ID, no
  tokens. Device identity is `sha256("ibridge-device"||host_alias||udid)[:16]`.
- **Append-only:** a receipt never overwrites existing evidence.
- Deny stays representable: a denied operation produces a DENIED receipt
  (exit 2), not silence.

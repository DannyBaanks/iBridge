# iBridge MVP evidence (Phases 3–9, executed 2026-09-19)

All raw outputs live under `evidence/` (SHA256SUMS generated). Rust unit +
integration tests: **17/17 green** (11 unit in-crate + 6 integration in
`ibridge/tests/bind_report.rs`). AppleBridge regression re-run at MVP close:
**116/116 PASS** (current execution, not the historical observation).

## Environment

- Host: Windows 11, hostname → host_alias `fb386f331eaaded1`
- Toolchain: rustc/cargo 1.97.1 (recorded; `cd ibridge && cargo build
  --release`); target dir `ibridge/target/`
- Dependencies: `idevice` resolved to **0.1.68** (semver range "0.1.57" —
  same resolution upstream builds get), `tokio`, `serde`, `serde_json`,
  `sha2`. Upstream `isideload` NOT pulled (install/signing seams unwired).
- Apple Mobile Device Service (usbmuxd): running, port 27015
- AppleBridge receiver: running (port 8377, alive since 2026-09-18)
- **Physical iPhone13,2 (iOS 18.7.8): NOT ATTACHED during this run** — it
  was attached and demonstrated on 2026-09-18 (Doctor report `8cd57985…`).

## Phase 3 — contract + discover (VERIFIED, device-dependent parts NOT_DEMONSTRATED)

- `ibridge.discover` implemented via the same `idevice` usbmuxd seam
  upstream iLoader uses (`UsbmuxdConnection`, `get_devices`,
  `LockdownClient::get_value(ProductType/ProductVersion)` — read-only).
- usbmuxd transport: **REACHABLE, verified** (binary-plist negotiation handled
  by the crate; a manual stdlib plist probe failed against AMDS — recorded
  as the reason the crate seam is the correct provider).
- Device enumeration: **0 devices at run time → physical discovery
  NOT_DEMONSTRATED this run** (honest absence; the machinery is verified by
  the 2026-09-18 upstream-GUI install on this host and by code-path parity
  with upstream `device.rs`).
- Structured output: `ibridge.device.v1` JSON (schema, host_alias, devices
  with alias/model/ios_version/connection_type/capabilities/observedAt).
- Aliases: `sha256(salt||udid)[:16]`; stability unit-tested; UDIDs never in
  receipts or stdout.

## Phase 6 — failure matrix (executed, `evidence/exit-codes.txt`)

| # | case | run | exit | verdict recorded |
|---|---|---|---|---|
| 1 | no device attached | runA1/runA2 | 4 | NOT_DEMONSTRATED (real physical state) |
| 2 | recognized iPhone attached | — | — | NOT_DEMONSTRATED (device absent this run) |
| 3 | discovery repeatability | runA1+runA2 | 4/4 | stable contract, alias machinery unit-tested |
| 4 | identity/runtime detection | — | — | NOT_DEMONSTRATED (needs device; code path mirrors upstream) |
| 5 | transport establishment | runA4 | 1 | usbmuxd reachable ✓; dead endpoint (USBMUXD_SOCKET_ADDRESS=127.0.0.1:1) → ERROR with ConnectionRefused |
| 6 | malformed pack | runB3 | 1 | garbage pack rejected, receipt records `malformed_pack_rejected` |
| 7 | valid Doctor pack | runB5/B6 | 3 | pack fetched from receiver, sha `f885ee35…` == canonical |
| 8 | protocol roundtrip | runB5 | 3 | receiver health + pack served + report schema check |
| 9 | report persistence | runB5 | 3 | report referenced by path + sha `8cd57985…` (AppleBridge-stored) |
| 10 | repeated execution | 15 receipts | — | append-only, zero overwrites (unit-tested + observed) |
| 11 | disconnect during operation | — | — | NOT_DEMONSTRATED (requires physical unplug while attached) |
| 12 | reconnect | — | — | NOT_DEMONSTRATED (superseded by 11) |
| 13 | unavailable capability | runB2 | 2 | unknown/bogus capability → DENIED |
| 14 | explicitly denied operation | runB1 | 2 | `--with signing` → DENIED receipt (fail closed) |
| 15 | invalid device selector | runA3 | 1 | `--expect-alias` miss → ERROR, receipt notes match=false |
| 16 | stale result/report detection | runB6 | 3 | `--max-age-hours 1` → classification HISTORICAL_STALE |

No destructive case was fabricated to fill the matrix; physical cases are
NOT_DEMONSTRATED with reasons.

## Phase 5 — vertical slice status

The full chain is implemented: authority → discover → receiver health
(auto-start if down) → pack fetch/validate → bind report → receipt. The only
step that cannot run unattended is the **on-phone Doctor tap** (iOS apps
cannot be remote-launched from a Windows host without Mac tooling — that
boundary is documented). With the phone absent, the orchestration was
demonstrated in degraded mode binding the REAL 2026-09-18 report
(`8cd57985…`): receipts `doctor-20260919063419-d6390614` (HISTORICAL →
PARTIAL) and `doctor-20260919063420-7ea5a4b9` (HISTORICAL_STALE → PARTIAL).
No partial evidence was promoted to PASS.

**Danny's 2-minute completion path (phone attached):**
`ibridge doctor --bind-report "C:\Development\ISyCo Git\AppleBridge\device-evidence\20260918T235012Z-8cd57985\report.json"`
→ device-bound receipt; or tap Run Doctor + Send on the phone with
`ibridge doctor --wait 300` → CURRENT → PASS.

## Phase 7 — regressions

- AppleBridge suite: **116/116 PASS** (re-run at MVP close)
- iBridge: 17/17 tests; build clean
- applebridge-probe: unchanged (no rebuild needed — Doctor untouched)
- Upstream iLoader GUI: not built locally (NOT_DEMONSTRATED; requires the
  full Tauri toolchain — the official Windows exe remains the reference
  build and was used successfully on this host on 2026-09-18)

## Worktree delta attribution

- OpencodeNative: 1 dirty line observed before AND after (other agent's WIP —
  untouched, not absorbed).
- AppleBridge: 135-file snapshot identical before/after (only
  `device-evidence/` grew with receipts from the receiver, which predates
  this MVP run).
- No unrelated ISyCo/OpenISy files touched.

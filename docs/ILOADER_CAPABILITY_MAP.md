# iLoader Capability Map (as inspected at baseline 348eefd)

All rows are UPSTREAM FACT unless labeled otherwise. Source: `src-tauri/src/*.rs`,
`src-tauri/Cargo.toml`, frontend `src/pages/*` of the fork at baseline.
Classifications: REUSE / WRAP / ADAPT / REPLACE / DROP / UNDECIDED.

**Authority rule (iBridge invariant):** pairing/transport grants COMMUNICATION
only. iBridge execution authority comes from the ISyCo flow
`discover → explicit allowed operation → execute → return → receipt`. A paired
device never gains implicit execute authority.

| capability | upstream evidence | failure modes (observed in code paths) | classification | authority / notes |
|---|---|---|---|---|
| device.discovery | `device.rs:list_devices()` via `idevice` crate `usbmuxd` feature; wireless via `core_device_proxy` | no device attached; usbmuxd service missing | REUSE | read-only; the natural `ibridge doctor` step 1 |
| device.identity | `DeviceInfo{name, id, u32 conn id, udid, connection_type, version}` | partial info on locked devices | ADAPT | receipts need a stable, minimal identity: salted-hash alias of UDID + iOS version (privacy decision → Phase 3) |
| device.pair (USB) | `pairing.rs:generate_lockdown_plist`, `place_pairing_cmd`, `export_pairing_cmd`, `installed_pairing_apps` | pairing rejected on-device | REUSE | manual pairing = communication establishment; logged as evidence |
| device.pair (wireless) | `pairing.rs:generate_rppairing`, `has_stored_rppairing`, `delete_stored_rppairing`; crate features `remote_pairing`, `tunnel_tcp_stack`, `xpc`, `rsd` | RP unsupported on old iOS | REUSE | wireless for modern devices; LOCAL OBSERVATION: Danny's install ran over this stack (works on this host) |
| device.transport.usb | `get_usbmuxd()`, `get_provider_from_connection()`; Windows via Apple Mobile Device Service | driver/service missing | REUSE | VERIFIED LOCALLY: Danny's iLoader exe + iPhone13,2 worked 2026-09-18 |
| device.transport.wifi | RP/tunnel (RemoteXPC) per Cargo features | firewall/network segmentation | REUSE | NOT_DEMONSTRATED on this host yet (Danny's run was USB) |
| app.install | `sideload.rs:sideload`, `sideload_operation`, `install_sidestore_operation`, `download(url)`; `isideload` crate (branch `apple-codesign-quick`) | Apple ID auth, 7-day free cert expiry, 3-app limit, bundle-ID collision | REUSE (via adapter, not GUI) | **the core host capability for iBridge**: deploy the AppleBridgeProbe/Doctor IPA |
| app.launch | **NOT PRESENT upstream** (no launch command in `lib.rs` handler) | — | UNDECIDED | would require new work (idevice launch/instproxy surface); NOT_DEMONSTRATED; Phase 9 candidate only |
| signing.* (Apple ID session) | `account.rs:login_new`, `login_stored`, `logged_in_as`, `invalidate_account`, `reset_anisette_state` (anisette via isideload) | 2FA, session expiry | WRAP | **most sensitive**: uses Danny's Apple ID; always explicit, never automated beyond install; no credential storage outside keyring |
| certificates.* | `account.rs:get_certificates`, `revoke_certificate` | cert limit on free accounts | WRAP | exposed only as explicit operations |
| appid.* | `account.rs:list_app_ids`, `delete_app_id` (creation implicit during sideload) | bundle-ID taken | WRAP | naming collisions (e.g. `com.applebridge.probe`) surfaced here |
| device.logs | `logging.rs` (tracing + tracing-appender, file logs); crash/diag relay NOT present upstream | log rotation | ADAPT | iBridge must emit structured logs into receipts/evidence |
| secure.storage | `secure_storage.rs` + `keyring` crate (win/apple/linux-native) | keyring unavailable → `force_disable_keyring` fallback | REUSE | secrets never enter evidence (redaction invariant carried over from AppleBridge) |
| GUI frontend | React pages: `AppIds`, `Certificates`, `Pairing`, `Settings` | — | UNDECIDED | keep for manual use; iBridge MVP is machine-first (CLI/host), GUI untouched in Phase 0–2 |
| auto-updater | `tauri-plugin-updater` (desktop only) | — | DROP from iBridge MVP | not a bridge capability |
| release workflows | `.github/workflows/build.yml`, `download-count.yml` | — | DROP from iBridge MVP | we build via own workflow when needed |

## Upstream build/runtime requirements (UPSTREAM FACT + LOCAL OBSERVATION)

- Tauri 2 + Rust (edition 2024) + React 19 + bun/npm; crates `idevice 0.1.57`,
  `isideload 0.3.17` (git pin), `keyring`, `tauri-plugin-*`.
- Windows: usbmuxd-equivalent via Apple Mobile Device Service (installed with
  iTunes/Apple Devices). LOCAL OBSERVATION: the official
  `iloader-windows-x64.exe` (Danny's copy) discovered and installed to the
  iPhone on this host on 2026-09-18 — the stack is proven here.
- Building the GUI locally: NOT_DEMONSTRATED in Phase 0–2 (no Rust toolchain
  check performed; not required for docs-only phases).

## What this means for iBridge (summary)

The entire host/device machinery iBridge needs already exists as **library
crates** (`idevice`, `isideload`) behind a thin Tauri command layer — an
adapter can wrap the crates directly without entangling the GUI.

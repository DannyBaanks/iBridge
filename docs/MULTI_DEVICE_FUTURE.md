# Multi-iPhone future surface — DESIGN ONLY (Phase 9, not implemented)

Do not build a five-iPhone distributed system before one iPhone has a clean,
reproducible contract. This document is the future design sketch.

## Command surface (future)

```
ibridge devices
DEVICE       IOS       ARCH     CAPABILITIES          STATE
iphone-A     18.7.8    arm64    doctor,verify,store   READY
iphone-B     17.6.1    arm64    doctor,compat         READY
iphone-C     —         —        —                     OFFLINE
```

- `DEVICE` = the same privacy alias as today (`sha256(salt||udid)[:16]`), so
  the table stays stable across sessions without storing UDIDs.
- `STATE` derives from the last *current* receipt per device + live
  discovery: READY / BUSY / OFFLINE / UNVERIFIED.
- `CAPABILITIES` is the **advertised** surface (what the provider stack
  reports possible), NOT the executable surface.

## Routing model (future)

```
intent
  ↓ required capability
  ↓ eligible devices (advertisement ∩ discovery ∩ health)
  ↓ explicit routing policy (owned-devices-only; one-device-default)
  ↓ physical host executes on the selected device
  ↓ receipt (device_alias-bound, append-only)
```

## The invariant that scales: communication ≠ authority

- Reachability (usbmuxd/WiFi tunnel) changes the DEVICE LIST, never the
  ALLOWED list. A device being online does not make any capability
  executable.
- Authority stays centralized in the host policy (the registry in
  `ibridge/src/core.rs`), same for one device or fifty.
- Capability advertisement (provider-level) and authority (host policy)
  are separate data structures by design; routing MUST intersect both.
- Per-device receipts keep the append-only + sha256-reference rules; a
  device table is a VIEW over receipts, not a new truth source.

## Open design questions (for a future compose)

1. Concurrent orchestration on multiple devices — single receiver with
   device-tagged evidence packages, or per-device evidence roots?
2. Wireless (RP/tunnel) discovery exposure — upstream `idevice` features
   exist; authority for `device.transport.wifi` is unchanged.
3. Device selection policy: explicit alias always, or "first READY wins"
   with a `--device` override? (recommend explicit-only).

# iBridge

**iBridge turns physical iOS devices into explicit, observable capabilities for
ISyCo-compatible hosts.**

```
Windows host (this CLI, Rust)
      │
      ├── provider.iloader     (device machinery, iLoader-derived: ideview/isideload crates)
      ├── provider.applebridge (protocol/evidence authority, Python)
      └── iOS Doctor           (Swift app on the phone: observes, never reasons)
      │
      ▼
  physical iPhone  →  device.report.v1  →  evidence + ibridge.receipt.v1
```

- **Upstream-derived functionality** (from
  [iloader](https://github.com/nab138/iloader) by nab138, MIT — this repo is a
  fork): device discovery/identity over usbmuxd via the `idevice` Rust crate,
  and the architectural seams for pairing, wireless transport and
  install/signing (`isideload`). iBridge reuses the discovery seam; the rest
  are documented boundaries, not yet wired into the CLI.
- **iBridge functionality** (ours): the thin host CLI (`ibridge status |
  discover | doctor`), the capability authority model
  (communication ≠ authority), `ibridge.receipt.v1`, and evidence binding.
- **AppleBridge functionality** (separate project at
  `C:\Development\ISyCo Git\AppleBridge`): probe packs, `device.report.v1`
  validation, the LAN evidence receiver, redaction, receipts. iBridge calls
  it semantically (HTTP + files), never imports it.
- **iOS Doctor functionality** (separate app repo
  [DannyBaanks/applebridge-probe](https://github.com/DannyBaanks/applebridge-probe)):
  the on-device probe suite that produces the reports. Unchanged by iBridge.

## Commands

```
ibridge status                       # version, providers, devices, receiver, last receipt
ibridge discover [--human] [--expect-alias A]   # enumerate devices via usbmuxd (read-only)
ibridge doctor [--bind-report P] [--wait S] [--max-age-hours H] [--with CAP]
```

Exit codes: `0` PASS · `1` ERROR · `2` DENIED · `3` PARTIAL ·
`4` NOT_DEMONSTRATED. Deny is always representable: `--with signing` writes a
DENIED receipt and exits 2 — signing/certificates/appid act on the linked
Apple account and are NOT implemented (fail closed).

Build (recorded toolchain: rustc/cargo 1.97.1, MSVC target):

```
cd ibridge && cargo build --release
```

Evidence of the MVP run lives in `evidence/` (see
`docs/IBRIDGE_MVP_EVIDENCE.md` and `evidence/SHA256SUMS`).

## Provenance and credit

This project is a fork of [iloader](https://github.com/nab138/iloader)
("User friendly sideloader", © nab138, MIT source code) renamed **iBridge**.
The "iloader" name, logos and graphical assets are NOT MIT-licensed
(`LICENSE-BRANDING`) and are retained here unmodified, with attribution:
iBridge is not an official iloader release and is not endorsed by nab138.
See https://iloader.app and `docs/UPSTREAM_PROVENANCE.md`
(preserved upstream README: `docs/UPSTREAM_README.md`).

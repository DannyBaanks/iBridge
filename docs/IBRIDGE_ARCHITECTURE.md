# iBridge Architecture

iBridge turns physical iOS devices into explicit, observable capabilities for
ISyCo-compatible hosts. It is NOT a GUI replacement for iLoader, NOT a
re-implementation of AppleBridge, and NOT an execution oracle — communication
is not authority.

## The invariant

```
Windows / ISyCo
      │
      ▼
   iBridge  ─────────────────────────────────────────┐
      │                                             │
      ├── provider.iloader (host/device machinery)   │  iLoader-derived code
      │     ├── device.discovery   (usbmuxd / RP)    │  = HOST CAPABILITIES
      │     ├── device.pair        (lockdown / rppairing)
      │     ├── device.transport   (USB / wireless tunnel)
      │     ├── app.install       (isideload sign+install)
      │     └── device.logs       (tracing → evidence)
      │
      ├── provider.applebridge (protocol/evidence)   │  AppleBridge-derived code
      │     ├── probe-pack.v1 / device.report.v1     │  = PROTOCOL SEMANTICS
      │     ├── result.v1 + receipts + redaction     │
      │     └── receiver (LAN evidence store)        │
      │
      └── device: ios-doctor (AppleBridgeProbe)       = DEVICE-OBSERVED VERIFICATION
             │
             ▼
        physical iPhone → report.v1 → host evidence → ISyCo/Bridge
```

- **iLoader-derived machinery = host/device capabilities.**
- **AppleBridge = protocol/evidence semantics.**
- **iOS Doctor = device-observed verification.**
Filenames may differ; the invariant matters more than the layout.

## Authority model (preserved from ISyCo)

```
discover capability
   ↓
explicit allowed operation   ← deny = absence of execute authority
   ↓
execute
   ↓
return (raw observation)
   ↓
receipt / evidence (sha256-bound, redacted)
```

Consequences:
- A paired/connected iPhone gains NOTHING until an operation is explicitly
  allowed. Pairing is recorded as communication establishment, not consent.
- Signing/Apple-ID operations (`signing.*`, `certificates.*`, `appid.*`) are
  the most sensitive surface: they act on Danny's Apple account and are
  always explicit, never automated in the MVP beyond app.install.
- No arbitrary shell into the device exists in the contract (mirrors
  AppleBridge's no-`ios.shell` rule).

## Language/boundary note (decision for Phase 3, options mapped)

The host machinery is Rust (`idevice`/`isideload` crates); the protocol
authority is Python (AppleBridge). Both options keep the invariant:

- **Option A (recommended): thin Rust host CLI + AppleBridge as protocol
  authority.** An `ibridge` Rust binary wraps the crates for
  discovery/pair/install; packs, report validation, receipts and evidence stay
  in AppleBridge (invoked as CLI/library). Smallest blast radius; GUI stays
  untouched for manual use.
- **Option B: AppleBridge orchestrates, calling an `ibridge-host` helper.**
  Python-first; the helper is a narrow Rust tool. More moving parts, same
  invariants.

Neither option modifies working AppleBridge code (execution discipline).

## Device identity (minimal, privacy-preserving — Phase 3 decision)

Receipts must answer "which physical device?" without becoming a person
tracker. Proposal: host stores `device_alias = sha256(salt || UDID)[:16]` plus
`ios_version` and `model_class` (e.g. iPhone13,2) — stable, non-personal,
sufficient for experiment identity. The Doctor already refuses to collect
UDID/serial. Final salt/alias scheme: Phase 3.

## Receipt questions (contract goal, Phase 3)

which host · which device (alias) · which runtime/iOS · which capability ·
which input/pack (sha256) · which execution · which result (raw) · when ·
PASS/FAIL/NOT_DEMONSTRATED/REQUIRES · where the evidence lives.

Partial evidence never becomes PASS.

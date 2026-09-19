# Integration Plan (Phases 3–9)

**STATUS: EXECUTED 2026-09-19 — see `docs/IBRIDGE_MVP_EVIDENCE.md` for the
evidence and `docs/MULTI_DEVICE_FUTURE.md` for the Phase 9 design.**
The plan below is the pre-execution draft, kept for provenance.

## Phase 3 — minimum iBridge contract (proposed, smallest)

MVP vertical slice, one physical iPhone (the proven iPhone13,2):

```
ibridge discover          → device list (usbmuxd; identity via DeviceInfo)
ibridge allow <operation>  → explicit authority grant (deny stays representable)
ibridge install-doctor     → sign+install AppleBridgeProbe via isideload (Option A)
                            (reuse of Danny's linked Apple session semantics —
                             account handling stays explicit)
[device] open app → Run Doctor → Send (as today; unchanged)
ibridge receipt            → binds: device alias + iOS version + pack sha256
                             + report sha256 + operation + verdict + evidence path
```

Protocol authority stays in AppleBridge: `probe-pack.v1` emission,
`device.report.v1` validation, cross-check, evidence packages, redaction.
The iBridge addition is exactly two things: (1) device identity/discovery/
install as explicit operations, (2) receipts that bind device + operation +
evidence.

## Phase 5 — first real vertical slice (acceptance)

Same 10-step chain as the COMPOSE, on the existing proven hardware. The
Doctor app and its 8 probes remain unchanged unless integration requires a
minimal change (e.g., app auto-run trigger is explicitly NOT added in MVP).

## Phase 6 — verification matrix (13 cases, to execute)

1. no device attached → deny/no-op, no crash
2. one recognized iPhone attached → discovery lists it
3. discovery repeated → stable identity
4. identity/runtime detection → iOS version matches device
5. transport establishment (USB; wireless optional/NOT_DEMONSTRATED)
6. malformed pack → rejected, no partial evidence
7. valid Doctor pack → decoded
8. result roundtrip → report stored, schema valid
9. report persisted on host (sha256 chain)
10. repeated execution → append-only evidence, no overwrites
11. disconnect during operation → observed as timeout/error, no fabricated
    verdict (TIMEOUT = observation; cause needs independent evidence)
12. reconnect → recovery path
13. denied/unavailable operation → Deny representable, exit code, receipt

## Phase 7 — regression gates

- AppleBridge suite: 116/116 unchanged (VERIFIED at fork time)
- applebridge-probe build+tests: run 35404585316 green (rerun on integration)
- iLoader-derived capabilities: exercised via the ibridge host; upstream GUI
  untouched (its own workflows not run on Windows — NOT_DEMONSTRATED)

## Phase 9 — future multi-device surface (design only, after MVP)

`ibridge devices` table (alias · iOS · arch · capabilities · status) and
intent→capability-routing→selected-physical-device→execute→receipt. Do not
build a five-iPhone distributed system before one iPhone has a clean,
reproducible contract.

## Non-goals (explicit)

No Mac-free-Apple-development claims, no arbitrary device execution, no
credential storage outside keyring, no stealth/persistence features, no
security-control circumvention, no wholesale iLoader absorption, no
unrelated ISyCo/OpenISy WIP repairs.

## Smallest Phase 3 action (on approval)

Decide Option A vs B (recommend A), then implement `ibridge discover` alone:
list usbmuxd devices, print DeviceInfo + alias, write a discovery receipt —
no install, no pairing changes yet.

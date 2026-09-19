# AppleBridge Capability Map (as inspected on disk, 2026-09-18)

LOCAL OBSERVATION (VERIFIED by execution unless noted). AppleBridge lives at
`C:\Development\ISyCo Git\AppleBridge` (no VCS on that folder — noted as a
provenance gap); its iOS app repo is
https://github.com/DannyBaanks/applebridge-probe (git, clean).

## Host (Python, zero-dep)

| component | file(s) | what it owns | state | iBridge role |
|---|---|---|---|---|
| contracts | `contracts.py` | capability/status grammar, exit codes, request hashing | 116/116 tests green (rerun at fork time) | **protocol authority** — REUSE unchanged |
| schemas | `schemas/{request,result,device.report,probe-pack}.v1.json` + zero-dep validator | frozen v1 contracts | freeze-tested (packaged==repo copies) | REUSE unchanged |
| providers | `providers/{base,mock,github_macos}.py` | Apple backend seam (capability != implementation) | github_macos VERIFIED (real runs) | REUSE; iBridge adds `provider.iloader_host` semantics alongside, never replacing |
| receiver | `receiver.py` + `applebridge receive` | LAN HTTP: GET probe-pack / POST device-report; schema validation; cross-check; evidence packages | VERIFIED_ON_WINDOWS (localhost roundtrip tests) | **transport + evidence store** — REUSE |
| device contract | `device_report.py` + `applebridge device-pack` | probe pack emission, summary recompute, cross_check rules | VERIFIED_ON_WINDOWS | REUSE unchanged |
| receipts/evidence | `receipts.py`, `device-evidence/`, `receipts/` | append-only receipts with sha256 | VERIFIED (real receipts on disk) | REUSE; iBridge receipts extend with device identity (Phase 3) |
| redaction | `redact.py` | secrets never stored | VERIFIED (tests) | REUSE |

## Device side (Swift, applebridge-probe repo @ 5a9fbdc)

| component | state | iBridge role |
|---|---|---|
| AppleBridgeProbe app + Doctor (8 probes) | built (run 35404585316, BUILD PASS), installed on Danny's iPhone13,2 via iLoader | **the device-observed verification** — unchanged in MVP |
| device.report.v1 emission | 7 VERIFIED_ON_IOS_DEVICE probes (report `8cd57985…`, cross-check CLEAN, pack sha `f885ee35…` Swift==Python byte-identical) | REUSE unchanged |
| transport app→host | HTTP over LAN (works) | REUSE; iBridge may add USB-triggered alternatives later — UNDECIDED |

## Evidence state at fork time (VERIFIED)

- OpencodeNative provider run `35318320518`: build+test PASS (Xcode 26.6,
  macos-26-arm64), receipts + artifact sha256 on disk.
- Probe app run `35404585316`: tests 9/9 PASS + unsigned IPA
  (sha256 `3055a18a…` as installed on device).
- **Real iPhone report `20260918T235012Z-8cd57985`**: iPhone13,2 · iOS 18.7.8,
  7 PASS / 0 FAIL / 1 NOT_DEMONSTRATED (`lifecycle.background`), verdict
  PARTIAL, total suite 147.57 ms — the first end-to-end vertical slice
  evidence, pre-iBridge.

## Provenance gap (recorded, not silently repaired)

`C:\Development\ISyCo Git\AppleBridge` is **not a git repository** — it cannot
be forked/linked from iBridge code. Phase 3 decision needed: (a) init git
history there, (b) publish to DannyBaanks, or (c) treat as vendored artifact.
No action taken (execution discipline: no unrelated repairs).

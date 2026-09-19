# UPSTREAM PROVENANCE — iBridge

## Fork record

| field | value |
|---|---|
| upstream project | iloader — "User friendly sideloader" |
| upstream URL | https://github.com/nab138/iloader |
| upstream owner | nab138 |
| fork (ours) | https://github.com/DannyBaanks/iBridge (renamed from `DannyBaanks/iloader`; GitHub redirect active) |
| baseline commit SHA | `348eefd7de78e9bc612c9d619b8b1e7a80ba3ba0` ("Update isideload & bump version") |
| fork date | 2026-09-18 |
| local path | `C:\Development\ISyCo Git\iBridge` |
| remotes | `origin` → DannyBaanks/iBridge · `upstream` → nab138/iloader |
| local integration branch | `ibridge/integration` (branch of baseline; `main` mirrors upstream) |
| upstream state at fork | default branch `main`, not archived, ~3.4k stars, last push 2026-09-10 |

Verification (executed 2026-09-18): `git log --oneline -3` shows upstream
history intact; `git rev-parse HEAD` == baseline SHA above; `gh repo view
DannyBaanks/iBridge` → `isFork: true`.

## Licensing (UPSTREAM FACT — read from the repository)

- **Source code: MIT License** (Copyright (c) 2025 nab138, `LICENSE`).
  Forking, modification, and redistribution are permitted with the copyright
  notice preserved.
- **Branding: separate restrictions** (`LICENSE-BRANDING`, all rights reserved
  2026 nab138): logos, icons, graphical assets, and the name "iloader" are NOT
  MIT-licensed. Forks MAY retain branding materials provided they (a) do not
  imply official affiliation/endorsement and (b) include a clear link to
  https://iloader.app or https://github.com/nab138/iloader.

### iBridge compliance decisions

1. The fork/repository is **renamed to iBridge** — our product does not use
   the "iloader" wordmark.
2. Upstream branding assets are **left untouched**.
3. This project is a derivative of [iloader](https://github.com/nab138/iloader)
   by nab138 — see also https://iloader.app. iBridge is NOT an official
   iloader release and is not endorsed by nab138.
4. MIT copyright notices retained; our modifications are documented below.

## Disambiguation (provenance caution)

Two unrelated things are called "iloader":

1. **nab138/iloader** (this fork's upstream) — Tauri 2 GUI sideloader in Rust,
   built on the `idevice` (pure-Rust libimobiledevice equivalent) and
   `isideload` crates.
2. A PyPI package `iloader` referenced by
   `OpencodeNative/.github/workflows/ios-build.yml` (`pip install --user
   iloader` in the optional, gated `sign` job). **PyPI returns 404 for
   `iloader` today (verified 2026-09-18)**, so that workflow path appears
   unexercised/stale. It is NOT this upstream. Not investigated further —
   OpencodeNative belongs to another work stream.

## Modifications made by iBridge (this branch, Phase 0–2)

- `docs/UPSTREAM_PROVENANCE.md` (this file)
- `docs/ILOADER_CAPABILITY_MAP.md`
- `docs/APPLEBRIDGE_CAPABILITY_MAP.md`
- `docs/IBRIDGE_ARCHITECTURE.md`
- `docs/INTEGRATION_PLAN.md`
- No source, asset, branding, or build changes in Phases 0–2.

## Components

- **Retained (planned reuse, see capability maps)**: device
  discovery/identity/pairing, USB + wireless transports, sideload/install
  machinery (`idevice` + `isideload` crates), logging, secure storage.
- **Replaced (conceptually, not yet)**: none yet — Phase 3+ may add an
  `ibridge` host entry point alongside the GUI.
- **Not used**: auto-updater plugin, release/download-count workflows,
  marketing website assets.

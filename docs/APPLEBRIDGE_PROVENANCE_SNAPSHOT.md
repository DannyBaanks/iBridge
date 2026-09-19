# AppleBridge provenance snapshot (no-VCS gap)

Discovery (Phase 0–2) found that
`C:\Development\ISyCo Git\AppleBridge` — the protocol/evidence authority — is
**not under version control**. Per the MVP compose, we did NOT casually
init/move/rewrite it. Instead, this exact material is fingerprinted.

- **Snapshot file**: `evidence/applebridge_provenance_snapshot.json`
- **Snapshot taken**: 2026-09-19T06:35:00Z
- **Source path**: `C:\Development\ISyCo Git\AppleBridge`
- **Files hashed**: 135 (source, schemas, docs, examples; excluding caches,
  artifacts/, receipts/, device-evidence/)
- **Tree sha256** (over the sorted path+hash list):
  `3abe4b37c4107058349d5eff6e651ddc383a8d449839d22813a4b3779500970e`

Any future change to the consumed AppleBridge material can be detected by
re-hashing and diffing against the snapshot. The device evidence referenced by
iBridge receipts (e.g. report `8cd57985…` stored under
`AppleBridge/device-evidence/20260918T235012Z-8cd57985/`) carries its own
per-file sha256 in the AppleBridge manifest.

**Recommended future decision (separate, not blocking):** publish AppleBridge
to DannyBaanks/<repo> or init local git, so provenance becomes first-class.

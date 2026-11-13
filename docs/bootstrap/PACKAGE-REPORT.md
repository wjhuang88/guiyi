# Historical bootstrap package report (July 28, 2026)

> **Archive-only, not current authoritative package metadata.** This file preserves the existence and intentions of the original 2026-07-28 alpha bootstrap snapshot. The root `PACKAGE-MANIFEST.json` from that date was **retired in ENG-027** because its 123-file inventory, sizes and hashes no longer matched a real checkout. Do not use it as release evidence.

## Historical snapshot

- Package name: GUIYI Engine, original `0.1.0-alpha.1` bootstrap.
- Original reported Bevy baseline: `0.18.x` (**historical only; current workspace uses Bevy `0.19`**).
- Dual licensing: `MIT OR Apache-2.0`.
- Bootstrap goal: stable IDs, typed Commands, semantic Queries, transactions/audit, deterministic build, headless Bevy ECS preview, tactical-RPG toolkit, protocol/CLI/Workbench and CI documentation.
- At bootstrap, the generated report stated no real Cargo resolution had been completed. **This is no longer true**: the current repository contains `Cargo.lock`, runs Rust `1.99.1` via the pinned toolchain and green GitHub Actions, and integrates Talos through the published `talos-runtime = 0.10.1` SDK.

## Authoritative current sources

- `Cargo.toml` and **committed `Cargo.lock`** for workspace, Bevy/Talos and dependency resolution.
- `rust-toolchain.toml` for the exact Rust version; `.github/workflows/ci.yml` for immutable Action pins and full quality gates.
- `docs/backlog/PRODUCT-BACKLOG.md` and `docs/validation/` for Story implementation/acceptance truth.
- Tracked Git tree for source inventory. A **real tagged release** must freshly produce its own artifact inventory and per-file hashes; do not restore a static repository-root list of unverified hashes.

See [release SOP](../sop/RELEASE.md) and [first-run](FIRST-RUN.md) for authoritative current procedures.

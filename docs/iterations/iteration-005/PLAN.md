# Iteration 005 — ENG-027 deterministic build and supply-chain hygiene

## Completion evidence (2026-11-14)

- **Governance:** documentation-only [Ready PR #78](https://github.com/wjhuang88/guiyi/pull/78) passed full exact-head CI 373 and squash-merged to `50de18d13186e61fbdda1620fab73f59049cdb96` before implementation.
- **Implementation:** [ENG-027 Draft PR #79](https://github.com/wjhuang88/guiyi/pull/79) fixes metadata, pins Rust 1.99.1 and all third-party Actions to upstream verified commit SHAs, enforces locked Cargo, installs the complete Apache-2.0 terms, retires the stale July-2026 manifest, and verifies fail-closed static regressions; the existing Cargo.lock/Dependabot stay preserved.
- **Code-run evidence:** complete pinned CI run **375** (`43465687598`) was green, including both jobs and three examples; a later checker refinement adds direct `uses:` reusable workflow pin enforcement. [The ENG-027 validation record](../../validation/ENG-027.md) details provenance, negative tests, compatibility and rollback; final exact-head success before squash merge is recorded in PR #79 and Issue #17.
- **No behavior/migration:** purely build/meta/license/docs maintenance, no engine content/schema migration or Talos SDK changes. ENG-030 real-model human acceptance is still separate and NOT RUN.

This completion record supersedes the prospective Ready stage below, which is retained as historical approved implementation sequencing.

## Day 0: scope and ownership

**Primary Story:** ENG-027 / [Issue #17](https://github.com/wjhuang88/guiyi/issues/17). This **docs-only** branch proposes Ready authorization; no production/CI implementation starts before this PR passes full CI and exact-head squash-merges to main. Implementation is a separate branch from the new authorized main.

**Owner:** GUIYI platform repository build/release governance. **Product contribution:** remove a P1 reproducibility and supply-chain audit gap for the Agent-first Talos→GUIYI semantic Tool workflow. This work may run in parallel while ENG-030's genuine external-model acceptance remains a distinct **HUMAN REQUIRED / NOT RUN** gate, and must not substitute for or bypass it.

**Baseline/contract:** [ENG-027 Ready contract](../../validation/ENG-027-READY-CONTRACT.md) resolves already-present `Cargo.lock` and Dependabot versus remaining floated build tools, action refs, repository metadata, license and legacy package manifest.

## Implementation plan (post-Ready only)

1. Begin with the working `Cargo.lock`; select `rustc 1.99.1` (version observed from green GitHub CI) as a reproducible pin in `rust-toolchain.toml` and CI installer, and verify current dependency/toolchain compatibility by `cargo metadata --locked` and full gates. Do **not** edit Bevy/Talos dependency versions.
2. Pin all GitHub Actions using 40-character **commit** SHAs from upstream refs (annotated tags must be peeled to commit), with readable tag/version comments. Retain Dependabot monthly Actions and weekly Cargo upgrade cadence, review upgrades instead of going back to floating tags.
3. Switch applicable cargo check/clippy/test/doc/run commands in CI and `scripts/first_run.sh` to `--locked`, retaining exact existing gates; add a stdlib-only fail-closed `scripts/check_reproducibility.py` and explicit negative self-tests to verify it catches intentionally floating refs, stale package manifests and truncated licenses.
4. Replace abbreviated Apache-2.0 notice with full standard license, preserve MIT text and `MIT OR Apache-2.0` header. Fix workspace repository URL. Delete the stale root `PACKAGE-MANIFEST.json` and mark July 2026 bootstrap report as historical; future *release* archives require newly generated inventories, not unmaintained current-root snapshots.
5. Document the exact SHA provenance and accepted tradeoffs (pin update cadence, lock enforcement, no offline guarantee), run full CI on the final HEAD, record validation/rollback. Merge and close only when complete.

## Quality/compatibility stop conditions

No third-party runtime dependencies, security scanners or `cargo-deny` without a separate Story. No new Talos/provider/planner, Command/Query/diagnostic contract, game content model or UI source of truth. If the pinned toolchain/Action breaks, keep PR unmerged and pick a reviewed working immutable ref with documented provenance, never a floating fallback. **Serialized migration:** none. **Rollback:** use approved pin upgrades, revert obsolete test scripts only if proven wrong, keep full license and locked builds.

**Gate:** fmt, workspace check/strict clippy/tests/docs, four baseline Python repository gates plus new reproducibility gate, `cargo metadata --locked --format-version 1`, and three current mainline executable examples. Full GitHub Actions must run the pinned third-party Actions successfully.

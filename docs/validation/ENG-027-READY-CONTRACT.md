# ENG-027 — Reproducible checkout, licensing and CI supply-chain contract

**Issue:** [#17](https://github.com/wjhuang88/guiyi/issues/17) · **P1** · **Proposed → Ready authorization proposal only** (docs-only).  
**Baseline:** main `8990d7eff4fa75cf94779232a2949e8cba96f011`, audited 2026-11-14.  
**Owner:** GUIYI engine platform build/repository governance; no changes to Talos/provider or game semantics.

## Why this is Ready-sized now

This bounded maintenance slice protects deterministic, agent-driven build/validation without changing the Gate-4 Talos product proof. The mainline CI already runs Rust checks, tests and three examples; drift in the toolchain, third-party Actions or stale package files makes it harder to reproduce and audit. Work may proceed in parallel with ENG-030's *separate real-model human acceptance*, but not replace it.

| Proposed deficiency | Baseline evidence | Authorized remediation |
| --- | --- | --- |
| Repo URL placeholder | `Cargo.toml` workspace `repository = "https://example.invalid/guiyi-engine"` | Change to `https://github.com/wjhuang88/guiyi`. No package identity/version change |
| Floating toolchain | `rust-toolchain.toml` `channel = "stable"`; GitHub CI run 367 logged `rustc 1.99.1` (2026-10-13) on main-compatible toolset | Pin `1.99.1` in `rust-toolchain.toml`; explicitly configure the Rust CI action to install the same exact version. Reassess in reviewed upgrades after dependency/MSRV checks |
| Missing lockfile | **Already satisfied:** main contains `Cargo.lock` | Do not recreate or mutate it unnecessarily; CI and local `scripts/first_run.sh` use `cargo metadata --locked` and applicable `cargo ... --locked` |
| Floating GitHub Actions | CI currently uses `checkout@v7`, `setup-python@v7`, `rust-cache@v2`, `rust-toolchain@stable` | Resolve each **official tag/branch object** to an immutable 40-character **commit** SHA (peel annotated tags), pin it with a readable `# vN or branch revision` comment, check the identity in review and run both CI jobs with the pinned refs |
| Apache license notice only | `LICENSE-APACHE` is 691-byte abbreviated notice; `LICENSE-MIT` is complete | Replace Apache notice with the complete standard Apache License Version 2.0 terms (sections 1–9, end statement and appendix), sourced from Apache's public reference text; retain full MIT license and `MIT OR Apache-2.0` notices. Do not change license grant/ownership |
| Stale `PACKAGE-MANIFEST.json` | Legacy dated 2026-07-28, only 123 files; main has more, plus `AGENTS.md` | **Retire/remove the obsolete root snapshot** rather than invent an unmaintained second file inventory. Explain that current repository truth is tracked Git files/Cargo metadata and release archives require fresh release-specific manifests. Document historic snapshot as historical, not authoritative |
| Outdated bootstrap report | `docs/bootstrap/PACKAGE-REPORT.md` still says Bevy 0.18 and lockfile missing; current Cargo uses Bevy 0.19 and lockfile is present | Correct provenance, Bevy baseline and historical status; update docs references to avoid implying a current release package exists |
| Dependabot absent | **Already satisfied:** `.github/dependabot.yml` exists: Cargo weekly / GitHub Actions monthly | Preserve config and document controlled review/upgrade cadence; do not duplicate Dependabot |

## Full scope (no other features)

1. Pin toolchain, CI Actions, repository metadata and the *existing* Cargo lock-based workflow without changing dependency features/versions or Talos Runtime SDK.
2. Add a lightweight **Python standard-library static repository gate**, checking 40-hex Action pin syntax, toolchain version alignment with CI, `Cargo.lock` presence, workspace URL, full license markers and prohibition of the retired manifest; run it in CI and `first_run.sh`. This is a deterministic hygiene check **not** a vulnerability scanner nor a signature verifier.
3. Use `--locked` for applicable cargo subcommands in CI and the local first-run script; run `cargo metadata --locked --format-version 1` as a cold-checkout smoke gate. Keep formatter arguments valid.
4. Replace license with the public, unmodified Apache-2.0 terms. Preserve MIT license, maintain any appropriate copyright notices outside the standard license text in repo source/docs.
5. Remove stale `PACKAGE-MANIFEST.json` and document that package-file hashes are generated only for an actual tagged release, not stored as a permanently outdated pseudo-authority. Correct bootstrap report and release SOP; clarify pin upgrade process.
6. Validate pinned Action SHAs against upstream ref objects; run **full GitHub Actions** and all three mainline executable examples at the final PR head. Include the exact resolved SHAs and CI run in the final validation record.

## Acceptance / negative paths

- [ ] Root metadata is the GitHub repo URL; `rust-toolchain.toml` and Rust CI install the **same concrete version**; `cargo metadata --locked` and all applicable CI Cargo gates run without rewriting the committed lockfile.
- [ ] CI references **only commit-addressed** third-party Actions; refs correspond to upstream commit objects, not arbitrary unverified values; pinned workflows execute green.
- [ ] Full standard MIT/Apache dual-license texts are present; the source of Apache canonical terms is noted.
- [ ] No stale `PACKAGE-MANIFEST.json` remains; references and bootstrap report explicitly distinguish historical records from current authoritative package state.
- [ ] Static reproducibility gate **fails closed** on an injected floating Action ref, floating toolchain or license truncation, and passes normal current state; no false network-based trust claim.
- [ ] `docs/validation/ENG-027.md` records exact upstream pin values and verified tag-to-commit resolutions, Rust version, locked commands, negative test evidence, full exact-head CI and three examples, migration (none) and rollback.
- [ ] An offline `cargo --offline` success is **not asserted** without a warm cache; reproducibility means locked resolution and pinned toolchain/actions, not guaranteed offline dependencies.

## Boundaries / dependencies

**Primary owner:** GUIYI platform repository maintenance and CI. The current north-star needs deterministic builds and auditable dependency upgrades; no engine semantic Commands, Queries, diagnostics, transactions, Tool schema, game schema or Bevy runtime logic changes. Keep Talos SDK dependency `=0.10.1` and Bevy `0.19` as currently locked; do not add duplicate model runtime or another Editor truth.

**Dependencies:** completed ENG-019, accepted ADR-0016, `AGENTS.md`, `docs/sop/DIRECTION-GOVERNANCE.md`, `docs/sop/DEVELOPMENT.md`, `docs/sop/QUALITY-GATES.md`, `docs/sop/RELEASE.md`. ENG-030's real-provider operator verification remains an explicit separate **HUMAN REQUIRED / BLOCKED** gate, not an ENG-027 dependency.

**Non-goals:** no `cargo-deny`/security advisory CI or new third-party dependencies without a new Story; no Rust/Bevy/Talos feature-version update; no deployment/release archive creation, publishing, source-of-truth duplication or compatibility migration. Do not invent package hashes.

**Failure/rollback:** if a pinned upstream Action or Rust version breaks the workflow, do not merge. Resolve to a different reviewed *immutable SHA/explicit Rust version*, record provenance and rerun full CI; never silently revert to floating tags. If package consumers actually depend on the removed stale file, file a separate compatibility/release Story rather than reinstating an unverified manifest. All changes are build/meta/doc-only; no serialized data migration.

## Implementation sequence (only after docs-only Ready PR exact-head CI and squash merge)

1. Create a separate `feature/ENG-027-*` branch from **newly authorized main**.
2. Implement source-of-truth, lock, actions and license corrections; add failing/working static gate probes and make CI run them.
3. Update release/docs and `docs/validation/ENG-027.md`; prove all local and GitHub CI gates on the final exact PR head.
4. Squash merge only if green; close Issue #17, mark Done, preserve separate ENG-030 human acceptance.

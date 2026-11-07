# Iteration 004 — Parallel P1 test-contract evidence (ENG-019)

## Completion record (2026-11-07)

ENG-019 / Issue #15 is **Done and closed**. Proposed→Ready docs-only PR #75 passed full CI 358 and squash-merged to `fee20353bc290140561715b840afed1477351848`. The separate test-only implementation PR #76 delivered six residual tests (CLI process 2, Preview process 2, AgentHost driver failure 1, Query missing/nonmutation 1), plus [the full per-category coverage report](../../validation/ENG-019.md). Final complete CI **run 369** (`42366056830`) passed on exact PR head `483a1ae940f4642a24cc3a190bd66adbb89ae8f1`, followed by exact-head squash merge `6b0648ccbbe880e0490d7dece0f7d0a3f7c216eb`. The eight categorized source groups contain 267 existing + 6 new = 273 Rust test attributes.

The implementation changed tests and documentation only, with no production API/schema/migration, Tool IDs, SDK/provider responsibilities or new dependencies. ENG-030 real-model human acceptance remains **required and unperformed**; ENG-027 remains Proposed. This completion record is documentation-only and supersedes prospective Ready wording below.

## Scope and ownership

**Primary Story:** ENG-019 / [Issue #15](https://github.com/wjhuang88/guiyi/issues/15) — Ready promotion **proposed in this docs-only PR**, no executable implementation is authorized until the complete CI passes and this PR merges.

**Primary owner:** GUIYI engine platform tests/CI, scoped to missing executable assertions. **North-star link:** strengthen the Agent-first platform's explicit machine-facing contract while ENG-030 independently awaits real-model operator acceptance.

**Parallel boundary:** This is not a new product direction, a replacement for ENG-030, a second Talos session/provider, or authorization for ENG-024/ENG-027. Do not cherry-pick changes from ENG-030's draft PR #74 or invalidate its human proof. Start an ENG-019 feature branch **from the new main produced by the Ready-promotion squash merge**, not from this docs-only branch.

## Reconciled baseline and why this is a bounded slice

See [ENG-019 coverage baseline](../../validation/ENG-019-COVERAGE-BASELINE.md). Earlier work already covers AgentHost permission/working set/budget, transaction dry-run/atomicity, Workbench JSONL process continuation, runtime integrity/lifecycle and ProjectStorage failpoint/restart; these remain authoritative. Residual high-impact gaps include CLI/Preview subprocess behavior, driver-error transition, missing Query/nonmutation assertions and test-to-contract inventory. The original Issue #15 predates much of this suite and is not permission to duplicate everything.

## Delivery order

1. Finalize the `AGENTS.md §13` matrix mapping each category to existing exact test names and distinguishing subprocess/golden/unit/fault-injection examples from source-only claims. Prove at least one missing category with a new failing test or independent reproducible fixture.
2. Add CLI subprocess tests for valid `capabilities --json`, init/validate and invalid project/compile; assert status, machine JSON, stderr/stdout channels and no partial artifact.
3. Add Preview subprocess tests for compiler-produced valid Stage artifact and missing/corrupted artifact; assert status, structured error code, stdout/stderr, headless operation.
4. Add AgentHost failing driver test and Query missing-data/read-only test only where baseline confirms the assertion is absent. Avoid changing production domain or Talos behavior.
5. Run the complete mainline suite and three examples, then document exact HEAD, category-by-category existing/new test counts, remaining limitations, migrations (none) and rollback. Run CI through GitHub Actions where no local Cargo toolchain is available.

## Gate

`cargo fmt --all -- --check`, `cargo check --workspace --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cargo doc --workspace --no-deps`; `python3 scripts/check_architecture.py`, `scripts/check_docs.py`, `scripts/validate_repository.py`, `scripts/static_rust_check.py`; `cargo run -p guiyi-example-minimal-project`, `cargo run -p guiyi-example-tactical-demo`, `cargo run -p guiyi-example-mock-agent-workflow`.

**Stop condition:** a test discovers a production bug/new semantic requirement. ENG-019 is test-only; record the regression and create a separate owning Story instead of silently changing the engine or weakening the acceptance test. No Story is Done until exact-head CI and category evidence pass.

## ENG-027 intake

ENG-027 / Issue #17 remains **Proposed**. Its scope must first be reconciled against existing committed `Cargo.lock` and Dependabot, then consider repository URL, toolchain pin, full license, pinned GitHub Actions, reproducible package-manifest policy and upgrade cadence. No ENG-027 implementation is authorized here.

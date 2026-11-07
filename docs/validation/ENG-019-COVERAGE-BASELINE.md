# ENG-019 — Reconciled test-coverage baseline and Ready contract

> **Status:** pre-implementation baseline / docs-only promotion proposal, not executable code or Done evidence.  
> **Issue:** [#15](https://github.com/wjhuang88/guiyi/issues/15)  
> **Baseline:** authorized main `1a7800149c66459caf642b3b72748c3f84960760`, observed 2026-11-07.  
> **Scope:** P1 platform test-quality/contract evidence parallel to ENG-030; no changes to that Story's human acceptance or live-provider proof.

## Why this Story remains useful (without repeating completed work)

The 2026-11-07 audit of executable test names and CI on main shows earlier ENG-006..017, ENG-020..023 and ENG-014 already implemented many cases described as missing in the original Issue #15. This matrix is a **source-level inventory**, not a claim that every listed test has been independently rerun during promotion. Existing GitHub Actions CI and previous validation documents are the execution baseline. ENG-019 must add only residual coverage and a reproducible trace from `AGENTS.md §13` to concrete test names.

| Required `AGENTS.md §13` category | Existing, explicitly inspected witness on main | Residual ENG-019 gate |
| --- | --- | --- |
| Core/schema: unit, round trip, invalid input, migration | `engine_content/src/lib.rs` tests unsafe paths / manifest deserialization; `engine_command/src/lib.rs` structural input/defaults; tactical content tests for Encounter/Dialogue/Navigation | Map serialization round-trip, invalid schema, compatibility evidence by owning Story; add focused regressions only for an uncovered mandatory behavior |
| Command: success, validation failure, dry-run, atomicity, diff, permission, audit | `engine_command/src/lib.rs` dry-run, atomicity, structural failure; `engine_agent_host/src/lib.rs` permission; ENG-023 tactical `tools/tests/intent.rs`; Talos integration persistent audit | Enumerate representative tests by clause, verify diff and audit assertions rather than assuming green happy path; add a specific missing assertion where necessary |
| Query: success, missing-data, read-only, structured output | `engine_query/src/lib.rs` graph/index and working-set tests; `engine_agent_host/tests/project_graph_index.rs` incremental graph / restricted workset | Explicitly verify missing-document rejection and non-mutation in direct QueryExecutor tests; ensure public JSON shape assertions |
| Runtime: lifecycle, repeats, invalid content, headless, cleanup | `engine_runtime/src/lib.rs`: wrong type/version/checksum/malformed/duplicate/late-object atomic failure, repeated load/unload; tactical demo example | Retain these tests, don't duplicate. Cover **preview executable process** success/invalid artifact separately (exit code, stdout/stderr, JSON) |
| AgentHost: permissions, budget, working set, driver error, completion | `engine_agent_host/src/lib.rs`: `permission_denial_is_structured_and_recorded`, `working_set_blocks_reads_and_mutations_outside_the_set`, `budget_is_enforced_and_budget_rejection_is_audited`, `driver_can_complete_after_exactly_maximum_actions`, `unknown_tools_are_rejected_recorded_and_do_not_stop_the_session` | Add explicit **driver error → Failed** state/history test without introducing another generic agent/provider loop |
| Content-schema: golden, round trip, strict diagnostics, migration, reference docs | `tests/fixtures/capabilities-command-schemas-v1.json`; `engine_cli/src/main.rs::capability_command_schemas_match_golden_fixture`; tactical content/validation tests; ENCOUNTER/DIALOGUE/STAGE references | Map individual representative coverage; update focused golden only if a contract test reveals an actual mismatch; no content migration in this test-only Story |
| CLI/protocol: exit code, JSON, noninteractive, stdout/stderr, example | `engine_editor/tests/jsonl_workbench.rs`: 4 process tests including multi-line continuation, errors, host-open failure; `engine_cli/src/main.rs`: capability input-schema golden and output-path unit test | **CLI process tests** for `capabilities --json`, `init` / `validate --json` happy/error, invalid project/compile, stdout/stderr and exit status. **Preview process tests** for valid/invalid/absent artifacts. No change to existing protocol |
| Storage/failure: atomic/restart/failpoint/path escape | `engine_content/src/storage.rs`: five failpoint/restart/atomic tempfile tests; `engine_content/src/lib.rs`: path traversal and symlink escape | Existing evidence suffices as baseline; add only a reproducible missing case if a hole is found |

**Coverage status legend:** 'Existing witness' means inspected named test in source, **not** full categorical certification; 'Residual' means required to close Issue #15. The implementation validation report must list category-level test counts (existing and newly added), exact functions/files, and actual executed CI commands/results. Do not count multiple checks of one test as independent tests.

## Ready implementation scope: one focused P1 quality slice

1. **Evidence map:** a complete machine-reviewable matrix (paths, exact test functions, `AGENTS.md §13` category, execution command and coverage status); reconcile existing tests before adding any new test. Distinguish direct unit/integration, subprocess, golden and fixture-only evidence.
2. **CLI process contracts:** run compiled `guiyi-engine-cli` with `CARGO_BIN_EXE_guiyi-engine-cli`; verify machine `--json` validity, stdout-only success payload, stderr-only fatal errors, stable exit codes, noninteractive behavior and invalid project/compile failure with no partial artifact emission. Assert structured JSON fields/diagnostic codes rather than human prose or platform paths.
3. **Preview process contracts:** run compiled `guiyi-engine-preview` with `CARGO_BIN_EXE_guiyi-engine-preview`; provide valid compiled Stage artifact using the existing compiler, then missing/corrupted artifact cases. Assert status/exit/output channels, runtime error codes and headless operation; do not introduce game logic or another preview authority.
4. **AgentHost driver error:** use a bounded failing `AgentLoopDriver` fixture and verify host termination/status/action history; keep driver/model/provider choices in Talos/outer test fixtures.
5. **Query missing/no-mutation:** add direct QueryExecutor regression for unknown document and assert source state unchanged, where existing tests do not already prove it. Preserve working-set / ProjectGraph authority.
6. **Deliver a count-by-category validation record** and keep complete mainline CI/three existing example commands green. No new GUI, SDK/provider client, production runtime path, dependency, CI bypass or product API contract.

## Acceptance and stop conditions

- Each clause in `AGENTS.md §13` has at least one concrete named executable witness or a fully enumerated discovered gap. **All residual clauses in this bounded contract must have passing tests**; a matrix of unchecked TODOs cannot count as Done.
- New CLI and Preview subprocess tests cover success and failure, machine-readable output and stdout/stderr separation, with deterministic fixture content and no external service.
- AgentHost driver-error failure state and QueryExecutor missing-data/non-mutation are tested without changing runtime responsibilities.
- CI executes the new tests via `cargo test --workspace --all-features`, plus fmt/check/strict clippy/docs, all repository Python checks and all three existing example executables. The ENG-030 branch's fourth example is not in authorized main; do not move or cherry-pick it as part of ENG-019.
- The final validation report gives per-category **existing/new count**, test-path/function mapping, fixture ownership, failing/green evidence, exact PR HEAD, compatibility and rollback assessment.
- **No production behavior change expected.** If tests reveal a runtime bug requiring a semantic fix, stop scope expansion and create/refine a separate owning Story; this test-only Story does not authorize new command behavior, protocol changes, Talos SDK changes or stable platform APIs.

## Architectural ownership / dependencies

**Primary owner:** GUIYI engine platform quality/tests; QA assertion code in existing crate-owned test modules and test fixtures. The GUIYI/Talos boundary of [ADR-0016](../decisions/ADR-0016-TALOS-AGENT-RUNTIME.md), typed Commands [ADR-0007](../decisions/ADR-0007-TYPED-COMMANDS.md), transactions [ADR-0008](../decisions/ADR-0008-TRANSACTIONS.md), diagnostics [ADR-0010](../decisions/ADR-0010-DIAGNOSTICS.md), headless [ADR-0013](../decisions/ADR-0013-HEADLESS-FIRST.md), and least privilege [ADR-0012](../decisions/ADR-0012-LEAST-PRIVILEGE.md) continue to govern all tests. No new direct Talos dependencies/SDK/version/features, dependency reversal or content serialization changes. The scoped work closes a mandatory reliability/contract-evidence gap while Gate-4 ENG-030 separately awaits human real-model acceptance.

**Required reads:** `AGENTS.md`, `docs/sop/DIRECTION-GOVERNANCE.md`, `docs/sop/DEVELOPMENT.md`, `docs/sop/ITERATION.md`, `docs/sop/QUALITY-GATES.md`, `docs/reference/COMMAND-QUERY-PROTOCOL.md`, `docs/reference/AGENT-SESSIONS.md`, `docs/reference/PROJECT-STORAGE.md`, `docs/reference/RUNTIME-ARTIFACT-CONTRACT.md` and the ADRs above.

**Migration:** none; tests and baseline matrix are additive. **Rollback:** remove ENG-019-specific tests and coverage inventory without reverting earlier regressions or weakening AGENTS/CI; any red test revealing a real defect must remain documented rather than deleted for a green badge.

## ENG-027 parallel intake

ENG-027 / [Issue #17](https://github.com/wjhuang88/guiyi/issues/17) is **still Proposed**. Verified pre-existing items: `Cargo.lock` is **already committed**; `.github/dependabot.yml` **already exists**; repository metadata is still `https://example.invalid/guiyi-engine`; `rust-toolchain.toml` uses floating `stable`; CI uses `@v7`, `@stable`, `@v2` actions; `LICENSE-APACHE` contains only a notice; `PACKAGE-MANIFEST.json` is an outdated snapshot. ENG-027's future Ready contract must reconcile those mixed statuses first and separate optional license/advisory tooling from required fixes. **No ENG-027 implementation is authorized by this document.**

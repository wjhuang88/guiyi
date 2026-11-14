# ADR-0008: Transactions and dry-run

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Commands apply to cloned state, produce diff, and commit atomically.

## Consequences

Makes agent changes predictable, reviewable, and rollback-ready.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.
## ENG-028 implementation clarification (2026-11-14)

### Forces and rejected alternatives

Agents need to preview and audit the exact *document effect* of an operation without committing a partial invalid state. **Rejected:** mutating live documents before validation, treating a dry-run as a persisted edit, or relying on an unjournaled multi-file write for crash safety. The current clone-and-diff model uses more memory and hashing work per transaction but has simple, testable failure semantics.

### Two distinct implemented boundaries

1. **In-memory Command boundary:** `CommandExecutor::execute_scoped` checks permissions and dry-run rights, normalizes/validates, clones `EngineState`, invokes one handler on the clone, computes before/after Document content hashes and checks actual changed IDs against allowed working-set scope. A dry-run returns a `TransactionReport` with `Previewed` status and does **not** assign the clone back. An apply assigns only the validated clone, reports `Applied`, and records an in-memory `AuditRecord`. A preview report in that audit **is not** a persisted transaction.
2. **Durable ProjectStorage boundary:** `ProjectStorage::commit` validates a `ProjectTransaction` plan, journals `Prepared` → `Applying`, applies non-manifest content first and manifest last, marks `Committed`, then ensures durable audit. `recover` rolls back incomplete `Prepared`/`Applying` journal entries and restores missing audit records for committed/rolled-back entries. The disk journal is **not** automatically populated by every in-memory `CommandExecutor` call; callers must use the authorized persistence integration.

### Nested, concurrent and undo policy (current gaps)

There is **no documented supported nested transaction API or multi-writer serialization/optimistic-concurrency guarantee** in the above public surfaces. Do not infer isolation between concurrent processes or silently merge overlapping edits. A future multi-writer/nested API must first choose a project-owned locking/version-check and conflict policy in a separate authorized Story, with failure and restart tests. The `engine_authoring::AuthoringProject` has local undo/redo state snapshots and a unit test, but **ENG-018 audited, permission/working-set-aware, restart-safe undo/redo remains Proposed**. Neither Command diff hashes nor storage rollback during crash recovery automatically constitute that feature. Never use raw file reversal or historical snapshot overwrite to bypass normal Command authorization.

### Compatibility, complexity and failure modes

Current Command clones the full `EngineState` and compares document content hashes over the two stores; the baseline is at least proportional to loaded project state and requires bounded practical fixtures. Current storage transaction journal/before-images use disk space and I/O proportional to the mutations. **No throughput, size or contention SLO is established.** Transaction-report serialization, documented audit fields and storage recovery states are compatibility contracts; changing them or introducing scalable deltas requires migration/rollback analysis and an approved Story. Command semantic validation failure must leave source unchanged; disk failpoints must recover to previous or committed state by journal stage. Supersede via a reviewed ADR only, preserving audit history.

### Evidence

- Sources: [CommandExecutor](../../crates/engine_command/src/lib.rs), [ProjectStorage journal](../../crates/engine_content/src/storage.rs), [authoring compatibility Undo/Redo](../../crates/engine_authoring/src/lib.rs), [Project Storage reference](../reference/PROJECT-STORAGE.md).
- Tests: `engine_command::tests::dry_run_does_not_mutate_state`, `failed_commands_are_atomic`, `scoped_execution_rejects_actual_cross_document_effects_atomically`, `engine_content::storage::tests::every_precommit_failure_recovers_the_previous_project_state`, `failures_after_commit_marker_preserve_new_state_and_restore_audit`, `successful_transaction_commits_manifest_last_and_survives_restart`; [ENG-011 validation](../validation/ENG-011.md).
- Scope gap: [ENG-018 / Issue #14](https://github.com/wjhuang88/guiyi/issues/14) remains Proposed. Runtime save-migration remains ENG-025 Proposed.

# ADR-0007: Typed commands

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

All authoritative mutations use registered typed commands.

## Consequences

Prevents uncontrolled file edits and centralizes validation and audit.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.
## ENG-028 implementation clarification (2026-11-14)

### Forces and rejected approaches

Agent-first authoring requires discoverable, permission-aware edits with a reviewable transaction result. **Rejected:** authoring through raw JSON file mutations, ad hoc Editor-side state or unregistered Tool handlers. They would bypass typed schema discovery, working-set limits and audit. **Cost:** handlers register stable Tool IDs and versioned input contracts, and editing clients must use Command execution rather than writing engine truth directly.

### Current execution invariants

- A `CommandHandler` registers its descriptor and typed input schema. Under [ADR-0015](ADR-0015-SCHEMA-DRIVEN-COMMAND-VALIDATION.md), structural normalization happens before document-access planning and semantic command validation; invalid input must not consume a transaction.
- `CommandExecutor::execute_scoped` checks required permissions and `DryRun` where relevant, applies the handler to a **cloned `EngineState`**, computes document hash diffs, and rejects mutations outside the caller's allowed set before replacing authoritative in-memory state. A preview returns `Previewed` without applying content; a successful apply returns `Applied` and appends a transaction report to its in-memory audit.
- `AgentHost` is the normal session Tool boundary: it checks session permissions, action budget, working-set planning and result recording. A direct engine command API is lower-level and must not be confused with an authorization-free public Agent Tool. Disk persistence is a **separate** `ProjectStorage` transaction boundary (ADR-0008); a successful in-memory Command alone does not prove durable save.

### Failure, compatibility, scaling and supersession

Unknown Tool IDs, schema failures, semantic diagnostics, missing permission and out-of-scope *actual diffs* must fail with a structured rejection and no unauthorized state replacement. The human message may change, while a published Tool ID/input dialect/diagnostic code needs a reviewed compatibility note and affected golden tests. Command execution currently clones the state and hashes changed documents for diff; no measured high-volume workload bound exists. Batch/orchestrated commands must not skip the established transaction and access checks. If future concurrency, optimized delta execution or alternate authoring surfaces are needed, they require a separately approved Story/ADR and regressions.

### Traceable evidence

- Code: [command executor](../../crates/engine_command/src/lib.rs), [AgentHost](../../crates/engine_agent_host/src/lib.rs), [Command/Query protocol](../reference/COMMAND-QUERY-PROTOCOL.md).
- Tests: `engine_command::tests::dry_run_does_not_mutate_state`, `failed_commands_are_atomic`, `scoped_execution_rejects_actual_cross_document_effects_atomically`, `engine_agent_host::tests::permission_denial_is_structured_and_recorded`; [ENG-010](../validation/ENG-010.md), [ENG-019](../validation/ENG-019.md).
- Related: [ADR-0008](ADR-0008-TRANSACTIONS.md), [ADR-0012](ADR-0012-LEAST-PRIVILEGE.md). Cross-session audited reversal remains ENG-018 Proposed.

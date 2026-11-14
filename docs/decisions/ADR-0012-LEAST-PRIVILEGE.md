# ADR-0012: Least privilege

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Each agent session receives explicit permissions and action budget.

## Consequences

Limits accidental code, process, commit, and publish operations.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.
## ENG-028 implementation clarification (2026-11-14)

### Threats, alternatives and cost

An Agent-visible Tool must not inherit arbitrary filesystem, content-edit or publishing authority merely because it executes locally. **Rejected:** an implicit full-permission “trusted agent”, schema validation as a substitute for authorization, and a working set implemented only by filtering visible Query results while permitting hidden writes. The cost is explicit session scope, budget accounting and a second check against the *actual resulting diff* of a mutation.

### Existing enforcement invariants

- `guiyi-engine-agent-host::AgentHost::execute` is the normal machine Tool entry point; it checks active session and action budget, required permission, and document-access planning before routing Command/Query work. Permission denial is prioritized before detailed schema failure where necessary to avoid leaking unauthorized shape details. Denied calls receive a typed `ToolResult`, contribute to action history and must not silently become successful mutations.
- Work-set restriction applies to both reads and writes: project-scanning Queries see only allowed documents, graph impact hides intermediate hidden nodes, and `CommandExecutor::execute_scoped` compares the **actual changed Document IDs** against the permitted set before applying a cloned state. Declared access plans alone are not a sufficient authorization guarantee.
- Domain execution permissions are GUIYI's authority even when Talos admits a Tool call; Talos owns generic model turn/approval/session decisions under ADR-0016, not GUIYI project access rules. A GUIYI domain action budget is a *scope/abuse control*, not a second model planner.

### Failure, compatibility, scale and supersession

A missing permission, exhausted budget, out-of-working-set read or out-of-set command diff must reject without unauthorized authoring state replacement. The action log may record the attempted call and structured refusal, but it does not imply permission to persist or expose secret document fields. Current code is a bounded single-host execution surface; it does **not** establish authenticated multi-tenant policy or globally synchronized cross-process budgets. Changing permission IDs, denial codes, role inheritance or scope semantics requires a dedicated Story/ADR, threat review, regression proof and migration plan; no implicit `Read`/edit exception may be introduced.

### Evidence

- Source: [session and host](../../crates/engine_agent_host/src/lib.rs), [scoped CommandExecutor](../../crates/engine_command/src/lib.rs), [working-set graph visibility](../../crates/engine_query/src/lib.rs), [Agent Sessions reference](../reference/AGENT-SESSIONS.md).
- Tests: `engine_agent_host::tests::permission_denial_is_structured_and_recorded`, `working_set_blocks_reads_and_mutations_outside_the_set`, `budget_is_enforced_and_budget_rejection_is_audited`, `permission_denial_precedes_schema_validation`, `engine_command::tests::scoped_execution_rejects_actual_cross_document_effects_atomically`, [Talos SDK security integration](../../integrations/talos_runtime/tests/runtime_integration.rs).
- Related: [ADR-0016](ADR-0016-TALOS-AGENT-RUNTIME.md), [ENG-008](../validation/ENG-008.md), [ENG-012](../validation/ENG-012.md).

# ADR-0014: UI is a client

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Human UI uses the same command/query APIs and owns no separate business truth.

## Consequences

Prevents divergence between AI and human workflows.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.
## ENG-028 implementation clarification (2026-11-14)

### Forces and rejected alternatives

GUIYI is **Agent-first**, with typed machine-facing Commands and Queries serving as the primary game-development/authoring surface. Human operators still need to inspect, approve and debug results. **Rejected:** an Editor-exclusive content graph, hidden UI state that silently rewrites project data, or a gesture that has no corresponding semantic Command/Query/diagnostic Tool. Such a design would split truth between Talos-driven authoring and human editing. The cost of a thin client is that visual interactions must be expressed in the same transaction/diff/permission model.

### Current implemented boundary vs Proposed capabilities

- The current `engine_editor` binary is a **JSONL Workbench**, not a full visual content Editor. It routes calls through `AgentHost` and the normal GUIYI Command/Query boundary, including structured denial, action history, working-set limits and validation. User-visible display state is never the authoritative game document.
- A future Human Workbench should be a *secondary* client: visualize Stage/Project Graph/diagnostics, review diffs and request explicit approval, and invoke the same typed Commands/Queries to apply authorized changes. No GUI-only mutation or local file patch may bypass a transaction, validation or persistence audit.
- **ENG-024 remains Proposed**: spatial viewport, transaction approvals, human rollback and richer UI are not claimed as already implemented. Likewise ENG-018's audited cross-session Undo/Redo and ENG-025's schema migration framework remain Proposed. The external real-model ENG-030 acceptance gate is independent of visual design.

### Failure, compatibility, scale and supersession

A human rejection, permission denial, stale document view, validation failure or conflict must not silently commit a local UI draft as project truth. A client may cache/render derived views, but invalidation must follow authoritative Document/ProjectGraph changes; no persisted second graph may be introduced. The existing JSONL client proves process-level recovery, not a production multi-user review queue, authenticated UI session or viewport performance ceiling. Future human UI Tool/approval schemas need versioned public contracts and cross-client parity tests. A change giving UI its own authoring authority requires a new/superseding ADR and explicit data migration, not merely adding an Editor package.

### Evidence

- Source: [Workbench client](../../crates/engine_editor/src/main.rs), [JSONL process tests](../../crates/engine_editor/tests/jsonl_workbench.rs), [AgentHost execution boundary](../../crates/engine_agent_host/src/lib.rs), [Direction governance](../sop/DIRECTION-GOVERNANCE.md).
- Tests: `unknown_tool_returns_one_line_and_following_call_succeeds`, `invalid_input_permission_and_validation_errors_do_not_stop_processing`, `host_level_project_open_failure_exits_nonzero_without_stdout`; `engine_agent_host::tests::working_set_blocks_reads_and_mutations_outside_the_set`.
- Product ownership: [ADR-0016](ADR-0016-TALOS-AGENT-RUNTIME.md) gives Talos generic Agent UX/turn lifecycle and GUIYI domain execution; [ADR-0013](ADR-0013-HEADLESS-FIRST.md) ensures all critical game-development semantics remain callable without any visual UI.

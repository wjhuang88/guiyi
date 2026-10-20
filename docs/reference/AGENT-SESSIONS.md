# Agent session execution boundary

## Status

This document describes the **current alpha GUIYI domain-execution boundary** after ENG-014 integrates Talos as the canonical first-party Agent runtime.

Under ADR-0016, Talos owns generic Agent/session/turn lifecycle. GUIYI retains domain execution security and audit semantics. The existing `AgentSession` / `AgentLoopDriver` API remains available only for alpha direct-client compatibility; the Talos first-party path does not use GUIYI's generic loop orchestration.

Do not extend GUIYI with generic provider, prompt, planning, context, memory, conversation, retry, stopping, or generic Agent lifecycle features.

## First-party Talos execution context

The Talos adapter exposes `GuiyiExecutionContext`, containing only GUIYI-owned state required to authorize and audit a Tool call:

- `session_id`: caller/session identity used by GUIYI command context and durable audit records;
- `permissions`: GUIYI domain `PermissionSet`;
- `working_set`: optional strict project-document access scope.

The adapter converts this narrow context to an internal compatibility `AgentSession` only because `AgentHost::execute` currently requires that alpha type. Talos objective, turn lifecycle, retry, stopping, final response, conversation history, and generic reasoning/action budget do not flow through that compatibility session.

Every Talos `AgentTool` invocation still enters the exact same `AgentHost::execute` path as direct GUIYI clients. Talos permission/approval may decide whether Talos attempts a call, but GUIYI independently enforces its own permissions and access scope after that decision.

## Legacy `AgentSession` compatibility

`AgentSession` remains a public alpha compatibility surface for the JSONL Workbench, embedded tests/examples, and other direct clients. Its current fields include:

- GUIYI-owned compatibility state: session identity, permissions, working set, action/audit records;
- transitional generic state: objective, status, `final_summary`, `AgentBudget`, and `actions_used`.

The transitional generic fields are not part of the first-party Talos runtime contract. No new first-party code should depend on them.

The current direct-client lifecycle still behaves as before: a new `AgentSession` begins in `Ready`, first valid Tool processing moves it to `Running`, and legacy `AgentHost::run` may produce `Completed`, `Stopped`, `BudgetExceeded`, or `Failed` terminal states. Calls made after a legacy terminal state receive `AGENT_SESSION_NOT_ACTIVE`.

This behavior is retained to avoid an unrelated alpha breaking change in ENG-014; it is not a second canonical Agent lifecycle beside Talos.

## `AgentLoopDriver` migration decision

`AgentLoopDriver` and `AgentHost::run` are legacy compatibility APIs with a sunset direction:

- the Talos first-party path does not call them;
- new provider/model/planner/session behavior must not be added to them;
- new first-party Agent features belong in Talos;
- removal or replacement requires a separate explicit migration once direct clients no longer need the compatibility surface.

`AgentHost::execute` remains the reusable authoritative GUIYI execution seam and is intentionally independent of Talos.

## Action budget decision

`AgentBudget.max_actions` currently limits direct legacy-session calls that enter Tool permission, access, and dispatch processing. Accepted, rejected, and failed Tool attempts consume one legacy action consistently; budget-denied attempts are recorded without incrementing `actions_used`.

ENG-014 classifies this as a **generic legacy Agent action budget**, not an independent GUIYI domain-safety invariant. Talos owns first-party reasoning/action stopping. Therefore the Talos adapter sets the internal compatibility session budget to an effectively non-limiting value instead of imposing a second stopping policy.

If GUIYI later needs a true game-domain safety limit, it must be modeled explicitly as domain policy with its own semantics and diagnostics, not smuggled back in as a duplicate generic Agent budget.

## Tool action history and durable audit

Every valid direct `ToolCall` still produces an `AgentActionRecord` containing:

- a monotonic compatibility-session sequence;
- the original call;
- the complete structured result.

This in-memory history remains useful to direct alpha clients but is not the canonical Talos conversation history.

Durable mutation audit remains owned by `ProjectStorage` / `ProjectTransaction`. The Talos persistence adapter commits only successful applied Commands through that existing boundary. Dry-runs do not create storage commits. Talos session/transcript state never substitutes for GUIYI transaction audit.

## Working-set semantics

An empty working set means unrestricted access to the session's loaded project.

A non-empty working set is a strict visibility and mutation boundary:

- every directly required document declared by a command or query must be present;
- project-scanning queries execute against a filtered `DocumentStore` containing only working-set documents;
- project-scanning commands are rejected because their effects cannot be proven bounded before execution;
- command transaction diffs are checked against the working set before state is committed;
- a command that declares an incomplete access plan is still rejected atomically if its real diff touches another document.

This means a caller limited to `stage.a` cannot read, modify, delete, list, or derive project-query results from `stage.b`.

Working-set enforcement is a GUIYI domain-security invariant and remains mandatory when Talos is the caller. ENG-014 integration tests deliberately allow the Talos approval layer to admit calls that GUIYI then rejects, proving the layers are independent.

## Project-scope semantics

A Talos adapter instance is created around one already scoped GUIYI `AgentHost` / project state. Persisted operation uses `ProjectStorage`, whose sandboxed `ProjectPath` boundary remains authoritative. Talos cannot use the Agent runtime to retarget the adapter to another project root or bypass GUIYI path validation.

Project-scope security therefore remains the composition of the selected GUIYI project, `ProjectStorage` sandboxing, Tool access planning, permissions, and working-set enforcement rather than a Talos permission rule.

## Document access plans

Commands and queries implement `document_access` and return a `DocumentAccessPlan`:

- `required`: directly required document IDs;
- `scans_project`: whether the Tool inspects the caller-visible project view.

The default access plan is a conservative project scan. New mutating Tools must provide a bounded plan before they are usable in restricted sessions.

Cross-document Tools declare every relevant document. For example:

- actor placement requires both the Stage and referenced actor definition;
- Stage connection requires both source and destination Stages;
- project reference and impact queries require the target and scan the filtered visible project.

## Structured execution errors

The unified executor returns structured `ToolResult` errors with stable codes, including:

- `AGENT_SESSION_NOT_ACTIVE` for direct legacy-session terminal calls;
- `AGENT_BUDGET_EXCEEDED` for direct legacy-session budget exhaustion;
- `AGENT_PERMISSION_DENIED`;
- `AGENT_WORKING_SET_DENIED`;
- `AGENT_ACCESS_PLAN_INVALID`;
- `AGENT_TOOL_NOT_FOUND`;
- `AGENT_TOOL_FAILED`.

The result preserves the original call ID and includes relevant details such as required permissions, denied document IDs, action usage, and working-set contents.

For Talos, the adapter serializes the complete GUIYI `ToolResult` into the Talos Tool result. GUIYI `Rejected` and `Failed` results become Talos Tool errors rather than runtime-fatal session errors. The Talos runtime may therefore inspect the structured failure and continue with a later legal Tool call.

JSONL parsing and framing codes are specified in [Command and query protocol](COMMAND-QUERY-PROTOCOL.md).

## JSONL Workbench and direct clients

The Workbench continues to create one mutable GUIYI compatibility session for its process and routes every valid decoded Tool call through `AgentHost::execute`.

Supported controls include:

```text
--max-actions <N>
--working-set <DOCUMENT_ID>
```

`--working-set` may be repeated. Omitting it selects unrestricted working-set semantics. `--read-only` changes permissions but does not bypass legacy budget, status, history, or working-set enforcement.

Tool-level results do not terminate the Workbench. Host infrastructure failures, such as project storage or stdin/stdout failure, remain process-level failures and terminate with a non-zero exit code.

The direct Workbench, CLI, and tests do **not** depend on or start `talos-runtime`. Talos is the canonical first-party Agent client, not the sole client of GUIYI's execution boundary.

## ENG-014 invariants

The implemented Talos Runtime SDK integration preserves all of the following:

- GUIYI remains authoritative for domain permissions, working sets, project scope, schema validation, and document access planning;
- every GUIYI Tool still reaches the normal validation/transaction/audit path;
- capability discovery is generated from the live GUIYI `ToolCatalog` rather than a duplicated static list;
- recoverable GUIYI Tool failures remain structured and do not require Talos runtime termination;
- direct Workbench/CLI/test clients remain usable without Talos;
- generic first-party Agent loop/session/turn state belongs to Talos rather than being reimplemented beside it;
- the remaining `AgentSession`/`AgentLoopDriver` overlap is explicitly compatibility-only and has a documented sunset direction;
- compatibility breaks remain explicit, tested, and separately authorized.

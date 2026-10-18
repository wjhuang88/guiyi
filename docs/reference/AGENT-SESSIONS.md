# Agent session execution boundary

## Status

This document describes the **current alpha GUIYI execution boundary** implemented by `engine_agent_host`. Under ADR-0016, Talos owns the first-party generic Agent/session/turn lifecycle going forward. ENG-014 will migrate or narrow the overlapping GUIYI lifecycle surface while preserving the domain security/audit guarantees documented here.

Until ENG-014 lands, the existing APIs remain authoritative current behavior. Do not extend them with generic provider, prompt, planning, context, memory, conversation, retry, or stopping features.

## Purpose

`AgentSession` is currently the mandatory GUIYI security and audit boundary for every Tool call. Embedded test loops, the JSONL Workbench, future clients, and other public callers execute through the same mutable `AgentHost::execute` path.

Clients must not call command or query executors directly when acting through this session-aware public boundary.

The long-term boundary retained by GUIYI is the domain execution/security context: caller identity, permissions, working set/access scope, transaction/audit evidence, and any explicitly domain-specific operation limits. Talos owns the generic first-party Agent loop and conversation/session lifecycle.

## Current session lifecycle

A new current-alpha `AgentSession` begins in `Ready`. Its first valid Tool attempt moves it to `Running`.

Current terminal states are:

- `Completed`: the current embedded loop API returned an explicit completion summary;
- `Stopped`: the current embedded loop API returned an explicit stop reason;
- `BudgetExceeded`: a Tool call was attempted after the current GUIYI action budget was exhausted;
- `Failed`: the current loop driver or unrecoverable session-host operation failed and the loop cannot continue safely.

Ordinary Tool-level outcomes do not make the session terminal. Successful, rejected, and failed `ToolResult` values are recorded and the next call may be processed. This includes unknown Tools, invalid input, permission denial, validation rejection, command/query failure, and other recoverable Tool outcomes.

Calls made after a terminal state receive `AGENT_SESSION_NOT_ACTIVE`. They are retained in action history and do not reactivate the session.

A budget rejection is the exception among structured rejections: it moves the current session to `BudgetExceeded` because no further Tool call is permitted under the current API.

ENG-014 must decide which lifecycle/budget concepts remain meaningful GUIYI domain-safety controls and which become Talos-only concerns. It must not preserve duplicate generic reasoning-loop state solely for backward familiarity.

## Action budget and history

`AgentBudget.max_actions` currently limits calls that enter Tool permission, access, and dispatch processing. Accepted, rejected, and failed Tool attempts all consume one action consistently.

A call rejected because the budget is already exhausted is still recorded for audit, but does not increase `actions_used`.

Because budget enforcement occurs only when a new Tool call is attempted, the current embedded loop may use exactly `max_actions` Tool calls and then return `Complete` or `Stop`.

Every valid `ToolCall` produces one `AgentActionRecord` containing:

- a monotonic session-local sequence;
- the original call;
- the complete structured result.

Protocol lines that cannot deserialize as a valid `ToolCall` produce a structured Workbench result but are not session actions because no valid call entered the session executor.

The auditability of GUIYI Tool attempts is a durable requirement. The specific generic Agent lifecycle representation is not.

## Working-set semantics

An empty working set means unrestricted access to the session's loaded project.

A non-empty working set is a strict visibility and mutation boundary:

- every directly required document declared by a command or query must be present;
- project-scanning queries execute against a filtered `DocumentStore` containing only working-set documents;
- project-scanning commands are rejected because their effects cannot be proven bounded before execution;
- command transaction diffs are checked against the working set before state is committed;
- a command that declares an incomplete access plan is still rejected atomically if its real diff touches another document.

This means a caller limited to `stage.a` cannot read, modify, delete, list, or derive project-query results from `stage.b`.

Working-set enforcement is a GUIYI domain-security invariant and remains mandatory when Talos is the caller.

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

The unified executor currently returns structured `ToolResult` errors with stable codes, including:

- `AGENT_SESSION_NOT_ACTIVE`;
- `AGENT_BUDGET_EXCEEDED`;
- `AGENT_PERMISSION_DENIED`;
- `AGENT_WORKING_SET_DENIED`;
- `AGENT_ACCESS_PLAN_INVALID`;
- `AGENT_TOOL_NOT_FOUND`;
- `AGENT_TOOL_FAILED`.

The result preserves the original call ID and includes relevant details such as required permissions, denied document IDs, action usage, and working-set contents.

Unknown Tools are recoverable rejected results. A caller may correct the Tool ID and continue. Other Tool-level failed results are likewise returned without automatically terminating the Workbench.

ENG-014 may rename or narrow generic `AGENT_*` lifecycle-oriented codes as part of an explicit migration, but permission/access/validation/transaction failures must remain structured and machine-actionable.

JSONL parsing and framing codes are specified in [Command and query protocol](COMMAND-QUERY-PROTOCOL.md).

## JSONL Workbench

The Workbench currently creates one mutable GUIYI session for its process and routes every valid decoded Tool call through `AgentHost::execute`.

Supported controls include:

```text
--max-actions <N>
--working-set <DOCUMENT_ID>
```

`--working-set` may be repeated. Omitting it selects unrestricted working-set semantics. `--read-only` changes permissions but does not bypass budget, status, history, or working-set enforcement.

Tool-level results do not terminate the Workbench. Host infrastructure failures, such as project storage or stdin/stdout failure, remain process-level failures and terminate with a non-zero exit code.

The direct Workbench remains supported after Talos integration because Talos is a first-party client, not the sole client of GUIYI's execution boundary.

## ENG-014 migration invariants

The Talos Runtime SDK integration must preserve all of the following while narrowing duplicate lifecycle behavior:

- GUIYI remains authoritative for domain permissions and working sets;
- every GUIYI Tool still reaches the normal validation/transaction/audit path;
- recoverable GUIYI Tool failures remain structured and do not require Talos runtime termination;
- direct Workbench/CLI/test clients remain usable without Talos;
- generic first-party Agent loop/session/turn state moves to Talos rather than being reimplemented beside it;
- compatibility breaks are explicit, documented, and covered by migration tests.

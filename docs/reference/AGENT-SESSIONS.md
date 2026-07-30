# Agent session execution boundary

## Purpose

`AgentSession` is the mandatory security and audit boundary for every tool call. Embedded agent loops, the JSONL Workbench, future UI clients, and other public clients execute through the same mutable `AgentHost::execute` path.

Clients must not call command or query executors directly when acting on behalf of an agent session.

## Session lifecycle

A new session begins in `Ready`. Its first valid tool attempt moves it to `Running`.

Terminal states are:

- `Completed`: an agent loop returned an explicit completion summary;
- `Stopped`: an agent loop returned an explicit stop reason;
- `BudgetExceeded`: a tool call was attempted after the action budget was exhausted;
- `Failed`: tool lookup, query execution, command execution, serialization, or another tool-level failure returned a failed result.

Calls made after a terminal state receive `AGENT_SESSION_NOT_ACTIVE`. They are retained in action history and do not reactivate the session.

Rejected permission, working-set, access-plan, and validation calls remain non-terminal unless the rejection is caused by an exhausted budget.

## Action budget and history

`AgentBudget.max_actions` limits calls that enter tool permission, access, and dispatch processing. Accepted, rejected, and failed tool attempts all consume one action consistently.

A call rejected because the budget is already exhausted is still recorded for audit, but does not increase `actions_used`.

Because budget enforcement occurs only when a new tool call is attempted, an agent loop may use exactly `max_actions` tool calls and then return `Complete` or `Stop`.

Every valid `ToolCall` produces one `AgentActionRecord` containing:

- a monotonic session-local sequence;
- the original call;
- the complete structured result.

## Working-set semantics

An empty working set means unrestricted access to the session's loaded project.

A non-empty working set is a strict visibility and mutation boundary:

- every directly required document declared by a command or query must be present;
- project-scanning queries execute against a filtered `DocumentStore` containing only working-set documents;
- project-scanning commands are rejected because their effects cannot be proven bounded before execution;
- command transaction diffs are checked against the working set before state is committed;
- a command that declares an incomplete access plan is still rejected atomically if its real diff touches another document.

This means a session limited to `stage.a` cannot read, modify, delete, list, or derive project-query results from `stage.b`.

## Document access plans

Commands and queries implement `document_access` and return a `DocumentAccessPlan`:

- `required`: directly required document IDs;
- `scans_project`: whether the tool inspects the caller-visible project view.

The default access plan is a conservative project scan. New mutating tools must provide a bounded plan before they are usable in restricted sessions.

Cross-document tools declare every relevant document. For example:

- actor placement requires both the Stage and referenced actor definition;
- Stage connection requires both source and destination Stages;
- project reference and impact queries require the target and scan the filtered visible project.

## Structured session errors

The unified executor returns structured `ToolResult` errors with stable codes, including:

- `AGENT_SESSION_NOT_ACTIVE`;
- `AGENT_BUDGET_EXCEEDED`;
- `AGENT_PERMISSION_DENIED`;
- `AGENT_WORKING_SET_DENIED`;
- `AGENT_ACCESS_PLAN_INVALID`;
- `AGENT_TOOL_NOT_FOUND`;
- `AGENT_TOOL_FAILED`.

The result preserves the original call ID and includes relevant details such as required permissions, denied document IDs, action usage, and working-set contents.

## JSONL Workbench

The Workbench creates one mutable session for its process and routes every decoded tool call through `AgentHost::execute`.

Supported session controls include:

```text
--max-actions <N>
--working-set <DOCUMENT_ID>
```

`--working-set` may be repeated. Omitting it selects unrestricted working-set semantics. `--read-only` changes permissions but does not bypass budget, status, history, or working-set enforcement.

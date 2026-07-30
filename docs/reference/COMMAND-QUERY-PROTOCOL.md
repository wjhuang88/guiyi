# Command and query protocol

## JSONL Workbench

Run:

```bash
cargo run -p guiyi-engine-workbench -- --project ./sample_projects/agent_tactical_demo
```

Send one JSON object per non-empty input line. A normal tool call has this shape:

```json
{"id":"1","tool":"stage.summary","input":{"stage_id":"stage.demo"},"dry_run":false}
```

The Workbench writes exactly one compact `ToolResult` JSON object to stdout for every non-empty input line. It does not write banners, logs, diagnostics prose, or human commentary to stdout.

Diagnostic and error details are carried inside the result. Process-level startup and I/O failures are written to stderr and terminate with a non-zero exit code.

## Result framing

A successful result uses `status: "ok"`. Tool calls that are syntactically valid but cannot be accepted use `status: "rejected"`. A tool that entered execution but failed may use `status: "failed"`.

Tool-level rejected or failed results do not terminate the Workbench process. The next input line is processed unless the session has reached a terminal policy state such as `budget_exceeded`.

The result preserves the input `id` as `call_id` whenever the ID can be parsed from a syntactically valid JSON object.

Example unknown-tool exchange:

```jsonl
{"id":"1","tool":"missing.tool","input":{},"dry_run":false}
{"id":"2","tool":"project.documents.list","input":{},"dry_run":false}
```

Expected output contains two lines: the first is a structured rejection with `AGENT_TOOL_NOT_FOUND`; the second is an ordinary successful query result.

## Protocol parse failures

Malformed JSON returns one rejected result with:

```text
PROTOCOL_INVALID_JSONL
```

A syntactically valid JSON value that cannot deserialize as a `ToolCall` returns:

```text
PROTOCOL_INVALID_CALL
```

For `PROTOCOL_INVALID_CALL`, a string-valued top-level `id` is preserved even when another field is invalid. When no usable ID exists, `call_id` is `invalid`.

## Tool error codes

Machine clients use `output.error.code`, not human messages, for recovery logic. Current session and protocol codes include:

- `PROTOCOL_INVALID_JSONL`;
- `PROTOCOL_INVALID_CALL`;
- `AGENT_TOOL_NOT_FOUND`;
- `AGENT_PERMISSION_DENIED`;
- `AGENT_ACCESS_PLAN_INVALID`;
- `AGENT_WORKING_SET_DENIED`;
- `AGENT_BUDGET_EXCEEDED`;
- `COMMAND_VALIDATION_FAILED`;
- `AGENT_TOOL_FAILED`.

Validation failures preserve the complete structured diagnostics array. Permission, access-plan, working-set, unknown-tool, invalid-input, command, and query results remain line-framed and do not inject prose into stdout.

## Process termination boundary

Ordinary tool outcomes are not host failures. Workbench termination is reserved for infrastructure failures, including:

- project root or manifest cannot be opened;
- project persistence fails after a successful mutation;
- stdin cannot be read;
- stdout cannot be written or flushed;
- a result cannot be encoded.

These failures produce a non-zero process exit and a stderr message. They may produce no stdout line because the JSONL host itself is no longer reliable.

## Commands

Commands mutate cloned engine state, produce a document diff, and only replace live state when application succeeds. Dry-run executes the same handler without committing the working state.

Built-in primitive commands:

- `document.create`
- `document.delete`
- `document.set_field`

Tactical high-level commands:

- `stage.create`
- `stage.create_spawn`
- `stage.place_actor`
- `stage.create_trigger`
- `stage.connect`

## Queries

- `project.documents.list`
- `project.document.get`
- `project.references.find`
- `project.impact.analyze`
- `stage.summary`
- `stage.validate`

## Capability discovery

```bash
cargo run -p guiyi-engine-cli -- capabilities --json
```

The result is the preferred source for agent tool metadata. Do not copy the entire tool API into a permanent prompt.

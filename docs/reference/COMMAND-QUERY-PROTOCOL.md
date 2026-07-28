# Command and query protocol

## JSONL workbench

Run:

```bash
cargo run -p guiyi-engine-workbench -- --project ./sample_projects/agent_tactical_demo
```

Send one `ToolCall` JSON object per line:

```json
{"id":"1","tool":"stage.summary","input":{"stage_id":"stage.demo"},"dry_run":false}
```

Receive one `ToolResult` per line.

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

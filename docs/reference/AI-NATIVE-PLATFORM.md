# AI-native platform

## Definition

AI-native means that the engine is designed around machine-consumable capabilities rather than automating mouse clicks in a traditional editor.

## Agent loop contract

```text
Observe → Query → Plan → Dry run → Apply transaction → Validate → Evaluate
                                     ↘ rollback or repair ↗
```

The external loop is connected through the `AgentLoopDriver` trait. Model calls, context compression, planning strategy, retries, and provider credentials remain outside engine core.

## First-class concepts

- Tool catalog with input/output schemas.
- Command and query descriptors.
- Agent session objective and working set.
- Least-privilege permission set.
- Action budget.
- Transaction diff and audit history.
- Structured diagnostics and suggested tools.
- Headless build and preview.

## Safety defaults

Content agents receive `Read`, `Plan`, `DryRun`, `EditContent`, `RunValidation`, `RunBuild`, and `RunPreview`. They do not receive code editing, external process, commit, or publish permissions by default.

## Integration point for another agent-loop project

Implement `AgentLoopDriver` in an adapter crate outside the engine core. The driver receives the session, discoverable catalog, and last result, then returns a tool call, completion, or stop directive.

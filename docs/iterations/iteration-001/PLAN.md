# Iteration 001 plan

## Goal

Create a self-contained AI-native engine alpha repository with one end-to-end tactical Stage path.

## Scope

- Workspace and governance.
- Core IDs, schema, documents, diagnostics, assets, build, and runtime.
- Command/query platform.
- Agent protocol, catalog, host, permissions, and mock loop.
- Tactical Stage authoring, tools, validation, compilation, and runtime.
- CLI, preview, JSONL workbench, examples, scripts, and CI.

## Not in scope

- Full visual editor.
- Encounter, dialogue, quest, inventory, or combat production systems.
- Direct wuxia integration.
- External model/provider calls.
- Dynamic plugins.

## Exit gate

- Static repository gates pass in the generation environment.
- Cargo gates are configured in CI and documented for first local run.
- No foundational crate depends on the tactical toolkit.
- Sample project and protocol calls are included.

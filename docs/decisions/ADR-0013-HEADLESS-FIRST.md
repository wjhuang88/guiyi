# ADR-0013: Headless first

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Validation, compile, query, command, preview, and tests work without a window.

## Consequences

Supports CI and autonomous agents.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.
## ENG-028 implementation clarification (2026-11-14)

### Forces, alternatives and costs

An Agent-first game-development pipeline must be callable from a CI worker or a remote operator without a graphical desktop. **Rejected:** requiring a Bevy window, scene Editor or interactive prompt as the only way to validate, compile, inspect or preview content. That would make continuous regression and Talos Tool execution dependent on a separate GUI. **Cost:** maintain real subprocess contracts, structured results, explicit process termination behavior and minimal headless ECS fixtures in addition to eventual visual inspection.

### Current noninteractive contracts

- `guiyi-engine-cli` supports `init`, `doctor`, `capabilities`, `validate` and `compile`. Machine-facing `--json` output is explicitly available for relevant commands; **successful** structured commands emit JSON on stdout, while fatal CLI-level errors currently use a nonzero exit and human-oriented stderr. Do **not** claim every fatal CLI error is already JSON merely because `--json` was requested.
- `guiyi-engine-preview --artifact … --json` loads and validates an integrity-stamped Stage artifact in a minimal Bevy ECS `World` without opening a window. On a runtime/artifact failure it writes structured JSON to stderr and exits nonzero. `entity_count` is the current World entity-table count, **not** a guaranteed count of only Stage-owned live objects.
- The existing `guiyi-engine-workbench` JSONL process is a headless human/machine client: one Tool request per input line, one structured Tool response per output line, and a recoverable malformed/denied call must not consume the following line or corrupt stdout with human prose. This is not an Editor-only authoring authority.
- GUIYI validators, command/query executors, compilers and runtime fixture tests must remain independently callable without a GUI. Talos provider invocation is a separate runtime concern, not a headless GUIYI prerequisite.

### Failure, versioning, scaling and supersession

Absent/invalid projects, invalid output paths, unsupported or tampered artifacts and denied Tools must produce nonzero or structured failure without persisting unauthorized partial artifacts. Public CLI flags, JSONL call/result fields, diagnostic codes and artifact versions are compatibility surfaces; changes need schema/protocol tests and an approved Story. JSON object key ordering, human wording and incidental ECS allocation cardinality should not become accidental cross-version contracts. CI fixtures establish deterministic headless operation, **not** measured FPS, startup latency, memory/throughput ceilings or the successful use of a genuine external model. Any change that makes a GUI mandatory must explicitly supersede this accepted ADR with migration and automation impacts.

### Evidence

- Source: [CLI](../../crates/engine_cli/src/main.rs), [Preview](../../crates/engine_preview/src/main.rs), [JSONL Workbench](../../crates/engine_editor/src/main.rs), [Runtime](../../crates/engine_runtime/src/lib.rs), [Headless runtime reference](../reference/RUNTIME-ARTIFACT-CONTRACT.md).
- Tests: `cli_subprocess_init_validate_compile_and_capabilities_are_noninteractive`, `cli_subprocess_failures_use_stderr_and_never_emit_partial_artifacts` in [ENG-019 CLI process tests](../../crates/engine_cli/tests/process_cli.rs); `preview_subprocess_accepts_valid_stage_artifact_without_a_window`, `preview_subprocess_refuses_missing_corrupt_and_wrong_type_artifacts` in [Preview process tests](../../crates/engine_preview/tests/process_preview.rs); `invalid_input_permission_and_validation_errors_do_not_stop_processing` in [Workbench JSONL tests](../../crates/engine_editor/tests/jsonl_workbench.rs).
- See [ENG-019 validation](../validation/ENG-019.md), [ADR-0002](ADR-0002-DOCUMENT-ARTIFACT-RUNTIME.md), [ADR-0014](ADR-0014-UI-CLIENT.md). ENG-030's live-model review remains **HUMAN REQUIRED / NOT RUN**.

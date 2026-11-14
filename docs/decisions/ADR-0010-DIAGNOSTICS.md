# ADR-0010: Machine-readable diagnostics

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Diagnostic code, severity, location, and suggested tools are authoritative.

## Consequences

Lets agents repair errors deterministically.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.
## ENG-028 implementation clarification (2026-11-14)

### Forces and rejected alternatives

Agents must decide whether to repair, revalidate or stop by machine result rather than interpreting unstable English wording. **Rejected:** relying exclusively on free-text errors or formatting diagnostics for a terminal. Such output loses stable identity, field paths and severity, and is difficult to regression-test. **Cost:** maintain meaningful code families, deterministic structured fields and compatibility tests when validators evolve.

### Existing invariant and public shape

`guiyi-engine-validation::Diagnostic` currently serializes `code`, `severity` (`info`, `warning`, `error`), `message`, optional `location` (document/object/field path), `suggested_tools: Vec<String>` and `auto_fixable`; `DiagnosticBag` summarizes severities and gates `has_errors()`. An absent optional location or suggested Tool is legitimate; do not require every validation failure to offer an automatic repair.

Structural command input diagnostics owned by [ADR-0015](ADR-0015-SCHEMA-DRIVEN-COMMAND-VALIDATION.md) use codes such as `COMMAND_INPUT_REQUIRED`, `COMMAND_INPUT_TYPE_MISMATCH` and `COMMAND_INPUT_CONSTRAINT_FAILED` with RFC 6901 JSON Pointer `location.field_path`. Domain validators additionally produce codes such as `STAGE_NO_SPAWN_POINT`. Consumers should branch on a **documented code and typed fields**, not the human message string or array position.

### Rejection, compatibility, scale and supersession

A blocking error prevents a successful validation/build unless the owner explicitly documents a nonblocking path. Structured rejections must remain machine-readable through AgentHost/JSONL/Talos Tool adapters; a permission failure must not reveal hidden document content through diagnostics. Changing a public code, severity interpretation, path encoding or Tool suggestion semantics is a compatibility change requiring its owning Story and appropriate golden/schema assertions. Error text may be refined without implying a code migration. Validator cost depends on the content examined; no repository-wide diagnostic volume limit or repair success rate is promised. Superseding the authoritative typed Diagnostic model requires a versioned protocol decision and migration guidance.

### Exact evidence

- Code: [Diagnostic and Bag](../../crates/engine_validation/src/lib.rs), [schema-driven input errors](../../crates/engine_schema/src/lib.rs), [tactical validators](../../toolkits/tactical_rpg/validation/src/lib.rs).
- Tests: `engine_validation::tests::summary_counts_severities`, `engine_command::tests::missing_required_field_produces_stable_diagnostic`, `wrong_type_produces_stable_diagnostic`, [tactical navigation diagnostics](../../toolkits/tactical_rpg/validation/tests/navigation.rs) and [JSONL error continuation](../../crates/engine_editor/tests/jsonl_workbench.rs).
- Related: [ADR-0007](ADR-0007-TYPED-COMMANDS.md), [ADR-0012](ADR-0012-LEAST-PRIVILEGE.md), [Command/Query protocol](../reference/COMMAND-QUERY-PROTOCOL.md).

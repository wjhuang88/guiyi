# ADR-0015: Schema-driven command validation

- Status: Accepted
- Date: 2026-07-31
- Story: ENG-010

## Context

Command input contracts currently have three independent representations:

1. `engine_schema::FieldSchema` and `ObjectSchema` describe schemas, but the free-form `constraints` map is not validated and most constraints are not emitted by `ObjectSchema::to_json_schema()`.
2. Command descriptors hand-author JSON-like `input_schema` values for capability discovery.
3. Command handlers deserialize input with Serde and then perform command-specific validation.

These representations can drift. An agent can therefore receive a capability schema that does not match runtime behavior, and invalid input can fail without a stable field path.

ENG-010 requires one reusable authority for advertised command schemas, structural runtime validation, defaults, constraints, and schema versions.

## Decision

### 1. Own a versioned GUIYI schema dialect

`guiyi-engine-schema` owns the command-input schema model, registration checks, machine-readable rendering, validation, and default normalization.

The public format is a deterministic JSON Schema-compatible subset identified by mandatory `x-schema-version`. It is not advertised as complete JSON Schema conformance.

Version 1 supports:

- value kinds: `any`, `string`, `integer`, `number`, `boolean`, `object`, and `array`;
- required and optional object fields;
- nullability;
- defaults;
- enums;
- numeric minimum and maximum bounds;
- string minimum and maximum lengths;
- array minimum and maximum lengths;
- unique array items;
- array item schemas;
- explicit object additional-property policy.

The concrete Rust representation may evolve from the current `FieldSchema` and `ObjectSchema`, but the serialized format must be deterministic and able to represent nested array items and nested object fields.

### 2. Reject invalid schema definitions during registration

Schema definitions are validated before entering a registry or command catalog.

Unknown unnamespaced keywords, constraints applied to incompatible value kinds, contradictory bounds, invalid defaults, duplicate field names, unsupported schema versions, and malformed nested schemas fail registration with a stable schema-definition error.

Future extensions must either be added to a new dialect version or use an `x-guiyi-*` namespace. Unknown ordinary keywords must never be silently ignored.

### 3. Validate and normalize before all command-specific processing

A command request is structurally validated and defaults are applied before:

1. document-access planning;
2. command-specific semantic validation;
3. command application.

The normalized value, not the raw caller value, is the input to those three stages. This prevents access planning, validation, and application from observing different effective inputs.

Serde deserialization remains an implementation mechanism after structural validation. It is not a second public schema authority.

### 4. Generate advertised command schemas from the runtime authority

Each registered command supplies a typed input schema owned by `guiyi-engine-schema`.

`CommandDescriptor.input_schema` and the agent `ToolDescriptor.input_schema` are generated from that same schema object. Hand-authored command input-schema JSON is not permitted after migration.

Capability output must expose `x-schema-version` for every migrated command input schema. Golden capability tests prevent silent drift.

Output schemas and query input schemas are outside ENG-010 unless a minimal shared refactor is required to keep public types coherent.

### 5. Preserve semantic validation after structural validation

Structural validation owns shape-level concerns such as required fields, kinds, nullability, enums, lengths, and bounds.

Command handlers continue to own domain semantics such as duplicate document IDs, Stage occupancy, reference validity, and cross-document invariants.

Do not duplicate a structural constraint in command-specific validation after migration.

### 6. Return stable diagnostics with JSON Pointer paths

Command structural failures return `Diagnostic` values with stable codes and `location.field_path` expressed as an RFC 6901 JSON Pointer.

The minimum stable code families are:

- `COMMAND_INPUT_REQUIRED`;
- `COMMAND_INPUT_TYPE_MISMATCH`;
- `COMMAND_INPUT_NULL_NOT_ALLOWED`;
- `COMMAND_INPUT_ENUM_MISMATCH`;
- `COMMAND_INPUT_CONSTRAINT_FAILED`;
- `COMMAND_INPUT_ADDITIONAL_PROPERTY`.

The root path is the empty string. Examples include `/stage_id`, `/path/0`, and `/conditions/2/type`.

Schema-definition failures use `SCHEMA_DEFINITION_INVALID` and include the schema type ID and offending keyword or field when available.

### 7. Preserve atomicity and session enforcement

A structural validation failure must occur before state cloning, transaction allocation, mutation, audit append, or persistent storage activity.

AgentSession permission, working-set, budget, and audit enforcement remain mandatory. Schema validation must not introduce an alternate execution path around the existing command executor.

## Compatibility

- Existing valid built-in command calls must remain valid.
- Defaults currently implemented with Serde must be represented in the schema and applied by schema normalization before Serde parsing.
- Additional properties remain allowed by default in dialect version 1 for compatibility; commands may explicitly set `additionalProperties: false`.
- Error text may change, but stable diagnostic codes and JSON Pointer paths become the machine-facing contract.
- A schema-version change requires an explicit compatibility note and golden capability update.

## Consequences

### Positive

- Capability discovery and runtime behavior share one authority.
- Agents receive deterministic field-level repair information.
- Schema drift becomes testable and registration failures are early.
- Built-in and tactical commands can remove duplicated shape checks.

### Costs

- `engine_command` gains a dependency on `engine_schema`.
- Command registration becomes fallible for schema-definition errors.
- Agent-host request preparation must validate before access planning.
- Golden capability fixtures become compatibility artifacts that require intentional updates.

## Rejected alternatives

### Continue hand-authored descriptor JSON plus Serde

Rejected because it preserves multiple authorities and cannot guarantee advertised/runtime parity.

### Adopt a full third-party JSON Schema implementation immediately

Rejected for ENG-010 because the project needs a small, stable, documented subset with engine-specific diagnostics and no unnecessary dependency expansion. A future Story may adopt a standards implementation behind the same versioned boundary.

### Validate only inside each command handler

Rejected because document-access planning happens before command execution in AgentSession and would still observe unvalidated raw input.

## Validation

Acceptance evidence is recorded in `docs/validation/ENG-010.md`. The implementation must include schema round-trip tests, invalid-definition tests, field-path diagnostics, default normalization, built-in and tactical command migration, capability golden output, atomic failure tests, and the full repository gate suite.

# Command schema contract

## Purpose

This document describes the implemented behavior of the GUIYI schema-driven command validation system introduced by ENG-010 and governed by [ADR-0015](../decisions/ADR-0015-SCHEMA-DRIVEN-COMMAND-VALIDATION.md).

The `guiyi-engine-schema` crate is the single authority for:
- command input schema definitions;
- structural validation at runtime;
- default normalization;
- deterministic machine-readable rendering;
- schema-definition error reporting;
- stable diagnostic codes with JSON Pointer field paths.

## Dialect version

The current dialect version is **1**, emitted as:

```json
"x-schema-version": 1
```

Every command input schema rendered through `SchemaNode::to_json_schema()` includes this marker.

## Supported keywords (dialect v1)

### Value kinds

| Kind | JSON Schema `type` | Notes |
|---|---|---|
| `any` | *(omitted)* | Accepts any JSON value |
| `string` | `"string"` | |
| `integer` | `"integer"` | Rejects floating-point numbers |
| `number` | `"number"` | Accepts both integers and floats |
| `boolean` | `"boolean"` | |
| `object` | `"object"` | Supports fields, required, additionalProperties |
| `array` | `"array"` | Supports items, minItems, maxItems, uniqueItems |

### Constraints

| Constraint | Applies to | JSON Schema keyword |
|---|---|---|
| Nullable | All kinds | `type` becomes `[kind, "null"]` |
| Default | All kinds | `default` |
| Enum | All kinds | `enum` |
| Minimum | `integer`, `number` | `minimum` |
| Maximum | `integer`, `number` | `maximum` |
| minLength | `string` | `minLength` |
| maxLength | `string` | `maxLength` |
| minItems | `array` | `minItems` |
| maxItems | `array` | `maxItems` |
| uniqueItems | `array` | `uniqueItems` |
| items | `array` | `items` (recursive schema) |
| fields | `object` | `properties` + `required` |
| additionalProperties | `object` | `additionalProperties` |

## Unknown keyword policy

Unknown unnamespaced keywords are rejected during schema definition validation with `SCHEMA_DEFINITION_INVALID`.

Future extensions must either:
1. Be added in a new dialect version, or
2. Use the `x-guiyi-*` namespace.

## Rendering contract

`SchemaNode::to_json_schema()` produces deterministic JSON with keys in struct declaration order:
1. `type` (or omitted for `any`)
2. `description`
3. `nullable` (expressed as type array)
4. `default`
5. `enum`
6. Numeric bounds (`minimum`, `maximum`)
7. String bounds (`minLength`, `maxLength`)
8. Array bounds (`minItems`, `maxItems`, `uniqueItems`, `items`)
9. Object structure (`properties`, `required`, `additionalProperties`)
10. `x-schema-version`

## Default application behavior

After structural validation succeeds, `validate_and_normalize()` applies defaults:

- Missing optional fields with a declared default receive that default value.
- Defaults are applied recursively to nested objects.
- The normalized value is used for document-access planning, semantic validation, and apply.

Defaults must conform to their own schema; a non-conforming default fails schema definition validation.

## Additional-properties policy

The default policy is **allowed** (`additionalProperties: true`) for dialect v1 compatibility.

Commands may explicitly set `additionalProperties: false` to reject unknown fields structurally.

## Diagnostic codes

Structural validation failures produce `Diagnostic` values with these stable codes:

| Code | Trigger |
|---|---|
| `COMMAND_INPUT_REQUIRED` | A required field is missing |
| `COMMAND_INPUT_TYPE_MISMATCH` | Value kind does not match the schema |
| `COMMAND_INPUT_NULL_NOT_ALLOWED` | Null provided for a non-nullable field |
| `COMMAND_INPUT_ENUM_MISMATCH` | Value is not in the allowed enum set |
| `COMMAND_INPUT_CONSTRAINT_FAILED` | Numeric bound, string length, array length, or uniqueness violation |
| `COMMAND_INPUT_ADDITIONAL_PROPERTY` | Unknown property on an `additionalProperties: false` object |
| `SCHEMA_DEFINITION_INVALID` | Schema definition is malformed (registration-time) |

## JSON Pointer (RFC 6901)

Diagnostic `location.field_path` uses RFC 6901 JSON Pointer:

| Path | Meaning |
|---|---|
| `""` | Root value |
| `/stage_id` | Root field `stage_id` |
| `/path/0` | First array item |
| `/conditions/2/type` | Nested object field |

Escaping: `~` becomes `~0`, `/` becomes `~1` (in that order).

## Validation and execution order

```
Permission check
  ↓ (deny → AGENT_PERMISSION_DENIED)
Structural validation + default normalization
  ↓ (fail → COMMAND_VALIDATION_FAILED with diagnostics)
Document-access planning
  ↓ (fail → AGENT_ACCESS_PLAN_INVALID)
Working-set enforcement
  ↓ (deny → AGENT_WORKING_SET_DENIED)
Semantic (domain) validation
  ↓ (fail → COMMAND_VALIDATION_FAILED with diagnostics)
Command apply (atomic transaction)
```

Structural validation occurs before any state mutation, transaction ID consumption, audit append, or persistent write.

## Migrated commands

### Built-in document commands (`engine_command`)

| Command | Schema highlights |
|---|---|
| `document.create` | `schema_version` default 1, minimum 1; `payload` default any |
| `document.delete` | `document_id` required string |
| `document.set_field` | `path` required array of string, minItems 1 |

### Tactical RPG commands (`tactical_rpg_tools`)

| Command | Schema highlights |
|---|---|
| `stage.create` | `name` minLength 1; `width`/`height` integer minimum 1 |
| `stage.place_actor` | `properties` optional, default `{}` |
| `stage.create_spawn` | `q`/`r` required integer |
| `stage.create_trigger` | `conditions`/`effects` optional array, default `[]` |
| `stage.connect` | All fields required string |

## Compatibility policy

- Existing valid command calls remain valid.
- `additionalProperties` defaults to allowed for backward compatibility.
- Error message text may change between versions; stable diagnostic codes and JSON Pointer paths are the machine-facing contract.
- A dialect version change requires an explicit compatibility note and golden capability update.

## Examples

### Object schema with defaults

```rust
SchemaNode::object(vec![
    FieldSchema::required("id", SchemaNode::string()),
    FieldSchema::optional("version", SchemaNode::integer()
        .with_minimum(1.0)
        .with_default(json!(1))),
    FieldSchema::optional("tags", SchemaNode::array(SchemaNode::string())
        .with_default(json!([]))),
])
```

Rendered output:

```json
{
  "type": "object",
  "properties": {
    "id": {"type": "string", "x-schema-version": 1},
    "version": {"type": "integer", "minimum": 1.0, "default": 1, "x-schema-version": 1},
    "tags": {"type": "array", "items": {"type": "string", "x-schema-version": 1}, "default": [], "x-schema-version": 1}
  },
  "required": ["id"],
  "additionalProperties": true,
  "x-schema-version": 1
}
```

### Structural validation failure

Input:
```json
{"id": 42}
```

Diagnostics:
```json
[
  {
    "code": "COMMAND_INPUT_TYPE_MISMATCH",
    "severity": "error",
    "message": "expected string, got number",
    "location": {"field_path": "/id"}
  }
]
```

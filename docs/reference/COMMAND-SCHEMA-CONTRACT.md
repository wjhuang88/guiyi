# Command schema contract

## Purpose

This document defines the implemented ENG-010 command-schema contract. The `guiyi-engine-schema` crate is the single authority for command input definitions, registration-time definition validation, runtime structural validation, default normalization, deterministic capability rendering, and stable JSON Pointer diagnostics.

Command handlers may add semantic validation after structural validation, but they must not maintain a second structural schema authority.

## Dialect version

The current public dialect version is **1** and is represented by:

```json
"x-schema-version": 1
```

A root definition must contain the supported version. A nested schema may repeat `x-schema-version`; when present, it must equal the root version. Missing nested version markers inherit the root version.

Unsupported, missing, or malformed versions fail definition validation with:

```text
code: SCHEMA_DEFINITION_INVALID
keyword: x-schema-version
path: /x-schema-version
```

## Value kinds

| Engine kind | JSON representation | Notes |
| --- | --- | --- |
| `any` | `type` omitted | Accepts any JSON value |
| `string` | `"string"` | |
| `integer` | `"integer"` | Floating-point values are rejected |
| `number` | `"number"` | Integers and floating-point values are accepted |
| `boolean` | `"boolean"` | |
| `object` | `"object"` | Supports properties, required, and additionalProperties |
| `array` | `"array"` | Supports items and array constraints |

## Nullable type representation

A non-nullable typed schema uses a string:

```json
"type": "string"
```

A nullable typed schema uses exactly this two-element array, in this order:

```json
"type": ["string", "null"]
```

The dialect rejects all other type-array forms, including:

```json
["null", "string"]
["string"]
["null"]
["string", "integer"]
["string", "null", "null"]
```

The first element must be one supported non-null kind and the second element must be the string `"null"`.

## Supported keywords and strict JSON value types

| Keyword | Required JSON value type | Applies to |
| --- | --- | --- |
| `type` | supported kind string, or exact `["kind", "null"]` | typed schemas |
| `description` | string | all kinds |
| `default` | any JSON value valid under the node | all kinds |
| `enum` | array | all kinds |
| `minimum` | finite number | integer, number |
| `maximum` | finite number | integer, number |
| `minLength` | non-negative integer | string |
| `maxLength` | non-negative integer | string |
| `items` | schema object | array |
| `minItems` | non-negative integer | array |
| `maxItems` | non-negative integer | array |
| `uniqueItems` | boolean | array |
| `properties` | object of schema objects | object |
| `required` | array of unique strings | object |
| `additionalProperties` | boolean | object |
| `x-schema-version` | positive integer | root and optional nested markers |
| `x-guiyi-*` | any JSON value | extension namespace |

Recognized keywords with the wrong JSON value type fail definition validation. The parser does not coerce strings, numbers, booleans, arrays, or objects.

## Keyword applicability by kind

Applicability is checked from keyword presence before values are folded into internal defaults. Explicit default-like or empty values cannot bypass kind checking.

Examples that are invalid:

```json
{"x-schema-version":1,"type":"string","uniqueItems":false}
{"x-schema-version":1,"type":"string","additionalProperties":true}
{"x-schema-version":1,"type":"string","properties":{}}
{"x-schema-version":1,"type":"string","required":[]}
{"x-schema-version":1,"type":"integer","minLength":0}
{"x-schema-version":1,"type":"string","minimum":0}
```

Object keywords apply only to objects, array keywords only to arrays, string bounds only to strings, and numeric bounds only to integers or numbers. Kind-specific keywords are not accepted on `any`.

## Definition invariants

Definition validation rejects, among other malformed definitions:

- contradictory minimum/maximum, minLength/maxLength, or minItems/maxItems bounds;
- duplicate object field names;
- duplicate or unknown `required` entries;
- defaults that do not validate against their own node;
- malformed nested schemas;
- unsupported nested schema versions;
- unknown unnamespaced keywords;
- invalid extension keys.

All programmatic and JSON entry points enforce the same invariants.

## Extension contract

There are two extension maps:

1. `SchemaNode.extensions`, attached to a schema node;
2. `SchemaDefinition.extensions`, attached to the definition envelope.

Both maps use the same rule:

```text
Every key must start with x-guiyi-
```

Consequently, extensions cannot override core dialect keywords such as `type`, `properties`, or `x-schema-version`. This applies to definitions constructed programmatically as well as definitions parsed from JSON.

Node extensions are preserved recursively through parse, runtime storage, rendering, and round-trip operations. Definition-envelope extensions are validated and preserved by registries that store the complete definition. Command capability descriptors are generated from `SchemaDefinition.root`; envelope extensions do not replace or mutate the rendered root schema.

## Unknown keyword policy

An unknown unnamespaced schema keyword fails registration with `SCHEMA_DEFINITION_INVALID`. New standard keywords require a dialect change or an explicit implementation update. Product-specific extensions must use `x-guiyi-*`.

## Rendering contract

`SchemaNode::to_json_schema()` deterministically renders the validated runtime authority. Core keywords are emitted by the renderer and cannot be overwritten by extension maps.

Capability descriptors use the registered definition root rather than separately hand-authored input JSON. The golden capability fixture verifies all eight migrated command schemas exactly.

## JSON Pointer diagnostics

Definition and runtime field paths use RFC 6901 JSON Pointer.

| Path | Meaning |
| --- | --- |
| `""` | root value |
| `/stage_id` | root object field |
| `/path/0` | first array element |
| `/properties/name/minLength` | nested definition keyword |

Dynamic path segments are escaped in this order:

```text
~ -> ~0
/ -> ~1
```

For example, the invalid key `bad/key~name` is reported at:

```text
/bad~1key~0name
```

## Default normalization

After structural validation succeeds, `validate_and_normalize()` applies schema defaults recursively. The same normalized input is then used for document-access planning, semantic validation, and command application.

Missing optional fields receive declared defaults. Explicit `null` is not treated as missing: it is accepted only when the field schema is nullable.

Current migrated defaults include:

- `document.create.schema_version` -> `1`;
- `document.create.payload` -> `null`;
- `stage.place_actor.properties` -> `{}`;
- `stage.create_trigger.conditions` -> `[]`;
- `stage.create_trigger.effects` -> `[]`.

Command input structs do not use Serde business defaults; the schema remains the sole default authority.

## Additional-properties policy

Dialect v1 allows additional object properties by default. A schema may set:

```json
"additionalProperties": false
```

to reject unknown fields structurally.

## Registry and command authority

### SchemaRegistry

Registration validates the complete `SchemaDefinition` before insertion. Invalid versions, envelope extensions, root definitions, or nested definitions leave no stored schema.

### CommandRegistry

Command registration is atomic:

```text
handler + SchemaDefinition
  -> validate version
  -> validate envelope extensions
  -> validate root recursively
  -> insert definition and handler
  -> generate descriptors from the registered root
```

If validation fails, the registry contains no handler, no definition, and no descriptor for that command ID.

## Validation and execution order

```text
Permission check
  -> structural validation and default normalization
  -> document-access planning
  -> working-set enforcement
  -> command-specific semantic validation
  -> atomic command application
```

Permission denial precedes schema validation. Structural failure occurs before state mutation, transaction ID consumption, audit append, or persistent write.

## Stable diagnostic codes

| Code | Trigger |
| --- | --- |
| `COMMAND_INPUT_REQUIRED` | required field missing |
| `COMMAND_INPUT_TYPE_MISMATCH` | runtime value kind mismatch |
| `COMMAND_INPUT_NULL_NOT_ALLOWED` | explicit null on a non-nullable node |
| `COMMAND_INPUT_ENUM_MISMATCH` | value outside enum |
| `COMMAND_INPUT_CONSTRAINT_FAILED` | numeric, string, or array constraint failure |
| `COMMAND_INPUT_ADDITIONAL_PROPERTY` | unknown field where additional properties are forbidden |
| `SCHEMA_DEFINITION_INVALID` | malformed definition at registration time |

Message wording may evolve, but stable codes and JSON Pointer paths are the machine-facing contract.

## Migrated commands

Built-in document commands:

- `document.create`;
- `document.delete`;
- `document.set_field`.

Tactical RPG commands:

- `stage.create`;
- `stage.place_actor`;
- `stage.create_spawn`;
- `stage.create_trigger`;
- `stage.connect`.

All eight capability schemas are generated from the same registered definitions used for runtime structural validation.

## Compatibility policy

- Existing valid command calls remain valid.
- Additional properties remain allowed by default for dialect v1 compatibility.
- A dialect version change requires an explicit compatibility note and golden capability update.
- Rollback must restore the previous descriptor and validation path as one coherent change; two public schema authorities must not coexist.

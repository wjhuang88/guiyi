# Dialogue model and tactical rule registry

## Purpose

ENG-022 adds a reusable tactical Dialogue graph and fulfills the existing Stage Trigger contract for registry-defined condition/effect invocations. GUIYI remains Agent-first: Dialogue authoring and rule discovery are semantic machine-facing capabilities; a future visual graph view is only a client of the same authoritative documents, Commands, Queries, diagnostics, transactions, and build path.

## Rule invocation contract

Conditions and effects keep one persisted JSON shape across Stage Triggers and Dialogue choices: an object containing a non-empty string `type` plus descriptor-specific fields. There is no Dialogue-only rule format and no second persisted registry truth.

`TacticalRuleRegistry` owns deterministic, distinct condition/effect namespaces. Descriptors contain ID, description, and a schema using GUIYI's existing `SchemaNode` dialect. Duplicate registration is rejected deterministically. The default registry contains:

- condition `always`: `{ "type": "always" }`;
- effect `emit_signal`: `{ "type": "emit_signal", "signal": <non-empty string> }`.

The registry validates authoring shape only; ENG-022 deliberately does not execute rules at runtime. Invocation failures map to shared diagnostics: `TACTICAL_RULE_TYPE_INVALID`, `TACTICAL_RULE_UNKNOWN`, and `TACTICAL_RULE_SCHEMA_INVALID`.

`tactical.rules.catalog` requires `Read`, declares no document access, and returns deterministic condition/effect descriptor arrays with IDs, descriptions, and machine-readable schemas. It is the canonical Agent discovery surface and is automatically exposed through the normal live catalog and ENG-014 Talos adapter.

## Dialogue authoring model

`DialogueDocument` contains stable document ID, name, `entry_node`, and authored `nodes`. `DialogueNode` contains stable `ObjectId`, speaker, text, and ordered choices. `DialogueChoice` contains stable `ObjectId`, text, optional target node ID, conditions, and effects. A null target is terminal.

Internal node/choice edges remain inside the Dialogue document and are not duplicated into a second graph truth. Stable authoring identity never depends on Bevy `Entity` IDs.

Document type: `tactical.dialogue.document`. Artifact type: `tactical.dialogue.artifact`.

## Validation

`validate_dialogue` is the shared candidate/build-facing validator. Blocking diagnostics are:

- `DIALOGUE_NAME_EMPTY`;
- `DIALOGUE_DUPLICATE_NODE_ID`;
- `DIALOGUE_ENTRY_NODE_NOT_FOUND`;
- `DIALOGUE_NODE_TEXT_EMPTY`;
- `DIALOGUE_DUPLICATE_CHOICE_ID`;
- `DIALOGUE_CHOICE_TEXT_EMPTY`;
- `DIALOGUE_CHOICE_TARGET_NOT_FOUND`;
- the shared `TACTICAL_RULE_*` errors.

`DIALOGUE_NODE_UNREACHABLE` is a warning so incremental authoring can add nodes before wiring every edge. Diagnostic locations retain document/object/field-path context for Agent repair.

## Semantic authoring Tools

`dialogue.create(dialogue_id, name, entry_node_id, speaker, text)` creates a valid document plus initial entry node in one transaction. `dialogue.add_node` adds one stable node. `dialogue.add_choice` adds one stable edge, permits a null target, validates source/target IDs, and validates conditions/effects through the shared registry.

All three Commands require `EditContent` and use the normal GUIYI schema, access-plan, permission, working-set, dry-run, transaction, diff, audit, persistence, and atomic validation path. There is no direct-file or Editor-private graph mutation path.

## Compiler and build

`DialogueCompiler` rejects blocking local structure/rule failures, preserves authored node/choice order and stable IDs, and emits a self-contained deterministic artifact containing name, entry node, nodes, choices, conditions, and effects through the standard `ArtifactEnvelope` source-hash/integrity path.

CLI/build registrations include Stage, Encounter, and Dialogue compilers. Strict build therefore produces all three tactical artifacts for valid project content and emits no partial artifacts when Dialogue/rule validation blocks the build. Actor definitions remain authoring-only where already configured.

## Compatibility and migration

Dialogue document/artifact types and Tool IDs are additive. Stage serialization and `stage.create_trigger` remain unchanged. Existing registered-valid `emit_signal` payloads remain valid. Previously unchecked malformed/unknown/schema-invalid rule invocations can now fail validation intentionally.

The only new dependency edge is tactical content -> internal `guiyi-engine-schema`; there is no new external dependency, no Talos SDK/version/feature change, and no engine crate depending on tactical toolkit code.

## Non-goals

ENG-022 does not add a runtime Dialogue VM, conversation session/presentation UI, localization/audio/cutscene sequencing, runtime condition/effect execution, world variables, quests, inventory/party/save-state semantics, game-specific rule expansion, a generic engine-core plugin registry, ENG-023 intent-level composition, or a visual Dialogue graph editor.

Executable acceptance and rollback evidence is recorded in `docs/validation/ENG-022.md`.

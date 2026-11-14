# ADR-0003: Stable IDs

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Persistent and cross-document references use validated stable IDs, never Bevy Entity.

## Consequences

Enables save, migration, references, and agent queries across runs.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.
## ENG-028 implementation clarification (2026-11-14)

### Forces, alternatives and tradeoffs

Document references, serialized artifacts, diagnostics and review logs must remain addressable when ECS entities are recreated or a process restarts. **Rejected:** using Bevy `Entity` as a persistent identifier or using a free-form display name as an unvalidated foreign key. Such values are either process-local or mutable/ambiguous. The cost of stable IDs is explicit identity validation and reference bookkeeping when content is renamed, moved or migrated.

### Authoritative invariants

- `guiyi-engine-core` defines validated `DocumentId`, `ObjectId`, `ArtifactId` and related domain identifiers. `DocumentEnvelope.header.id` is its source identity; `ContentReference` records source/target document and optional object with a semantic kind. Display names are not canonical identity.
- `engine_query::ProjectGraphIndex` derives links from the authoritative envelope references; it is rebuildable and may surface dangling targets for diagnostics. The index does **not** own or rewrite the referenced content.
- Compiled Artifact IDs and runtime Stage instance IDs have different lifetimes. An ECS `Entity` or a temporary Stage instance ID must not be stored as a durable game/content reference.

### Rejections, compatibility and scale

A missing referenced document is not resolved by guessing a new ID; strict project validation/build must report the unresolved target and refuse a successful derived build where that reference is required. Renaming a document's display name does not justify changing its ID. **Automatic bulk ID remapping, cross-version identity migration and save-ID translation are not current capabilities**; require ENG-025 or another authorized migration contract. ID/reference indexing is derived from the loaded document set and refreshes on authorized changes. No maximum project-size or migration-throughput promise is made. A future policy changing ID formats or remapping behavior must define a version/migration compatibility story and supersede this decision explicitly.

### Evidence and related decisions

- Source: [core ID model](../../crates/engine_core/src/lib.rs), [envelope references](../../crates/engine_content/src/lib.rs), [Project Graph index](../../crates/engine_query/src/lib.rs), [Project Graph contract](../reference/PROJECT-GRAPH.md).
- Executable witnesses: `engine_query::tests::incremental_index_refreshes_changed_sources_and_preserves_dangling_targets`, `engine_agent_host/tests/project_graph_index.rs::applied_command_refreshes_graph_dry_run_does_not_and_state_replacement_invalidates`, strict build's `unresolved_reference_fails_before_compilation` in [engine_build](../../crates/engine_build/src/lib.rs).
- See [ADR-0011](ADR-0011-PROJECT-GRAPH.md) and [ADR-0002](ADR-0002-DOCUMENT-ARTIFACT-RUNTIME.md). ENG-025 remains Proposed; this clarification introduces no migration.

# ADR-0011: Semantic project graph

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Documents declare semantic references used for impact queries.

## Consequences

Avoids repeated full-text scanning and supports safe refactoring.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.
## ENG-028 implementation clarification (2026-11-14)

### Forces, rejected alternatives and cost

Agents need fast semantic dependencies and impact analysis without turning a derived index into an editable database. **Rejected:** scanning arbitrary source text to infer edges and persisting a second, independently mutable graph as project truth. Both could diverge from authoritative `DocumentEnvelope.references` and leak edges absent from explicit schemas. **Cost:** maintain indexed source/target references, explicit cache invalidation or incremental refresh after edits, and working-set-filtered views.

### Current mechanics and invariants

`guiyi-engine-query::ProjectGraphIndex` derives `outgoing_references`, `incoming_references` and adjacency from loaded `DocumentStore` envelopes; its snapshots are derived and rebuildable. It supports upsert/removal/changed-source refresh and preserves dangling target references for inspection, not as a promise that the target exists. The caller must validate unresolved references before strict build where needed.

`AgentHost` builds the index from the **complete authoritative project state** and filters project/graph queries by the caller-visible working set; hidden nodes are neither returned nor used as intermediate impact traversal steps. A direct external mutation of public alpha `host.state` requires `invalidate_project_graph()` or `replace_state()`; no magic cache invalidation is promised for arbitrary field writes. A dry-run does not refresh the persisted authoritative graph.

### Failure, compatibility, scale and supersession

A Query targeting a missing document or lacking Read permission rejects without semantic mutation; a derived index is not permitted to invent target content. Changing edge kind, visibility or traversal semantics requires explicit compatibility discussion and tests for hidden-node leaks. Index build/refresh costs depend on loaded documents and reference fan-out; this ADR establishes **no measured upper bound or cache SLO**. If data volume warrants a different index strategy, it must still derive from envelopes and preserve working-set isolation, or be approved by a superseding ADR with migration and rollback.

### Evidence

- Sources: [graph/index QueryExecutor](../../crates/engine_query/src/lib.rs), [AgentHost index invalidation](../../crates/engine_agent_host/src/lib.rs), [Project Graph contract](../reference/PROJECT-GRAPH.md).
- Tests: `engine_query::tests::incremental_index_refreshes_changed_sources_and_preserves_dangling_targets`, `indexed_queries_do_not_cross_hidden_working_set_nodes`, `get_missing_document_and_successful_queries_preserve_authoritative_state`, [Graph integration tests](../../crates/engine_agent_host/tests/project_graph_index.rs).
- See [ADR-0003](ADR-0003-STABLE-IDS.md), [ADR-0012](ADR-0012-LEAST-PRIVILEGE.md) and [ENG-012 validation](../validation/ENG-012.md).

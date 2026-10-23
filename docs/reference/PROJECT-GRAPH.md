# Project Graph

## Purpose

The Project Graph is GUIYI's derived semantic relationship index over authoring Documents. It exists so machine-facing Queries, Talos Tool calls, direct clients, and future human tooling can ask relationship and impact questions without rebuilding or text-scanning the project on every request.

`DocumentEnvelope.references` is the source of truth. The Project Graph is not a second authoring format and is not persisted as independent game data.

## Ownership

GUIYI owns Project Graph semantics and enforcement. Talos can discover and call graph-backed GUIYI Query Tools, but Talos does not build, cache, authorize, or mutate GUIYI's graph.

The implementation lives in `guiyi-engine-query`:

- `ProjectGraph` is the serializable document-adjacency snapshot surface.
- `ProjectReference` is the normalized semantic edge view derived from `ContentReference`.
- `ProjectGraphIndex` is the maintained incremental index.
- `QueryExecutor` owns the derived index for normal query execution.
- `AgentHost` initializes the index from the complete authoritative `EngineState` and coordinates mutation refreshes.

## Source-of-truth rule

The index is always recoverable from Documents:

```text
DocumentEnvelope.references
        ↓ build / refresh
ProjectGraphIndex
        ↓
project.references.find
project.impact.analyze
```

No graph edge may exist as authoritative state independently of a `ContentReference`. Project open/recovery can discard the complete index and rebuild it from `DocumentStore`.

## Lifecycle

`AgentHost::new` builds one graph from the complete authoritative Document store. This is important because a later restricted working-set Query may receive only a filtered visible store; the graph itself must never be initialized from that partial security view.

After a successful applied Command, `AgentHost` reads `TransactionReport.document_diffs` and refreshes only those source Document IDs in the existing graph. The refresh operation handles:

- newly created Documents;
- changed reference lists;
- deleted source Documents;
- references that remain pointed at a deleted or not-yet-present target.

A dry-run produces a preview transaction but does not mutate authoritative Documents and therefore does not refresh graph state. Rejected and failed Commands likewise leave the graph unchanged.

For recovery or external state replacement, `AgentHost::replace_state` replaces the authoritative `EngineState` and invalidates the derived graph. The next graph snapshot/query rebuilds from Documents. `AgentHost::invalidate_project_graph` and `AgentHost::rebuild_project_graph` exist for compatibility callers that still mutate the public alpha `state` surface directly.

Direct `QueryExecutor` owners can use `rebuild_project_graph`, `refresh_project_graph`, `invalidate_project_graph`, and `project_graph_snapshot` without constructing Talos.

## Working-set security

The Project Graph is an optimization, never an authorization bypass.

`AgentHost` continues to enforce Query permissions and `DocumentAccessPlan`. For a restricted session it supplies a filtered visible `DocumentStore` to the Query. Graph-backed Queries then apply that visible view to results and traversal:

- `project.references.find` does not return references whose source Document is hidden;
- `project.impact.analyze` does not return hidden Documents;
- impact traversal does not cross a hidden intermediate Document to reach a visible downstream Document.

The maintained graph can contain the complete project, but every restricted result is still bounded by GUIYI's working-set authority.

## Query compatibility

`QueryHandler::execute` remains the compatibility path for a standalone handler call. `QueryHandler::execute_indexed` has a default implementation that delegates to `execute`, so custom Queries do not need graph changes unless they opt into indexed semantics.

The built-in `project.references.find` and `project.impact.analyze` override `execute_indexed` and use the maintained `ProjectGraphIndex` when invoked through `QueryExecutor` / `AgentHost`. Their Tool IDs and machine-readable schemas are unchanged, so the live Tool catalog and Talos adapter continue exposing the same capabilities.

`QueryExecutor::execute` now requires mutable access because it owns derived cache lifecycle. This is an alpha machine-facing API change; callers that own a long-lived executor should retain it mutably rather than reconstructing it per Query.

## Persistence and determinism

The graph is not written to project manifests or content files. Transaction/audit/persistence truth remains the authoring Document state and existing transaction evidence.

Index collections use ordered maps/sets and normalized ordered reference vectors so snapshots and query results remain deterministic for the same Documents.

## Rollback

ENG-012 can be rolled back by removing the incremental cache and AgentHost refresh/invalidation hooks. `DocumentEnvelope.references` remains authoritative, and graph-backed Queries can fall back to one-shot construction/scanning without any migration of project content or persistence formats.

See also:

- `docs/decisions/ADR-0011-PROJECT-GRAPH.md`
- `docs/reference/ARCHITECTURE.md`
- `docs/reference/AGENT-SESSIONS.md`
- `docs/validation/ENG-012.md`

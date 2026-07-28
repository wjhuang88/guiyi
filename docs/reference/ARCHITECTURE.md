# Architecture

## Dependency direction

```text
Bevy ECS
   ↑
engine_core / schema / content / validation
   ↑
command / query / asset / build / runtime
   ↑
protocol / agent_tools / agent_host / preview
   ↑
CLI / workbench / tactical_rpg toolkit
   ↑
game-specific repositories
```

The repository enforces important edges with `scripts/check_architecture.py`.

## Sources of truth

- Authoring truth: `DocumentEnvelope` and registered document schemas.
- Build truth: deterministic `ArtifactEnvelope` output.
- Runtime truth: short-lived ECS entities owned by a Stage instance.
- Mutation truth: registered commands and transaction reports.
- Read truth: registered queries and project semantic references.
- Error truth: machine-readable diagnostics with stable codes.

## State boundary

```text
Definition → Session → Persistent
Document → Artifact → Runtime Instance
```

A Bevy `Entity` is never a persistent identity. Cross-document and saved references use stable IDs.

## Extension boundary

Engine core knows only projects, documents, objects, assets, commands, queries, diagnostics, transactions, artifacts, runtime instances, and permissions. Tactical concepts live in the toolkit. Game concepts live in separate extensions.

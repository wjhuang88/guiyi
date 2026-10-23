# Architecture

## First-party integration boundary

Talos is the canonical first-party AI Agent runtime. GUIYI is the game-development execution environment Talos consumes.

The supported embedding boundary is the published crates.io `talos-runtime` SDK. GUIYI's normal first-party integration has one direct Talos dependency, `talos-runtime`, at an outer adapter boundary. ENG-014 implements that boundary in `integrations/talos_runtime`.

```text
crates.io talos-runtime
        |
        v
integrations/talos_runtime
        |
        +----------------------+
        |                      |
        v                      v
GUIYI Tool catalog        GUIYI protocol
        |                      |
        +----------+-----------+
                   v
       domain execution boundary
          /                   \
         v                     v
queries / Project Graph   commands / transactions
         \                     /
          +---------+---------+
                    v
 validation / build / preview / artifacts
                    |
                    v
                 Bevy ECS
```

The adapter uses only public `talos-runtime` composition contracts such as `RuntimeBuilder`, `AgentTool`, `RuntimeHandle`, provider traits, and typed session events. GUIYI's live Tool catalog remains capability/schema authority: ENG-014 materializes one Talos `AgentTool` per eligible live `ToolDescriptor` instead of maintaining a second static list or a catch-all `guiyi.call_tool` facade.

For Queries, the Talos Tool parameters are the GUIYI input schema directly. For Commands, the adapter adds only a transport envelope containing the GUIYI command input plus the GUIYI protocol-level `dry_run` flag; the nested command schema remains the live GUIYI schema. Tool execution always delegates to `AgentHost::execute`, so Talos approval never replaces GUIYI permission, working-set, access-plan, schema, transaction, diagnostic, or audit enforcement.

Direct CLI, JSONL Workbench, tests, and future human clients may enter through the same GUIYI machine-facing boundary without Talos.

## Repository dependency direction

```text
Bevy ECS
   ↑
engine_core / schema / content / validation
   ↑
command / query / asset / build / runtime
   ↑
protocol / agent_tools / current domain execution boundary / preview
   ↑
CLI / workbench / tactical_rpg toolkit
   ↑
integrations/talos_runtime
   ↑
Talos and game-specific repositories
```

The arrows describe allowed outward consumers, not permission for engine-core crates to depend upward. In particular:

- reusable GUIYI engine crates must not depend on Talos;
- `guiyi-talos-runtime` is the only repository package allowed a direct Talos dependency;
- the direct dependency is exactly crates.io `talos-runtime = 0.10.1`, with default features disabled and no optional Talos features enabled for ENG-014;
- the normal integration must not directly depend on Talos implementation crates such as `talos-agent`, `talos-core`, `talos-session`, or `talos-permission` merely for convenience;
- transitive Talos implementation crates present in `Cargo.lock` are implementation details of the published facade, not GUIYI direct dependencies;
- Git/path Talos dependencies are temporary development overrides, not the committed normal dependency source;
- tactical-RPG crates may depend on engine crates; the reverse is forbidden;
- game-specific repositories consume GUIYI and may use Talos, but GUIYI never depends on a game repository.

`scripts/check_architecture.py` enforces the direct Talos dependency boundary and the location of the adapter in addition to existing GUIYI dependency rules.

## Responsibility boundary

### Talos owns

- provider/model invocation and credentials;
- conversation and turn lifecycle;
- prompt/planning policy;
- context selection and compression;
- generic memory and Skills;
- generic Tool-choice policy;
- retry, stopping, and generic cancellation behavior.

### GUIYI owns

- game/project Documents and stable identities;
- semantic Project Graph and impact analysis;
- typed Commands and Queries;
- GUIYI Tool schemas and capability metadata;
- project/domain permissions and working-set enforcement;
- dry-run, diff, transaction, audit, rollback/undo policy;
- diagnostics and repair evidence;
- validation, build, artifacts, preview, persistence, and Bevy runtime loading.

Talos may request an operation; GUIYI remains authoritative for whether that operation is valid, allowed, and committed.

## Talos integration execution path

The implemented first-party path is:

```text
RuntimeBuilder / Talos runtime
        ↓
GuiyiTalosAdapter
        ↓
live ToolCatalog → one AgentTool per ToolDescriptor
        ↓
AgentHost::execute
        ↓
permission + working-set + document-access planning
        ↓
QueryExecutor OR CommandExecutor
        ↓
schema normalization / validation / dry-run / transaction report
        ↓
ProjectStorage + ProjectTransaction for persisted apply
```

`ProjectStoragePersistence` is an outer coordinator over the existing `ProjectStorage` / `ProjectTransaction` authority. It does not introduce a second persistence model. Dry-runs are never storage commits; successful applied mutations are committed through the existing project transaction journal, and persistence failure is reported as a structured Talos Tool failure after rolling the in-memory document state back.

## Semantic Project Graph boundary

ENG-012 implements ADR-0011 as a maintained derived index rather than a per-query rebuild. `DocumentEnvelope.references` remains the only authoring source of truth for semantic relationships; the Project Graph is recoverable cache state and is not persisted as a second project format.

The normal lifecycle is:

```text
complete authoritative DocumentStore
        ↓ build once
ProjectGraphIndex owned by QueryExecutor
        ↑
        | refresh only TransactionReport.document_diffs
        |
successful applied Command
```

`AgentHost::new` initializes the graph from the complete authoritative `EngineState`. `project.references.find` and `project.impact.analyze` use the maintained index on the normal `QueryExecutor` / `AgentHost` path. Successful applied Commands refresh only changed source Document IDs from transaction diffs. Dry-run, rejected, and failed Commands do not refresh graph state.

State replacement or persistence rollback uses `AgentHost::replace_state`, which invalidates the derived graph so the next graph access rebuilds from authoritative Documents. Direct clients can explicitly rebuild, refresh, invalidate, or snapshot the graph without Talos.

A complete in-memory graph never weakens working-set security. Restricted graph Queries still receive the caller-visible project view: hidden reference sources are not returned, and impact traversal stops at hidden intermediate Documents rather than crossing them to reveal downstream nodes.

`ProjectGraph` remains the serializable document-adjacency snapshot surface; `ProjectGraphIndex` owns incremental semantic-reference metadata. Query Tool IDs and schemas remain stable, so Talos discovers the same live GUIYI capabilities without owning graph semantics.

See `docs/reference/PROJECT-GRAPH.md` for the detailed lifecycle, compatibility, security, and rollback contract.

## Legacy Agent-host compatibility boundary

`engine_agent_host` still contains both valuable GUIYI execution safety and transitional generic loop/session concepts. ENG-014 deliberately does not make an alpha-breaking removal because the JSONL Workbench, examples, and tests still use the direct API.

The migration decision is:

- `AgentHost::execute` remains the authoritative GUIYI domain-execution seam;
- `AgentLoopDriver` and `AgentHost::run` are legacy compatibility APIs and are not used by the Talos first-party path; no new first-party functionality should be built on them;
- `AgentSession.objective`, generic lifecycle status, `final_summary`, and loop orchestration are not part of the Talos adapter contract and remain compatibility-only state;
- `AgentBudget.max_actions` is a legacy generic action budget, not an independent GUIYI domain-safety policy. The Talos adapter disables it as a practical second stopping budget while the compatibility `AgentSession` is still required internally;
- the Talos adapter exposes the narrower `GuiyiExecutionContext`: session/caller identity for audit, GUIYI permissions, and working-set scope;
- a later explicit migration may remove or redesign legacy generic fields after direct alpha clients have a narrow domain execution context of their own.

This is a sunset decision, not permission to weaken GUIYI security. Permissions, working sets, document-access plans, transaction checks, diagnostics, persistence, and audit remain GUIYI-owned regardless of caller.

## Sources of truth

- Authoring truth: `DocumentEnvelope` and registered document schemas.
- Semantic relationship truth: `DocumentEnvelope.references`; `ProjectGraphIndex` is derived cache state only.
- Build truth: deterministic `ArtifactEnvelope` output.
- Runtime truth: short-lived ECS entities owned by a Stage instance.
- Mutation truth: registered Commands and transaction reports.
- Read truth: registered Queries and project semantic references.
- Error truth: machine-readable diagnostics with stable codes.
- First-party Agent-runtime truth: Talos through the published `talos-runtime` SDK.

## State boundary

```text
Definition → Session → Persistent
Document → Artifact → Runtime Instance
```

A Bevy `Entity` is never a persistent identity. Cross-document and saved references use stable IDs.

`Session` in GUIYI domain/reference material must be qualified when ambiguity matters: Talos owns generic Agent/session lifecycle, while GUIYI retains only domain execution context carrying caller identity, access scope, permissions, and audit/transaction metadata.

## Extension boundary

Engine core knows only projects, documents, objects, assets, Commands, Queries, diagnostics, transactions, artifacts, runtime instances, permissions, and other reusable game-development primitives. Tactical concepts live in the toolkit. Game concepts live in separate extensions. Generic AI-Agent runtime concepts belong in Talos rather than expanding GUIYI engine core.

# Architecture

## First-party integration boundary

Talos is the canonical first-party AI Agent runtime. GUIYI is the game-development execution environment Talos consumes.

The supported embedding boundary is the published crates.io `talos-runtime` SDK. GUIYI's normal first-party integration has one direct Talos dependency, `talos-runtime`, at an outer adapter boundary.

```text
crates.io talos-runtime
        |
        v
GUIYI Talos adapter (outer integration only)
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

The adapter uses public `talos-runtime` composition contracts such as `RuntimeBuilder` and `AgentTool`. GUIYI's live Tool catalog remains capability/schema authority; relevant capabilities should be exposed as individual Talos Tools rather than hidden behind a catch-all call when their schemas are available.

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
outer integrations such as the Talos adapter
   ↑
Talos and game-specific repositories
```

The arrows describe allowed outward consumers, not permission for engine-core crates to depend upward. In particular:

- reusable GUIYI engine crates must not depend on Talos;
- the Talos adapter may depend on crates.io `talos-runtime` and public GUIYI machine-facing APIs;
- the normal integration must not directly depend on Talos implementation crates such as `talos-agent` merely for convenience;
- Git/path Talos dependencies are temporary development overrides, not the committed normal dependency source;
- tactical-RPG crates may depend on engine crates; the reverse is forbidden;
- game-specific repositories consume GUIYI and may use Talos, but GUIYI never depends on a game repository.

The repository enforces important internal edges with `scripts/check_architecture.py`. ENG-014 must extend the architecture gate when the Talos adapter crate/path is introduced.

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

## Transitional boundary

The current `engine_agent_host` contains both valuable GUIYI execution safety and generic loop/session concepts. `AgentLoopDriver`, agent objective/final-summary state, loop orchestration, and overlapping lifecycle state are transitional alpha APIs.

Until ENG-014 completes:

- preserve their current compatibility unless the Story explicitly approves migration;
- do not expand them with provider, model, prompt, planning, memory, context-compression, conversation, or retry features;
- keep command/query execution safety in GUIYI;
- record the final Talos adapter and execution-context shape in reference docs and tests.

## Sources of truth

- Authoring truth: `DocumentEnvelope` and registered document schemas.
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

`Session` in GUIYI domain/reference material must be qualified when ambiguity matters: Talos owns generic Agent/session lifecycle, while GUIYI may retain a domain execution context carrying caller identity, access scope, permissions, and audit/transaction metadata.

## Extension boundary

Engine core knows only projects, documents, objects, assets, Commands, Queries, diagnostics, transactions, artifacts, runtime instances, permissions, and other reusable game-development primitives. Tactical concepts live in the toolkit. Game concepts live in separate extensions. Generic AI-Agent runtime concepts belong in Talos rather than expanding GUIYI engine core.

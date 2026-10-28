# AI-native platform

## Definition

AI-native means that GUIYI is designed around machine-consumable game-development capabilities rather than automating mouse clicks in a traditional editor.

GUIYI is not itself the first-party general Agent runtime. Talos owns that role. GUIYI provides the structured world, Tools, safety boundaries, diagnostics, and deterministic execution evidence that Talos consumes.

## Agent-first development surface

The primary GUIYI development surface is the Agent interaction layer, not a traditional editor-centric workflow. At the product level, the Talos + GUIYI Tool path is intended to occupy the development role that a conventional game-engine Editor normally occupies:

```text
understand project
    ↓
query semantic state
    ↓
make typed game-content changes
    ↓
inspect dry-run diff + diagnostics
    ↓
apply transaction
    ↓
validate / build / preview
    ↓
repair or continue iteration
```

A normal game-development task is therefore not considered fully machine-native when it can only be completed through an editor-only button, mouse gesture, hidden UI state, or direct source-file mutation. The preferred fix is to expose or refine the relevant semantic Command, Query, diagnostic, build/preview operation, or toolkit intent Tool.

A visual GUIYI Editor may exist, but its architectural role is secondary: review, visualization, inspection, debugging, approval, rollback, and exceptional human intervention. It consumes the same project models, Commands, Queries, diagnostics, transactions, build pipeline, and preview/runtime boundaries as the Agent path. It does not own a separate mutation model or a second authoring truth.

This does not mean every traditional editor affordance must have a one-to-one Tool. GUIYI should prefer semantic, intent-level capabilities over exposing raw GUI-equivalent primitives.

## Canonical first-party Agent loop

Talos runs the first-party reasoning loop:

```text
Observe → Query → Plan → Dry run → Apply transaction → Validate → Evaluate
                                     ↘ rollback or repair ↗
```

GUIYI participates at Query, Tool execution, transaction, validation, build, and preview boundaries. Model calls, context compression, planning strategy, generic memory, retries, provider credentials, conversation state, and generic stopping policy remain in Talos.

## Supported integration contract

The approved and implemented first-party dependency boundary is the published crates.io **`talos-runtime`** SDK. GUIYI does not directly embed Talos implementation crates when the runtime facade exposes the needed contracts.

The implemented path is:

```text
crates.io talos-runtime
        ↓
RuntimeBuilder + GUIYI AgentTool adapters
        ↓
GUIYI Talos adapter
        ↓
GUIYI Tool catalog + domain execution boundary
        ↓
Commands / Queries / Validation / Build / Preview
```

ENG-014 adapts each relevant live GUIYI Tool descriptor into an individual Talos `AgentTool`, preserving Tool identity and machine-readable schema. The adapter translates Talos Tool invocation into the normal GUIYI `AgentHost::execute` boundary and translates GUIYI results, diagnostics, and transaction evidence back to Talos.

The current approved SDK line is `talos-runtime 0.10.1` from crates.io, pinned through the committed `Cargo.lock`, with `default-features = false` and no optional Talos features enabled. `integrations/talos_runtime` is the only GUIYI package allowed to declare the direct Talos dependency. Direct dependencies on `talos-agent`, `talos-core`, `talos-session`, `talos-permission`, or other Talos implementation crates remain prohibited without explicit Story/ADR approval.

Because Talos is pre-1.0, minor-version upgrades are explicit compatibility work. Re-check the public facade, re-exports, Tool contract, typed events, permission behavior, feature requirements, and lockfile resolution before changing the approved line.

The adapter does not bypass GUIYI command validation, permissions, working-set checks, project scope, access planning, transactions, diagnostics, audit, or persistence. Talos permission/approval is additive and cannot substitute for GUIYI domain security.

Direct CLI/Workbench clients remain supported without Talos.

## First-class GUIYI concepts

- Tool catalog with input/output schemas.
- Command and Query descriptors.
- Domain execution context with caller identity and access scope.
- Least-privilege GUIYI permission set.
- Semantic Project Graph.
- Transaction diff and audit history.
- Structured diagnostics and suggested Tools.
- Headless validation, build, and preview.
- Tactical-RPG domain and intent Tools.

Generic conversation history, model/provider configuration, prompt policy, Agent memory, reasoning-loop budgets, and retry policy are not GUIYI product concepts unless an accepted ADR explicitly changes the boundary.

## Safety defaults

Content-authoring calls should normally be limited to GUIYI permissions equivalent to `Read`, `Plan`, `DryRun`, `EditContent`, `RunValidation`, `RunBuild`, and `RunPreview`. Code editing, schema editing, external processes, commit, and publish remain outside ordinary content-authoring authority.

Talos may apply its own generic permission and approval policy, but GUIYI still enforces its domain security and transaction invariants for every caller. ENG-014 tests deliberately admit a write through Talos permission rules and then prove GUIYI can still reject it for insufficient domain permission or working-set scope.

## Compatibility-only alpha APIs

`AgentLoopDriver`, `AgentHost::run`, and the generic loop/lifecycle portions of `engine_agent_host::AgentSession` predate the Talos convergence decision. After ENG-014 they remain compatibility-only alpha surfaces, not a second canonical first-party Agent framework.

The Talos path uses the narrower GUIYI-owned `GuiyiExecutionContext` and does not call `AgentLoopDriver` / `AgentHost::run`. `AgentSession` is still constructed internally as a compatibility seam because the authoritative `AgentHost::execute` API currently accepts it, and direct JSONL Workbench/tests/examples still use the alpha session surface.

`AgentBudget` is classified as a legacy generic Agent action budget rather than a distinct GUIYI domain-safety invariant. The Talos adapter does not impose it as a second practical stopping policy; Talos owns first-party reasoning/action stopping.

Until a separately authorized compatibility migration removes these alpha surfaces:

- no new model/provider abstraction belongs in GUIYI;
- no generic prompting, planning, memory, conversation, context-compression, retry, or stopping subsystem should be added to `engine_agent_host`;
- existing domain execution safety and direct-client behavior must remain intact;
- any breaking API migration must be explicit and tested.

See [Agent session execution boundary](AGENT-SESSIONS.md) for the implemented migration semantics.

## Product proof

ENG-014 establishes the real Talos Runtime SDK baseline: Talos discovers schema-bearing GUIYI Tools, executes Queries, dry-runs and applies Commands through GUIYI's authoritative boundary, receives structured recoverable failures, and leaves durable transaction/audit evidence while direct clients remain independent.

The broader target proof is a real Talos-driven workflow that creates or edits a tactical-RPG slice, responds to structured diagnostics, repairs invalid content, builds and previews the result, and leaves a reviewable transaction trail. That product-level proof remains owned by later Stories such as ENG-030; mock/scripted providers in ENG-014 are integration acceptance evidence, not the final product demo.

ENG-012 and ENG-013 intentionally increase semantic capability density on this path: project relationships become queryable as maintained semantic state, and common Stage-object edits become typed transactional operations. These are examples of replacing editor-only authoring gestures with discoverable game-semantic capabilities rather than reproducing GUI mechanics one click at a time.

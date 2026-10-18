# AI-native platform

## Definition

AI-native means that GUIYI is designed around machine-consumable game-development capabilities rather than automating mouse clicks in a traditional editor.

GUIYI is not itself the first-party general agent runtime. Talos owns that role. GUIYI provides the structured world, Tools, safety boundaries, diagnostics, and deterministic execution evidence that Talos consumes.

## Canonical first-party agent loop

Talos runs the first-party reasoning loop:

```text
Observe → Query → Plan → Dry run → Apply transaction → Validate → Evaluate
                                     ↘ rollback or repair ↗
```

GUIYI participates at Query, Tool execution, transaction, validation, build, and preview boundaries. Model calls, context compression, planning strategy, generic memory, retries, provider credentials, conversation state, and generic stopping policy remain in Talos.

## Integration contract

The intended first-party path is:

```text
Talos / talos-runtime
        ↓
GUIYI Talos adapter
        ↓
GUIYI Tool catalog + domain execution boundary
        ↓
Commands / Queries / Validation / Build / Preview
```

The adapter translates between Talos Tool/runtime contracts and GUIYI machine-facing contracts. It must not bypass GUIYI command validation, permissions, working-set checks, transactions, diagnostics, or persistence.

Direct CLI/Workbench clients remain supported without Talos.

## First-class GUIYI concepts

- Tool catalog with input/output schemas.
- Command and query descriptors.
- Domain execution context with caller identity and access scope.
- Least-privilege GUIYI permission set.
- Semantic Project Graph.
- Transaction diff and audit history.
- Structured diagnostics and suggested Tools.
- Headless validation, build, and preview.
- Tactical-RPG domain and intent Tools.

Generic conversation history, model/provider configuration, prompt policy, agent memory, reasoning-loop budgets, and retry policy are not GUIYI product concepts unless an accepted ADR explicitly changes the boundary.

## Safety defaults

Content-authoring calls should normally be limited to GUIYI permissions equivalent to `Read`, `Plan`, `DryRun`, `EditContent`, `RunValidation`, `RunBuild`, and `RunPreview`. Code editing, schema editing, external processes, commit, and publish remain outside ordinary content-authoring authority.

Talos may apply its own generic permission and approval policy, but GUIYI still enforces its domain security and transaction invariants for every caller.

## Transitional alpha APIs

The current `AgentLoopDriver` and generic loop/lifecycle portions of `engine_agent_host::AgentSession` predate the Talos convergence decision. They are transitional compatibility surfaces, not a mandate to grow a second agent framework.

ENG-014 owns the implementation migration. Until then:

- no new model/provider abstraction belongs in GUIYI;
- no generic prompting, planning, memory, conversation, context-compression, retry, or stopping subsystem should be added to `engine_agent_host`;
- existing domain execution safety must remain intact;
- any breaking API migration must be explicit and tested.

## Product proof

The target proof is a real Talos-driven workflow that discovers GUIYI capabilities, creates or edits a tactical-RPG slice, responds to structured diagnostics, repairs invalid content, builds and previews the result, and leaves a reviewable transaction trail. Mock drivers are useful for unit tests but are not the product-level proof of AI-native operation.

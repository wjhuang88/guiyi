# ADR-0016: Talos is the canonical first-party agent runtime

- Status: Accepted
- Date: 2026-10-18
- Story: ENG-029
- Supersedes: ADR-0009

## Context

GUIYI is an AI-native, headless-first game-development infrastructure. Its durable product value is the machine-verifiable game world and the safe execution surface around that world: typed commands, semantic queries, transactions, diagnostics, validation, build artifacts, preview, and Bevy runtime behavior.

ADR-0009 correctly kept provider/model logic outside engine core, but it left the external agent loop intentionally unnamed. Since then, Talos has become the canonical first-party Rust agent runtime and exposes an embeddable `talos-runtime` facade. Keeping a second generic loop/session/planning abstraction growing inside GUIYI would duplicate Talos, blur ownership, and make both projects harder to evolve.

Current GUIYI code already contains transitional overlap in `engine_agent_host`, including `AgentLoopDriver`, agent lifecycle status, objective/final-summary state, and loop orchestration. This ADR does not claim that overlap is already removed. ENG-014 owns the implementation migration.

## Decision

### 1. Talos owns first-party agent intelligence and runtime behavior

The canonical first-party AI execution path for GUIYI uses Talos, through a reviewed and explicitly pinned `talos-runtime` dependency at an outer integration boundary.

Talos owns generic concerns including:

- model/provider invocation and credentials;
- conversation and turn lifecycle;
- prompting and planning policy;
- context selection and compression;
- generic memory and Skills;
- tool-choice policy;
- retry, stopping, and generic cancellation policy;
- generic agent progress and completion semantics.

GUIYI must not implement a competing generic agent framework for these concerns.

### 2. GUIYI owns game-world semantics and execution authority

GUIYI remains authoritative for:

- project/document/artifact/runtime models;
- semantic project graph and impact analysis;
- domain and toolkit command/query catalogs;
- command schema validation and normalization;
- project-relative access boundaries;
- domain permissions and working-set enforcement;
- dry-run, diff, transaction, audit, rollback/undo policy;
- diagnostics and repair-relevant structured evidence;
- validation, compilation, artifact integrity, preview, and Bevy runtime behavior.

Talos may request a GUIYI Tool action, but GUIYI decides whether that action is valid and permitted and returns the authoritative structured result.

### 3. Depend inward from a thin adapter; never from engine core

The intended integration direction is:

```text
Talos / talos-runtime
        |
        v
GUIYI Talos adapter (outer integration)
        |
        v
GUIYI tool execution boundary
        |
        +--> queries / project graph
        +--> commands / transactions
        +--> validation / build / preview
        |
        v
GUIYI content + artifacts + Bevy runtime
```

The adapter may depend on both `talos-runtime` and GUIYI public machine-facing APIs. Engine-core, command, query, validation, build, runtime, content, and tactical-domain crates must not depend on Talos.

Human CLI/Workbench clients and tests may continue to call the same GUIYI tool/protocol boundary directly; Talos is not required for non-agent clients.

### 4. Separate agent lifecycle from domain execution context

Long term, Talos owns agent/session lifecycle. GUIYI should retain only the execution context required to authorize and audit a domain operation, such as:

- caller/session identity;
- GUIYI permissions;
- project working set or access scope;
- transaction/audit metadata;
- domain-specific operation limits when needed for safety.

A GUIYI domain limit is not a second reasoning-loop budget. If both Talos and GUIYI enforce limits, they must have different documented purposes.

### 5. Freeze transitional generic loop expansion until ENG-014

`AgentLoopDriver` and the generic loop/lifecycle parts of the current `engine_agent_host::AgentSession` are transitional compatibility surfaces.

Until ENG-014 is completed:

- do not add provider/model logic to them;
- do not add generic planning, memory, prompt, context-compression, retry, or conversation features;
- do not create a second first-party agent-loop implementation;
- do not remove or break them outside the migration Story.

ENG-014 must define the compatibility/deprecation path and keep GUIYI's domain execution safety properties intact while moving first-party loop ownership to Talos.

### 6. Talos upgrades are explicit architecture dependencies

ENG-014 must pin a reviewed Talos release or revision. Upgrades must be deliberate dependency changes with compatibility validation for the adapter and machine-facing Tool contract. GUIYI must not silently follow Talos `main`.

## Consequences

### Positive

- GUIYI can focus on the game-development world model and high-value domain tools.
- Talos and GUIYI have a single, legible ownership boundary.
- New GUIYI tools are exercised by a real first-party agent runtime instead of a growing mock framework.
- CLI, tests, and future human clients remain independent consumers of the same safe execution surface.

### Costs

- ENG-014 must migrate or narrow existing `engine_agent_host` loop/session responsibilities.
- The integration adds an explicit Talos dependency at the adapter edge and therefore requires version/revision governance.
- Some public alpha APIs may need a documented deprecation or breaking migration before 1.0.

## Rejected alternatives

### Continue a vendor-neutral first-party `AgentLoopDriver` as GUIYI's main runtime

Rejected because GUIYI would continue accumulating generic agent-runtime responsibilities already provided by Talos. A narrow internal trait may remain as an implementation detail only if ENG-014 proves it is useful and does not become a second agent framework.

### Move GUIYI domain safety into Talos

Rejected because command validity, working-set semantics, transactions, artifact integrity, and game-domain invariants must remain authoritative for every client, not only Talos.

### Make engine core depend directly on Talos

Rejected because it would couple reusable game-domain infrastructure to one agent runtime and would make direct CLI/Workbench use harder to preserve.

### Support multiple first-party agent runtimes immediately

Rejected as premature abstraction. Talos is the canonical first-party runtime. Other runtimes may integrate later through the same public machine-facing GUIYI boundary if a concrete Story requires them.

## Validation

ENG-029 validates documentation and dependency-direction rules. ENG-014 must provide executable integration evidence showing that Talos can discover GUIYI capabilities, invoke queries and commands, receive structured diagnostics/transaction results, continue after recoverable Tool failures, and complete a bounded workflow without provider/model logic entering GUIYI engine core.

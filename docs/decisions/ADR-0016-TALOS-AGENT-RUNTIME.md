# ADR-0016: Talos is the canonical first-party agent runtime

- Status: Accepted
- Date: 2026-10-18
- Story: ENG-029
- Supersedes: ADR-0009

## Context

GUIYI is an AI-native, headless-first game-development infrastructure. Its durable product value is the machine-verifiable game world and the safe execution surface around that world: typed commands, semantic queries, transactions, diagnostics, validation, build artifacts, preview, and Bevy runtime behavior.

ADR-0009 correctly kept provider/model logic outside engine core, but it left the external agent loop intentionally unnamed. Talos now provides the canonical first-party Rust agent runtime and publishes the supported embeddable SDK facade as the `talos-runtime` crate. Keeping a second generic loop/session/planning abstraction growing inside GUIYI would duplicate Talos, blur ownership, and make both projects harder to evolve.

Current GUIYI code contains transitional overlap in `engine_agent_host`, including `AgentLoopDriver`, agent lifecycle status, objective/final-summary state, and loop orchestration. This ADR does not claim that overlap is already removed. ENG-014 owns the implementation migration.

## Decision

### 1. Talos owns first-party agent intelligence and runtime behavior

The canonical first-party AI execution path for GUIYI uses Talos through the published `talos-runtime` SDK.

Talos owns generic concerns including:

- model/provider invocation and credentials;
- conversation and turn lifecycle;
- prompting and planning policy;
- context selection and compression;
- generic memory and Skills;
- Tool-choice policy;
- retry, stopping, and generic cancellation policy;
- generic agent progress and completion semantics.

GUIYI must not implement a competing generic agent framework for these concerns.

### 2. GUIYI owns game-world semantics and execution authority

GUIYI remains authoritative for:

- project/document/artifact/runtime models;
- semantic Project Graph and impact analysis;
- domain and toolkit Command/Query catalogs;
- command schema validation and normalization;
- project-relative access boundaries;
- domain permissions and working-set enforcement;
- dry-run, diff, transaction, audit, rollback/undo policy;
- diagnostics and repair-relevant structured evidence;
- validation, compilation, artifact integrity, preview, and Bevy runtime behavior.

Talos may request a GUIYI Tool action, but GUIYI decides whether that action is valid and permitted and returns the authoritative structured result.

### 3. Use the published runtime SDK as the dependency boundary

ENG-014 must consume the crates.io `talos-runtime` package as the default integration path. The dependency is declared at an outer integration crate/package, uses an explicitly reviewed pre-1.0 version requirement, and is resolved through the committed `Cargo.lock`.

The default integration must not use a Git dependency or a checkout-relative path dependency. Those are allowed only as temporary development overrides for testing a Talos fix, never as the committed normal dependency source.

GUIYI should depend directly on **one Talos crate: `talos-runtime`**. The SDK explicitly re-exports the canonical provider, Tool, permission, sandbox, message and session-event types needed for ordinary embedding. GUIYI must not add direct dependencies on `talos-agent`, `talos-core`, `talos-permission`, `talos-session`, or other `talos-*` implementation crates merely for convenience.

If ENG-014 discovers that the supported `talos-runtime` surface cannot express a required integration behavior, the default response is to improve the Talos SDK first. Adding another direct Talos dependency requires a separate compatibility rationale in the active Story and, when it changes the architectural boundary, a new or superseding ADR.

Because Talos is pre-1.0, GUIYI treats a Talos minor-version upgrade as an explicit compatibility change: update the version requirement intentionally, refresh the lockfile, run adapter contract tests, and record migration impact. Do not silently track Talos `main`.

### 4. Integrate through Talos `RuntimeBuilder` and `AgentTool`

The first implementation should use the public SDK composition surface rather than Talos internals:

```text
talos-runtime::RuntimeBuilder
        |
        +--> provider selected/configured by the embedding product
        +--> GUIYI AgentTool adapters
        +--> approval / sandbox policy as required by the embedding product
        |
        v
Talos RuntimeHandle / SessionEvent
```

GUIYI's live `ToolCatalog` remains the capability authority. ENG-014 should adapt GUIYI capabilities into Talos `AgentTool` instances so the model sees the real Tool IDs, descriptions and schemas. A single catch-all `guiyi.call_tool` wrapper is not the default because it would throw away capability-level discovery and schema guidance.

The adapter translates Talos Tool invocation into the normal GUIYI Tool execution boundary and translates GUIYI `ToolResult`, diagnostics and transaction evidence back into a Talos Tool result. It must not bypass GUIYI validation, permissions, working sets, transactions, persistence, or diagnostics.

Use the smallest `talos-runtime` feature set needed for this integration. Do not enable Talos shared coding-tool composition merely to call GUIYI Tools.

### 5. Depend inward from a thin adapter; never from engine core

The intended integration direction is:

```text
Talos / talos-runtime from crates.io
        |
        v
GUIYI Talos adapter (outer integration)
        |
        v
GUIYI Tool execution boundary
        |
        +--> queries / Project Graph
        +--> commands / transactions
        +--> validation / build / preview
        |
        v
GUIYI content + artifacts + Bevy runtime
```

The adapter may depend on both `talos-runtime` and GUIYI public machine-facing APIs. Engine-core, command, query, validation, build, runtime, content, and tactical-domain crates must not depend on Talos.

Human CLI/Workbench clients and tests may continue to call the same GUIYI Tool/protocol boundary directly; Talos is not required for non-agent clients.

### 6. Separate agent lifecycle from domain execution context

Long term, Talos owns agent/session lifecycle. GUIYI should retain only the execution context required to authorize and audit a domain operation, such as:

- caller/session identity;
- GUIYI permissions;
- project working set or access scope;
- transaction/audit metadata;
- domain-specific operation limits when needed for safety.

A GUIYI domain limit is not a second reasoning-loop budget. If both Talos and GUIYI enforce limits, they must have different documented purposes.

### 7. Freeze transitional generic loop expansion until ENG-014

`AgentLoopDriver` and the generic loop/lifecycle parts of the current `engine_agent_host::AgentSession` are transitional compatibility surfaces.

Until ENG-014 is completed:

- do not add provider/model logic to them;
- do not add generic planning, memory, prompt, context-compression, retry, or conversation features;
- do not create a second first-party agent-loop implementation;
- do not remove or break them outside the migration Story.

ENG-014 must define the compatibility/deprecation path and keep GUIYI's domain execution safety properties intact while moving first-party loop ownership to Talos.

## Consequences

### Positive

- GUIYI focuses on game-world semantics and high-value domain Tools.
- Talos and GUIYI have one legible ownership boundary.
- GUIYI uses Talos's supported embeddable SDK instead of internal implementation crates.
- Capability discovery remains schema-rich because GUIYI capabilities are exposed as real Talos Tools.
- CLI, tests, and future human clients remain independent consumers of the same safe execution surface.

### Costs

- ENG-014 must migrate or narrow existing `engine_agent_host` loop/session responsibilities.
- Talos pre-1.0 upgrades require explicit compatibility validation.
- Some public alpha APIs may need a documented deprecation or breaking migration before 1.0.

## Rejected alternatives

### Directly embed `talos-agent` or other lower-level Talos crates

Rejected as the normal path because Talos documents `talos-runtime` as the supported SDK facade and treats `talos-agent` as an implementation dependency. GUIYI should not couple itself to Talos internals when the SDK already exports the required embedding contracts.

### Use a Git dependency on Talos `main`

Rejected as the committed default because it makes clean builds non-reproducible across Talos development and bypasses the published SDK/release boundary.

### Continue a vendor-neutral first-party `AgentLoopDriver` as GUIYI's main runtime

Rejected because GUIYI would continue accumulating generic agent-runtime responsibilities already provided by Talos. A narrow internal trait may remain only if ENG-014 proves it is useful as a domain-execution seam rather than a second agent framework.

### Move GUIYI domain safety into Talos

Rejected because command validity, working-set semantics, transactions, artifact integrity, and game-domain invariants must remain authoritative for every client, not only Talos.

### Make engine core depend directly on Talos

Rejected because it would couple reusable game-domain infrastructure to one agent runtime and make direct CLI/Workbench use harder to preserve.

### Support multiple first-party agent runtimes immediately

Rejected as premature abstraction. Talos is the canonical first-party runtime. Other runtimes may integrate later through the same public machine-facing GUIYI boundary if a concrete Story requires them.

## Validation

ENG-029 validates documentation and dependency-direction rules. ENG-014 must provide executable integration evidence showing that a registry-sourced `talos-runtime` dependency can discover GUIYI capabilities, invoke queries and commands, receive structured diagnostics/transaction results, continue after recoverable Tool failures, and complete a bounded workflow without provider/model logic or direct Talos implementation-crate dependencies entering GUIYI engine core.

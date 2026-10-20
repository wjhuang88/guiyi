# Project charter

## Mission

Build an independent Bevy infrastructure project for AI-driven tactical RPG development. GUIYI exposes deterministic, discoverable, transactional, and machine-verifiable game-development capabilities. Talos is the canonical first-party AI agent runtime that consumes those capabilities; GUIYI does not implement a competing generic agent framework.

A human interface is optional and uses the same command/query execution surface as Talos and other machine clients.

## Product boundary

GUIYI owns the game-development world and its execution rules:

1. Engine domain and content core.
2. Command, query, validation, build, runtime, asset, transaction, and diagnostic platform.
3. Machine-facing protocol, Tool catalog, and domain execution/security boundary.
4. Tactical RPG toolkit and higher-level intent Tools.
5. Game-specific extensions maintained outside this repository.

Talos owns first-party generic agent-runtime behavior: model/provider integration, conversation/turn lifecycle, prompting/planning, context management, generic memory/Skills, Tool-choice policy, retry, stopping, and generic cancellation behavior.

The first-party dependency direction is:

```text
Talos / talos-runtime
        |
        v
GUIYI Talos adapter
        |
        v
GUIYI Tool/query/command boundary
        |
        v
GUIYI content, build, artifacts, preview, Bevy runtime
```

No reusable GUIYI engine-core crate depends on Talos. Direct CLI, Workbench, tests, and future human clients may consume the GUIYI boundary without Talos.

## Current product north star

The next meaningful product proof is not another generic engine subsystem or a visual editor. It is a Talos-driven production workflow in which Talos can:

1. discover GUIYI capabilities;
2. query a project semantically;
3. dry-run and apply typed domain changes;
4. consume diagnostics and repair invalid content;
5. validate and build;
6. preview a playable tactical-RPG slice;
7. leave a reviewable transaction/audit trail.

Roadmap and Backlog work should preferentially remove blockers to this workflow.

## Alpha success criteria

- A project can be initialized through CLI.
- Machine clients can discover capabilities without a static mega-prompt.
- A client can dry-run, apply, validate, query, and audit Stage changes.
- Stage documents compile into artifacts and load into Bevy ECS headlessly.
- Repeated Stage load/unload has an automated lifecycle test.
- Engine core has no tactical RPG, game-specific, provider-specific, or generic agent-planning dependency.
- CI executes formatting, compilation, linting, tests, docs, architecture, and link gates.
- The first-party AI path is demonstrably Talos → GUIYI Tools rather than a second GUIYI-owned agent loop.

## Non-goals before 1.0

- General-purpose DCC replacement.
- General animation, shader, or behavior-tree authoring.
- Rust dynamic-library plugins.
- Multiplayer collaborative editing.
- A full visual editor before the machine-facing command/query APIs and Talos production workflow stabilize.
- Embedding provider/model logic in GUIYI engine core.
- Building a second generic agent framework, conversation runtime, memory system, planner, or provider abstraction that duplicates Talos.
- Moving GUIYI domain safety, transaction, validation, or artifact authority into Talos.

# Implementation roadmap

The roadmap defines direction and sequencing. Only `docs/backlog/PRODUCT-BACKLOG.md` authorizes implementation.

## Current convergence path

Near-term work converges on one proof: **Talos can use GUIYI to create, repair, validate, build, and preview a playable tactical-RPG slice through machine-facing Tools.**

### Gate 0 — Direction and architecture

Owner: ENG-029.

Exit when Talos is the canonical first-party agent runtime, the published `talos-runtime` SDK is the approved dependency boundary, GUIYI/Talos/Bevy ownership is explicit, generic agent-runtime expansion inside GUIYI is prohibited, and ENG-014 has a clear integration contract.

### Gate 1 — Talos runtime SDK integration

Owner: ENG-014.

Use the crates.io `talos-runtime` SDK as GUIYI's only direct Talos dependency by default. Register GUIYI's live capabilities as Talos `AgentTool` adapters through `RuntimeBuilder`; do not use a catch-all Tool when per-capability schemas are available.

Exit when:

- the dependency is registry-sourced, intentionally versioned, and locked;
- no direct dependency on Talos implementation crates is required for the ordinary integration path;
- Talos can discover GUIYI capabilities and invoke at least one query and one mutating command through normal GUIYI permission, working-set, schema, dry-run, transaction, diagnostic, and persistence paths;
- recoverable Tool failures remain structured and the Talos runtime can continue;
- reusable GUIYI engine crates have no model/provider/planning/conversation responsibilities;
- current `AgentLoopDriver` and overlapping agent-session responsibilities have a documented compatibility/deprecation decision.

### Gate 2 — Semantic game-world interface

Owners: ENG-012 and ENG-013.

Build the semantic Project Graph and high-level Stage move/remove commands. The goal is to increase what Talos can understand and safely do, not to add another generic AI layer.

### Gate 3 — Tactical RPG vertical

Owners: ENG-020 through ENG-023 as they are promoted.

Preferred order:

1. navigation and reachability validation;
2. encounter document/compiler;
3. dialogue graph and condition/effect registry;
4. higher-level intent Tools such as `tactical.make_stage_playable`.

### Gate 4 — Talos-driven production demo

Owner: ENG-030 when promoted.

A real Talos session must discover capabilities, inspect a project, create or edit a playable tactical-RPG slice, respond to structured GUIYI diagnostics, repair at least one defect, validate, build, preview, and leave a reviewable transaction/audit summary. A mock driver or scripted Tool sequence is not sufficient product-level evidence.

### Gate 5 — Human workbench

Owner: ENG-024 after the production demo is credible.

The human surface is a client for visibility, transaction review, diagnostics, approval, rollback, and spatial inspection. It must not become a second authoring truth or a GUI-only feature path.

### Gate 6 — Stabilization and generality

Owners include ENG-025 and later stabilization Stories.

Add migrations, cache/performance, crash recovery, release packaging, compatibility policy, and a second non-wuxia sample.

---

## Historical capability phases

The original phases remain useful as capability history. The convergence gates above control near-term sequencing.

## Phase 0 — Repository and governance

Workspace, CI, ADRs, SOP, documentation, examples, and architecture gates.

## Phase 1 — Domain core

Stable IDs, schema, documents, diagnostics, semantic references, assets, and deterministic artifacts.

## Phase 2 — Mutation platform

Typed commands, dry-run, atomic state replacement, transaction diff, audit, undo, and redo.

## Phase 3 — Query and machine-facing execution platform

Structured queries, capability catalog, protocol, permissions, working sets, domain execution context, and the outer Talos runtime SDK adapter. Generic agent-runtime behavior belongs in Talos.

## Phase 4 — Stage vertical infrastructure

Stage authoring, validation, compiler, Bevy ECS runtime ownership, headless preview, and lifecycle regression tests.

## Phase 5 — Tactical RPG toolkit

Grid painting, navigation, actors, encounters, dialogue, quests, conditions/effects, party, inventory, and higher-level intent Tools.

## Phase 6 — Talos-driven production demo

Talos creates a playable slice through GUIYI Tools, repairs diagnostics, validates/builds/previews it, and produces a reviewable transaction report.

## Phase 7 — Human workbench

Visual project graph, Stage viewport, transaction review, diagnostics, approve, and rollback.

## Phase 8 — Production stabilization

Schema migrations, cache, performance, crash recovery, release packages, compatibility policy, and a second non-wuxia game sample.

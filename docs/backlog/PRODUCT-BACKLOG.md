# Product backlog

This is the only executable requirement list. Roadmap items and proposals do not authorize implementation.

GitHub Issue state was last synchronized on 2026-10-18. GitHub `OPEN` means unresolved; the `Status` below remains the implementation authorization.

## Current direction

The current product proof is a real Talos-driven workflow over GUIYI's game-development capabilities.

Ownership rule:

- Talos: model/provider, conversation/turn lifecycle, planning, context, generic memory/Skills, Tool-choice policy, retry and stopping.
- GUIYI: game/project semantics, Tool catalog, Queries, Commands, access scope, transactions, diagnostics, validation, build, artifacts, preview and Bevy runtime.
- Tactical RPG toolkit: reusable tactical-domain semantics and intent Tools.
- Game repositories: game-specific mechanics, content and progression.
- Human UI: a client of the same GUIYI machine-facing APIs, not a second authoring truth.

Current primary sequence:

```text
ENG-029 direction governance
    ↓
ENG-014 Talos runtime integration
    ↓
ENG-012 Project Graph + ENG-013 semantic Stage commands
    ↓
ENG-020..023 tactical RPG vertical
    ↓
ENG-030 Talos-driven playable-slice demo
    ↓
ENG-024 human review workbench
```

A different sequence requires an explicit backlog/change-control decision.

## In Progress

### ENG-029 Converge GUIYI AI boundary on Talos and add product-direction governance

- Type: Architecture / Governance
- Priority: P0
- Status: In Progress
- GitHub: [#50](https://github.com/wjhuang88/guiyi/issues/50)
- Goal: make Talos the canonical first-party AI agent runtime, keep GUIYI authoritative for game-domain execution, and prevent a second generic Agent framework from growing inside GUIYI.
- Gate: ADR/reference/roadmap/backlog/SOP agree on ownership; current generic loop/session overlap is explicitly transitional; ENG-014 owns the implementation migration.
- Architecture: `docs/decisions/ADR-0016-TALOS-AGENT-RUNTIME.md`.
- Non-goal: no Rust implementation or public API break in ENG-029.

## Ready — Current implementation sequence

### ENG-014 Talos runtime integration

- Type: Integration / Architecture
- Priority: P0
- Status: Ready after ENG-029
- Depends on: ENG-029.
- Goal: integrate a pinned `talos-runtime` through a thin outer adapter and stop growing GUIYI-owned generic Agent-loop behavior.
- Gate: Talos discovers GUIYI capabilities and executes a query plus a mutating command through normal GUIYI permission, working-set, schema, dry-run, transaction, diagnostic and persistence boundaries. Recoverable Tool failures remain structured. Reusable GUIYI engine crates gain no provider/model/planning/conversation logic.
- Migration: decide the compatibility/deprecation path for `AgentLoopDriver` and overlapping `AgentSession` lifecycle fields.
- Architecture: `docs/decisions/ADR-0016-TALOS-AGENT-RUNTIME.md`.

### ENG-012 Project graph index

- Status: Ready after ENG-014 baseline
- Goal: maintain an incremental semantic graph and expose reusable project relationships/impact to Talos and other clients.

### ENG-013 Stage object move/remove commands

- Status: Ready after ENG-014 baseline
- Goal: add high-level typed Stage edits with dry-run, transaction diff and bounds diagnostics.

## Done

| ID | Outcome | Evidence |
| --- | --- | --- |
| ENG-001 | Independent workspace | repository structure |
| ENG-002 | Governance documents | docs structure |
| ENG-003 | AI-native command/query/protocol foundations | Iteration 001 validation |
| ENG-004 | Stage vertical skeleton | Iteration 001 validation |
| ENG-005 | CLI, preview and JSONL Workbench | Iteration 001 validation |
| ENG-006 | Compiler-backed CI acceptance baseline | Issue #5 / PR #20 |
| ENG-007 | Project path sandbox | Issue #6 / `docs/validation/ENG-007.md` |
| ENG-008 | Session-aware Tool security boundary | Issue #7 / `docs/validation/ENG-008.md` |
| ENG-009 | Recoverable structured JSONL Tool errors | Issue #8 / `docs/validation/ENG-009.md` |
| ENG-010 | Schema-driven command validation | Issue #9 / `docs/validation/ENG-010.md` |
| ENG-011 | Crash-safe ProjectStorage and journal | Issue #10 / `docs/validation/ENG-011.md` |
| ENG-015 | Tool ID collision rejection | Issue #11 / `docs/validation/ENG-015.md` |
| ENG-016 | Strict BuildPipeline | Issue #12 / `docs/validation/ENG-016.md` |
| ENG-017 | Runtime artifact integrity | Issue #13 / `docs/validation/ENG-017.md` |

Completed Story details remain in their Issues, PRs, validation records and iteration records; this file stays the compact prioritization surface.

## Proposed — Synced issue intake

### ENG-018 Audited, permission-aware undo/redo

- Type: Command
- Priority: P1
- Status: Proposed
- GitHub: [#14](https://github.com/wjhuang88/guiyi/issues/14)
- Promotion gate: define journal reversal, dependent-transaction and restart policy.

### ENG-019 Close required test coverage gaps

- Type: Quality
- Priority: P1
- Status: Proposed
- GitHub: [#15](https://github.com/wjhuang88/guiyi/issues/15)
- Promotion gate: reconcile coverage already delivered by ENG-006 through ENG-018 and scope only remaining gaps.

### ENG-026 Enforce GitHub branch protection and Story/PR governance

- Type: Governance
- Priority: P1
- Status: Proposed
- GitHub: [#16](https://github.com/wjhuang88/guiyi/issues/16)
- Promotion gate: define a solo-maintainer exception that still preserves an audit trail and required checks.

### ENG-027 Fix reproducibility, licensing and CI supply-chain metadata

- Type: Maintenance
- Priority: P1
- Status: Proposed
- GitHub: [#17](https://github.com/wjhuang88/guiyi/issues/17)
- Promotion gate: separate required reproducibility fixes from optional new audit capabilities.

### ENG-028 Expand priority ADRs into implementation-grade decisions

- Type: Architecture
- Priority: P2
- Status: Proposed
- GitHub: [#18](https://github.com/wjhuang88/guiyi/issues/18)
- Promotion gate: link known violations to remediation Stories rather than describing temporary alpha behavior as compliant.

## Proposed — Product direction

- ENG-020 Navigation and reachability validation.
- ENG-021 Encounter document and compiler.
- ENG-022 Dialogue graph and condition/effect registry.
- ENG-023 Intent-level `tactical.make_stage_playable` Tool.
- ENG-024 Human transaction-review and spatial-inspection workbench.
- ENG-025 Content and save schema migration framework.
- ENG-030 Talos-driven playable-slice production demo. Promotion requires ENG-014 plus enough ENG-012/013/020-023 capability for a real Talos session to create or edit a tactical-RPG slice, repair at least one GUIYI diagnostic, validate, build, preview and produce a reviewable audit summary.

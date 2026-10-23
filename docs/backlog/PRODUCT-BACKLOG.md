# Product backlog

This is the only executable requirement list.

GitHub Issue state was last synchronized on 2026-10-23. GitHub `OPEN` means an Issue is unresolved; the `Status` field below remains the implementation authorization.

## Current direction and ownership

The current product proof is a real Talos-driven workflow over GUIYI's game-development capabilities.

Ownership rule:

- Talos: model/provider, conversation/turn lifecycle, planning, context, generic memory/Skills, Tool-choice policy, retry and stopping.
- GUIYI engine: game/project semantics, Tool catalog, Queries, Commands, access scope, transactions, diagnostics, validation, build, artifacts, preview and Bevy runtime.
- Tactical RPG toolkit: reusable tactical-domain semantics and intent Tools.
- Game repositories: game-specific mechanics, content and progression.
- Human UI: a client of the same GUIYI machine-facing APIs, not a second authoring truth.

First-party AI dependency rule:

- use the published crates.io `talos-runtime` SDK as the default and only direct Talos dependency;
- the currently approved and integrated SDK line is `0.10.1`;
- resolve through committed `Cargo.lock` and treat pre-1.0 Talos minor-version changes as explicit compatibility work;
- do not commit a Git dependency on Talos `main` or direct dependencies on Talos implementation crates merely for convenience;
- expose GUIYI's live capabilities as Talos `AgentTool` adapters through the public `RuntimeBuilder` surface.

Current primary sequence:

```text
ENG-013 semantic Stage commands
    ↓
ENG-020..023 tactical RPG vertical
    ↓
ENG-030 Talos-driven playable-slice demo
    ↓
ENG-024 human review workbench
```

ENG-014 established the accepted Talos Runtime SDK baseline and ENG-012 established the incremental semantic Project Graph baseline for this sequence. A different primary sequence requires an explicit Backlog/change-control decision. Parallel governance, security, quality, or reproducibility work is allowed when it protects this path without replacing the primary product outcome.

## Done — Direction convergence

### ENG-029 Converge GUIYI AI boundary on Talos and add product-direction governance

- Type: Architecture / Governance
- Priority: P0
- Status: Done
- GitHub: [#50](https://github.com/wjhuang88/guiyi/issues/50) — completed by PR #51
- Goal: make Talos the canonical first-party AI agent runtime, make `talos-runtime` the supported SDK dependency boundary, keep GUIYI authoritative for game-domain execution, and prevent a second generic Agent framework from growing inside GUIYI.
- Gate: ADR/reference/roadmap/backlog/SOP agree on ownership and registry-first SDK use; current generic loop/session overlap is explicitly transitional; ENG-014 owns the implementation migration.
- Architecture: `docs/decisions/ADR-0016-TALOS-AGENT-RUNTIME.md`.
- Evidence: `docs/validation/ENG-029.md`; PR #51 clean-head CI passed before merge.
- Non-goal: no Rust implementation or public API break in ENG-029.

## Done — Iteration 001

### ENG-001 Initialize independent workspace

- Status: Done
- Gate: workspace manifests and crate skeletons exist.

### ENG-002 Establish governance documents

- Status: Done
- Gate: reference, backlog, iteration, ADR, roadmap, SOP, template, proposal, and archive directories exist.

### ENG-003 Establish AI-native architecture

- Status: Done
- Gate: command, query, protocol, agent tools, and agent host crates provide executable foundations.
- Evidence: compiler-backed acceptance is recorded in `docs/iterations/iteration-001/VALIDATION.md`.

### ENG-004 Establish Stage vertical skeleton

- Status: Done
- Gate: Stage document compiles and loads/unloads through Bevy ECS with lifecycle tests.
- Evidence: compiler-backed acceptance is recorded in `docs/iterations/iteration-001/VALIDATION.md`.

### ENG-005 Establish CLI, preview, and JSONL workbench

- Status: Done
- Gate: source implementations, CLI, preview, Workbench, and examples pass the configured CI gates.
- Evidence: compiler-backed acceptance is recorded in `docs/iterations/iteration-001/VALIDATION.md`.

### ENG-006 Establish real Cargo/CI acceptance baseline and correct Iteration 001 status

- Type: Quality
- Priority: P0
- Status: Done
- GitHub: [#5](https://github.com/wjhuang88/guiyi/issues/5) — completed by PR #20
- Goal: run the complete Rust gate suite, repair all failures, commit reproducibility evidence, and make Iteration 001 status match executed validation.
- Gate: configured `rust` and `repository-gates` CI jobs pass; validation records exact results and `Cargo.lock` is committed.
- Evidence: CI run `30552961077` (run number 32) passed on 2026-07-30.

## Done — Immediate safety

### ENG-007 Sandbox all project paths and prevent filesystem escape

- Type: Security
- Priority: P0
- Status: Done
- GitHub: [#6](https://github.com/wjhuang88/guiyi/issues/6) — completed by PR #21
- Goal: route project reads and mutations through one validated project-relative filesystem boundary.
- Gate: absolute paths, parent traversal, platform prefixes, and symlink escapes are rejected with stable structured diagnostics, and adversarial tests prove external files remain unchanged.
- Evidence: `docs/validation/ENG-007.md`; CI run `30554258905` (run number 40) passed on 2026-07-30.

### ENG-008 Enforce AgentSession at every tool entry point

- Type: Security
- Priority: P0
- Status: Done
- GitHub: [#7](https://github.com/wjhuang88/guiyi/issues/7) — completed by PR #22
- Goal: enforce permissions, working sets, budgets, action history, and deterministic session status through one session-aware executor used by every client.
- Gate: JSONL Workbench, embedded Agent Host loops, and future clients cannot bypass permission, working-set, budget, status, or audit enforcement.
- Evidence: `docs/validation/ENG-008.md`; CI run `30556104345` (run number 50) passed on 2026-07-30.
- Direction note: ADR-0016 preserves the domain-security guarantees while ENG-014 owns migration of overlapping generic Agent lifecycle behavior to Talos.

### ENG-009 Return structured JSONL tool errors without terminating the Workbench

- Type: Protocol
- Priority: P0
- Status: Done
- GitHub: [#8](https://github.com/wjhuang88/guiyi/issues/8) — completed by PR #23
- Goal: return one structured result for every non-empty input line and keep the Workbench alive after recoverable protocol and tool errors.
- Gate: unknown tools, invalid input, permission denial, and validation failures preserve the call ID where parseable, emit stable error codes, keep stdout JSONL-only, and do not stop subsequent calls; host infrastructure failures still exit non-zero.
- Evidence: `docs/validation/ENG-009.md`; CI run `30557194781` (run number 61) passed on 2026-07-30.

### ENG-011 Add crash-safe ProjectStorage and a persistent transaction journal

- Type: Persistence
- Priority: P0
- Status: Done
- GitHub: [#10](https://github.com/wjhuang88/guiyi/issues/10) — completed by PR #25
- Goal: provide a single crash-safe project persistence layer for documents, manifests, autosaves, transaction snapshots, and audit history.
- Gate: failure-injection tests prove atomic multi-file commits, idempotent recovery, durable audit history, and preservation of the last known-good project state.
- Evidence: `docs/validation/ENG-011.md`; CI run `30559656903` (run number 98) passed on 2026-07-30.

### ENG-015 Reject command/query Tool ID collisions

- Type: Protocol
- Priority: P0
- Status: Done
- GitHub: [#11](https://github.com/wjhuang88/guiyi/issues/11) — completed by PR #26
- Goal: guarantee that every discoverable Tool ID maps to exactly one executable capability across command and query registries.
- Gate: duplicate IDs within or across registries fail catalog construction with a stable structured error, while unique catalogs remain deterministic.
- Evidence: `docs/validation/ENG-015.md`; CI run `30560400169` (run number 105) passed on 2026-07-30.

### ENG-016 Make BuildPipeline strict about validation and compiler coverage

- Type: Build
- Priority: P0
- Status: Done
- GitHub: [#12](https://github.com/wjhuang88/guiyi/issues/12) — completed by PR #27
- Goal: reject unresolved references, blocking diagnostics, and buildable documents without registered compilers before writing artifacts.
- Gate: library, CLI, and examples share strict build semantics; failed builds report every skipped document and produce no partial artifacts.
- Evidence: `docs/validation/ENG-016.md`; CI run `30561168272` (run number 112) passed on 2026-07-30.

### ENG-017 Validate artifact integrity before runtime load

- Type: Runtime
- Priority: P0
- Status: Done
- GitHub: [#13](https://github.com/wjhuang88/guiyi/issues/13) — completed by PR #28
- Goal: reject incompatible, corrupted, or semantically invalid artifacts before spawning ECS entities.
- Gate: type, version, checksum/integrity, duplicate ID, and object validation failures are atomic and structured; valid artifacts retain the repeated load/unload guarantee.
- Evidence: `docs/validation/ENG-017.md`; CI run `30562016077` (run number 118) passed on 2026-07-30.

## Done — Iteration 002

### ENG-010 Schema-driven command validation

- Type: Schema
- Priority: P1
- Status: Done
- GitHub: [#9](https://github.com/wjhuang88/guiyi/issues/9) — completed
- Implementation PR: #30
- Acceptance PRs: #31, #32, #33, #34, #35
- Governance closure PR: #38
- Goal: replace duplicated command parsing contracts with reusable schema-driven structural validation and stable field-path diagnostics.
- Gate: advertised schemas, runtime validation, constraints, schema versions, extensions, defaults, and built-in command behavior share one tested authority.
- Architecture: `docs/decisions/ADR-0015-SCHEMA-DRIVEN-COMMAND-VALIDATION.md`.
- Reference: `docs/reference/COMMAND-SCHEMA-CONTRACT.md`.
- Evidence: `docs/validation/ENG-010.md`; PR #35 CI run `30641388046` (run number 136) passed.

## Done — Iteration 003 Talos runtime baseline

### ENG-014 Talos runtime SDK integration

- Type: Integration / Architecture
- Priority: P0
- Status: Done
- GitHub: [#52](https://github.com/wjhuang88/guiyi/issues/52) — completed by PR #53
- Depends on: ENG-029 / #50 (Done).
- Goal: integrate the published `talos-runtime` SDK through a thin outer adapter and stop growing GUIYI-owned generic Agent-loop behavior.
- Dependency result: crates.io `talos-runtime = 0.10.1` is the only direct Talos dependency, with `default-features = false`, no optional Talos features, committed lockfile resolution, and an architecture gate preventing Talos dependencies from leaking into reusable GUIYI core/toolkit crates.
- Tool-adapter result: live GUIYI Tool descriptors become individual Talos `AgentTool`s through `RuntimeBuilder`, preserving Tool IDs, descriptions, and machine-readable schemas instead of using a catch-all wrapper.
- Execution result: Talos executes Queries and dry-run/applied Commands through the normal GUIYI permission, working-set, access-plan, schema, transaction, diagnostic, audit, and persistence boundaries; recoverable GUIYI Tool failures remain structured and Talos continues.
- Migration result: Talos owns first-party generic Agent/session lifecycle; `AgentLoopDriver` / `AgentHost::run` are compatibility-only with a sunset direction; Talos-facing code uses `GuiyiExecutionContext`; legacy `AgentBudget` is not duplicated as a second first-party stopping policy.
- Direct-client result: JSONL Workbench, CLI, examples, and tests remain usable without starting Talos.
- Architecture: `docs/decisions/ADR-0016-TALOS-AGENT-RUNTIME.md`.
- Evidence: `docs/validation/ENG-014.md`; PR #53 clean-head CI run 185 passed the complete suite before squash merge `5ce7dae09e55e0c40c8345d67e81bd38f44b67b4`.

## Done — Iteration 003 semantic graph baseline

### ENG-012 Project graph index

- Type: Query / Architecture
- Priority: P0
- Status: Done
- GitHub: [#54](https://github.com/wjhuang88/guiyi/issues/54) — implementation PR #55; final merge remains conditioned on clean-head CI for the closure-synchronized head.
- Depends on: ENG-014 / #52 (Done).
- Goal: maintain an incremental semantic graph rather than rebuilding it for each normal graph-backed Query, and expose reusable game-world relationships/impact to Talos and other clients.
- Source-of-truth result: `DocumentEnvelope.references` remains authoritative; `ProjectGraphIndex` is derived in-memory cache state and is not persisted as a second project truth.
- Query result: `project.references.find` and `project.impact.analyze` consume the maintained index on the normal `QueryExecutor` / `AgentHost` path while keeping their Tool IDs and schemas stable.
- Mutation result: `AgentHost` builds from complete authoritative state and refreshes only `TransactionReport.document_diffs` after successful applied Commands; dry-run, rejected, and failed Commands do not refresh graph state.
- Security result: restricted working sets neither return hidden reference sources nor traverse hidden intermediate graph nodes during impact analysis.
- Recovery result: state replacement and Talos persistence rollback invalidate derived graph state; direct clients have explicit rebuild/refresh/invalidate/snapshot APIs without Talos.
- Compatibility: `QueryExecutor::execute` now takes `&mut self` because it owns derived cache lifecycle; direct `QueryHandler::execute` remains a compatibility path and `execute_indexed` has a default delegate.
- Architecture: `docs/decisions/ADR-0011-PROJECT-GRAPH.md`; `docs/reference/PROJECT-GRAPH.md`.
- Evidence: `docs/validation/ENG-012.md`; PR #55 CI run 195 passed the complete implementation suite before final closure synchronization. Final merge remains gated on green CI for the current PR head.

## Ready — Current implementation sequence

### ENG-013 Stage object move/remove commands

- Status: Ready
- Goal: add high-level commands with dry-run, transaction diff, and bounds diagnostics so Talos can make semantic Stage edits without file-level mutation.

## Proposed — Synced GitHub issue intake

### ENG-018 Make undo and redo audited, permission-aware transaction operations

- Type: Command
- Priority: P1
- Status: Proposed
- GitHub: [#14](https://github.com/wjhuang88/guiyi/issues/14) — Open
- Dependencies: ENG-008 and ENG-011.
- Goal: expose undo and redo through the same permission, working-set, dry-run, transaction, persistence, and audit model as normal mutations.
- Promotion gate: select and document the journal reversal policy, including dependent transactions and restart behavior.

### ENG-019 Close required test coverage gaps

- Type: Quality
- Priority: P1
- Status: Proposed
- GitHub: [#15](https://github.com/wjhuang88/guiyi/issues/15) — Open
- Goal: map every test category required by `AGENTS.md` to executable unit, integration, process, lifecycle, failure, or golden tests.
- Promotion gate: reconcile coverage already owned by ENG-006 through ENG-018 and scope only the remaining gaps.

### ENG-026 Enforce GitHub branch protection and Story/PR governance

- Type: Governance
- Priority: P1
- Status: Proposed
- GitHub: [#16](https://github.com/wjhuang88/guiyi/issues/16) — Open
- Goal: enforce pull requests, required checks, branch protection, Story linkage, repository labels, and auditable exceptions in GitHub.
- Promotion gate: document a solo-maintainer review and emergency policy that cannot silently bypass required checks.

### ENG-027 Fix repository reproducibility, licensing, and CI supply-chain metadata

- Type: Maintenance
- Priority: P1
- Status: Proposed
- GitHub: [#17](https://github.com/wjhuang88/guiyi/issues/17) — Open
- Goal: correct repository metadata, pin the Rust toolchain and Actions, commit the lockfile, repair license files, and prevent package-manifest drift.
- Promotion gate: separate any optional new dependency-audit capability from the required reproducibility corrections.

### ENG-028 Expand accepted ADRs into implementation-grade decisions

- Type: Architecture
- Priority: P2
- Status: Proposed
- GitHub: [#18](https://github.com/wjhuang88/guiyi/issues/18) — Open
- Goal: add alternatives, invariants, failure behavior, compatibility, scaling assumptions, and exact evidence to the ADRs governing current safety and execution behavior.
- Promotion gate: link known code violations to remediation Stories instead of retroactively describing them as compliant.

## Proposed — Product direction

- ENG-020 Navigation and reachability validation.
- ENG-021 Encounter document and compiler.
- ENG-022 Dialogue graph and condition/effect registry.
- ENG-023 Intent-level `tactical.make_stage_playable` Tool.
- ENG-024 Human transaction-review and spatial-inspection workbench.
- ENG-025 Content and save schema migration framework.
- ENG-030 Talos-driven playable-slice production demo: a real Talos session creates or repairs a tactical-RPG slice, consumes at least one structured GUIYI diagnostic, validates, builds, previews, and produces a reviewable transaction/audit summary. Promotion requires ENG-014 plus enough ENG-012/013/020-023 capability to make the demo meaningful.

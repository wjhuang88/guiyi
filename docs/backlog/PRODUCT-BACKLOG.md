# Product backlog

This is the only executable requirement list.

GitHub Issue state was last synchronized on 2026-07-30. GitHub `OPEN` means only
that an Issue is unresolved; the `Status` field below remains the implementation
authorization.

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

## Ready — Immediate safety

### ENG-007 Sandbox all project paths and prevent filesystem escape

- Type: Security
- Priority: P0
- Status: Ready
- GitHub: [#6](https://github.com/wjhuang88/guiyi/issues/6) — Open
- Goal: route project reads and mutations through one validated project-relative filesystem boundary.
- Gate: absolute paths, parent traversal, platform prefixes, and symlink escapes are rejected with stable structured diagnostics, and adversarial tests prove external files remain unchanged.

### ENG-008 Enforce AgentSession at every tool entry point

- Type: Security
- Priority: P0
- Status: Ready
- GitHub: [#7](https://github.com/wjhuang88/guiyi/issues/7) — Open
- Goal: enforce permissions, working sets, budgets, action history, and deterministic session status through one session-aware executor used by every client.
- Gate: CLI, JSONL Workbench, Agent Host, and future clients cannot bypass permission, working-set, budget, or audit enforcement.

### ENG-009 Return structured JSONL tool errors without terminating the Workbench

- Type: Protocol
- Priority: P0
- Status: Ready
- GitHub: [#8](https://github.com/wjhuang88/guiyi/issues/8) — Open
- Goal: return one structured result for every input line and keep the Workbench alive after tool-level rejection or failure.
- Gate: unknown tools, invalid input, permission denial, and validation failures preserve the call ID, emit stable error codes, and do not corrupt stdout or stop subsequent calls.

### ENG-011 Add crash-safe ProjectStorage and a persistent transaction journal

- Type: Persistence
- Priority: P0
- Status: Ready
- GitHub: [#10](https://github.com/wjhuang88/guiyi/issues/10) — Open
- Goal: provide one crash-safe storage and journal boundary for documents, manifests, autosaves, transaction records, and recovery.
- Gate: failure-injection tests prove atomic multi-file commits, idempotent recovery, durable audit history, and preservation of the last known-good project state.

### ENG-015 Reject command/query Tool ID collisions

- Type: Protocol
- Priority: P0
- Status: Ready
- GitHub: [#11](https://github.com/wjhuang88/guiyi/issues/11) — Open
- Goal: guarantee that every discoverable Tool ID resolves to exactly one command or query.
- Gate: duplicate IDs within or across registries fail catalog construction with a stable structured error, while unique catalogs remain deterministic.

### ENG-016 Make BuildPipeline strict about validation and compiler coverage

- Type: Build
- Priority: P0
- Status: Ready
- GitHub: [#12](https://github.com/wjhuang88/guiyi/issues/12) — Open
- Goal: reject unresolved references, blocking diagnostics, and buildable documents without registered compilers before writing artifacts.
- Gate: library, CLI, and examples share strict build semantics; failed builds report every skipped document and produce no partial artifacts.

### ENG-017 Validate artifact integrity before runtime load

- Type: Runtime
- Priority: P0
- Status: Ready
- GitHub: [#13](https://github.com/wjhuang88/guiyi/issues/13) — Open
- Goal: reject incompatible, corrupted, or semantically invalid artifacts before spawning ECS entities.
- Gate: type, version, checksum/integrity, duplicate ID, and object validation failures are atomic and structured; valid artifacts retain the repeated load/unload guarantee.

## Ready — Iteration 002

### ENG-010 Schema-driven command validation

- Type: Schema
- Priority: P1
- Status: Ready
- GitHub: [#9](https://github.com/wjhuang88/guiyi/issues/9) — Open
- Goal: replace duplicated command parsing contracts with reusable schema-driven structural validation and stable field-path diagnostics.
- Gate: advertised schemas, runtime validation, constraints, schema versions, and built-in command behavior share one tested authority.

### ENG-012 Project graph index

- Status: Ready
- Goal: maintain an incremental semantic graph rather than rebuilding it for each query.

### ENG-013 Stage object move/remove commands

- Status: Ready
- Goal: add high-level commands with dry-run and bounds diagnostics.

### ENG-014 External agent-loop adapter

- Status: Ready
- Goal: integrate the separate agent-loop project behind `AgentLoopDriver` without provider coupling.

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
- ENG-023 Intent-level `tactical.make_stage_playable` tool.
- ENG-024 Visual transaction-review workbench.
- ENG-025 Content and save schema migration framework.

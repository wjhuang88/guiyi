# Product backlog

This is the only executable requirement list.

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

### ENG-004 Establish Stage vertical skeleton

- Status: Done
- Gate: Stage document compiles and loads/unloads through Bevy ECS with lifecycle tests.

### ENG-005 Establish CLI, preview, and JSONL workbench

- Status: Done
- Gate: source implementations and examples are included; Cargo execution remains pending in the package-generation environment.

## Ready — Iteration 002

### ENG-010 Schema-driven command validation

Replace hand-authored command parsing diagnostics with reusable schema validation.

### ENG-011 Persistent transaction journal

Persist transaction snapshots and audit records to `.agent-sessions/` with recovery tests.

### ENG-012 Project graph index

Maintain an incremental semantic graph rather than rebuilding it for each query.

### ENG-013 Stage object move/remove commands

Add high-level commands with dry-run and bounds diagnostics.

### ENG-014 External agent-loop adapter

Integrate the separate agent-loop project behind `AgentLoopDriver` without provider coupling.

## Proposed

- ENG-020 Navigation and reachability validation.
- ENG-021 Encounter document and compiler.
- ENG-022 Dialogue graph and condition/effect registry.
- ENG-023 Intent-level `tactical.make_stage_playable` tool.
- ENG-024 Visual transaction-review workbench.
- ENG-025 Content and save schema migration framework.

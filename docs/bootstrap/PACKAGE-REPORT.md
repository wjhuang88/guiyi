# Generated package report

## Package identity

- Name: GUIYI Engine
- Version: `0.1.0-alpha.1`
- Runtime baseline: Bevy `0.18.x`
- License: `MIT OR Apache-2.0`
- Generated: 2026-07-28

## Delivered systems

- Stable IDs and least-privilege permissions.
- Machine-readable schema and diagnostic foundations.
- Authoring Document and compiled Artifact envelopes.
- Compiler registry and build pipeline.
- Typed command registry with dry-run, atomic apply, diffs, and audit records.
- Query registry and semantic reference/impact graph.
- Agent Tool Catalog and JSONL protocol.
- Agent sessions, budgets, permissions, host routing, and an external loop trait.
- Asset manifest and slot indirection.
- Bevy ECS Stage ownership, loading, unloading, and lifecycle regression tests.
- Tactical RPG Stage document, high-level tools, validation, compiler, and runtime markers.
- CLI project initialization, doctor, capabilities, validation, and compilation commands.
- Headless preview runner and JSONL workbench.
- Minimal, tactical, and scripted-agent examples.
- CI, ADRs, SOP, roadmap, backlog, templates, and archived bootstrap guidance.

## Deliberate exclusions

- No direct dependency or adapter for the wuxia game repository.
- No game-specific concepts such as spirit altar, lifespan, lamp oil, medicine-body, factions, or concrete story content.
- No embedded model provider or concrete external agent loop.
- No full visual editor in the alpha package.
- No fabricated `Cargo.lock`; it must be produced by the first real Cargo resolution.

## Required acceptance action

Run `./scripts/first_run.sh` on a machine with the Rust stable toolchain and dependency access. Repair any compiler, formatter, Clippy, or Bevy API issue before accepting Iteration 001 as fully green.

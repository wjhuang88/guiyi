# Iteration 002 plan

## Goal

Make command capability schemas executable contracts, beginning with ENG-010 as the only active implementation Story.

## Authorization

- ENG-010 is In Progress.
- ENG-012, ENG-013, and ENG-014 remain Ready but are not authorized for parallel implementation until ENG-010 ownership is clear and a separate branch/PR is assigned.
- Proposed Stories remain non-executable.

## Active Story

### ENG-010 Schema-driven command validation

GitHub Issue: #9

Branch: `agent/ENG-010-schema-validation`

Architecture decision: `docs/decisions/ADR-0015-SCHEMA-DRIVEN-COMMAND-VALIDATION.md`

Validation record: `docs/validation/ENG-010.md`

## Work packages

### A. Schema authority

Owner: `crates/engine_schema`

Deliverables:

- versioned GUIYI schema dialect;
- validated constraint vocabulary;
- deterministic machine-readable rendering;
- recursive object/array support required by built-in inputs;
- structural validator;
- default normalization;
- stable schema-definition errors;
- unit, round-trip, invalid-definition, and golden-schema tests.

### B. Command execution integration

Owner: `crates/engine_command`

Deliverables:

- typed command input schema on every command handler;
- command registration failure when a schema definition is invalid;
- shared request preparation before document-access planning, semantic validation, and apply;
- structural failures returned as stable diagnostics with JSON Pointer field paths;
- no state, transaction, or audit mutation on structural failure.

### C. Built-in command migration

Owners:

- `crates/engine_command` for document commands;
- `toolkits/tactical_rpg/tools` for tactical commands.

Deliverables:

- remove hand-authored command input-schema JSON;
- represent Serde defaults in the schema authority;
- remove duplicated structural checks;
- retain domain-semantic validation;
- cover required fields, kinds, bounds, enums, arrays, nullability, and defaults with tests.

### D. Capability and client propagation

Owners:

- `crates/engine_agent_tools`;
- `crates/engine_agent_host`;
- `crates/engine_cli`;
- JSONL Workbench integration points.

Deliverables:

- generated schemas flow unchanged into `ToolDescriptor` and capability output;
- every migrated command advertises `x-schema-version`;
- structured validation failures remain JSONL-safe and do not terminate the Workbench;
- capability golden fixtures detect drift.

### E. Documentation and closure

Deliverables before Ready for review:

- current behavior documented in `docs/reference/COMMAND-SCHEMA-CONTRACT.md`;
- `docs/validation/ENG-010.md` completed with exact commands and CI run IDs;
- Backlog changed to Done only after clean-head CI is green;
- Issue #9 closed as completed by the squash-merged PR;
- obsolete `status:ready` label removed at closure.

## Required implementation order

1. Add failing schema-definition, rendering, normalization, and validation tests.
2. Implement the engine-schema authority without command integration.
3. Integrate request preparation into command registration/execution and AgentSession access planning.
4. Migrate engine built-in commands.
5. Migrate tactical commands.
6. Add capability golden output and process-level invalid-input coverage.
7. Run targeted gates, then the full repository gate suite.
8. Complete reference and validation documentation.
9. Obtain a green clean-head CI run before changing Backlog status or PR review state.

## Non-goals

- Query input-schema migration, except for a minimal compatibility refactor explicitly documented in the PR.
- Output-schema redesign.
- Full JSON Schema standards compliance.
- Third-party schema dependency adoption.
- ENG-012 project graph implementation.
- ENG-013 move/remove commands.
- ENG-014 external agent-loop adapter.
- Any Proposed Story.

## Quality gates

Targeted gates:

```bash
cargo test -p guiyi-engine-schema
cargo test -p guiyi-engine-command
cargo test -p guiyi-engine-agent-tools
cargo test -p guiyi-engine-agent-host
cargo test -p tactical-rpg-tools
cargo run -p guiyi-engine-cli -- capabilities --json
```

Full gates:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
python3 scripts/check_architecture.py
python3 scripts/check_docs.py
python3 scripts/validate_repository.py
```

Also run all repository examples required by CI.

## Exit gate

ENG-010 exits the iteration slice only when:

- all Issue #9 acceptance criteria have executable tests;
- advertised schemas and runtime validation are generated from the same authority;
- invalid input has stable code and JSON Pointer path;
- built-in and tactical commands are migrated;
- capability output exposes schema versions;
- no validation failure can mutate state or terminate the Workbench;
- targeted gates and the full clean-head CI are green;
- reference, validation, Backlog, PR, labels, and Issue state are synchronized.

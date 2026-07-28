# AGENTS.md

## 1. Purpose

This file defines the operating contract for AI coding agents working in the GUIYI Engine repository.

GUIYI Engine is an **AI-native, headless-first game development infrastructure built on Bevy**. Its primary clients are AI agents that operate through typed commands, structured queries, transactions, machine-readable diagnostics, validation gates, build tools, and preview tools.

Human-facing interfaces are clients of the same command/query platform. They are not separate sources of truth.

These instructions apply to the entire repository unless a more specific nested `AGENTS.md` explicitly overrides part of them.

---

## 2. Mission and product boundary

The engine is intended to support game projects with requirements such as:

- Data-driven content authoring
- Stage-based exploration
- Tactical or turn-based gameplay
- Actors, triggers, encounters, dialogue, quests, and world variables
- Deterministic validation and compilation
- Headless preview and automated testing
- AI-agent-driven content production

The engine is not a game-specific codebase.

The following concepts must not enter engine-core crates:

- Specific characters, locations, factions, stories, or lore
- Spirit altar, lifespan, lamp oil, medicine body, or other wuxia-specific mechanics
- A concrete game's `GameState`, `WorldState`, progression model, or save format
- Project-specific content IDs or hard-coded asset paths

The historical wuxia project is a requirements and implementation reference only. It is not a dependency and does not require compatibility unless a Backlog Story explicitly approves such work.

---

## 3. Sources of truth

Use the following source hierarchy.

1. `docs/backlog/PRODUCT-BACKLOG.md`
   - The only executable requirement list.
   - No implementation work starts without a Story ID.

2. `docs/decisions/`
   - Approved architecture decisions.
   - A code change that contradicts an accepted ADR requires a new ADR or an explicit superseding decision.

3. `docs/reference/`
   - Current, approved system behavior and architecture.
   - Update these documents when implementation changes the current truth.

4. `docs/sop/`
   - Required development, iteration, agent-operation, quality, and release procedures.

5. `docs/roadmap/`
   - Direction and sequencing, but not authorization to implement a task.

6. `docs/proposals/`
   - Unapproved ideas.
   - A proposal is not an implementation instruction.

7. `docs/archive/`
   - Historical material only.
   - Never treat archived documents as current truth unless a current Story explicitly references them.

When documents conflict, stop expanding scope. Follow the highest source above and record the inconsistency in the active Story or validation report.

---

## 4. Repository map

Expected high-level ownership:

```text
crates/
├── engine_core/          Stable IDs, versions, permissions, fundamental types
├── engine_schema/        Machine-readable type and field schemas
├── engine_content/       Documents, artifacts, compilers, content envelopes
├── engine_command/       Mutating commands, transactions, diffs, audit behavior
├── engine_query/         Read-only semantic queries and impact analysis
├── engine_validation/    Diagnostics, validation reports, stable error codes
├── engine_runtime/       Bevy ECS runtime loading and lifecycle ownership
├── engine_asset/         Asset identity, manifests, dependencies, slots
├── engine_build/         Deterministic build orchestration and reports
├── engine_protocol/      Tool-call and tool-result wire contracts
├── engine_agent_host/    Sessions, permissions, budgets, agent-loop integration
├── engine_agent_tools/   Engine capability exposure and tool descriptors
├── engine_preview/       Headless preview runner
├── engine_cli/           Human- and machine-facing command-line entry points
├── engine_editor/        Optional visual client of command/query APIs
└── engine_testkit/       Shared fixtures, probes, builders, and test utilities

toolkits/
└── tactical_rpg/
    ├── content/          Genre-level documents and definitions
    ├── runtime/          Genre-level runtime behavior
    ├── tools/            Genre-level commands, queries, and intent tools
    └── validation/       Genre-level diagnostics and validation

examples/
├── minimal_project/
├── tactical_demo/
└── mock_agent_workflow/
```

Do not move responsibilities across these boundaries without a Story and, when architectural, an ADR.

---

## 5. Dependency direction

The intended dependency direction is:

```text
Bevy ECS
   ↑
engine_core / engine_schema / engine_content / engine_validation
   ↑
engine_command / engine_query / engine_asset / engine_build / engine_runtime
   ↑
engine_protocol / engine_agent_tools / engine_agent_host / engine_preview
   ↑
engine_cli / engine_editor / tactical_rpg toolkit
   ↑
game-specific repositories
```

Mandatory rules:

- `engine_core` must remain independent of tactical-RPG and game-specific crates.
- Engine crates must never depend on a game repository.
- Runtime crates must not depend on editor UI crates.
- Tactical-RPG crates may depend on engine crates; the reverse is forbidden.
- Game-specific concepts belong in external extensions.
- Bevy `Entity` must never be used as a persistent or cross-document identity.
- Authoring documents, compiled artifacts, runtime instances, and persistent game state must remain separate.

Run:

```bash
python3 scripts/check_architecture.py
```

after modifying manifests or crate dependencies.

---

## 6. Mandatory work intake

Before modifying code or executable project configuration:

1. Identify a Story ID in `docs/backlog/PRODUCT-BACKLOG.md`.
2. Confirm that the Story is `Ready` or `In Progress`.
3. Read all referenced ADR and reference documents.
4. Run the baseline gates relevant to the task.
5. Record any pre-existing failure before changing files.

Do not start implementation from:

- A roadmap bullet
- An archived design
- A proposal
- An informal comment
- A TODO found in code
- A convenient refactor opportunity

When necessary work is outside the active Story:

- Do not silently expand scope.
- Add or propose a separate backlog item.
- Keep the current patch limited to the approved objective.

Documentation-only typo and broken-link fixes may use a documentation Story or an explicitly approved maintenance task.

---

## 7. Standard implementation loop

Follow this sequence for every implementation Story:

```text
Read Story
→ Read ADR/reference
→ Run baseline
→ Add failing test or reproducible validation
→ Implement smallest vertical change
→ Run local gates
→ Update tool/schema/diagnostic metadata
→ Update reference documentation
→ Run full gates
→ Produce validation and rollback notes
```

Preferred iteration behavior:

- One coherent objective
- One to three primary Stories per iteration
- Small vertical slices
- Failure paths implemented with the success path
- Lifecycle repetition tests for stateful runtime work
- Structured evidence rather than narrative claims

A Story is not Done merely because the happy path works.

---

## 8. AI-native implementation rules

### 8.1 Machine-first capability

Every core capability must be callable without a GUI.

A feature is incomplete when it can only be used through:

- Mouse interaction
- Editor-only state
- Direct manual file editing
- Debug-only buttons
- An undocumented internal function

The expected access order is:

```text
Typed domain API
→ Command or Query Registry
→ Protocol and Tool Catalog
→ CLI / Agent Host / Editor client
```

### 8.2 Commands are the mutation authority

All authoring-state mutations must use registered typed commands.

Do not make the normal mutation path:

- Editing JSON/RON text directly
- Mutating a document structure from UI code
- Writing directly to ECS runtime state as authoring truth
- Bypassing the transaction executor
- Adding a second private command path for one client

Every mutating command must provide:

- Stable command ID
- Purpose and behavioral description
- Machine-readable input schema
- Machine-readable output schema
- Required permissions
- Declared side effects
- Structured errors
- Atomic application
- Dry-run support
- A transaction diff
- Audit information
- Rollback or undo behavior where applicable

High-level tools must delegate to domain or primitive commands. They must not bypass validation and transactions.

### 8.3 Queries are read-only

Queries must not mutate documents, artifacts, runtime state, caches with semantic effects, or session permissions.

Queries should expose semantic project state, including:

- Documents and objects
- References
- Dependencies
- Impact analysis
- Diagnostics
- Available tools
- Schemas
- Build and preview status

Prefer semantic queries over arbitrary file scans.

### 8.4 Diagnostics are authoritative

Machine-readable diagnostics are the error source of truth.

Every diagnostic intended for automation should include, where applicable:

- Stable code
- Severity
- Human-readable message
- Document ID
- Object ID
- Field path
- Related IDs
- Suggested actions or related tools
- Whether automatic repair is safe

Do not rely on log text as the only error channel.

Do not silently ignore malformed or unsupported content.

### 8.5 Capability discovery

Agents should discover capabilities from the live catalog.

Preferred commands:

```bash
cargo run -p guiyi-engine-cli -- capabilities --json
cargo run -p guiyi-engine-workbench -- --project <project-path>
```

Do not duplicate the full tool API in long-lived prompts or documentation when it can be generated from descriptors.

### 8.6 Agent-loop boundary

Model-provider calls, prompting strategy, context compression, planning policy, and retry policy do not belong in engine core.

External agent loops integrate through the approved host boundary, such as `AgentLoopDriver`.

The engine owns:

- Tool catalog
- Session state
- Working set
- Permissions
- Action budgets
- Command and query execution
- Validation feedback
- Transaction history

The external loop owns:

- Model invocation
- Planning strategy
- Context selection/compression
- Tool-choice policy
- Retry and stopping strategy

Do not add provider credentials, provider SDK assumptions, or model-specific prompts to engine-core crates.

---

## 9. Agent session and permission discipline

Use least privilege.

A content-authoring agent should normally receive only:

- Read
- Plan
- DryRun
- EditContent
- RunValidation
- RunBuild
- RunPreview

It should not receive by default:

- EditCode
- EditSchema
- RunExternalProcess
- CommitChanges
- Publish

Before broad or destructive actions:

1. Confirm the session working set.
2. Confirm permissions.
3. Perform a dry run.
4. Inspect the diff and diagnostic delta.
5. Apply one coherent transaction.
6. Validate immediately.
7. Repair or roll back if errors increase.

Respect action budgets and stop conditions. Do not evade them by batching unrelated operations into one oversized tool call.

Never add a code path that grants implicit full permissions because the caller is “trusted” or “local.”

---

## 10. Document, artifact, runtime, and persistence boundaries

Maintain both separations:

```text
Document → Artifact → Runtime Instance
Definition → Session → Persistent
```

### Document

- Authoring source of truth
- Stable IDs
- Human- and machine-reviewable
- Versioned and validated
- May contain authoring metadata

### Artifact

- Deterministically compiled
- References resolved
- Editor-only metadata removed
- Suitable for runtime loading
- Rebuildable from clean authoring inputs

### Runtime instance

- Short-lived ECS entities
- Owned by an explicit runtime or Stage instance
- Safe to load and unload repeatedly
- Never the source of authoring truth

### Persistent state

- Explicitly modeled
- Uses stable identities
- Versioned independently from content schemas
- Never a raw serialized ECS World

Do not serialize Bevy `Entity` IDs as content or save references.

Do not let runtime mutations silently rewrite source documents.

---

## 11. Rust and Bevy coding standards

### Rust

- Keep code warning-free under strict Clippy.
- Prefer explicit domain types over unstructured strings and maps.
- Use `Result` and structured error types for recoverable failure.
- Avoid `unwrap`, `expect`, and indexing in non-test code unless an invariant is local, proven, and documented.
- Keep public APIs documented.
- Keep serialization formats deterministic when artifact or protocol output is involved.
- Avoid adding dependencies without a concrete Story need.
- Put shared dependency versions in the workspace when appropriate.
- Preserve object safety and serialization boundaries required by tool registries.
- Prefer small, testable units over large manager types.

### Bevy

- Declare runtime ownership explicitly.
- Use scoped marker components for cleanup.
- Test repeated state entry, loading, unloading, and error recovery.
- Avoid global-resource resets on ordinary state transitions.
- Do not duplicate authoritative state across Resources and Components.
- Keep authoring and editor types outside runtime-only crates where practical.
- Headless tests should use minimal plugin sets.
- Avoid window/render dependencies in content, schema, command, query, and validation crates.

---

## 12. Tool and protocol standards

Every registered tool must declare:

- Stable tool ID
- Title
- Purpose
- Input schema
- Output schema
- Required permissions
- Side effects
- Related tools
- Structured failure behavior
- Dry-run support when mutating

Protocol behavior must be:

- Deterministic
- Machine-readable
- Versionable
- Independent from terminal formatting
- Explicit about success, failure, and partial results

For JSONL workbench operations:

- One tool call per input line
- One tool result per output line
- No human prose on stdout that would corrupt the protocol
- Operational logs go to stderr or structured result fields
- Invalid input returns a structured error rather than terminating the process unexpectedly

Add or update protocol examples when public tool behavior changes.

---

## 13. Testing requirements by change type

### Core or schema changes

Require:

- Unit tests
- Round-trip serialization tests
- Invalid-input tests
- Public API documentation
- Compatibility or migration impact notes

### Command changes

Require:

- Success test
- Validation-failure test
- Dry-run test
- Atomicity test
- Diff test
- Permission test
- Audit or transaction-result test

### Query changes

Require:

- Correct-result test
- Missing-data test
- Read-only behavior
- Stable structured output where public

### Runtime changes

Require:

- Lifecycle test
- Repeated load/unload or enter/exit test
- Invalid-content rejection
- Headless integration test
- Entity/resource cleanup verification

### Agent-host changes

Require:

- Permission denial test
- Budget enforcement test
- Working-set enforcement test
- Driver error test
- Stop/completion behavior test
- No provider coupling in engine core

### Content-schema changes

Require:

- Golden data
- Round-trip test
- Strict invalid-input diagnostics
- Migration impact
- Reference documentation update

### CLI or protocol changes

Require:

- Exit-code test
- JSON output test
- Noninteractive behavior
- stderr/stdout separation
- Documentation or example update

### Documentation changes

Require:

- Link check
- Consistency with Backlog and ADR status
- No executable tasks introduced outside the Backlog

---

## 14. Required quality gates

Before declaring a Story complete, run:

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

The convenience entry point is:

```bash
./scripts/first_run.sh
```

Also run task-specific examples, CLI commands, protocol fixtures, build commands, or previews required by the Story.

Do not state that a gate passed unless it was actually run in the current environment.

When a toolchain or dependency is unavailable:

- Run every gate that is available.
- Record the exact unavailable command.
- Record the environmental cause.
- Do not mark the complete gate set as passed.
- Leave clear first-run instructions for the next environment.

---

## 15. Documentation obligations

Update documentation in the same change when behavior changes.

At minimum consider:

- `docs/reference/`
- `docs/decisions/`
- `docs/backlog/PRODUCT-BACKLOG.md`
- Active iteration plan and validation report
- Protocol examples
- Root `README.md`
- `CHANGELOG.md`

Rules:

- Reference documents describe current approved truth.
- ADRs explain architectural decisions, not routine implementation notes.
- Backlog status must match implementation status.
- Unimplemented work must not be marked Done.
- New scope discovered during implementation becomes a separate Story.
- Breaking changes require migration or upgrade notes.

---

## 16. Branch, commit, and pull-request rules

Branch names:

```text
feature/ENG-xxx-short-name
fix/ENG-xxx-short-name
docs/ENG-xxx-short-name
```

Commit examples:

```text
feat(command): ENG-021 add transactional dry-run
test(runtime): ENG-034 cover repeated stage unload
docs(adr): ENG-009 approve agent permission model
```

A pull request must include:

- Story ID
- Summary
- Scope and non-goals
- Validation commands and exit codes
- Machine-facing API impact
- Migration and compatibility impact
- Risks
- Rollback method
- Documentation changes
- Remaining diagnostics or known limitations

Do not mix unrelated Stories in one pull request.

Do not create commits, push branches, or publish releases unless the active session has explicit permission.

---

## 17. Forbidden patterns

Do not introduce any of the following without an approved ADR that explicitly replaces this rule:

- Game-specific concepts in engine-core crates
- Direct document mutation outside commands
- A GUI-only authoring feature
- Agent automation based on mouse or keyboard simulation
- Provider-specific AI logic in engine core
- Unstructured logs as the sole diagnostics API
- Persistent references based on Bevy `Entity`
- Runtime ECS World snapshots as content truth
- Silent fallback to empty/default content after validation errors
- Multiple authoritative copies of the same state
- Editor UI owning domain state independently
- Commands without dry-run or atomicity
- Queries with semantic side effects
- Implicit full agent permissions
- Broad refactors unrelated to the active Story
- Hidden compatibility behavior with the historical wuxia project
- Claims that unexecuted tests passed

---

## 18. Completion report format

At the end of an implementation task, provide:

```text
Story:
- ENG-XXX

Implemented:
- ...

Changed machine-facing APIs:
- ...

Validation:
- command -> exit code/result

Diagnostics:
- before:
- after:

Documentation:
- ...

Compatibility or migration:
- ...

Risks and limitations:
- ...

Rollback:
- ...

Backlog follow-ups:
- ...
```

For agent-authored content transactions, also include:

- Agent session ID
- Transaction IDs
- Changed document IDs
- Dry-run result
- Validation-gate result
- Remaining diagnostics

---

## 19. Current repository status note

This repository was initially generated in an environment that did not have the Rust toolchain available. Static repository, architecture, documentation, protocol, and ZIP-integrity checks were executed during package generation, but the full Cargo gate suite must be executed in a Rust-enabled environment before treating the initial alpha as compiler-verified.

Before beginning the first implementation Story in a fresh checkout, run:

```bash
./scripts/first_run.sh
```

Record any baseline failure before modifying files.

---

## 20. Default agent behavior

When no more specific instruction is provided:

1. Stay within the active Story.
2. Prefer semantic queries over arbitrary scans.
3. Prefer typed commands over direct edits.
4. Use least privilege.
5. Dry-run broad mutations.
6. Make one coherent transaction at a time.
7. Validate immediately after mutation.
8. Roll back when diagnostics worsen unexpectedly.
9. Keep engine core independent from toolkits and games.
10. Report evidence, not confidence.

# Iteration 003 plan

## Goal

Converge GUIYI on the published Talos runtime SDK, then increase the semantic game-development capabilities that Talos and direct clients can safely use without weakening GUIYI's domain execution authority.

At the product-surface level, this iteration advances an Agent-first development flow: the Agent interaction layer should perform the normal semantic authoring, validation, build, preview, and repair work that a traditional engine Editor would otherwise mediate. Human UI remains a secondary client of the same machine-facing platform.

## Authorization

- ENG-029 is Done and merged through PR #51 (`4dc57fe7203795e20b45042dfca22ceeb8adae0c`).
- ENG-014 / #52 is Done and squash-merged through PR #53 (`5ce7dae09e55e0c40c8345d67e81bd38f44b67b4`).
- ENG-012 / #54 is Done and squash-merged through PR #55 (`eed94d8d7d6a63b73c103198af1c33564ffa6bfe`). Final clean-head CI run 207 passed before merge.
- ENG-013 / #57 is Done and squash-merged through PR #58 (`cb6c2a5016d04fd59e97926ae62b416bff6b41a0`). Final clean-head CI run 234 passed before merge.
- ENG-020 / #59 is Done and squash-merged through PR #61 (`80eaee3338e5a772b1b56a2b44e97f8eef73afb4`). Final clean-head CI run 248 passed before merge.
- ENG-021 / #62 implementation acceptance is complete on `feature/ENG-021-encounter-document-compiler` through draft PR #64; merge remains gated on final closure-head CI.
- ENG-022..023 remain Proposed and non-executable until separately promoted.
- Other Proposed Stories remain non-executable.

## Completed baseline — ENG-014

ENG-014 established crates.io `talos-runtime = 0.10.1` as the only direct Talos dependency, exposed live GUIYI Tool descriptors as individual Talos `AgentTool`s, kept GUIYI authoritative for domain security/transactions/persistence, and preserved direct Talos-independent clients.

Architecture: `docs/decisions/ADR-0016-TALOS-AGENT-RUNTIME.md`.

Validation: `docs/validation/ENG-014.md`; clean-head run 185 passed before merge.

## Completed baseline — ENG-012

ENG-012 established `ProjectGraphIndex` as a reusable incremental derived semantic-reference index while keeping `DocumentEnvelope.references` authoritative.

Delivered guarantees include:

- `project.references.find` and `project.impact.analyze` use the maintained index on the normal executor/host path;
- applied transaction diffs refresh changed source Documents only;
- dry-run/rejected/failed Commands do not mutate graph state;
- working-set restrictions block hidden results and hidden-node traversal;
- state replacement/persistence rollback invalidates derived graph state;
- direct clients can rebuild/refresh/invalidate/snapshot without Talos.

Architecture/reference: `docs/decisions/ADR-0011-PROJECT-GRAPH.md`, `docs/reference/PROJECT-GRAPH.md`.

Validation: `docs/validation/ENG-012.md`; final clean-head run 207 passed before PR #55 squash-merged as `eed94d8d7d6a63b73c103198af1c33564ffa6bfe`.

## Completed Story slice — ENG-013

### ENG-013 Add semantic Stage object move/remove Commands

GitHub Issue: #57

Branch: `feature/ENG-013-stage-object-move-remove`

Merged PR: #58

Validation record: `docs/validation/ENG-013.md`

Architecture authorities:

- `docs/decisions/ADR-0007-TYPED-COMMANDS.md`
- `docs/decisions/ADR-0008-TRANSACTIONS.md`
- `docs/decisions/ADR-0010-DIAGNOSTICS.md`

Reference: `docs/reference/STAGE-MODEL.md`.

### Problem

The tactical toolkit could create Stages and place Actors, spawn points, triggers, and connections, but lacked semantic move/remove operations for existing stable Stage objects. That gap pushed a normal authoring action toward lower-level document mutation rather than the Agent-first semantic Tool surface.

### Delivered work packages

#### A. Semantic Commands

Implemented and registered:

- `stage.move_object(stage_id, object_id, q, r)`;
- `stage.remove_object(stage_id, object_id)`.

Both commands use the normal schema, permission, document-access, dry-run, transaction, diff, audit, persistence, and AgentHost path. No private Editor or Talos-specific mutation route was introduced.

#### B. Shared Stage validation

Candidate mutations reuse `validate_stage`.

- invalid moves reuse `STAGE_OBJECT_OUT_OF_BOUNDS`;
- unknown stable object IDs use `STAGE_OBJECT_NOT_FOUND` with Stage/object location;
- non-blocking candidate diagnostics such as `STAGE_NO_SPAWN_POINT` remain in transaction reports.

#### C. Stable identity and semantic references

Moving an object changes only its coordinate while preserving stable ID, object kind, properties, and references.

Removing an Actor regenerates the Stage envelope without that object's `actor_definition` reference. The normal ENG-012 applied-transaction refresh updates Project Graph reference results; there is no second graph mutation path.

#### D. Security and capability discovery

The two new Commands are live schema-bearing catalog capabilities, so the ENG-014 Talos adapter discovers them automatically. `AgentHost` still owns GUIYI permission and working-set enforcement, and direct non-Talos clients remain supported.

The CLI public command-schema golden was intentionally expanded from 8 to 10 Commands, proving the new capabilities are part of machine-facing discovery rather than hidden toolkit helpers.

#### E. Agent-first development surface

ENG-013 is a semantic capability-density Story. A normal “move/remove a Stage object” development task is now possible through discoverable typed Tools rather than an editor-only gesture or direct file patch. This is the expected pattern for later tactical capabilities: semantic Tool coverage precedes optional visual affordances.

The broader product rule was synchronized into `docs/reference/AI-NATIVE-PLATFORM.md`, `docs/sop/DIRECTION-GOVERNANCE.md`, and the Product Backlog: the Agent interaction layer is the primary development surface, while a future visual Editor remains a secondary client of the same authoritative APIs.

## Non-goals preserved

- No Stage connection edit/remove command.
- No object rename or batch move/remove.
- No occupancy, collision, pathfinding, reachability, encounter, dialogue, or ENG-020+ tactical rules.
- No visual Editor authoring path.
- No generic document-patch replacement for semantic commands.
- No Talos provider/planner/session/runtime change.
- No second persisted Project Graph truth.

## Validation

Required full suite:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
python3 scripts/check_architecture.py
python3 scripts/check_docs.py
python3 scripts/validate_repository.py
python3 scripts/static_rust_check.py
cargo run -p guiyi-example-minimal-project
cargo run -p guiyi-example-tactical-demo
cargo run -p guiyi-example-mock-agent-workflow
```

Validation history:

- run 210 exposed an internal-module qualification failure in `static_rust_check.py` and rustfmt drift;
- run 214 passed repository gates, fmt, check, and strict Clippy, then correctly failed the CLI capability golden because the live catalog grew from 8 to 10 command schemas;
- implementation-head run 228 (`40723376393`) on `10fb887f0c825b8fd13a9b2c929de6a24b1e04652` passed the complete suite;
- final clean-head run 234 (`40724726043`) on `5f7408fa55e7aad30824ed4bbae6d2e9faa230b9` passed the complete suite before PR #58 squash-merged as `cb6c2a5016d04fd59e97926ae62b416bff6b41a0`.

Local Cargo execution is unavailable in the current execution environment; no local gate is claimed as passed.

## ENG-013 exit gate

Implementation acceptance is satisfied for:

- move apply preserving object identity/non-position fields and producing an applied diff;
- move dry-run producing preview evidence without authoritative mutation;
- structured out-of-bounds rejection;
- remove apply deleting the stable object and reporting candidate warnings;
- atomic `STAGE_OBJECT_NOT_FOUND` failure;
- Actor removal updating semantic reference/Project Graph query results through the normal AgentHost transaction path;
- authoritative permission and working-set denial;
- live schema-bearing capability discovery through the normal catalog and direct clients;
- Agent-first/reference/backlog/validation documentation synchronization.

ENG-013 is fully closed.

## Completed Story slice — ENG-020

### ENG-020 Add tactical navigation map and reachability validation

GitHub Issue: #59

Primary owner: GUIYI tactical-RPG toolkit.

ENG-020 established backward-compatible Stage blocked-cell authoring data, deterministic Hex/Square reachability, structured navigation diagnostics, `stage.set_navigation_blocked`, and `stage.reachability` as machine-facing Agent-first capabilities.

Delivered guarantees include:

- defaulted `StageNavigation.blocked` data and Stage artifact metadata preservation;
- deterministic tactical-content BFS for HexAxial/Square only;
- stable navigation diagnostics including bounds, duplicate blocked cells, blocked spawns, and unreachable objects;
- normal schema/permission/access/dry-run/transaction/diff/audit/persistence behavior for navigation mutation;
- structured reachability Query results;
- AgentHost permission/working-set enforcement and live catalog discovery;
- command capability golden growth from 10 to 11 Commands.

Final clean-head run 248 (`41057321015`) on `70aabd59b0bfe8da138e148ab9ccbcb4600e03f7` passed the complete suite before PR #61 squash-merged as `80eaee3338e5a772b1b56a2b44e97f8eef73afb4`. Full evidence, compatibility, risks, and rollback are recorded in `docs/validation/ENG-020.md`.

## Closing Story slice — ENG-021

### ENG-021 Add Encounter document, compiler, and semantic authoring commands

GitHub Issue: #62

Branch: `feature/ENG-021-encounter-document-compiler`

Implementation PR: #64

Validation record: `docs/validation/ENG-021.md`

Primary owner: GUIYI tactical-RPG toolkit.

ENG-021 makes Encounter composition a typed machine-facing development capability. It adds additive Encounter authoring/artifact types, deterministic compilation, project-aware validation, semantic Stage/actor/spawn references, and three schema-bearing authoring Commands without introducing a visual-Editor-only path or runtime combat system.

### Delivered ENG-021 work packages

#### A. Encounter content and compiler

- `EncounterDocument` owns one Stage and ordered stable participants;
- each participant owns stable ID, actor-definition ID, spawn-point ID, and side;
- `EncounterDocument::to_envelope` emits deterministic `encounter_stage`, `encounter_actor_definition`, and `encounter_spawn_point` references;
- `EncounterCompiler` produces `tactical.encounter.artifact` through the normal source-hash/integrity path and rejects invalid local authoring structure.

#### B. Shared validation

Local and project-aware validators emit stable Encounter diagnostics for empty names, incremental zero-participant state, duplicate participant IDs, empty side, missing/wrong Stage, missing spawn, and invalid spawn kind. Generic unresolved document references remain owned by the existing strict reference/build gates.

#### C. Semantic Commands

Implemented and registered:

- `encounter.create(encounter_id, name, stage_id)`;
- `encounter.add_participant(encounter_id, stage_id, participant_id, actor_definition, spawn_point, side)`;
- `encounter.remove_participant(encounter_id, participant_id)`.

All use the normal schema, permission, document-access, dry-run, transaction, diff, audit, persistence, and AgentHost path. `add_participant` includes `stage_id` as an access-plan assertion that must match persisted Encounter Stage, ensuring working-set checks cover every document read without creating a second Stage truth.

#### D. Project Graph and security

Encounter participant mutations regenerate authoritative `DocumentEnvelope.references`; the normal ENG-012 applied-transaction refresh makes Project Graph Queries observe add/remove relationships. Read-only callers are rejected, and excluding the Stage from the working set rejects participant addition even when Encounter and actor-definition documents are visible.

#### E. Build / CLI / executable example

CLI validation reuses project-aware Encounter validation. CLI and tactical-demo builds register Stage + Encounter compilers while marking actor definitions authoring-only. Strict build coverage proves valid Stage+Encounter composition produces both artifacts, while missing actor definitions reuse `BUILD_UNRESOLVED_REFERENCE` and produce no partial artifacts. The tactical demo creates Encounter content through semantic Commands and compiles it in the normal executable path.

#### F. Capability discovery / Agent-first surface

The three new Commands are live schema-bearing catalog capabilities, so the existing ENG-014 Talos adapter discovers them automatically. The command-schema golden grows from 11 to 14 Commands. Existing document and Project Graph Queries remain the read side rather than introducing a redundant Encounter projection.

### ENG-021 validation history

- governance run 250 (`41088787964`) passed the complete suite before Ready promotion PR #63 merged as `1335f3eee36835bfd02048a24d7b35785f913f79`;
- early runs 252 / 258 stopped at rustfmt while the implementation was still being assembled and are formatting repair evidence only;
- run 264 (`41091020910`) passed repository gates, fmt, and workspace check, then strict Clippy rejected test-local helper shadowing; production code compiled and the test names were repaired without suppressing lints;
- implementation-head run 265 (`41091412663`) on exact head `9553ad90038d825f610cadf6b047e9da96d841a8` passed fmt, check, strict Clippy, all workspace tests, doc, repository gates, and all three examples.

Closure synchronization changes the branch head. ENG-021 remains merge-gated until one final complete clean-head CI run succeeds. Full evidence, compatibility, risks, and rollback are recorded in `docs/validation/ENG-021.md`.

After ENG-021 closes, ENG-022 is the preferred next Gate-3 capability candidate, but remains Proposed until separately promoted. ENG-023 also remains Proposed and non-executable.

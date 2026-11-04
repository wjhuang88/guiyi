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
- ENG-021 / #62 is Done and squash-merged through PR #64 (`1387415f7145e56632bfbca2d3562d48236bf589`). Final clean-head CI run 269 passed before merge.
- ENG-022 / #65 is Done and squash-merged through PR #68 (`742ba403e7e475784d75cb1f7c756ca065947913`). Final clean-head CI run 298 (`41184421365`) passed on exact head `f791a72961ad5b7a41465c9ed1be19fa02c6a2c9` before merge.
- ENG-023 / #69 is fully Done, PR #71 squash-merged as `d181a405e0578f0157621cf450ab878d33c38557`, Issue closed. Ready-promotion run 300 authorized baseline `ed8b07c5873f05e15b7c95c3c6f429c6abfe963b`. Final clean-head full CI run 311 (`41649202665`) passed on exact head `bfa5175b591b2d543e4fdf04e3f51c5e8b90ee65` before merge.
- ENG-030 / #72 is the next Gate-4 Story. This docs-only change promotes its explicit contract to Ready; implementation starts only after the entire CI suite passes on the promotion PR and that PR merges. Other Proposed Stories remain non-executable.

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
- implementation-head run 228 (`40723376393`) on `10fb887f0c82566863931c14fe222eba1d033147` passed the complete suite;
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

## Completed Story slice — ENG-021

### ENG-021 Add Encounter document, compiler, and semantic authoring commands

GitHub Issue: #62

Branch: `feature/ENG-021-encounter-document-compiler`

Merged PR: #64

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
- implementation-head run 265 (`41091412663`) on exact head `9553ad90038d825f610cadf6b047e9da96d841a8` passed fmt, check, strict Clippy, all workspace tests, doc, repository gates, and all three examples;
- final clean-head run 269 (`41092509962`) on exact head `5cfded23c7003e92ef7eb6741c0036e46cac220d` passed the complete suite before PR #64 squash-merged as `1387415f7145e56632bfbca2d3562d48236bf589`.

ENG-021 is fully closed. Full evidence, compatibility, risks, and rollback are recorded in `docs/validation/ENG-021.md`.

## Completed Story slice — ENG-022

### ENG-022 Add Dialogue graph and condition/effect registry

GitHub Issue: #65

Branch: `feature/ENG-022-dialogue-rule-registry`

Merged PR: #68

Validation record: `docs/validation/ENG-022.md`

Primary owner: GUIYI tactical-RPG toolkit.

ENG-022 closes the previously unimplemented “registry-defined conditions/effects” contract already exposed by Stage Triggers, and adds an Agent-first Dialogue graph that reuses the same rule vocabulary rather than introducing an Editor-only graph or second persisted rule format.

### Delivered ENG-022 work packages

#### A. Shared tactical rule registry

A deterministic tactical registry now has distinct condition/effect namespaces and schema-bearing descriptors backed by the existing GUIYI `SchemaNode` dialect. Built-ins are `always` and the existing sample-compatible `emit_signal`. Duplicate registration is rejected; malformed, unknown, and schema-invalid invocations map to stable `TACTICAL_RULE_*` diagnostics.

#### B. Stage Trigger validation

`validate_stage` now validates Trigger conditions/effects through the same default registry used by Dialogue. Stage serialization and `stage.create_trigger` input remain unchanged; valid historical `emit_signal` content continues to pass, while previously unchecked invalid rule payloads are intentionally rejected.

#### C. Dialogue content, validation, and compiler

Additive `tactical.dialogue.document` and `tactical.dialogue.artifact` types preserve stable node/choice `ObjectId`s and authored ordering. `validate_dialogue` reports stable name/entry/duplicate/text/target/rule diagnostics plus non-blocking unreachable-node warnings. `DialogueCompiler` rejects blocking local failures and emits deterministic self-contained artifact payloads through the standard source-hash/integrity path.

#### D. Semantic Commands and rule discovery

Implemented and registered:

- `dialogue.create`;
- `dialogue.add_node`;
- `dialogue.add_choice`;
- read-only `tactical.rules.catalog`.

The Commands use normal GUIYI schema, permission, document-access, dry-run, transaction, diff, audit, persistence, validation, and working-set boundaries. The catalog has no document access/mutation and returns deterministic IDs/descriptions/machine-readable schemas. Live catalog registration makes all four capabilities automatically visible to the existing ENG-014 Talos adapter without Talos-specific code.

#### E. Build, CLI, golden, and executable example

CLI validation recognizes Dialogue content; Stage, Encounter, and Dialogue compilers are registered together in strict build while actor definitions remain authoring-only. Tests prove valid projects produce all three artifacts and blocking Dialogue/rule failures produce no partial artifacts. The command-schema golden grows from 14 to 17 Commands. The tactical demo authors Dialogue only through semantic Commands and compiles it through the same build path.

#### F. Reproducibility and dependency direction

The tactical content crate adds only an internal path dependency on `guiyi-engine-schema`; no external dependency or Talos SDK/version/feature change occurs. `Cargo.lock` is synchronized for that workspace dependency edge. Dependency direction remains toolkit -> reusable engine crates.

### ENG-022 validation history

- Ready-promotion PR #67 passed complete CI run 271 before merge and authorized implementation on baseline `7be300590847422ce2e4d683cd0f0ba00405c018`;
- early implementation runs stopped at rustfmt while the implementation was still being assembled and are formatting-repair evidence only;
- implementation-head run 290 (`41167411681`) on exact head `f3c5d30b80d2061bb7652dcb96112655291c4cf1` passed repository gates, fmt, workspace check, strict Clippy, all workspace tests, docs, and all three examples;
- a temporary branch-only helper synchronized `Cargo.lock`; helper files were removed before final validation and are not part of the merged product surface;
- final clean-head run 298 (`41184421365`) on exact head `f791a72961ad5b7a41465c9ed1be19fa02c6a2c9` passed the complete suite before PR #68 squash-merged as `742ba403e7e475784d75cb1f7c756ca065947913` and Issue #65 closed.

### ENG-022 exit gate

Implementation acceptance is satisfied for all Issue #65 scenarios: shared rule registration/schema validation, backwards-compatible Stage Trigger validation, Dialogue graph round-trip/compiler integrity, structured graph diagnostics, non-blocking unreachable warnings, semantic Command dry-run/apply/atomic failure behavior, AgentHost permission/working-set enforcement, deterministic read-only rule discovery, live capability discovery, strict no-partial build failure, command golden update, and executable tactical-demo coverage.

No runtime Dialogue VM or rule executor, quest/world-state/save framework, generic engine-core plugin registry, visual graph authoring path, or Talos runtime change was introduced.

ENG-022 is fully closed.

## Completed Story slice — ENG-023

### ENG-023 Add intent-level `tactical.make_stage_playable` Tool

GitHub Issue: #69

Implementation PR: #71

Branch: `feature/ENG-023-make-stage-playable`

Validation record: `docs/validation/ENG-023.md`

Primary owner: GUIYI tactical-RPG toolkit, primarily `toolkits/tactical_rpg/tools`, reusing existing tactical Stage validation and existing engine Command/AgentHost execution boundaries.

ENG-023 closes Gate 3 with a bounded high-level authoring intent. An Agent can request “make this Stage locally playable”, inspect a deterministic primitive-command plan, dry-run the resulting diff, and apply one normal transaction without direct file mutation or Editor-private automation.

### Delivered ENG-023 work packages

#### A. Existing Stage truth remains authoritative

Local playability is derived only from `validate_stage`: no errors and no `STAGE_NO_SPAWN_POINT`. The Story adds no persisted playability flag, repair-plan truth, second validator, graph, audit log, or transaction model.

#### B. Live semantic intent Command

`tactical.make_stage_playable(stage_id, spawn_object_id?, spawn_profile?)` is registered in the normal tactical Command registry with `EditContent`, target-Stage access, machine-visible fallback defaults `spawn.player` / `player`, and normal live Tool discovery. The existing ENG-014 adapter therefore sees the capability without Talos-specific registration code.

#### C. Deterministic bounded plan

The machine result exposes ordered primitive Command IDs/inputs and reason codes. Automatic repair is limited to:

- `stage.set_navigation_blocked(..., false)` for blocked existing spawn cells; and
- `stage.create_spawn` for a missing spawn at the first valid unblocked grid coordinate in deterministic `q`-major / `r`-minor order.

The complete candidate is preflighted on cloned state. Unsupported or ambiguous remaining blockers reject atomically with authoritative diagnostics rather than committing a partial best-effort change.

#### D. Primitive ownership and one outer transaction

Authored changes reuse `SetNavigationBlockedCommand::apply` and `CreateSpawnCommand::apply`. The intent does not hand-edit the Stage fields those primitive Commands own. One outer GUIYI transaction provides dry-run/applied status, diff, audit, persistence, and applied-only Project Graph refresh; no synthetic sub-transactions or generic composite-command framework was added.

#### E. Security, discovery, and executable proof

Focused tests cover permission denial, working-set denial, no-op behavior, deterministic plan ordering/fallback placement, primitive-equivalence, blocker preservation, dry-run state/graph discipline, and one outer transaction. The CLI command-schema golden grows from 17 to 18 Commands. The tactical demo repairs a Stage through the intent and then continues through existing Actor/Trigger/Encounter/Dialogue authoring, strict build, and Stage runtime load.

### ENG-023 validation history

- Ready promotion PR #70 passed full CI run 300 (`41646029568`) on exact docs head `010da76632ca13d2b98264ff2070ea511f1b08b3` before squash merge established authorized implementation baseline `ed8b07c5873f05e15b7c95c3c6f429c6abfe963b`;
- run 302 stopped at rustfmt and is formatting-repair evidence only; repository gates were green;
- run 307 (`41648195957`) passed repository gates, fmt, and workspace check, then test-target compilation under strict Clippy exposed that AgentHost transaction evidence is serialized JSON; the test was corrected to deserialize the existing `TransactionReport` payload without changing production behavior;
- implementation-head run 308 (`41648591032`) on exact head `f8bcfd6c156215279e20f220810a3ab57882d5d7` passed repository gates, fmt, check, strict Clippy, all workspace tests, docs, and all three executable examples;
- repository policy requires the exact closure-docs head to pass the same complete suite before PR #71 squash merge.

No local Cargo gate is claimed; GitHub Actions is the executable validation source.

### ENG-023 exit gate

Issue #69 acceptance is satisfied by the implementation and tests: existing Stage validity remains authoritative; bounded primitive-command planning is deterministic; already-playable, blocked-spawn, missing-spawn, multi-action, blocker, permission, working-set, dry-run, transaction, graph, live-discovery, golden, and executable integration scenarios are covered; no generic composite framework or Talos/runtime/world-state scope was introduced.

Compatibility is additive and rollback removes only the intent Tool/tests/golden/demo/docs while preserving primitive Stage capabilities and project truth.

ENG-023 is implementation-complete. ENG-030 is the next primary candidate but remains Proposed until its own explicit change-control promotion.

## Ready Story slice — ENG-030

### ENG-030 Prove a real Talos-driven playable tactical slice

GitHub Issue: #72 (complete implementation contract)

Primary owner: GUIYI engine platform's outer Talos integration and executable proof, using the tactical toolkit's existing Stage/Encounter/Dialogue semantics and existing engine Command/Query/security/persistence/build/preview authority. Talos owns generic model/provider, planning, turn lifecycle, retry, and stopping.

### Current baseline and gap

- ENG-014 already supplies `GuiyiTalosAdapter`, live schema-bearing Talos `AgentTool`s and persistence/dry-run handling. ENG-023 already supplies bounded intent-level Stage repair.
- `mock_agent_workflow` uses a scripted GUIYI driver and `tactical_demo` uses direct CommandExecutor. Existing Talos integration tests use a scripted provider; none counts as a real model-driven production demo.
- Existing CLI validation/compile and headless Preview binaries implement authoritative project operations but are not separately callable by Talos through the current live GUIYI Command/Query catalog. Issue #72 authorizes only **demo-scoped typed operational adapters** to those existing GUIYI validation/build/runtime APIs, preserving permission/project working-set scope. Any stable public platform Tool requires separate change-control.

### Machine-facing acceptance

1. A real Talos `RuntimeBuilder`/`RuntimeHandle` session with operator-injected real `LanguageModel` discovers the existing live semantic Tool catalog and the limited explicit demo operational Tools. GUIYI must not own a competing model/provider/planning/session runtime.
2. Talos inspects project truth and uses typed Commands for Stage/Encounter/Dialogue authoring. The model must first receive authentic `STAGE_NO_SPAWN_POINT` or equivalent structured diagnostics, then select `tactical.make_stage_playable` dry-run, inspect its diff and apply a bounded repair. An adaptive fixture test proves behavior depends on diagnostic results, rather than a hard-coded sequence.
3. The same Talos session explicitly requests project validation, strict three-artifact tactical build and headless Stage preview from that exact successful build. A hidden post-session CLI/scripted substitute or pre-existing artifact does not satisfy acceptance. Build failure must produce no partial artifacts or successful preview.
4. Enforce project scope, `EditContent`/`RunValidation`/`RunBuild`/`RunPreview`, working-set and permission denials, with structured failure/continuation. Verify Bevy preview load/unload lifecycle and artifact integrity.
5. Confirm persisted Stage/Encounter/Dialogue/references survive restart; dry-run/rejected changes do not persist. Report observed Talos events/Tool results, authoritative transaction IDs/diffs, diagnostics, artifact hashes, preview and durable GUIYI audit as a *derived, sanitized* review packet.
6. Deterministic CI fixture tests exercise the whole path, but **a sanitized real model-provider Talos execution is mandatory before Done**. If credentials or a supplied real provider are unavailable, record the blocker and do not misstate fixture CI as product proof.

### Ownership / compatibility / non-goals / rollback

- No new direct Talos dependencies, SDK version/features, generic provider client, GUIYI Agent loop, second graph/transaction/artifact/persistence truth, content-schema migrations, or reverse engine/toolkit dependencies.
- No combat, world-state, quest, party, inventory, save, visual Editor/workbench (ENG-024), arbitrary auto-repair, shell or raw file authoring.
- An external real model is injected through Talos's supported SDK facade; model credentials/configuration, decision policy and retries remain Talos/operator-owned.
- Stop for separate change-control/ADR if generic Tool kinds, engine-wide stable APIs, transaction authority, new Talos dependencies or a second state truth prove required. The rollback deletes only the example/fixture/operational demo adapters/tests/docs; established GUIYI APIs remain intact.

### Gates

- **HUMAN REQUIRED / BLOCKED:** a separate operator real-provider Talos execution and signed sanitized evidence. Use [ENG-030 operator handoff](../../validation/ENG-030-MANUAL-ACCEPTANCE.md). Deterministic CI is automated regression evidence only; it cannot close this gate. Non-human ENG-030 hardening continues in PR #74, not in Proposed Stories.
- This Ready promotion uses a docs-only PR from `docs/ENG-030-talos-playable-slice-ready`; complete CI and merge are prerequisites to creating `feature/ENG-030-talos-playable-slice` from its new main.
- Implementation requires failing acceptance tests, narrow code, live Tool metadata, diagnostics and documentation, real provider proof, complete Rust/repository/example suite, final clean-head exact-head CI, then squash merge.
- Implementation evidence so far: draft PR #74 runs the real Talos SDK with a structured-result-dependent CI fixture, semantic Stage/Encounter/Dialogue authoring, intent dry-run/apply, demo-scoped validate/build/preview, sanitized audit and permission/failure tests. Implementation-head CI run 336 (`41751127308`) passed the complete extended suite on `bcdfcab38ed2530e0a0b93fee9fd4671c7ac84e3`; full details are in `docs/validation/ENG-030.md`. This is **not** real-model product acceptance; the issue/PR must stay open/draft until a sanitized genuine model-provider session and final clean-head CI are available.

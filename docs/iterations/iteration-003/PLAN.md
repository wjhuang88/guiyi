# Iteration 003 plan

## Goal

Converge GUIYI on the published Talos runtime SDK, then increase the semantic game-development capabilities that Talos and direct clients can safely use without weakening GUIYI's domain execution authority.

At the product-surface level, this iteration advances an Agent-first development flow: the Agent interaction layer should perform the normal semantic authoring, validation, build, preview, and repair work that a traditional engine Editor would otherwise mediate. Human UI remains a secondary client of the same machine-facing platform.

## Authorization

- ENG-029 is Done and merged through PR #51 (`4dc57fe7203795e20b45042dfca22ceeb8adae0c`).
- ENG-014 / #52 is Done and squash-merged through PR #53 (`5ce7dae09e55e0c40c8345d67e81bd38f44b67b4`).
- ENG-012 / #54 is Done and squash-merged through PR #55 (`eed94d8d7d6a63b73c103198af1c33564ffa6bfe`). Final clean-head CI run 207 passed before merge.
- ENG-013 / #57 is the active Ready/In-Progress capability Story on branch `feature/ENG-013-stage-object-move-remove` through draft PR #58.
- ENG-020..023 remain outside the ENG-013 patch and follow after ENG-013 closes.
- Proposed Stories remain non-executable.

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

## Active Story slice — ENG-013

### ENG-013 Add semantic Stage object move/remove Commands

GitHub Issue: #57

Branch: `feature/ENG-013-stage-object-move-remove`

Draft PR: #58

Architecture authorities:

- `docs/decisions/ADR-0007-TYPED-COMMANDS.md`
- `docs/decisions/ADR-0008-TRANSACTIONS.md`
- `docs/decisions/ADR-0010-DIAGNOSTICS.md`

Reference: `docs/reference/STAGE-MODEL.md`.

### Problem

The tactical toolkit can create Stages and place Actors, spawn points, triggers, and connections, but it lacked semantic move/remove operations for existing stable Stage objects. That gap forced a normal authoring action toward lower-level document mutation rather than the Agent-first semantic Tool surface.

### ENG-013 work packages

#### A. Semantic Commands

Implement and register:

- `stage.move_object(stage_id, object_id, q, r)`;
- `stage.remove_object(stage_id, object_id)`.

Both commands use the normal schema, permission, document-access, dry-run, transaction, diff, audit, persistence, and AgentHost path. No private editor or Talos-specific mutation route is introduced.

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

#### E. Agent-first development surface

ENG-013 is intentionally a semantic capability-density Story. A normal “move/remove a Stage object” development task must be possible through discoverable typed Tools rather than an editor-only gesture or direct file patch. This is the model for later tactical capabilities: semantic Tool coverage precedes optional visual affordances.

## Non-goals

- No Stage connection edit/remove command.
- No object rename or batch move/remove.
- No occupancy, collision, pathfinding, reachability, encounter, dialogue, or ENG-020+ tactical rules.
- No visual Editor authoring path.
- No generic document-patch replacement for semantic commands.
- No Talos provider/planner/session/runtime change.
- No second persisted Project Graph truth.

## Validation

ENG-013 requires:

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

Intermediate failures are evidence, not acceptance:

- run 210 exposed an internal-module qualification failure in `static_rust_check.py` and rustfmt drift;
- run 214 passed repository gates, fmt, check, and strict Clippy, then correctly failed the CLI capability golden because the live catalog grew from 8 to 10 command schemas.

The CLI golden is being synchronized to the intentional public capability expansion. ENG-013 remains In Progress until implementation/docs are synchronized and one final current clean-head CI run passes the complete suite.

Local Cargo execution is unavailable in the current execution environment; no local gate is claimed as passed.

## ENG-013 exit gate

ENG-013 is Done only when:

- move apply preserves object identity/non-position fields and produces an applied diff;
- move dry-run produces preview evidence without authoritative mutation;
- out-of-bounds move is rejected with structured bounds diagnostics;
- remove apply deletes the stable object and reports candidate warnings;
- missing object move/remove returns `STAGE_OBJECT_NOT_FOUND` atomically;
- Actor removal updates semantic reference/Project Graph query results through the normal AgentHost transaction path;
- permission and working-set denial remain authoritative;
- both Commands are discoverable with machine-readable schemas through the live catalog/Talos adapter and direct clients;
- Stage/reference/backlog/validation documentation is synchronized, including the Agent-first development-surface rule;
- current clean-head GitHub CI passes the complete repository suite.

After ENG-013 closes, the primary implementation sequence continues with ENG-020..023 tactical-RPG vertical capabilities, then ENG-030 Talos-driven playable-slice proof, then the human review workbench.

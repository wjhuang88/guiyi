# Iteration 003 plan

## Goal

Converge GUIYI's product and architecture direction on the published Talos runtime SDK, then increase the semantic game-world capabilities Talos and direct clients can safely use without weakening GUIYI's domain execution authority.

## Authorization

- ENG-029 is Done and merged through PR #51 (`4dc57fe7203795e20b45042dfca22ceeb8adae0c`).
- ENG-014 / #52 is Done and squash-merged through PR #53 (`5ce7dae09e55e0c40c8345d67e81bd38f44b67b4`).
- ENG-012 / #54 is the active Story on branch `feature/ENG-012-project-graph-index` through draft PR #55.
- ENG-013 remains Ready and is not part of the ENG-012 implementation branch.
- Proposed Stories remain non-executable.

## Completed Story slice — ENG-014

### ENG-014 Integrate Talos runtime SDK as GUIYI's first-party Agent runtime

GitHub Issue: #52

Merged PR: #53

Architecture decision: `docs/decisions/ADR-0016-TALOS-AGENT-RUNTIME.md`

Validation record: `docs/validation/ENG-014.md`

ENG-014 established crates.io `talos-runtime = 0.10.1` as the only direct Talos dependency, exposed GUIYI's live Tool catalog as individual Talos `AgentTool`s, kept GUIYI authoritative for domain security/transactions/persistence, and preserved direct Talos-independent clients.

The final ENG-014 clean-head CI run 185 passed the complete repository suite before merge.

## Active Story slice — ENG-012

### ENG-012 Maintain an incremental semantic Project Graph index

GitHub Issue: #54

Branch: `feature/ENG-012-project-graph-index`

Implementation PR: #55

Architecture authority: `docs/decisions/ADR-0011-PROJECT-GRAPH.md`

Reference: `docs/reference/PROJECT-GRAPH.md`

Validation record: `docs/validation/ENG-012.md`

### Problem

The existing semantic graph contract was correct but normal graph-backed Queries rebuilt or scanned relationships per invocation. Iteration 003 now turns the accepted semantic-reference model into a reusable derived index without introducing a second project truth.

### ENG-012 work packages

#### A. Derived graph/index boundary

Target:

- keep `DocumentEnvelope.references` as authoritative semantic relationship data;
- maintain `ProjectGraphIndex` in `guiyi-engine-query`;
- preserve `ProjectGraph` as the serializable document-adjacency snapshot compatibility surface;
- introduce no external graph database or Talos dependency in reusable engine crates.

#### B. Query reuse

Target:

- `project.references.find` uses the maintained index on the normal `QueryExecutor` / `AgentHost` path;
- `project.impact.analyze` uses the maintained index on that path;
- direct Query clients can rebuild, refresh, invalidate, and snapshot the graph without Talos.

#### C. Incremental mutation lifecycle

Target:

```text
GUIYI applied Command
  ↓
TransactionReport.document_diffs
  ↓
refresh only changed source Document IDs
  ↓
ProjectGraphIndex
  ↓
next relationship / impact Query
```

Dry-run, rejected, and failed Commands must not change graph state. State replacement or persistence rollback invalidates the derived graph and rebuilds later from authoritative Documents.

#### D. Working-set isolation

Target:

- initialize the maintained graph from complete authoritative `EngineState`;
- continue using GUIYI access planning and filtered working-set project views;
- do not return hidden source Documents;
- do not traverse hidden intermediate graph nodes to reach otherwise visible downstream Documents.

#### E. Compatibility and Talos availability

Target:

- keep existing graph Query Tool IDs and schemas stable;
- keep the live Tool catalog as the capability source consumed by the ENG-014 Talos adapter;
- keep Workbench/CLI/tests/direct GUIYI clients Talos-independent;
- document `QueryExecutor::execute(&mut self, ...)` as an alpha receiver-level compatibility change.

## Non-goals preserved for ENG-012

- No ENG-013 Stage move/remove implementation.
- No ENG-020+ tactical feature implementation.
- No visual editor/workbench expansion or visual graph UI.
- No persisted Project Graph as a second authoring truth.
- No generic graph database or new external graph dependency.
- No Project Graph ownership moved into Talos.
- No broad object/reference-model redesign beyond the existing `ContentReference` contract.
- No GUIYI-owned provider/model abstraction, conversation runtime, planner, memory subsystem, retry/stopping framework, or second Agent loop.

## Validation

Required full suite for ENG-012:

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

The first ENG-012 implementation-head CI run 189 established that repository gates remained green but stopped at `cargo fmt --all -- --check`; all later Rust checks on that run were skipped. That run is failure evidence, not acceptance evidence. ENG-012 remains gated on a later clean current-head CI run that passes the complete suite.

Local clone/Cargo execution remains unavailable in the current execution environment because outbound DNS cannot resolve `github.com`; no local gate is claimed as passed.

## ENG-012 exit gate

ENG-012 is Done only when:

- Issue #54 acceptance scenarios A-F have executable coverage;
- the maintained graph is reusable rather than rebuilt for every normal graph-backed Query;
- applied transaction diffs incrementally refresh changed source Documents;
- dry-run/rejected/failed paths leave graph state unchanged;
- working-set restrictions block hidden results and hidden-node traversal;
- rollback/state replacement cannot leave a stale derived graph;
- graph Queries remain available through the live GUIYI Tool catalog/Talos adapter and direct clients;
- architecture/reference/backlog/validation records are synchronized;
- current clean-head GitHub CI passes the complete repository suite.

After ENG-012 closes, ENG-013 remains the next Ready capability Story before the ENG-020..023 tactical vertical sequence.

# Iteration 003 plan

## Goal

Converge GUIYI's product and architecture direction on the published Talos runtime SDK and implement the first real Talos-driven GUIYI Tool vertical slice without weakening GUIYI's domain execution authority.

## Authorization

- ENG-029 is Done and merged through PR #51 (`4dc57fe7203795e20b45042dfca22ceeb8adae0c`).
- ENG-014 / #52 implementation acceptance is complete in PR #53; final merge remains gated on green CI for the current closure-synchronized head.
- ENG-012 and ENG-013 are unblocked by the accepted ENG-014 integration baseline and remain the next GUIYI capability sequence.
- Proposed Stories remain non-executable.

## Completed Story slice

### ENG-014 Integrate Talos runtime SDK as GUIYI's first-party Agent runtime

GitHub Issue: #52

Branch: `feature/ENG-014-talos-runtime-integration`

Implementation PR: #53

Architecture decision: `docs/decisions/ADR-0016-TALOS-AGENT-RUNTIME.md`

Validation record: `docs/validation/ENG-014.md`

## Confirmed external baseline

ENG-014 re-verified the approved Talos release line as `0.10.1` before implementation.

The committed dependency is crates.io `talos-runtime = 0.10.1` with default features disabled and no optional Talos features enabled. `Cargo.lock` is committed. GUIYI does not directly depend on Talos implementation crates such as `talos-agent`, `talos-core`, `talos-session`, or `talos-permission`.

The integration uses public SDK contracts re-exported by `talos-runtime`, including `RuntimeBuilder`, `AgentTool`, `RuntimeHandle` behavior, provider traits, permission rules, Tool results, and typed session events.

## Delivered work packages

### A. Outer integration boundary

Delivered:

- added `integrations/talos_runtime` as the only GUIYI package with a direct Talos dependency;
- enforced that boundary in `scripts/check_architecture.py`;
- kept engine core, schema/content, command/query, validation/build/runtime, and toolkit core independent of Talos;
- added no provider/model/planner/conversation/retry abstractions inside GUIYI.

### B. Live Tool adaptation

Delivered:

- materialized Talos `AgentTool`s from GUIYI's live `ToolCatalog`;
- preserved Tool IDs, descriptions, and live input schemas;
- exposed Commands individually rather than through a catch-all Tool;
- carried GUIYI protocol `dry_run` metadata without duplicating command schema authority;
- routed every call to the existing `AgentHost::execute` execution seam.

### C. Real execution vertical slice

Accepted path:

```text
Talos RuntimeBuilder
  ↓
register GUIYI AgentTools
  ↓
submit objective
  ↓
GUIYI Query
  ↓
GUIYI dry-run Command
  ↓
GUIYI apply Command
  ↓
ProjectStorage / ProjectTransaction
  ↓
structured ToolResult
  ↓
Talos continues
```

Targeted integration tests cover capability discovery, Query execution, dry-run/apply transaction behavior, durable storage/audit, GUIYI permission and working-set rejection after Talos admission, and continuation after recoverable GUIYI Tool failure.

### D. Agent lifecycle migration

Decisions:

- Talos owns the canonical first-party Agent/session/turn lifecycle;
- `AgentHost::execute` remains GUIYI's reusable authoritative execution seam;
- `AgentLoopDriver` / `AgentHost::run` remain compatibility-only alpha APIs and are not used by the Talos path;
- `AgentSession` remains temporarily for direct clients and as an internal compatibility seam, but Talos-facing API uses the narrower `GuiyiExecutionContext`;
- `AgentBudget` is classified as a legacy generic action budget rather than a GUIYI domain-safety invariant; the Talos adapter does not impose it as a second stopping policy;
- direct JSONL Workbench, CLI, examples, and tests remain usable without starting Talos.

### E. Documentation and closure

Delivered:

- architecture, AI-native platform, and Agent-session references describe the implemented boundary and migration decision;
- Product Backlog and this plan reflect the accepted ENG-014 baseline;
- `docs/validation/ENG-014.md` records exact acceptance and CI evidence;
- final merge/Done authority remains green CI on the current PR head after closure-status synchronization.

## Non-goals preserved

- No ENG-012 Project Graph implementation.
- No ENG-013 Stage move/remove implementation.
- No ENG-020+ tactical feature implementation.
- No visual editor/workbench expansion.
- No generic Talos coding-tool integration.
- No GUIYI-owned provider/model abstraction, conversation runtime, planner, memory subsystem, retry/stopping framework, or second Agent loop.
- No direct dependency on Talos implementation crates or Talos `main`.

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

PR #53 CI run `39809155717` (run number 181) passed the complete suite on implementation head `8bddfb4235455b92249b96edd31f4574564edb95` before closure-status synchronization. The current PR head must also be green before merge; no later edit should be made solely to copy that final run number into documentation.

## Exit gate

ENG-014 acceptance is complete because:

- Issue #52 acceptance scenarios A-F are covered by executable evidence;
- `talos-runtime` remains the only direct Talos dependency and the architecture gate enforces it;
- Talos discovers and executes GUIYI live Tools through public SDK contracts;
- GUIYI remains authoritative for permissions, working sets, access plans, schema validation, dry-run, transactions, persistence, diagnostics, and audit;
- recoverable Tool failures do not terminate the Talos runtime;
- direct clients remain Talos-independent;
- lifecycle/budget compatibility decisions are documented without silently breaking alpha APIs;
- validation evidence is recorded.

Final Story closure and merge remain conditioned on green GitHub CI for the closure-synchronized PR head.

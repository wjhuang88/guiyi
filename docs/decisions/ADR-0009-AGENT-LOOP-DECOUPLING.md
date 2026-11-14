# ADR-0009: Agent loop decoupling

- Status: Superseded by [ADR-0016](ADR-0016-TALOS-AGENT-RUNTIME.md)
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

The engine exposes `AgentLoopDriver` and does not own provider/model logic.

## Consequences

Allows the separate agent-loop project to be integrated by adapter.

## Supersession

ADR-0016 preserves the important decoupling rule but makes the previously unnamed first-party runtime explicit: Talos is the canonical first-party agent runtime, GUIYI retains game-world execution authority, and the existing generic loop/session overlap is transitional work owned by ENG-014.

This file is retained as historical architecture evidence. Do not use it to justify expanding a second generic agent-loop implementation inside GUIYI.

## Validation

Historical validation is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts. Current agent-runtime direction is validated under ENG-029 and ENG-014.
## ENG-028 historical implementation note (2026-11-14)

### Original forces, alternatives and current precedence

The original decoupling decision prevented model/provider concerns from leaking into reusable engine crates, but deliberately left the external first-party runtime unnamed. Maintaining a separate GUIYI-owned `AgentLoopDriver` compatibility surface was useful for early scripted tests. **The alternative of expanding that driver into a second generic first-party provider/planner/session framework is now rejected by accepted [ADR-0016](ADR-0016-TALOS-AGENT-RUNTIME.md).** This historical file is not a mandate to preserve two competing generic Agent runtimes.

### Historical implementation vs current authority

- `guiyi-engine-agent-host` still exposes transitional `AgentLoopDriver`, scripted fixtures and `AgentSession` state. Those APIs are compatibility evidence, **not** the canonical first-party production Agent lifecycle contract.
- `integrations/talos_runtime` uses the published crates.io `talos-runtime` SDK and delegates game-domain Tool invocation to GUIYI. Talos owns model/provider/turn/selection/retry/stopping; GUIYI owns typed game Commands/Queries, permissions, working sets, validation, audit and persistence.
- Runtime/provider failures must remain distinguishable from a GUIYI domain Tool rejection. Neither route may escalate a denied Tool into an authorized mutation. The outer adapter must not create its own authoritative game state.

### Compatibility, scale, failures and supersession

New generic AI loop behavior belongs in Talos, not in `engine_core`, `engine_agent_host` or the tactical toolkit. Existing tests of the old driver establish its compatibility behavior only and do not prove model-driven interaction. Talos SDK compatibility is governed by ADR-0016's reviewed pre-1.0 dependency and pinned lockfile. This ADR is **already superseded**; any further agent-runtime ownership change must update/supersede ADR-0016 explicitly with migration tests, never revive ADR-0009 by editing its status. No provider latency, token rate or session-size guarantee is established in GUIYI.

### Evidence

- Sources: [outer Talos adapter](../../integrations/talos_runtime/src/lib.rs), [legacy driver and bounded host](../../crates/engine_agent_host/src/lib.rs), [Agent Sessions reference](../reference/AGENT-SESSIONS.md).
- Tests: [Talos SDK adapter integration tests](../../integrations/talos_runtime/tests/runtime_integration.rs), `engine_agent_host::tests::driver_can_complete_after_exactly_maximum_actions`, `exhausted_driver_marks_session_failed_without_losing_action_history` and ENG-014 [validation](../validation/ENG-014.md).
- The actual-model ENG-030 session remains **HUMAN REQUIRED / NOT RUN** under [Issue #72](https://github.com/wjhuang88/guiyi/issues/72), independent of these deterministic compatibility fixtures.

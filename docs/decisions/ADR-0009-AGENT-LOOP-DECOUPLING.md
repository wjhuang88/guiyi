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

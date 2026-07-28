# ADR-0009: Agent loop decoupling

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

The engine exposes AgentLoopDriver and does not own provider/model logic.

## Consequences

Allows the separate agent-loop project to be integrated by adapter.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.

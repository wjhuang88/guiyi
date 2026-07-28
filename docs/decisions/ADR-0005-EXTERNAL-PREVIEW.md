# ADR-0005: External preview

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Preview is a headless or separate process client of compiled artifacts.

## Consequences

Isolates editor/agent host failures from runtime execution.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.

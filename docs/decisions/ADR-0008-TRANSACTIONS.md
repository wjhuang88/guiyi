# ADR-0008: Transactions and dry-run

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Commands apply to cloned state, produce diff, and commit atomically.

## Consequences

Makes agent changes predictable, reviewable, and rollback-ready.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.

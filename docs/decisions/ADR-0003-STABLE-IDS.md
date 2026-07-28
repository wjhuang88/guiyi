# ADR-0003: Stable IDs

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Persistent and cross-document references use validated stable IDs, never Bevy Entity.

## Consequences

Enables save, migration, references, and agent queries across runs.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.

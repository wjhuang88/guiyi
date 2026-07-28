# ADR-0014: UI is a client

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Human UI uses the same command/query APIs and owns no separate business truth.

## Consequences

Prevents divergence between AI and human workflows.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.

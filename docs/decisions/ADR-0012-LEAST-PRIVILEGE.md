# ADR-0012: Least privilege

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Each agent session receives explicit permissions and action budget.

## Consequences

Limits accidental code, process, commit, and publish operations.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.

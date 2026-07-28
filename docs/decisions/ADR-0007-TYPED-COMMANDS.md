# ADR-0007: Typed commands

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

All authoritative mutations use registered typed commands.

## Consequences

Prevents uncontrolled file edits and centralizes validation and audit.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.

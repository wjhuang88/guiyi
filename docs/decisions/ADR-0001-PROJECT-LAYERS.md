# ADR-0001: Project layers

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Engine core, tactical toolkit, and game extensions use one-way dependencies.

## Consequences

Prevents game-specific concepts from contaminating reusable infrastructure.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.

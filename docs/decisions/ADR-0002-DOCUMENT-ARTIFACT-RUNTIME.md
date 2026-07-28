# ADR-0002: Document, Artifact, Runtime

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Authoring documents compile into artifacts; artifacts instantiate runtime entities.

## Consequences

Allows Git-friendly content, deterministic builds, and safe runtime lifecycle.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.

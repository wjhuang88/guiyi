# ADR-0004: Static extensions

- Status: Accepted
- Date: 2026-07-28

## Context

GUIYI Engine is an AI-native Bevy infrastructure project for tactical RPG production.

## Decision

Initial extensions are statically registered Rust crates.

## Consequences

Avoids unstable Rust dynamic ABI while preserving extension contracts.

## Validation

The decision is represented by workspace boundaries, public APIs, examples, tests, or repository gate scripts.

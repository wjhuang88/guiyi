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
## ENG-028 implementation clarification (2026-11-14)

### Forces and rejected alternatives

Machine-authored content must survive restarts, but Bevy ECS entities are transient runtime allocations. **Rejected:** serializing the ECS `World` or writing live `Entity` IDs into Documents as the editor/save authority. That would tie content to process-local entity allocation and make review/build/reload nondeterministic. **Cost:** GUIYI must retain a typed compiler, artifact integrity check and explicit load/unload lifecycle instead of directly editing rendered instances.

### Invariants and current mechanism

- A `DocumentEnvelope` in `guiyi-engine-content` is authoritative authoring data with stable document ID and schema version; its `references` declare semantic edges. A `DocumentCompiler` reads an envelope and creates an `ArtifactEnvelope` containing source document, source hash, compiler version, artifact type, payload and integrity hash. The Artifact is a reproducible derived output, not a second editable document.
- `guiyi-engine-runtime::StageRuntimeManager::load` validates the **Stage artifact type, compiler version, integrity hash, payload and object IDs before spawning entities**. On validation error the current World and instance registry remain unchanged. Successful load owns entities under a Stage instance; `unload` despawns its owned entities rather than rewriting Documents.
- No runtime component/entity ID is a persistent content or save identity. The current Stage runtime is a tested vertical slice, not proof that all future game objects have a complete save/migration implementation.

### Failure, versioning and scaling

Invalid/tampered/unsupported artifacts are rejected with structured `RUNTIME_ARTIFACT_*` codes; an artifact must be rebuilt from source after authoritative content changes. The compiler-version check is an independent compatibility gate; an engine API, content schema or save schema change is **not** automatically an artifact upgrade. Such migrations need an authorized compatibility Story (ENG-025 remains Proposed). The current loader processes a Stage's object collection and creates one instance-owned entity per descriptor; no published maximum size or throughput/SLO has been measured. Repeated load/unload and cleanup tests provide correctness evidence, **not** a production performance bound. Supersede this ADR only with explicit new persistence/compilation ownership and a migration plan.

### Exact evidence

- Source: [Document/Artifact models](../../crates/engine_content/src/lib.rs), [Stage runtime manager](../../crates/engine_runtime/src/lib.rs), [runtime artifact reference](../reference/RUNTIME-ARTIFACT-CONTRACT.md).
- Tests: `engine_runtime::tests::wrong_artifact_type_is_rejected_atomically`, `corrupted_payload_hash_is_rejected_atomically`, `repeated_load_and_unload_does_not_leak_entities` and ENG-019 [Preview executable regression](../../crates/engine_preview/tests/process_preview.rs).
- Safety dependencies: [ADR-0003](ADR-0003-STABLE-IDS.md), [ADR-0008](ADR-0008-TRANSACTIONS.md), [ENG-019 validation](../validation/ENG-019.md), ENG-025 Proposed.

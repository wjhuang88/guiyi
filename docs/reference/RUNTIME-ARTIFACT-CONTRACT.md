# Runtime artifact contract

## Envelope integrity

Every newly compiled `ArtifactEnvelope` contains two distinct hashes:

- `source_hash` identifies the authoring document used by the compiler;
- `artifact_hash` protects the runtime envelope and payload.

`artifact_hash` is the deterministic hash of the artifact ID, artifact type, source document ID, compiler version, source hash, and payload. It deliberately excludes itself. Compilers construct artifacts through `ArtifactEnvelope::new`, which calculates the hash after all protected fields are populated.

A missing or mismatched `artifact_hash` is a runtime integrity failure. `source_hash` is not overloaded as an artifact checksum.

## Stage compatibility

`StageRuntimeManager` currently accepts only:

- artifact type `tactical.stage.artifact`;
- compiler version `1`.

Compatibility is explicit. Supporting another version requires a versioned reader or migration path rather than weakening the checks for the current contract.

## Validation order

Before creating an ECS entity or active Stage record, the runtime validates:

1. artifact type;
2. compiler version;
3. artifact checksum;
4. Stage payload deserialization;
5. metadata object shape;
6. uniqueness of runtime object IDs;
7. object property shape and required tactical object fields.

All objects are validated before spawning begins. Therefore malformed later objects cannot leave entities from earlier objects behind. A failed load leaves the world entity count, active Stage count, and generated instance sequence unchanged.

## Structured failures

Runtime failures expose a stable code and structured details. Current codes include:

- `RUNTIME_ARTIFACT_TYPE_MISMATCH`;
- `RUNTIME_ARTIFACT_VERSION_UNSUPPORTED`;
- `RUNTIME_ARTIFACT_INTEGRITY_FAILED`;
- `RUNTIME_ARTIFACT_PAYLOAD_INVALID`;
- `RUNTIME_OBJECT_ID_DUPLICATE`;
- `RUNTIME_OBJECT_INVALID`;
- `RUNTIME_STAGE_NOT_LOADED`.

The headless Preview preserves these codes. With `--json`, an invalid artifact produces a structured JSON error on stderr and exits non-zero.

#![forbid(unsafe_code)]

//! Bevy ECS runtime stage ownership and repeatable load/unload lifecycle.

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use guiyi_engine_content::{ArtifactEnvelope, RuntimeObjectDescriptor, StageArtifactPayload};
use guiyi_engine_core::{ArtifactId, EngineTypeId, ObjectId, StageInstanceId};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const STAGE_RUNTIME_ARTIFACT_TYPE: &str = "tactical.stage.artifact";
pub const STAGE_RUNTIME_COMPILER_VERSION: u32 = 1;
pub const RUNTIME_ARTIFACT_TYPE_MISMATCH: &str = "RUNTIME_ARTIFACT_TYPE_MISMATCH";
pub const RUNTIME_ARTIFACT_VERSION_UNSUPPORTED: &str = "RUNTIME_ARTIFACT_VERSION_UNSUPPORTED";
pub const RUNTIME_ARTIFACT_INTEGRITY_FAILED: &str = "RUNTIME_ARTIFACT_INTEGRITY_FAILED";
pub const RUNTIME_ARTIFACT_PAYLOAD_INVALID: &str = "RUNTIME_ARTIFACT_PAYLOAD_INVALID";
pub const RUNTIME_OBJECT_ID_DUPLICATE: &str = "RUNTIME_OBJECT_ID_DUPLICATE";
pub const RUNTIME_OBJECT_INVALID: &str = "RUNTIME_OBJECT_INVALID";
pub const RUNTIME_STAGE_NOT_LOADED: &str = "RUNTIME_STAGE_NOT_LOADED";

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct StageOwned {
    pub instance_id: StageInstanceId,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalPersistent;

#[derive(Component, Debug, Clone, PartialEq)]
pub struct RuntimeObject {
    pub object_id: ObjectId,
    pub type_id: EngineTypeId,
    pub properties: Value,
}

#[derive(Debug)]
pub struct RuntimeStage {
    pub instance_id: StageInstanceId,
    pub artifact_id: ArtifactId,
    pub entities: Vec<Entity>,
}

#[derive(Resource, Debug, Default)]
pub struct StageRuntimeManager {
    next_instance: u64,
    active: BTreeMap<StageInstanceId, RuntimeStage>,
}

impl StageRuntimeManager {
    pub fn load(
        &mut self,
        world: &mut World,
        artifact: &ArtifactEnvelope,
    ) -> Result<StageInstanceId, RuntimeError> {
        let payload = validate_stage_artifact(artifact)?;

        self.next_instance += 1;
        let instance_id = StageInstanceId::new(format!("stage-instance-{:08}", self.next_instance))
            .expect("generated stage instance identifier is valid");
        let mut entities = Vec::with_capacity(payload.objects.len());
        for object in payload.objects {
            let entity = world
                .spawn((
                    StageOwned {
                        instance_id: instance_id.clone(),
                    },
                    RuntimeObject {
                        object_id: object.id,
                        type_id: object.type_id,
                        properties: object.properties,
                    },
                ))
                .id();
            entities.push(entity);
        }
        self.active.insert(
            instance_id.clone(),
            RuntimeStage {
                instance_id: instance_id.clone(),
                artifact_id: artifact.id.clone(),
                entities,
            },
        );
        Ok(instance_id)
    }

    pub fn unload(
        &mut self,
        world: &mut World,
        instance_id: &StageInstanceId,
    ) -> Result<usize, RuntimeError> {
        let stage = self
            .active
            .remove(instance_id)
            .ok_or_else(|| RuntimeError::StageNotLoaded(instance_id.clone()))?;
        let count = stage.entities.len();
        for entity in stage.entities {
            let _ = world.despawn(entity);
        }
        Ok(count)
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

fn validate_stage_artifact(
    artifact: &ArtifactEnvelope,
) -> Result<StageArtifactPayload, RuntimeError> {
    let expected_type = EngineTypeId::from_static(STAGE_RUNTIME_ARTIFACT_TYPE);
    if artifact.artifact_type != expected_type {
        return Err(RuntimeError::ArtifactTypeMismatch {
            expected: expected_type,
            actual: artifact.artifact_type.clone(),
        });
    }
    if artifact.compiler_version != STAGE_RUNTIME_COMPILER_VERSION {
        return Err(RuntimeError::ArtifactVersionUnsupported {
            supported: STAGE_RUNTIME_COMPILER_VERSION,
            actual: artifact.compiler_version,
        });
    }

    let computed_hash = artifact.compute_artifact_hash().map_err(|error| {
        RuntimeError::ArtifactIntegrityFailed {
            declared: artifact.artifact_hash.clone(),
            computed: format!("unavailable: {error}"),
        }
    })?;
    if artifact.artifact_hash.is_empty() || artifact.artifact_hash != computed_hash {
        return Err(RuntimeError::ArtifactIntegrityFailed {
            declared: artifact.artifact_hash.clone(),
            computed: computed_hash,
        });
    }

    let payload: StageArtifactPayload = serde_json::from_value(artifact.payload.clone())
        .map_err(|error| RuntimeError::InvalidArtifactPayload(error.to_string()))?;
    if !payload.metadata.is_object() {
        return Err(RuntimeError::InvalidArtifactPayload(
            "Stage metadata must be a JSON object".into(),
        ));
    }

    let mut object_ids = BTreeSet::new();
    for object in &payload.objects {
        if !object_ids.insert(object.id.clone()) {
            return Err(RuntimeError::DuplicateObjectId(object.id.clone()));
        }
        validate_runtime_object(object)?;
    }
    Ok(payload)
}

fn validate_runtime_object(object: &RuntimeObjectDescriptor) -> Result<(), RuntimeError> {
    let properties = object
        .properties
        .as_object()
        .ok_or_else(|| RuntimeError::InvalidObject {
            object_id: object.id.clone(),
            message: "runtime object properties must be a JSON object".into(),
        })?;

    if object.type_id.as_str().starts_with("tactical.") {
        for required in ["position", "object"] {
            if !properties.contains_key(required) {
                return Err(RuntimeError::InvalidObject {
                    object_id: object.id.clone(),
                    message: format!("tactical runtime object is missing `{required}`"),
                });
            }
        }
    }
    Ok(())
}

pub struct StageRuntimePlugin;

impl Plugin for StageRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StageRuntimeManager>();
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("stage is not loaded: {0}")]
    StageNotLoaded(StageInstanceId),
    #[error("artifact type mismatch: expected {expected}, found {actual}")]
    ArtifactTypeMismatch {
        expected: EngineTypeId,
        actual: EngineTypeId,
    },
    #[error("artifact compiler version {actual} is unsupported; supported version is {supported}")]
    ArtifactVersionUnsupported { supported: u32, actual: u32 },
    #[error("artifact integrity check failed: declared {declared}, computed {computed}")]
    ArtifactIntegrityFailed { declared: String, computed: String },
    #[error("artifact payload is invalid: {0}")]
    InvalidArtifactPayload(String),
    #[error("duplicate runtime object ID: {0}")]
    DuplicateObjectId(ObjectId),
    #[error("runtime object {object_id} is invalid: {message}")]
    InvalidObject {
        object_id: ObjectId,
        message: String,
    },
}

impl RuntimeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::StageNotLoaded(_) => RUNTIME_STAGE_NOT_LOADED,
            Self::ArtifactTypeMismatch { .. } => RUNTIME_ARTIFACT_TYPE_MISMATCH,
            Self::ArtifactVersionUnsupported { .. } => RUNTIME_ARTIFACT_VERSION_UNSUPPORTED,
            Self::ArtifactIntegrityFailed { .. } => RUNTIME_ARTIFACT_INTEGRITY_FAILED,
            Self::InvalidArtifactPayload(_) => RUNTIME_ARTIFACT_PAYLOAD_INVALID,
            Self::DuplicateObjectId(_) => RUNTIME_OBJECT_ID_DUPLICATE,
            Self::InvalidObject { .. } => RUNTIME_OBJECT_INVALID,
        }
    }

    pub fn details(&self) -> Value {
        match self {
            Self::StageNotLoaded(instance_id) => json!({"stage_instance": instance_id}),
            Self::ArtifactTypeMismatch { expected, actual } => {
                json!({"expected_type": expected, "actual_type": actual})
            }
            Self::ArtifactVersionUnsupported { supported, actual } => {
                json!({"supported_version": supported, "actual_version": actual})
            }
            Self::ArtifactIntegrityFailed { declared, computed } => {
                json!({"declared_hash": declared, "computed_hash": computed})
            }
            Self::InvalidArtifactPayload(message) => json!({"reason": message}),
            Self::DuplicateObjectId(object_id) => json!({"object_id": object_id}),
            Self::InvalidObject { object_id, message } => {
                json!({"object_id": object_id, "reason": message})
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_core::{deterministic_hash, DocumentId};
    use serde_json::json;

    fn descriptor(id: &str) -> RuntimeObjectDescriptor {
        RuntimeObjectDescriptor {
            id: ObjectId::new(id).unwrap(),
            type_id: EngineTypeId::from_static("test.object"),
            properties: json!({}),
        }
    }

    fn artifact_with_objects(objects: Vec<RuntimeObjectDescriptor>) -> ArtifactEnvelope {
        ArtifactEnvelope::new(
            ArtifactId::from_static("artifact.stage.test"),
            EngineTypeId::from_static(STAGE_RUNTIME_ARTIFACT_TYPE),
            DocumentId::from_static("stage.test"),
            STAGE_RUNTIME_COMPILER_VERSION,
            deterministic_hash(b"test"),
            serde_json::to_value(StageArtifactPayload {
                objects,
                metadata: json!({}),
            })
            .unwrap(),
        )
        .unwrap()
    }

    fn artifact() -> ArtifactEnvelope {
        artifact_with_objects(vec![descriptor("object.one")])
    }

    fn stage_owned_count(world: &mut World) -> usize {
        world.query::<&StageOwned>().iter(world).count()
    }

    fn assert_atomic_failure(
        manager: &mut StageRuntimeManager,
        world: &mut World,
        artifact: &ArtifactEnvelope,
        expected_code: &str,
    ) {
        let entities_before = world.entities().len();
        let active_before = manager.active_count();
        let error = manager.load(world, artifact).unwrap_err();
        assert_eq!(error.code(), expected_code);
        assert_eq!(world.entities().len(), entities_before);
        assert_eq!(manager.active_count(), active_before);
    }

    #[test]
    fn wrong_artifact_type_is_rejected_atomically() {
        let mut artifact = artifact();
        artifact.artifact_type = EngineTypeId::from_static("different.artifact");
        assert_atomic_failure(
            &mut StageRuntimeManager::default(),
            &mut World::new(),
            &artifact,
            RUNTIME_ARTIFACT_TYPE_MISMATCH,
        );
    }

    #[test]
    fn unsupported_version_is_rejected_atomically() {
        let mut artifact = artifact();
        artifact.compiler_version = STAGE_RUNTIME_COMPILER_VERSION + 1;
        assert_atomic_failure(
            &mut StageRuntimeManager::default(),
            &mut World::new(),
            &artifact,
            RUNTIME_ARTIFACT_VERSION_UNSUPPORTED,
        );
    }

    #[test]
    fn corrupted_payload_hash_is_rejected_atomically() {
        let mut artifact = artifact();
        artifact.payload["metadata"] = json!({"corrupted": true});
        assert_atomic_failure(
            &mut StageRuntimeManager::default(),
            &mut World::new(),
            &artifact,
            RUNTIME_ARTIFACT_INTEGRITY_FAILED,
        );
    }

    #[test]
    fn malformed_payload_is_rejected_after_valid_checksum() {
        let mut artifact = artifact();
        artifact.payload = json!({"objects": "not-an-array", "metadata": {}});
        artifact.refresh_artifact_hash().unwrap();
        assert_atomic_failure(
            &mut StageRuntimeManager::default(),
            &mut World::new(),
            &artifact,
            RUNTIME_ARTIFACT_PAYLOAD_INVALID,
        );
    }

    #[test]
    fn duplicate_object_ids_are_rejected_atomically() {
        let artifact =
            artifact_with_objects(vec![descriptor("object.one"), descriptor("object.one")]);
        assert_atomic_failure(
            &mut StageRuntimeManager::default(),
            &mut World::new(),
            &artifact,
            RUNTIME_OBJECT_ID_DUPLICATE,
        );
    }

    #[test]
    fn invalid_late_object_leaves_no_partial_entities() {
        let mut invalid = descriptor("object.invalid");
        invalid.properties = json!([]);
        let artifact = artifact_with_objects(vec![descriptor("object.valid"), invalid]);
        assert_atomic_failure(
            &mut StageRuntimeManager::default(),
            &mut World::new(),
            &artifact,
            RUNTIME_OBJECT_INVALID,
        );
    }

    #[test]
    fn tactical_object_requires_runtime_properties() {
        let mut invalid = descriptor("actor.invalid");
        invalid.type_id = EngineTypeId::from_static("tactical.actor");
        let artifact = artifact_with_objects(vec![invalid]);
        let error = StageRuntimeManager::default()
            .load(&mut World::new(), &artifact)
            .unwrap_err();
        assert_eq!(error.code(), RUNTIME_OBJECT_INVALID);
        assert_eq!(error.details()["object_id"], json!("actor.invalid"));
    }

    #[test]
    fn repeated_load_and_unload_does_not_leak_entities() {
        let mut world = World::new();
        let mut manager = StageRuntimeManager::default();
        for _ in 0..100 {
            let id = manager.load(&mut world, &artifact()).unwrap();
            assert_eq!(stage_owned_count(&mut world), 1);
            assert_eq!(manager.unload(&mut world, &id).unwrap(), 1);
            assert_eq!(stage_owned_count(&mut world), 0);
        }
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn unload_does_not_touch_global_entities() {
        let mut world = World::new();
        let global = world.spawn(GlobalPersistent).id();
        let mut manager = StageRuntimeManager::default();
        let id = manager.load(&mut world, &artifact()).unwrap();
        manager.unload(&mut world, &id).unwrap();
        assert!(world.get::<GlobalPersistent>(global).is_some());
        assert_eq!(stage_owned_count(&mut world), 0);
    }
}

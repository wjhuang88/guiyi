#![forbid(unsafe_code)]

//! Bevy ECS runtime stage ownership and repeatable load/unload lifecycle.

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use guiyi_engine_content::{ArtifactEnvelope, StageArtifactPayload};
use guiyi_engine_core::{ArtifactId, EngineTypeId, ObjectId, StageInstanceId};
use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;

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
        let payload: StageArtifactPayload = serde_json::from_value(artifact.payload.clone())?;
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
    #[error("artifact payload is invalid: {0}")]
    InvalidArtifact(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_content::RuntimeObjectDescriptor;
    use guiyi_engine_core::{deterministic_hash, DocumentId};
    use serde_json::json;

    fn artifact() -> ArtifactEnvelope {
        let payload = StageArtifactPayload {
            objects: vec![RuntimeObjectDescriptor {
                id: ObjectId::from_static("object.one"),
                type_id: EngineTypeId::from_static("test.object"),
                properties: json!({}),
            }],
            metadata: json!({}),
        };
        ArtifactEnvelope {
            id: ArtifactId::from_static("artifact.stage.test"),
            artifact_type: EngineTypeId::from_static("runtime.stage"),
            source_document: DocumentId::from_static("stage.test"),
            compiler_version: 1,
            source_hash: deterministic_hash(b"test"),
            payload: serde_json::to_value(payload).unwrap(),
        }
    }

    #[test]
    fn repeated_load_and_unload_does_not_leak_entities() {
        let mut world = World::new();
        let mut manager = StageRuntimeManager::default();
        for _ in 0..100 {
            let id = manager.load(&mut world, &artifact()).unwrap();
            assert_eq!(world.entities().count_spawned(), 1);
            assert_eq!(manager.unload(&mut world, &id).unwrap(), 1);
            assert_eq!(world.entities().count_spawned(), 0);
        }
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn unload_does_not_touch_global_entities() {
        let mut world = World::new();
        world.spawn(GlobalPersistent);
        let mut manager = StageRuntimeManager::default();
        let id = manager.load(&mut world, &artifact()).unwrap();
        manager.unload(&mut world, &id).unwrap();
        assert_eq!(world.entities().count_spawned(), 1);
    }
}

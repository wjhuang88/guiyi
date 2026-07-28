#![forbid(unsafe_code)]

//! Tactical RPG runtime markers layered on generic runtime objects.

use bevy_ecs::prelude::*;
use guiyi_engine_runtime::RuntimeObject;
use tactical_rpg_content::{ACTOR_OBJECT_TYPE, TRIGGER_OBJECT_TYPE};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalActor;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalTrigger;

pub fn decorate_runtime_objects(world: &mut World) -> usize {
    let mut query = world.query::<(Entity, &RuntimeObject)>();
    let entities = query
        .iter(world)
        .filter_map(|(entity, object)| match object.type_id.as_str() {
            ACTOR_OBJECT_TYPE => Some((entity, true)),
            TRIGGER_OBJECT_TYPE => Some((entity, false)),
            _ => None,
        })
        .collect::<Vec<_>>();
    for (entity, actor) in &entities {
        if *actor {
            world.entity_mut(*entity).insert(TacticalActor);
        } else {
            world.entity_mut(*entity).insert(TacticalTrigger);
        }
    }
    entities.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_core::{EngineTypeId, ObjectId};
    use serde_json::json;

    #[test]
    fn actor_objects_receive_actor_marker() {
        let mut world = World::new();
        let entity = world
            .spawn(RuntimeObject {
                object_id: ObjectId::from_static("actor.one"),
                type_id: EngineTypeId::from_static(ACTOR_OBJECT_TYPE),
                properties: json!({}),
            })
            .id();
        assert_eq!(decorate_runtime_objects(&mut world), 1);
        assert!(world.get::<TacticalActor>(entity).is_some());
    }
}

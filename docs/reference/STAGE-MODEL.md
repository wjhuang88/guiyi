# Stage model

A Stage is a loadable and independently previewable gameplay unit. It is not a raw serialization of a Bevy `World`.

## Authoring

`StageDocument` contains stable object IDs, coordinate-space settings, placed objects, triggers, spawn points, and semantic connections.

Stage authoring is machine-first. Normal Stage edits should be exposed as semantic typed Commands rather than requiring direct document-file mutation or an editor-only gesture. A visual Editor, when present, invokes the same command/query platform used by Agent clients.

### Semantic object editing

`stage.move_object` moves one existing `StageObject` by stable `ObjectId` within its Stage. Its input is `stage_id`, `object_id`, `q`, and `r`. Moving an object preserves the stable ID, object kind, properties, and semantic references; only its position changes.

`stage.remove_object` removes one existing `StageObject` by stable `ObjectId`. Removing an Actor also removes that object's derived `actor_definition` reference when the Stage envelope is regenerated. On an applied transaction, the normal Project Graph refresh therefore observes the reference removal without a second mutation path.

Both Commands:

- require GUIYI `EditContent` authority and declare access to the target Stage;
- are registered in the live tactical Tool catalog with machine-readable schemas;
- execute through the normal Command/AgentHost permission, working-set, dry-run, transaction, diff, audit, persistence, and graph-refresh boundaries;
- validate the candidate Stage with `validate_stage` before commit;
- support normal dry-run preview without changing authoritative documents;
- return `STAGE_OBJECT_NOT_FOUND` with Stage/object location when the stable object ID does not exist.

Out-of-bounds move candidates reuse the existing `STAGE_OBJECT_OUT_OF_BOUNDS` diagnostic, including Stage ID, object ID, and the position field location. Non-blocking diagnostics created by a legal removal, such as `STAGE_NO_SPAWN_POINT` after deleting the last spawn point, remain in the transaction report for Agent repair/planning.

ENG-013 deliberately does not define batch movement/removal, connection editing, occupancy/collision, pathfinding, or game-specific tactical rules.

## Build

`StageCompiler` resolves the Stage into a generic `StageArtifactPayload` containing runtime object descriptors. Invalid authoring content must be rejected before runtime.

## Runtime

`StageRuntimeManager` creates a new Stage instance ID and adds `StageOwned` to every spawned entity. Unload deletes only entities owned by that instance; global persistent entities remain.

## Toolkit scope

The toolkit includes Actor, Trigger, SpawnPoint, Marker, and StageConnection concepts. It does not include any specific game's resurrection, lifespan, faction, or world-state mechanics.

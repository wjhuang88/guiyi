# Stage model

A Stage is a loadable and independently previewable gameplay unit. It is not a raw serialization of a Bevy `World`.

## Authoring

`StageDocument` contains stable object IDs, coordinate-space settings, placed objects, triggers, spawn points, and semantic connections.

## Build

`StageCompiler` resolves the Stage into a generic `StageArtifactPayload` containing runtime object descriptors. Invalid authoring content must be rejected before runtime.

## Runtime

`StageRuntimeManager` creates a new Stage instance ID and adds `StageOwned` to every spawned entity. Unload deletes only entities owned by that instance; global persistent entities remain.

## Toolkit scope

The toolkit includes Actor, Trigger, SpawnPoint, Marker, and StageConnection concepts. It does not include any specific game's resurrection, lifespan, faction, or world-state mechanics.

# Stage model

A Stage is a loadable and independently previewable gameplay unit. It is not a raw serialization of a Bevy `World`.

## Authoring

`StageDocument` contains stable object IDs, coordinate-space settings, navigation authoring data, placed objects, triggers, spawn points, and semantic connections.

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

ENG-013 deliberately does not define batch movement/removal, connection editing, occupancy/collision, pathfinding, or game-specific tactical rules. Executable acceptance and rollback evidence is recorded in `docs/validation/ENG-013.md`.

### Navigation authoring and reachability

ENG-020 adds a small persisted navigation authoring surface directly to the Stage instead of creating a second navigation file or graph truth. `StageDocument.navigation.blocked` is the ordered authoring list of blocked grid cells. It defaults to empty, so Stage payloads written before ENG-020 continue to deserialize without migration.

Blocked-cell navigation applies only to `HexAxial` and `Square` coordinate spaces. `Free2d` and `Free3d` Stages do not pretend to have grid traversal; machine-facing reachability returns a structured unsupported result for those spaces.

The tactical content crate owns deterministic unweighted grid traversal:

- Hex axial neighbor order is fixed as east, south-east, south-west, west, north-west, north-east in axial delta form `(1,0)`, `(0,1)`, `(-1,1)`, `(-1,0)`, `(0,-1)`, `(1,-1)`;
- Square neighbor order is fixed as east, south, west, north: `(1,0)`, `(0,1)`, `(-1,0)`, `(0,-1)`;
- breadth-first search returns a deterministic shortest path for equal-cost grid movement;
- blocked cells and cells outside Stage bounds are never traversed;
- start and goal coordinates must be inside grid bounds;
- object occupancy is deliberately not treated as a navigation blocker in ENG-020.

`stage.set_navigation_blocked(stage_id, q, r, blocked)` is the semantic authoring Command for one cell. It requires `EditContent`, declares access to only the target Stage, is idempotent, and uses the normal schema, candidate validation, dry-run, transaction, diff, audit, persistence, permission, and working-set path. The stored blocked-cell list is kept deterministic after semantic mutation. A visual navigation painter, if added later, must call this same authoritative capability rather than write a private editor format.

`stage.reachability(stage_id, from_object_id, to_object_id)` is the read-only semantic Query for two stable Stage objects. It requires GUIYI read/validation authority and target-Stage access. For grid Stages it returns:

- `reachable: true`, shortest `distance`, and a deterministic coordinate `path` when connected;
- `reachable: false`, `distance: null`, and an empty path when disconnected;
- a structured `STAGE_OBJECT_NOT_FOUND` diagnostic result when either stable object ID is missing;
- a structured `STAGE_NAV_UNSUPPORTED_COORDINATE_SPACE` result for `Free2d`/`Free3d`;
- a structured `STAGE_NAV_ENDPOINT_OUT_OF_BOUNDS` result if an endpoint encoded by Stage content is outside grid bounds.

Domain-invalid/unsupported reachability is represented in the Query's machine-readable result instead of adding a tactical-specific branch to the generic engine `QueryError` protocol. GUIYI `AgentHost` remains authoritative for permission and working-set rejection before the Query executes.

`validate_stage` adds navigation diagnostics to the same shared validator used by semantic Commands and build validation:

- `STAGE_NAV_BLOCKED_CELL_OUT_OF_BOUNDS` — error for a blocked grid coordinate outside Stage bounds;
- `STAGE_NAV_DUPLICATE_BLOCKED_CELL` — warning for duplicate persisted blocked coordinates; semantic mutation itself canonicalizes the normal authored list;
- `STAGE_NAV_SPAWN_BLOCKED` — error when a spawn point occupies a blocked cell;
- `STAGE_OBJECT_UNREACHABLE` — error when a non-spawn Stage object cannot be reached from any spawn point on a structurally valid grid Stage;
- `STAGE_NAV_UNSUPPORTED_COORDINATE_SPACE` — error when blocked-cell authoring data is attached to a free-space Stage.

Reachability validation intentionally does not define movement cost, terrain types, dynamic occupancy/collision, jump/fly/swim modes, line of sight, cover, turn rules, navmesh generation, or runtime actor path-following. Those are separate tactical/runtime concerns rather than hidden assumptions in the ENG-020 grid contract.

Executable acceptance and rollback evidence is recorded in `docs/validation/ENG-020.md`.

### Intent-level local playability repair

ENG-023 adds `tactical.make_stage_playable(stage_id, spawn_object_id?, spawn_profile?)` as a higher-level semantic Command. It does not create a second Stage-validity system. The existing `validate_stage` result remains authoritative, and this intent defines **locally playable** only as:

1. `validate_stage` reports no `Error` diagnostics; and
2. `STAGE_NO_SPAWN_POINT` is absent, because a standard preview profile otherwise has no Stage entry point.

The optional fallback spawn identity/profile default to `spawn.player` and `player` in the machine-readable input schema. These values are used only when a missing spawn must be created; collisions or invalid candidates are reported rather than silently renamed.

Before mutation, the intent derives a deterministic ordered repair plan from the authoritative Stage. Every authored action is exposed as an existing primitive semantic Command ID plus machine-readable input and reason codes. ENG-023 deliberately limits automatic repair to:

- unblocking an existing spawn cell through `stage.set_navigation_blocked(..., blocked=false)` when `STAGE_NAV_SPAWN_BLOCKED` applies; and
- creating one deterministic fallback spawn through `stage.create_spawn` when `STAGE_NO_SPAWN_POINT` is the repairable gap. Grid candidates are considered in stable `q`-major, then `r` order, skipping blocked cells, and a candidate is accepted only when the resulting Stage satisfies the same local-playability predicate.

The intent preflights the complete candidate on cloned engine state and reuses the existing primitive Command mutation implementations. Apply then runs as one normal outer GUIYI Command transaction. There are no synthetic sub-transaction IDs, no second audit log, and no direct private `StageDocument` mutation path for fields already owned by the primitive Commands. Normal permission, working-set, schema, dry-run, diff, persistence, audit, and applied-only Project Graph refresh behavior remains authoritative in `CommandExecutor` / `AgentHost`.

A successful machine result contains the Stage ID, `playable_before`, `playable_after`, ordered plan, and before/after diagnostics. The normal `ToolResult.transaction` remains transaction truth. An already-playable Stage produces an empty plan and no content diff. Dry-run produces normal preview evidence without authoritative mutation.

ENG-023 does not attempt to repair out-of-bounds objects, duplicate IDs, invalid dimensions, unsupported navigation state, arbitrary unreachable topology, malformed tactical rules, cross-document build/reference failures, connection design, or defects requiring deletion/renaming/game-specific choices. Those remain structured blockers from the existing authorities; the intent rejects atomically instead of applying a partial best-effort repair.

Local playability is intentionally not whole-project buildability. The intent does not start Talos, run a model, invoke build/preview automatically, create Encounter/Dialogue content, or own runtime combat/world state. The tactical demo proves that an intent-repaired Stage can continue through the existing validation/build/runtime path; ENG-030 owns the real Talos-driven production proof.

Executable acceptance and rollback evidence is recorded in `docs/validation/ENG-023.md`.

### Shared tactical rule validation

ENG-022 fulfills the pre-existing `stage.create_trigger` promise that Trigger conditions/effects are registry-defined. Trigger authoring keeps the same persisted JSON invocation shape: each condition/effect is an object with a string `type` and descriptor-specific fields. `validate_stage` validates those invocations through the same default tactical rule registry used by Dialogue.

The minimum registry keeps the existing sample effect `{ "type": "emit_signal", "signal": "..." }` valid and adds condition `{ "type": "always" }`. Malformed type values report `TACTICAL_RULE_TYPE_INVALID`, unknown registered types report `TACTICAL_RULE_UNKNOWN`, and descriptor-schema mismatches report `TACTICAL_RULE_SCHEMA_INVALID`, with Stage/trigger/field-path context. This is an intentional validation tightening for previously unchecked invalid payloads; Stage serialization and the `stage.create_trigger` Tool schema do not change.

Agents discover the rule vocabulary through the read-only `tactical.rules.catalog` Query rather than duplicated prompt text. A visual trigger editor, if added later, must consume the same catalog and semantic Command path. Full shared-rule and Dialogue semantics are documented in `docs/reference/DIALOGUE-MODEL.md`.

## Build

`StageCompiler` resolves the Stage into a generic `StageArtifactPayload` containing runtime object descriptors. Invalid authoring content must be rejected before runtime. Navigation authoring data is preserved in Stage artifact metadata so runtime/tooling consumers can inspect the same authored blocked cells without creating a second source of truth.

## Runtime

`StageRuntimeManager` creates a new Stage instance ID and adds `StageOwned` to every spawned entity. Unload deletes only entities owned by that instance; global persistent entities remain.

ENG-020 does not add runtime path-following or movement AI. The runtime may consume compiled navigation metadata in later Stories, but authoring/validation ownership remains in the tactical toolkit.

## Toolkit scope

The toolkit includes Actor, Trigger, SpawnPoint, Marker, StageConnection, reusable Stage grid-navigation concepts, and bounded intent-level Stage repair. It does not include any specific game's resurrection, lifespan, faction, world-state mechanics, or game-specific movement rules.

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

The toolkit includes Actor, Trigger, SpawnPoint, Marker, StageConnection, and reusable Stage grid-navigation concepts. It does not include any specific game's resurrection, lifespan, faction, world-state mechanics, or game-specific movement rules.

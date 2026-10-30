# Encounter model

An Encounter is an authoring document that binds one tactical Stage to a stable set of participants. It describes composition, not live combat state.

## Ownership

`EncounterDocument` is owned by the tactical-RPG toolkit. It contains:

- a stable Encounter `DocumentId`;
- a display name;
- one authoritative Stage `DocumentId`;
- ordered `EncounterParticipant` entries.

Each participant has a stable `ObjectId`, an actor-definition `DocumentId`, a Stage spawn-point `ObjectId`, and a non-empty side string.

Encounter does not own Stage geometry/navigation, actor-definition data, runtime initiative, turn scheduling, damage, AI tactics, victory/defeat state, rewards, or save-state progression.

## Semantic references

`EncounterDocument::to_envelope` emits authoritative `DocumentEnvelope.references`:

- `encounter_stage`: Encounter -> Stage;
- `encounter_actor_definition`: participant -> actor-definition document;
- `encounter_spawn_point`: participant -> spawn object in the Encounter Stage.

The Project Graph remains derived from these references. There is no separate Encounter graph or persisted relationship index.

## Compiler

`EncounterCompiler` compiles `tactical.encounter.document` to `tactical.encounter.artifact` through the normal `ArtifactEnvelope` source-hash and integrity path.

The artifact payload preserves the Encounter name, Stage ID, participant order, stable participant IDs, actor-definition IDs, spawn-point IDs, and sides. The compiler performs deterministic local authoring checks; project-aware reference and spawn semantics remain validation/build responsibilities rather than hidden compiler reads.

Actor-definition documents remain authoring-only for this slice. CLI and tactical-demo build profiles register both Stage and Encounter compilers while marking the actor-definition type authoring-only.

## Validation

Local Encounter diagnostics include:

- `ENCOUNTER_NAME_EMPTY`;
- `ENCOUNTER_NO_PARTICIPANTS` as a non-blocking incremental-authoring warning;
- `ENCOUNTER_DUPLICATE_PARTICIPANT_ID`;
- `ENCOUNTER_PARTICIPANT_SIDE_EMPTY`.

Project-aware validation additionally reports:

- `ENCOUNTER_STAGE_NOT_FOUND`;
- `ENCOUNTER_STAGE_INVALID_TYPE`;
- `ENCOUNTER_SPAWN_NOT_FOUND`;
- `ENCOUNTER_SPAWN_INVALID_KIND`.

Generic unresolved document references continue to use the existing build/reference gates. Encounter does not duplicate the engine-wide missing-reference system.

## Semantic authoring Commands

`encounter.create(encounter_id, name, stage_id)` creates a new Encounter bound to an existing Stage. A newly created Encounter may contain zero participants; the transaction reports `ENCOUNTER_NO_PARTICIPANTS` so an Agent can continue authoring incrementally.

`encounter.add_participant(encounter_id, stage_id, participant_id, actor_definition, spawn_point, side)` adds one stable participant. `stage_id` is an access-plan assertion: it must equal the persisted Encounter Stage and exists so AgentHost can enforce working-set authority over every document the Command reads. It is not a second Stage truth.

`encounter.remove_participant(encounter_id, participant_id)` removes by stable participant ID. Removing the final participant is legal and returns the no-participants warning; an unknown ID returns `ENCOUNTER_PARTICIPANT_NOT_FOUND` without mutation.

All three Commands use the normal GUIYI schema, permission, document-access, dry-run, transaction, diff, audit, persistence, and Project Graph refresh boundaries. They are live catalog capabilities and therefore flow through the existing Talos adapter without Talos-specific registration.

## Agent-first development surface

Encounter composition is a normal development task and must be possible without a GUI-only table, hidden editor state, or direct file patching. Agents author Encounter state through semantic Commands and inspect relationships through the existing document and Project Graph Queries.

A future visual Encounter editor is a secondary client of the same Commands, Queries, diagnostics, transactions, and project truth. It must not create a parallel Encounter format or mutation path.

## ENG-021 boundary

ENG-021 intentionally excludes runtime combat systems, conditions/effects/dialogue, intent-level playable-stage automation, party/inventory/quest systems, and visual Encounter UI. Those concerns require separately authorized Stories.

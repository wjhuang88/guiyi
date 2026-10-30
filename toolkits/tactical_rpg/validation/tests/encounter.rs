use guiyi_engine_content::DocumentStore;
use guiyi_engine_core::{DocumentId, ObjectId};
use serde_json::json;
use tactical_rpg_content::{
    EncounterDocument, EncounterParticipant, HexCoord, StageDocument, StageObject, StageObjectKind,
};
use tactical_rpg_validation::{
    validate_encounter, validate_encounter_project, ENCOUNTER_DUPLICATE_PARTICIPANT_ID,
    ENCOUNTER_NAME_EMPTY, ENCOUNTER_NO_PARTICIPANTS, ENCOUNTER_PARTICIPANT_SIDE_EMPTY,
    ENCOUNTER_SPAWN_INVALID_KIND, ENCOUNTER_SPAWN_NOT_FOUND, ENCOUNTER_STAGE_INVALID_TYPE,
    ENCOUNTER_STAGE_NOT_FOUND,
};

fn stage() -> StageDocument {
    let mut stage = StageDocument::new_hex(DocumentId::from_static("stage.demo"), "Demo", 4, 4);
    stage.objects.push(StageObject {
        id: ObjectId::from_static("spawn.enemy"),
        position: HexCoord::new(1, 1),
        object: StageObjectKind::SpawnPoint {
            profile: "enemy".into(),
        },
        properties: json!({}),
    });
    stage.objects.push(StageObject {
        id: ObjectId::from_static("marker.not_spawn"),
        position: HexCoord::new(2, 1),
        object: StageObjectKind::Marker {
            marker_type: "test".into(),
        },
        properties: json!({}),
    });
    stage
}

fn participant(spawn: &str) -> EncounterParticipant {
    EncounterParticipant {
        id: ObjectId::from_static("participant.guard"),
        actor_definition: DocumentId::from_static("actor.guard"),
        spawn_point: ObjectId::new(spawn).unwrap(),
        side: "enemy".into(),
    }
}

#[test]
fn encounter_validation_reports_local_authoring_failures() {
    let mut encounter = EncounterDocument::new(
        DocumentId::from_static("encounter.demo"),
        "",
        DocumentId::from_static("stage.demo"),
    );
    let bag = validate_encounter(&encounter);
    assert!(bag
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_NAME_EMPTY));
    assert!(bag
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_NO_PARTICIPANTS));

    let mut first = participant("spawn.enemy");
    first.side.clear();
    encounter.name = "Demo".into();
    encounter.participants = vec![first.clone(), first];
    let bag = validate_encounter(&encounter);
    assert!(bag
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_DUPLICATE_PARTICIPANT_ID));
    assert!(bag
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_PARTICIPANT_SIDE_EMPTY));
}

#[test]
fn encounter_project_validation_checks_stage_and_spawn_semantics() {
    let mut store = DocumentStore::default();
    store.insert(stage().to_envelope().unwrap()).unwrap();

    let mut encounter = EncounterDocument::new(
        DocumentId::from_static("encounter.demo"),
        "Demo",
        DocumentId::from_static("stage.demo"),
    );
    encounter.participants.push(participant("spawn.enemy"));
    assert!(!validate_encounter_project(&encounter, &store).has_errors());

    encounter.participants[0].spawn_point = ObjectId::from_static("spawn.missing");
    let missing = validate_encounter_project(&encounter, &store);
    assert!(missing
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_SPAWN_NOT_FOUND));

    encounter.participants[0].spawn_point = ObjectId::from_static("marker.not_spawn");
    let invalid = validate_encounter_project(&encounter, &store);
    assert!(invalid
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_SPAWN_INVALID_KIND));
}

#[test]
fn encounter_project_validation_reports_missing_or_wrong_stage() {
    let encounter = EncounterDocument::new(
        DocumentId::from_static("encounter.demo"),
        "Demo",
        DocumentId::from_static("stage.missing"),
    );
    let store = DocumentStore::default();
    let missing = validate_encounter_project(&encounter, &store);
    assert!(missing
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_STAGE_NOT_FOUND));

    let mut wrong = DocumentStore::default();
    let mut wrong_document = stage().to_envelope().unwrap();
    wrong_document.header.id = DocumentId::from_static("stage.missing");
    wrong_document.header.type_id = guiyi_engine_core::EngineTypeId::from_static("other.document");
    wrong.upsert(wrong_document);
    let invalid = validate_encounter_project(&encounter, &wrong);
    assert!(invalid
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_STAGE_INVALID_TYPE));
}

use guiyi_engine_content::{CompileContext, DocumentCompiler};
use guiyi_engine_core::{DocumentId, ObjectId};
use tactical_rpg_content::{
    EncounterArtifactPayload, EncounterCompiler, EncounterDocument, EncounterParticipant,
    ENCOUNTER_ARTIFACT_TYPE, ENCOUNTER_DOCUMENT_TYPE,
};

fn encounter() -> EncounterDocument {
    EncounterDocument {
        id: DocumentId::from_static("encounter.demo"),
        name: "Demo Encounter".into(),
        stage: DocumentId::from_static("stage.demo"),
        participants: vec![EncounterParticipant {
            id: ObjectId::from_static("participant.guard"),
            actor_definition: DocumentId::from_static("actor.guard"),
            spawn_point: ObjectId::from_static("spawn.enemy"),
            side: "enemy".into(),
        }],
    }
}

fn context() -> CompileContext {
    CompileContext {
        project_root: ".".into(),
        profile: "test".into(),
    }
}

#[test]
fn encounter_round_trip_emits_deterministic_semantic_references() {
    let encounter = encounter();
    let envelope = encounter.to_envelope().unwrap();
    assert_eq!(envelope.header.type_id.as_str(), ENCOUNTER_DOCUMENT_TYPE);
    assert_eq!(envelope.references.len(), 3);

    assert_eq!(envelope.references[0].kind, "encounter_stage");
    assert_eq!(envelope.references[0].target_document.as_str(), "stage.demo");
    assert!(envelope.references[0].source_object.is_none());

    assert_eq!(envelope.references[1].kind, "encounter_actor_definition");
    assert_eq!(
        envelope.references[1].source_object.as_ref().unwrap().as_str(),
        "participant.guard"
    );
    assert_eq!(envelope.references[1].target_document.as_str(), "actor.guard");

    assert_eq!(envelope.references[2].kind, "encounter_spawn_point");
    assert_eq!(envelope.references[2].target_document.as_str(), "stage.demo");
    assert_eq!(
        envelope.references[2].target_object.as_ref().unwrap().as_str(),
        "spawn.enemy"
    );

    assert_eq!(EncounterDocument::from_envelope(&envelope).unwrap(), encounter);
}

#[test]
fn encounter_compiler_preserves_stable_participants_and_integrity() {
    let encounter = encounter();
    let envelope = encounter.to_envelope().unwrap();
    let artifact = EncounterCompiler.compile(&envelope, &context()).unwrap();
    assert_eq!(artifact.artifact_type.as_str(), ENCOUNTER_ARTIFACT_TYPE);
    assert_eq!(artifact.source_hash, envelope.content_hash().unwrap());
    assert_eq!(artifact.artifact_hash, artifact.compute_artifact_hash().unwrap());

    let payload: EncounterArtifactPayload = serde_json::from_value(artifact.payload).unwrap();
    assert_eq!(payload.name, "Demo Encounter");
    assert_eq!(payload.stage.as_str(), "stage.demo");
    assert_eq!(payload.participants, encounter.participants);
}

#[test]
fn encounter_compiler_rejects_invalid_authoring_content() {
    let mut empty_name = encounter();
    empty_name.name = "  ".into();
    assert!(EncounterCompiler
        .compile(&empty_name.to_envelope().unwrap(), &context())
        .is_err());

    let mut duplicate = encounter();
    duplicate.participants.push(duplicate.participants[0].clone());
    assert!(EncounterCompiler
        .compile(&duplicate.to_envelope().unwrap(), &context())
        .is_err());

    let mut empty_side = encounter();
    empty_side.participants[0].side = "".into();
    assert!(EncounterCompiler
        .compile(&empty_side.to_envelope().unwrap(), &context())
        .is_err());
}

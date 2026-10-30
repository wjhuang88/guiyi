use guiyi_engine_build::{BuildPipeline, BuildProfile, BUILD_UNRESOLVED_REFERENCE};
use guiyi_engine_content::{
    CompileContext, CompilerRegistry, DocumentEnvelope, DocumentHeader, DocumentStore,
};
use guiyi_engine_core::{DocumentId, EngineTypeId, ObjectId};
use serde_json::json;
use tactical_rpg_content::{
    EncounterCompiler, EncounterDocument, EncounterParticipant, HexCoord, StageCompiler,
    StageDocument, StageObject, StageObjectKind, ACTOR_DEFINITION_TYPE,
};

fn actor_definition() -> DocumentEnvelope {
    DocumentEnvelope {
        header: DocumentHeader {
            id: DocumentId::from_static("actor.guard"),
            type_id: EngineTypeId::from_static(ACTOR_DEFINITION_TYPE),
            schema_version: 1,
            display_name: "Guard".into(),
        },
        references: Vec::new(),
        payload: json!({"name": "Guard"}),
    }
}

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
    stage
}

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

fn pipeline() -> BuildPipeline {
    let mut compilers = CompilerRegistry::default();
    compilers.register(StageCompiler).unwrap();
    compilers.register(EncounterCompiler).unwrap();
    let profile = BuildProfile::default()
        .with_authoring_only_type(EngineTypeId::from_static(ACTOR_DEFINITION_TYPE));
    BuildPipeline::with_profile(compilers, profile)
}

fn context() -> CompileContext {
    CompileContext {
        project_root: ".".into(),
        profile: "test".into(),
    }
}

#[test]
fn strict_build_compiles_stage_and_encounter_together() {
    let mut store = DocumentStore::default();
    store.insert(actor_definition()).unwrap();
    store.insert(stage().to_envelope().unwrap()).unwrap();
    store.insert(encounter().to_envelope().unwrap()).unwrap();

    let report = pipeline().build(&store, &context());
    assert!(report.succeeded(), "diagnostics: {:?}", report.diagnostics);
    assert_eq!(report.artifacts.len(), 2);
    assert!(report
        .artifacts
        .iter()
        .any(|artifact| artifact.source_document.as_str() == "stage.demo"));
    assert!(report
        .artifacts
        .iter()
        .any(|artifact| artifact.source_document.as_str() == "encounter.demo"));
}

#[test]
fn strict_build_reuses_generic_missing_reference_gate() {
    let mut store = DocumentStore::default();
    store.insert(stage().to_envelope().unwrap()).unwrap();
    store.insert(encounter().to_envelope().unwrap()).unwrap();

    let report = pipeline().build(&store, &context());
    assert!(!report.succeeded());
    assert!(report.artifacts.is_empty());
    assert!(report
        .diagnostics
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == BUILD_UNRESOLVED_REFERENCE));
}

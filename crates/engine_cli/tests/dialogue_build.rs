use guiyi_engine_build::{BuildPipeline, BuildProfile};
use guiyi_engine_content::{
    CompileContext, CompilerRegistry, DocumentEnvelope, DocumentHeader, DocumentStore,
};
use guiyi_engine_core::{DocumentId, EngineTypeId, ObjectId};
use serde_json::json;
use tactical_rpg_content::{
    DialogueChoice, DialogueCompiler, DialogueDocument, DialogueNode, EncounterCompiler,
    EncounterDocument, EncounterParticipant, HexCoord, StageCompiler, StageDocument, StageObject,
    StageObjectKind, ACTOR_DEFINITION_TYPE,
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

fn dialogue() -> DialogueDocument {
    DialogueDocument {
        id: DocumentId::from_static("dialogue.demo"),
        name: "Gate Talk".into(),
        entry_node: ObjectId::from_static("node.entry"),
        nodes: vec![DialogueNode {
            id: ObjectId::from_static("node.entry"),
            speaker: "Guard".into(),
            text: "Halt.".into(),
            choices: vec![DialogueChoice {
                id: ObjectId::from_static("choice.end"),
                text: "Understood.".into(),
                target: None,
                conditions: vec![json!({"type": "always"})],
                effects: vec![json!({
                    "type": "emit_signal",
                    "signal": "dialogue.complete"
                })],
            }],
        }],
    }
}

fn pipeline() -> BuildPipeline {
    let mut compilers = CompilerRegistry::default();
    compilers.register(StageCompiler).unwrap();
    compilers.register(EncounterCompiler).unwrap();
    compilers.register(DialogueCompiler).unwrap();
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

fn valid_store() -> DocumentStore {
    let mut store = DocumentStore::default();
    store.insert(actor_definition()).unwrap();
    store.insert(stage().to_envelope().unwrap()).unwrap();
    store.insert(encounter().to_envelope().unwrap()).unwrap();
    store.insert(dialogue().to_envelope().unwrap()).unwrap();
    store
}

#[test]
fn strict_build_compiles_stage_encounter_and_dialogue_together() {
    let report = pipeline().build(&valid_store(), &context());
    assert!(report.succeeded(), "diagnostics: {:?}", report.diagnostics);
    assert_eq!(report.artifacts.len(), 3);
    for source in ["stage.demo", "encounter.demo", "dialogue.demo"] {
        assert!(report
            .artifacts
            .iter()
            .any(|artifact| artifact.source_document.as_str() == source));
    }
}

#[test]
fn blocking_dialogue_rule_failure_produces_no_partial_artifacts() {
    let mut store = valid_store();
    let mut invalid = dialogue();
    invalid.nodes[0].choices[0].effects = vec![json!({
        "type": "emit_signal",
        "signal": ""
    })];
    store.upsert(invalid.to_envelope().unwrap());

    let report = pipeline().build(&store, &context());
    assert!(!report.succeeded());
    assert!(report.artifacts.is_empty());
    assert!(report
        .diagnostics
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("emit_signal")));
}

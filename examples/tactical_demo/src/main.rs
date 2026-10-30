#![forbid(unsafe_code)]

use bevy_ecs::world::World;
use guiyi_engine_build::{BuildPipeline, BuildProfile};
use guiyi_engine_command::{
    CommandContext, CommandExecutor, CommandRegistry, CommandRequest, EngineState,
};
use guiyi_engine_content::{CompileContext, CompilerRegistry, DocumentEnvelope, DocumentHeader};
use guiyi_engine_core::{DocumentId, EngineTypeId, PermissionSet, ToolId};
use guiyi_engine_runtime::StageRuntimeManager;
use serde_json::json;
use tactical_rpg_content::{EncounterCompiler, StageCompiler, ACTOR_DEFINITION_TYPE};
use tactical_rpg_tools::register_tactical_commands;

fn main() {
    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    let mut executor = CommandExecutor::new(commands);
    let context = CommandContext {
        actor: "tactical-demo".into(),
        permissions: PermissionSet::content_author(),
    };
    let mut state = EngineState::default();
    state
        .documents
        .insert(DocumentEnvelope {
            header: DocumentHeader {
                id: DocumentId::from_static("actor.guard.basic"),
                type_id: EngineTypeId::from_static(ACTOR_DEFINITION_TYPE),
                schema_version: 1,
                display_name: "Basic Guard".into(),
            },
            references: Vec::new(),
            payload: json!({"health": 10, "movement": 4}),
        })
        .unwrap();

    for (tool, input) in [
        (
            "stage.create",
            json!({"id": "stage.demo", "name": "AI Test Range", "width": 8, "height": 8}),
        ),
        (
            "stage.create_spawn",
            json!({"stage_id": "stage.demo", "object_id": "spawn.player", "profile": "player", "q": 1, "r": 1}),
        ),
        (
            "stage.place_actor",
            json!({"stage_id": "stage.demo", "object_id": "actor.guard", "definition": "actor.guard.basic", "q": 4, "r": 3}),
        ),
        (
            "encounter.create",
            json!({"encounter_id": "encounter.demo", "name": "Demo Encounter", "stage_id": "stage.demo"}),
        ),
        (
            "encounter.add_participant",
            json!({
                "encounter_id": "encounter.demo",
                "stage_id": "stage.demo",
                "participant_id": "participant.guard",
                "actor_definition": "actor.guard.basic",
                "spawn_point": "spawn.player",
                "side": "enemy"
            }),
        ),
    ] {
        executor
            .execute(
                CommandRequest {
                    command: ToolId::new(tool).unwrap(),
                    input,
                    dry_run: false,
                },
                &context,
                &mut state,
            )
            .unwrap();
    }

    let mut compilers = CompilerRegistry::default();
    compilers.register(StageCompiler).unwrap();
    compilers.register(EncounterCompiler).unwrap();
    let profile = BuildProfile::default()
        .with_authoring_only_type(EngineTypeId::from_static(ACTOR_DEFINITION_TYPE));
    let report = BuildPipeline::with_profile(compilers, profile).build(
        &state.documents,
        &CompileContext {
            project_root: ".".into(),
            profile: "demo".into(),
        },
    );
    assert!(report.succeeded(), "diagnostics: {:?}", report.diagnostics);
    assert_eq!(report.artifacts.len(), 2);
    assert_eq!(report.skipped_documents.len(), 1);

    let stage_artifact = report
        .artifacts
        .iter()
        .find(|artifact| artifact.source_document.as_str() == "stage.demo")
        .unwrap();
    let mut world = World::new();
    let mut runtime = StageRuntimeManager::default();
    let instance = runtime.load(&mut world, stage_artifact).unwrap();
    println!(
        "stage instance {} loaded with {} entities; Encounter compiled",
        instance,
        world.entities().len()
    );
}

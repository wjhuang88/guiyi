#![forbid(unsafe_code)]

use bevy_ecs::world::World;
use guiyi_engine_build::BuildPipeline;
use guiyi_engine_command::{
    CommandContext, CommandExecutor, CommandRegistry, CommandRequest, EngineState,
};
use guiyi_engine_content::{CompileContext, CompilerRegistry};
use guiyi_engine_core::{PermissionSet, ToolId};
use guiyi_engine_runtime::StageRuntimeManager;
use serde_json::json;
use tactical_rpg_content::StageCompiler;
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
    let report = BuildPipeline::new(compilers).build(
        &state.documents,
        &CompileContext {
            project_root: ".".into(),
            profile: "demo".into(),
        },
    );
    assert!(report.succeeded());
    let mut world = World::new();
    let mut runtime = StageRuntimeManager::default();
    let instance = runtime.load(&mut world, &report.artifacts[0]).unwrap();
    println!(
        "stage instance {} loaded with {} entities",
        instance,
        world.entities().len()
    );
}

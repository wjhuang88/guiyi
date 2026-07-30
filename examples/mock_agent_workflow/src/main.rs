#![forbid(unsafe_code)]

use guiyi_engine_agent_host::{AgentDirective, AgentHost, AgentSession, ScriptedAgentDriver};
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_command::{CommandExecutor, CommandRegistry, EngineState};
use guiyi_engine_core::{AgentSessionId, PermissionSet, ToolId};
use guiyi_engine_protocol::ToolCall;
use guiyi_engine_query::{register_builtin_queries, QueryExecutor, QueryRegistry};
use serde_json::json;
use tactical_rpg_tools::{register_tactical_commands, register_tactical_queries};

fn main() {
    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).unwrap();
    register_tactical_queries(&mut queries).unwrap();
    let catalog = ToolCatalog::from_registries(&commands, &queries);
    let mut host = AgentHost::new(
        EngineState::default(),
        CommandExecutor::new(commands),
        QueryExecutor::new(queries),
        catalog,
    );
    let mut session = AgentSession::new(
        AgentSessionId::from_static("session.demo"),
        "Create a playable tactical Stage",
        PermissionSet::content_author(),
    );
    let mut driver = ScriptedAgentDriver::new([
        AgentDirective::Tool(ToolCall {
            id: "1".into(),
            tool: ToolId::from_static("stage.create"),
            input: json!({"id": "stage.agent", "name": "Agent Stage", "width": 6, "height": 6}),
            dry_run: true,
        }),
        AgentDirective::Tool(ToolCall {
            id: "2".into(),
            tool: ToolId::from_static("stage.create"),
            input: json!({"id": "stage.agent", "name": "Agent Stage", "width": 6, "height": 6}),
            dry_run: false,
        }),
        AgentDirective::Tool(ToolCall {
            id: "3".into(),
            tool: ToolId::from_static("stage.create_spawn"),
            input: json!({"stage_id": "stage.agent", "object_id": "spawn.player", "profile": "player", "q": 1, "r": 1}),
            dry_run: false,
        }),
        AgentDirective::Tool(ToolCall {
            id: "4".into(),
            tool: ToolId::from_static("stage.validate"),
            input: json!({"stage_id": "stage.agent"}),
            dry_run: false,
        }),
        AgentDirective::Complete {
            summary: "Created and validated the Stage".into(),
        },
    ]);
    host.run(&mut driver, &mut session).unwrap();
    println!("{}", serde_json::to_string_pretty(&session).unwrap());
}

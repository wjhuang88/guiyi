use guiyi_engine_agent_host::{
    AgentHost, AgentSession, AGENT_PERMISSION_DENIED, AGENT_WORKING_SET_DENIED,
};
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_command::{
    CommandContext, CommandError, CommandExecutor, CommandRegistry, CommandRequest, EngineState,
    TransactionStatus,
};
use guiyi_engine_core::{AgentSessionId, DocumentId, PermissionSet, ToolId};
use guiyi_engine_protocol::{ToolCall, ToolResultStatus};
use guiyi_engine_query::{register_builtin_queries, QueryExecutor, QueryRegistry, QueryRequest};
use serde_json::json;
use tactical_rpg_content::DialogueDocument;
use tactical_rpg_tools::{register_tactical_commands, register_tactical_queries};
use tactical_rpg_validation::{DIALOGUE_CHOICE_TARGET_NOT_FOUND, TACTICAL_RULE_UNKNOWN};

const DIALOGUE_ID: &str = "dialogue.eng022";

fn command_executor() -> CommandExecutor {
    let mut registry = CommandRegistry::default();
    register_tactical_commands(&mut registry).unwrap();
    CommandExecutor::new(registry)
}

fn context() -> CommandContext {
    CommandContext {
        actor: "eng022-test".into(),
        permissions: PermissionSet::content_author(),
    }
}

fn create_request(dry_run: bool) -> CommandRequest {
    CommandRequest {
        command: ToolId::from_static("dialogue.create"),
        input: json!({
            "dialogue_id": DIALOGUE_ID,
            "name": "Gate Talk",
            "entry_node_id": "node.entry",
            "speaker": "Guard",
            "text": "Halt."
        }),
        dry_run,
    }
}

fn read_dialogue(state: &EngineState) -> DialogueDocument {
    DialogueDocument::from_envelope(
        state
            .documents
            .get(&DocumentId::from_static(DIALOGUE_ID))
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn dialogue_commands_preserve_dry_run_atomicity_and_stable_graph_identity() {
    let mut executor = command_executor();
    let mut state = EngineState::default();
    let before = state.clone();
    let preview = executor
        .execute(create_request(true), &context(), &mut state)
        .unwrap();
    assert_eq!(preview.status, TransactionStatus::Previewed);
    assert_eq!(preview.document_diffs.len(), 1);
    assert_eq!(state, before);

    executor
        .execute(create_request(false), &context(), &mut state)
        .unwrap();
    let add_node = executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("dialogue.add_node"),
                input: json!({
                    "dialogue_id": DIALOGUE_ID,
                    "node_id": "node.exit",
                    "speaker": "Guard",
                    "text": "Pass."
                }),
                dry_run: false,
            },
            &context(),
            &mut state,
        )
        .unwrap();
    assert_eq!(add_node.status, TransactionStatus::Applied);
    assert!(add_node
        .diagnostics
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "DIALOGUE_NODE_UNREACHABLE"));

    executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("dialogue.add_choice"),
                input: json!({
                    "dialogue_id": DIALOGUE_ID,
                    "from_node_id": "node.entry",
                    "choice_id": "choice.pass",
                    "text": "We mean no harm.",
                    "target_node_id": "node.exit",
                    "conditions": [{"type": "always"}],
                    "effects": [{"type": "emit_signal", "signal": "dialogue.pass"}]
                }),
                dry_run: false,
            },
            &context(),
            &mut state,
        )
        .unwrap();
    let dialogue = read_dialogue(&state);
    assert_eq!(dialogue.nodes[0].id.as_str(), "node.entry");
    assert_eq!(dialogue.nodes[0].choices[0].id.as_str(), "choice.pass");
    assert_eq!(dialogue.nodes[0].choices[0].target.as_ref().unwrap().as_str(), "node.exit");
    assert_eq!(executor.audit_log().len(), 4);
}

#[test]
fn invalid_dialogue_choice_rule_and_target_are_rejected_atomically() {
    let mut executor = command_executor();
    let mut state = EngineState::default();
    executor
        .execute(create_request(false), &context(), &mut state)
        .unwrap();
    let before = state.clone();

    let unknown = executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("dialogue.add_choice"),
                input: json!({
                    "dialogue_id": DIALOGUE_ID,
                    "from_node_id": "node.entry",
                    "choice_id": "choice.bad-rule",
                    "text": "Bad",
                    "target_node_id": null,
                    "conditions": [{"type": "missing"}],
                    "effects": []
                }),
                dry_run: false,
            },
            &context(),
            &mut state,
        )
        .unwrap_err();
    let CommandError::ValidationFailed(bag) = unknown else {
        panic!("expected validation failure");
    };
    assert!(bag
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == TACTICAL_RULE_UNKNOWN));
    assert_eq!(state, before);

    let missing = executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("dialogue.add_choice"),
                input: json!({
                    "dialogue_id": DIALOGUE_ID,
                    "from_node_id": "node.entry",
                    "choice_id": "choice.bad-target",
                    "text": "Missing",
                    "target_node_id": "node.missing",
                    "conditions": [],
                    "effects": []
                }),
                dry_run: false,
            },
            &context(),
            &mut state,
        )
        .unwrap_err();
    let CommandError::ValidationFailed(bag) = missing else {
        panic!("expected validation failure");
    };
    assert!(bag
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DIALOGUE_CHOICE_TARGET_NOT_FOUND));
    assert_eq!(state, before);
}

#[test]
fn trigger_rule_validation_is_atomic_on_the_existing_command_surface() {
    let mut executor = command_executor();
    let mut state = EngineState::default();
    for request in [
        CommandRequest {
            command: ToolId::from_static("stage.create"),
            input: json!({"id": "stage.rules", "name": "Rules", "width": 4, "height": 4}),
            dry_run: false,
        },
        CommandRequest {
            command: ToolId::from_static("stage.create_spawn"),
            input: json!({
                "stage_id": "stage.rules",
                "object_id": "spawn.player",
                "profile": "player",
                "q": 0,
                "r": 0
            }),
            dry_run: false,
        },
    ] {
        executor.execute(request, &context(), &mut state).unwrap();
    }
    let before = state.clone();
    let error = executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("stage.create_trigger"),
                input: json!({
                    "stage_id": "stage.rules",
                    "object_id": "trigger.bad",
                    "activation": "enter",
                    "q": 1,
                    "r": 0,
                    "conditions": [],
                    "effects": [{"type": "missing"}]
                }),
                dry_run: false,
            },
            &context(),
            &mut state,
        )
        .unwrap_err();
    let CommandError::ValidationFailed(bag) = error else {
        panic!("expected validation failure");
    };
    assert!(bag
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == TACTICAL_RULE_UNKNOWN));
    assert_eq!(state, before);
}

fn host() -> AgentHost {
    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).unwrap();
    register_tactical_queries(&mut queries).unwrap();
    let catalog = ToolCatalog::from_registries(&commands, &queries).unwrap();
    let mut state = EngineState::default();
    let mut executor = CommandExecutor::new(commands);
    executor
        .execute(create_request(false), &context(), &mut state)
        .unwrap();
    AgentHost::new(state, executor, QueryExecutor::new(queries), catalog)
}

#[test]
fn rule_catalog_is_read_only_deterministic_and_live_discoverable() {
    let mut queries = QueryRegistry::default();
    register_tactical_queries(&mut queries).unwrap();
    let mut executor = QueryExecutor::new(queries);
    let output = executor
        .execute(
            QueryRequest {
                query: ToolId::from_static("tactical.rules.catalog"),
                input: json!({}),
            },
            &guiyi_engine_query::QueryContext {
                actor: "eng022-test".into(),
                permissions: PermissionSet::read_only(),
            },
            &Default::default(),
        )
        .unwrap();
    assert_eq!(output["conditions"][0]["id"], "always");
    assert_eq!(output["effects"][0]["id"], "emit_signal");
    assert!(output["effects"][0]["schema"].is_object());

    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).unwrap();
    register_tactical_queries(&mut queries).unwrap();
    let catalog = ToolCatalog::from_registries(&commands, &queries).unwrap();
    for tool in [
        "dialogue.create",
        "dialogue.add_node",
        "dialogue.add_choice",
        "tactical.rules.catalog",
    ] {
        assert!(catalog.list().any(|descriptor| descriptor.id.as_str() == tool));
    }
}

#[test]
fn agent_host_enforces_dialogue_permission_and_working_set() {
    let mut host = host();
    let before = host.state.clone();
    let call = |id: &str| ToolCall {
        id: id.into(),
        tool: ToolId::from_static("dialogue.add_node"),
        input: json!({
            "dialogue_id": DIALOGUE_ID,
            "node_id": "node.denied",
            "speaker": "Guard",
            "text": "Denied"
        }),
        dry_run: false,
    };

    let mut read_only = AgentSession::new(
        AgentSessionId::from_static("session.eng022.read"),
        "ENG-022",
        PermissionSet::read_only(),
    );
    let denied = host.execute(&mut read_only, call("permission"));
    assert_eq!(denied.status, ToolResultStatus::Rejected);
    assert_eq!(denied.output["error"]["code"], AGENT_PERMISSION_DENIED);
    assert_eq!(host.state, before);

    let mut restricted = AgentSession::new(
        AgentSessionId::from_static("session.eng022.scope"),
        "ENG-022",
        PermissionSet::content_author(),
    );
    restricted.working_set = vec![DocumentId::from_static("some.other.document")];
    let scoped = host.execute(&mut restricted, call("scope"));
    assert_eq!(scoped.status, ToolResultStatus::Rejected);
    assert_eq!(scoped.output["error"]["code"], AGENT_WORKING_SET_DENIED);
    assert_eq!(host.state, before);

    let mut read_rules = AgentSession::new(
        AgentSessionId::from_static("session.eng022.rules"),
        "ENG-022",
        PermissionSet::read_only(),
    );
    read_rules.working_set = vec![DocumentId::from_static("some.other.document")];
    let rules = host.execute(
        &mut read_rules,
        ToolCall {
            id: "rules".into(),
            tool: ToolId::from_static("tactical.rules.catalog"),
            input: json!({}),
            dry_run: false,
        },
    );
    assert_eq!(rules.status, ToolResultStatus::Ok);
    assert_eq!(rules.output["conditions"][0]["id"], "always");
}

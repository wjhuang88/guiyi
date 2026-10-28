use guiyi_engine_agent_host::{
    AgentHost, AgentSession, AGENT_PERMISSION_DENIED, AGENT_WORKING_SET_DENIED,
};
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_command::{
    CommandContext, CommandError, CommandExecutor, CommandRegistry, CommandRequest, EngineState,
    TransactionStatus,
};
use guiyi_engine_content::{DocumentEnvelope, DocumentHeader};
use guiyi_engine_core::{
    AgentSessionId, DocumentId, EngineTypeId, ObjectId, PermissionSet, ToolId,
};
use guiyi_engine_protocol::{ToolCall, ToolResultStatus};
use guiyi_engine_query::{register_builtin_queries, QueryExecutor, QueryRegistry};
use serde_json::{json, Value};
use tactical_rpg_content::{HexCoord, StageDocument, StageObject, StageObjectKind};
use tactical_rpg_tools::register_tactical_commands;

const STAGE_ID: &str = "stage.eng013";
const SPAWN_ID: &str = "spawn.player";
const ACTOR_ID: &str = "actor.hero.instance";
const ACTOR_DEFINITION_ID: &str = "actor.hero.definition";

fn actor_definition() -> DocumentEnvelope {
    DocumentEnvelope {
        header: DocumentHeader {
            id: DocumentId::from_static(ACTOR_DEFINITION_ID),
            type_id: EngineTypeId::from_static("tactical.actor.definition"),
            schema_version: 1,
            display_name: "Hero".into(),
        },
        references: Vec::new(),
        payload: json!({}),
    }
}

fn stage_document() -> StageDocument {
    let mut stage = StageDocument::new_hex(DocumentId::from_static(STAGE_ID), "ENG-013", 4, 4);
    stage.objects.push(StageObject {
        id: ObjectId::from_static(SPAWN_ID),
        position: HexCoord::new(1, 1),
        object: StageObjectKind::SpawnPoint {
            profile: "player".into(),
        },
        properties: json!({"label": "entry"}),
    });
    stage.objects.push(StageObject {
        id: ObjectId::from_static(ACTOR_ID),
        position: HexCoord::new(2, 2),
        object: StageObjectKind::Actor {
            definition: DocumentId::from_static(ACTOR_DEFINITION_ID),
        },
        properties: json!({"facing": "west", "level": 3}),
    });
    stage
}

fn engine_state() -> EngineState {
    let mut state = EngineState::default();
    state.documents.insert(actor_definition()).unwrap();
    state
        .documents
        .insert(stage_document().to_envelope().unwrap())
        .unwrap();
    state
}

fn command_executor() -> CommandExecutor {
    let mut registry = CommandRegistry::default();
    register_tactical_commands(&mut registry).unwrap();
    CommandExecutor::new(registry)
}

fn author_context() -> CommandContext {
    CommandContext {
        actor: "eng013-test".into(),
        permissions: PermissionSet::content_author(),
    }
}

fn read_stage(state: &EngineState) -> StageDocument {
    StageDocument::from_envelope(
        state
            .documents
            .get(&DocumentId::from_static(STAGE_ID))
            .unwrap(),
    )
    .unwrap()
}

fn error_codes(error: CommandError) -> Vec<String> {
    match error {
        CommandError::ValidationFailed(bag) => bag
            .diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect(),
        other => panic!("expected validation failure, got {other:?}"),
    }
}

#[test]
fn move_dry_run_then_apply_preserves_identity_and_reports_diff_and_audit() {
    let mut executor = command_executor();
    let mut state = engine_state();
    let original = read_stage(&state)
        .objects
        .into_iter()
        .find(|object| object.id == ObjectId::from_static(ACTOR_ID))
        .unwrap();

    let request = || CommandRequest {
        command: ToolId::from_static("stage.move_object"),
        input: json!({
            "stage_id": STAGE_ID,
            "object_id": ACTOR_ID,
            "q": 3,
            "r": 1
        }),
        dry_run: true,
    };
    let preview = executor
        .execute(request(), &author_context(), &mut state)
        .unwrap();
    assert_eq!(preview.status, TransactionStatus::Previewed);
    assert_eq!(preview.document_diffs.len(), 1);
    assert_eq!(preview.document_diffs[0].document_id.as_str(), STAGE_ID);
    assert_eq!(preview.document_diffs[0].change, "modified");
    assert_eq!(preview.output["from"], json!({"q": 2, "r": 2}));
    assert_eq!(preview.output["to"], json!({"q": 3, "r": 1}));
    assert_eq!(read_stage(&state).objects[1].position, HexCoord::new(2, 2));
    assert_eq!(executor.audit_log().len(), 1);
    assert_eq!(
        executor.audit_log()[0].report.status,
        TransactionStatus::Previewed
    );

    let mut apply = request();
    apply.dry_run = false;
    let report = executor
        .execute(apply, &author_context(), &mut state)
        .unwrap();
    assert_eq!(report.status, TransactionStatus::Applied);
    assert_eq!(report.document_diffs.len(), 1);
    assert_eq!(executor.audit_log().len(), 2);

    let moved = read_stage(&state)
        .objects
        .into_iter()
        .find(|object| object.id == ObjectId::from_static(ACTOR_ID))
        .unwrap();
    assert_eq!(moved.id, original.id);
    assert_eq!(moved.object, original.object);
    assert_eq!(moved.properties, original.properties);
    assert_eq!(moved.position, HexCoord::new(3, 1));
}

#[test]
fn move_bounds_and_missing_object_fail_with_structured_diagnostics_atomically() {
    let mut executor = command_executor();
    let mut state = engine_state();
    let before = state.clone();

    let out_of_bounds = executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("stage.move_object"),
                input: json!({
                    "stage_id": STAGE_ID,
                    "object_id": ACTOR_ID,
                    "q": 9,
                    "r": 9
                }),
                dry_run: false,
            },
            &author_context(),
            &mut state,
        )
        .unwrap_err();
    match out_of_bounds {
        CommandError::ValidationFailed(bag) => {
            let diagnostic = bag
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == "STAGE_OBJECT_OUT_OF_BOUNDS")
                .unwrap();
            let location = diagnostic.location.as_ref().unwrap();
            assert_eq!(location.document.as_ref().unwrap().as_str(), STAGE_ID);
            assert_eq!(location.object.as_ref().unwrap().as_str(), ACTOR_ID);
            assert_eq!(location.field_path.as_deref(), Some("position"));
        }
        other => panic!("expected validation failure, got {other:?}"),
    }
    assert_eq!(state, before);
    assert!(executor.audit_log().is_empty());

    for tool in ["stage.move_object", "stage.remove_object"] {
        let input = if tool == "stage.move_object" {
            json!({
                "stage_id": STAGE_ID,
                "object_id": "object.missing",
                "q": 1,
                "r": 1
            })
        } else {
            json!({
                "stage_id": STAGE_ID,
                "object_id": "object.missing"
            })
        };
        let error = executor
            .execute(
                CommandRequest {
                    command: ToolId::new(tool).unwrap(),
                    input,
                    dry_run: false,
                },
                &author_context(),
                &mut state,
            )
            .unwrap_err();
        assert_eq!(error_codes(error), vec!["STAGE_OBJECT_NOT_FOUND"]);
        assert_eq!(state, before);
    }
}

#[test]
fn removing_last_spawn_returns_warning_and_applied_diff() {
    let mut executor = command_executor();
    let mut state = engine_state();
    let report = executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("stage.remove_object"),
                input: json!({"stage_id": STAGE_ID, "object_id": SPAWN_ID}),
                dry_run: false,
            },
            &author_context(),
            &mut state,
        )
        .unwrap();

    assert_eq!(report.status, TransactionStatus::Applied);
    assert_eq!(report.document_diffs.len(), 1);
    assert!(report
        .diagnostics
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "STAGE_NO_SPAWN_POINT"));
    assert!(read_stage(&state)
        .objects
        .iter()
        .all(|object| object.id != ObjectId::from_static(SPAWN_ID)));
    assert_eq!(executor.audit_log().len(), 1);
}

#[test]
fn permission_denial_leaves_stage_unchanged() {
    let mut executor = command_executor();
    let mut state = engine_state();
    let before = state.clone();
    let error = executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("stage.move_object"),
                input: json!({
                    "stage_id": STAGE_ID,
                    "object_id": ACTOR_ID,
                    "q": 1,
                    "r": 2
                }),
                dry_run: false,
            },
            &CommandContext {
                actor: "read-only".into(),
                permissions: PermissionSet::read_only(),
            },
            &mut state,
        )
        .unwrap_err();
    assert!(matches!(error, CommandError::PermissionDenied(_)));
    assert_eq!(state, before);
}

#[test]
fn live_command_descriptors_advertise_move_and_remove_schemas() {
    let mut registry = CommandRegistry::default();
    register_tactical_commands(&mut registry).unwrap();
    for (tool, required) in [
        (
            "stage.move_object",
            ["stage_id", "object_id", "q", "r"].as_slice(),
        ),
        ("stage.remove_object", ["stage_id", "object_id"].as_slice()),
    ] {
        let descriptor = registry
            .descriptors()
            .into_iter()
            .find(|descriptor| descriptor.id.as_str() == tool)
            .unwrap();
        assert!(descriptor.input_schema.is_object());
        let advertised = descriptor.input_schema["required"].as_array().unwrap();
        for field in required {
            assert!(advertised.contains(&Value::String((*field).into())));
        }
    }
}

fn host() -> AgentHost {
    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).unwrap();
    let catalog = ToolCatalog::from_registries(&commands, &queries).unwrap();
    AgentHost::new(
        engine_state(),
        CommandExecutor::new(commands),
        QueryExecutor::new(queries),
        catalog,
    )
}

fn session(id: &'static str, permissions: PermissionSet) -> AgentSession {
    AgentSession::new(AgentSessionId::from_static(id), "ENG-013", permissions)
}

#[test]
fn actor_removal_refreshes_project_graph_through_agent_host() {
    let mut host = host();
    let mut session = session("session.eng013.graph", PermissionSet::content_author());
    let references = |id: &str| ToolCall {
        id: id.into(),
        tool: ToolId::from_static("project.references.find"),
        input: json!({"target_document": ACTOR_DEFINITION_ID}),
        dry_run: false,
    };

    let before = host.execute(&mut session, references("refs-before"));
    assert_eq!(before.status, ToolResultStatus::Ok);
    assert_eq!(before.output.as_array().unwrap().len(), 1);
    assert_eq!(before.output[0]["source_document"], STAGE_ID);
    assert_eq!(before.output[0]["source_object"], ACTOR_ID);

    let removed = host.execute(
        &mut session,
        ToolCall {
            id: "remove-actor".into(),
            tool: ToolId::from_static("stage.remove_object"),
            input: json!({"stage_id": STAGE_ID, "object_id": ACTOR_ID}),
            dry_run: false,
        },
    );
    assert_eq!(removed.status, ToolResultStatus::Ok);
    assert_eq!(removed.transaction.as_ref().unwrap()["status"], "applied");

    let after = host.execute(&mut session, references("refs-after"));
    assert_eq!(after.status, ToolResultStatus::Ok);
    assert_eq!(after.output, json!([]));
}

#[test]
fn agent_host_enforces_permission_and_working_set_for_new_commands() {
    let mut host = host();
    let before = host.state.clone();

    let mut read_only = session("session.eng013.read", PermissionSet::read_only());
    let denied = host.execute(
        &mut read_only,
        ToolCall {
            id: "move-denied".into(),
            tool: ToolId::from_static("stage.move_object"),
            input: json!({
                "stage_id": STAGE_ID,
                "object_id": ACTOR_ID,
                "q": 1,
                "r": 2
            }),
            dry_run: false,
        },
    );
    assert_eq!(denied.status, ToolResultStatus::Rejected);
    assert_eq!(denied.output["error"]["code"], AGENT_PERMISSION_DENIED);
    assert_eq!(host.state, before);

    let mut restricted = session("session.eng013.scope", PermissionSet::content_author());
    restricted.working_set = vec![DocumentId::from_static(ACTOR_DEFINITION_ID)];
    let scoped = host.execute(
        &mut restricted,
        ToolCall {
            id: "remove-outside-scope".into(),
            tool: ToolId::from_static("stage.remove_object"),
            input: json!({"stage_id": STAGE_ID, "object_id": ACTOR_ID}),
            dry_run: false,
        },
    );
    assert_eq!(scoped.status, ToolResultStatus::Rejected);
    assert_eq!(scoped.output["error"]["code"], AGENT_WORKING_SET_DENIED);
    assert_eq!(host.state, before);
}

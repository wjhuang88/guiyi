use guiyi_engine_agent_host::{
    AgentHost, AgentSession, AGENT_PERMISSION_DENIED, AGENT_WORKING_SET_DENIED,
};
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_command::{
    CommandContext, CommandError, CommandExecutor, CommandRegistry, CommandRequest, EngineState,
    TransactionStatus,
};
use guiyi_engine_core::{AgentSessionId, DocumentId, ObjectId, PermissionSet, ToolId};
use guiyi_engine_protocol::{ToolCall, ToolResultStatus};
use guiyi_engine_query::{QueryContext, QueryExecutor, QueryRegistry, QueryRequest};
use serde_json::{json, Value};
use tactical_rpg_content::{
    CoordinateSpace, HexCoord, StageDocument, StageNavigation, StageObject, StageObjectKind,
};
use tactical_rpg_tools::{register_tactical_commands, register_tactical_queries};

const STAGE_ID: &str = "stage.eng020";
const SPAWN_ID: &str = "spawn.player";
const GOAL_ID: &str = "marker.goal";

fn stage(width: u32, height: u32) -> StageDocument {
    let mut stage = StageDocument::new_hex(
        DocumentId::from_static(STAGE_ID),
        "ENG-020 navigation",
        width,
        height,
    );
    stage.objects.push(StageObject {
        id: ObjectId::from_static(SPAWN_ID),
        position: HexCoord::new(0, 0),
        object: StageObjectKind::SpawnPoint {
            profile: "player".into(),
        },
        properties: json!({}),
    });
    stage.objects.push(StageObject {
        id: ObjectId::from_static(GOAL_ID),
        position: HexCoord::new(width as i32 - 1, 0),
        object: StageObjectKind::Marker {
            marker_type: "goal".into(),
        },
        properties: json!({}),
    });
    stage
}

fn state_with(stage: StageDocument) -> EngineState {
    let mut state = EngineState::default();
    state
        .documents
        .insert(stage.to_envelope().unwrap())
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
        actor: "eng020-test".into(),
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

fn set_blocked_request(q: i32, r: i32, blocked: bool, dry_run: bool) -> CommandRequest {
    CommandRequest {
        command: ToolId::from_static("stage.set_navigation_blocked"),
        input: json!({
            "stage_id": STAGE_ID,
            "q": q,
            "r": r,
            "blocked": blocked
        }),
        dry_run,
    }
}

#[test]
fn navigation_command_dry_run_and_apply_are_reviewable_and_idempotent() {
    let mut executor = command_executor();
    let mut state = state_with(stage(4, 4));
    let before = state.clone();

    let preview = executor
        .execute(
            set_blocked_request(3, 3, true, true),
            &author_context(),
            &mut state,
        )
        .unwrap();
    assert_eq!(preview.status, TransactionStatus::Previewed);
    assert_eq!(preview.document_diffs.len(), 1);
    assert_eq!(preview.document_diffs[0].document_id.as_str(), STAGE_ID);
    assert_eq!(preview.output["changed"], true);
    assert_eq!(state, before);
    assert_eq!(executor.audit_log().len(), 1);

    let applied = executor
        .execute(
            set_blocked_request(3, 3, true, false),
            &author_context(),
            &mut state,
        )
        .unwrap();
    assert_eq!(applied.status, TransactionStatus::Applied);
    assert_eq!(applied.document_diffs.len(), 1);
    assert_eq!(applied.output["changed"], true);
    assert_eq!(
        read_stage(&state).navigation.blocked,
        vec![HexCoord::new(3, 3)]
    );

    let idempotent = executor
        .execute(
            set_blocked_request(3, 3, true, false),
            &author_context(),
            &mut state,
        )
        .unwrap();
    assert_eq!(idempotent.status, TransactionStatus::Applied);
    assert_eq!(idempotent.output["changed"], false);
    assert!(idempotent.document_diffs.is_empty());

    let unblocked = executor
        .execute(
            set_blocked_request(3, 3, false, false),
            &author_context(),
            &mut state,
        )
        .unwrap();
    assert_eq!(unblocked.output["changed"], true);
    assert!(read_stage(&state).navigation.blocked.is_empty());
    assert_eq!(executor.audit_log().len(), 4);
}

#[test]
fn navigation_command_rejects_bounds_and_disconnection_atomically() {
    let mut executor = command_executor();
    let mut state = state_with(stage(3, 1));
    let before = state.clone();

    let bounds = executor
        .execute(
            set_blocked_request(9, 0, true, false),
            &author_context(),
            &mut state,
        )
        .unwrap_err();
    let CommandError::ValidationFailed(bag) = bounds else {
        panic!("expected validation failure");
    };
    assert!(bag
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "STAGE_NAV_BLOCKED_CELL_OUT_OF_BOUNDS"));
    assert_eq!(state, before);
    assert!(executor.audit_log().is_empty());

    let disconnect = executor
        .execute(
            set_blocked_request(1, 0, true, false),
            &author_context(),
            &mut state,
        )
        .unwrap_err();
    let CommandError::ValidationFailed(bag) = disconnect else {
        panic!("expected validation failure");
    };
    let diagnostic = bag
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "STAGE_OBJECT_UNREACHABLE")
        .unwrap();
    assert_eq!(
        diagnostic
            .location
            .as_ref()
            .and_then(|location| location.object.as_ref())
            .unwrap()
            .as_str(),
        GOAL_ID
    );
    assert_eq!(state, before);
    assert!(executor.audit_log().is_empty());
}

fn query_executor() -> QueryExecutor {
    let mut registry = QueryRegistry::default();
    register_tactical_queries(&mut registry).unwrap();
    QueryExecutor::new(registry)
}

fn reachability_input(from: &str, to: &str) -> Value {
    json!({
        "stage_id": STAGE_ID,
        "from_object_id": from,
        "to_object_id": to
    })
}

#[test]
fn reachability_query_returns_deterministic_path_and_structured_failures() {
    let context = QueryContext {
        actor: "eng020-query".into(),
        permissions: PermissionSet::content_author(),
    };

    let reachable_state = state_with(stage(3, 1));
    let mut executor = query_executor();
    let reachable = executor
        .execute(
            QueryRequest {
                query: ToolId::from_static("stage.reachability"),
                input: reachability_input(SPAWN_ID, GOAL_ID),
            },
            &context,
            &reachable_state.documents,
        )
        .unwrap();
    assert_eq!(reachable["status"], "ok");
    assert_eq!(reachable["reachable"], true);
    assert_eq!(reachable["distance"], 2);
    assert_eq!(
        reachable["path"],
        json!([
            {"q": 0, "r": 0},
            {"q": 1, "r": 0},
            {"q": 2, "r": 0}
        ])
    );

    let mut disconnected_stage = stage(3, 1);
    disconnected_stage.navigation = StageNavigation {
        blocked: vec![HexCoord::new(1, 0)],
    };
    let disconnected_state = state_with(disconnected_stage);
    let mut executor = query_executor();
    let disconnected = executor
        .execute(
            QueryRequest {
                query: ToolId::from_static("stage.reachability"),
                input: reachability_input(SPAWN_ID, GOAL_ID),
            },
            &context,
            &disconnected_state.documents,
        )
        .unwrap();
    assert_eq!(disconnected["status"], "ok");
    assert_eq!(disconnected["reachable"], false);
    assert!(disconnected["distance"].is_null());
    assert_eq!(disconnected["path"], json!([]));

    let mut executor = query_executor();
    let missing = executor
        .execute(
            QueryRequest {
                query: ToolId::from_static("stage.reachability"),
                input: reachability_input(SPAWN_ID, "object.missing"),
            },
            &context,
            &reachable_state.documents,
        )
        .unwrap();
    assert_eq!(missing["status"], "invalid");
    assert!(missing["reachable"].is_null());
    assert_eq!(missing["diagnostics"][0]["code"], "STAGE_OBJECT_NOT_FOUND");

    let mut free_stage = stage(3, 1);
    free_stage.coordinate_space = CoordinateSpace::Free2d;
    let free_state = state_with(free_stage);
    let mut executor = query_executor();
    let unsupported = executor
        .execute(
            QueryRequest {
                query: ToolId::from_static("stage.reachability"),
                input: reachability_input(SPAWN_ID, GOAL_ID),
            },
            &context,
            &free_state.documents,
        )
        .unwrap();
    assert_eq!(unsupported["status"], "unsupported");
    assert_eq!(
        unsupported["diagnostics"][0]["code"],
        "STAGE_NAV_UNSUPPORTED_COORDINATE_SPACE"
    );
}

fn host() -> AgentHost {
    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    let mut queries = QueryRegistry::default();
    register_tactical_queries(&mut queries).unwrap();
    let catalog = ToolCatalog::from_registries(&commands, &queries).unwrap();
    AgentHost::new(
        state_with(stage(4, 4)),
        CommandExecutor::new(commands),
        QueryExecutor::new(queries),
        catalog,
    )
}

fn session(id: &'static str, permissions: PermissionSet) -> AgentSession {
    AgentSession::new(AgentSessionId::from_static(id), "ENG-020", permissions)
}

#[test]
fn agent_host_enforces_navigation_permissions_and_working_set() {
    let mut host = host();
    let before = host.state.clone();

    let mut read_only = session("session.eng020.read", PermissionSet::read_only());
    let command_denied = host.execute(
        &mut read_only,
        ToolCall {
            id: "block-denied".into(),
            tool: ToolId::from_static("stage.set_navigation_blocked"),
            input: json!({"stage_id": STAGE_ID, "q": 3, "r": 3, "blocked": true}),
            dry_run: false,
        },
    );
    assert_eq!(command_denied.status, ToolResultStatus::Rejected);
    assert_eq!(
        command_denied.output["error"]["code"],
        AGENT_PERMISSION_DENIED
    );
    assert_eq!(host.state, before);

    let query_denied = host.execute(
        &mut read_only,
        ToolCall {
            id: "query-denied".into(),
            tool: ToolId::from_static("stage.reachability"),
            input: reachability_input(SPAWN_ID, GOAL_ID),
            dry_run: false,
        },
    );
    assert_eq!(query_denied.status, ToolResultStatus::Rejected);
    assert_eq!(
        query_denied.output["error"]["code"],
        AGENT_PERMISSION_DENIED
    );

    let mut restricted = session("session.eng020.scope", PermissionSet::content_author());
    restricted.working_set = vec![DocumentId::from_static("stage.other")];
    let scoped = host.execute(
        &mut restricted,
        ToolCall {
            id: "block-scoped".into(),
            tool: ToolId::from_static("stage.set_navigation_blocked"),
            input: json!({"stage_id": STAGE_ID, "q": 3, "r": 3, "blocked": true}),
            dry_run: false,
        },
    );
    assert_eq!(scoped.status, ToolResultStatus::Rejected);
    assert_eq!(scoped.output["error"]["code"], AGENT_WORKING_SET_DENIED);
    assert_eq!(host.state, before);
}

#[test]
fn live_catalog_exposes_navigation_command_and_query_schemas() {
    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    let mut queries = QueryRegistry::default();
    register_tactical_queries(&mut queries).unwrap();
    let catalog = ToolCatalog::from_registries(&commands, &queries).unwrap();

    for tool in ["stage.set_navigation_blocked", "stage.reachability"] {
        let descriptor = catalog
            .list()
            .find(|descriptor| descriptor.id.as_str() == tool)
            .unwrap();
        assert!(descriptor.input_schema.is_object());
        assert!(descriptor.input_schema["required"].is_array());
    }

    let mut host = host();
    let mut author = session("session.eng020.author", PermissionSet::content_author());
    let result = host.execute(
        &mut author,
        ToolCall {
            id: "reachability".into(),
            tool: ToolId::from_static("stage.reachability"),
            input: reachability_input(SPAWN_ID, GOAL_ID),
            dry_run: false,
        },
    );
    assert_eq!(result.status, ToolResultStatus::Ok);
    assert_eq!(result.output["reachable"], true);
    assert_eq!(result.output["distance"], 3);
}

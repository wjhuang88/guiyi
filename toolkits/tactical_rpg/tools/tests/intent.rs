use guiyi_engine_agent_host::{
    AgentHost, AgentSession, AGENT_PERMISSION_DENIED, AGENT_WORKING_SET_DENIED,
};
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_command::{
    CommandContext, CommandError, CommandExecutor, CommandRegistry, CommandRequest, EngineState,
    TransactionReport, TransactionStatus,
};
use guiyi_engine_core::{AgentSessionId, DocumentId, ObjectId, PermissionSet, ToolId};
use guiyi_engine_protocol::{ToolCall, ToolResultStatus};
use guiyi_engine_query::{QueryExecutor, QueryRegistry};
use serde_json::{json, Value};
use tactical_rpg_content::{
    HexCoord, StageDocument, StageNavigation, StageObject, StageObjectKind,
};
use tactical_rpg_tools::{register_tactical_commands, register_tactical_queries};
use tactical_rpg_validation::validate_stage;

const STAGE_ID: &str = "stage.eng023";
const TOOL_ID: &str = "tactical.make_stage_playable";

fn stage() -> StageDocument {
    StageDocument::new_hex(DocumentId::from_static(STAGE_ID), "ENG-023 intent", 4, 4)
}

fn spawn(id: &'static str, q: i32, r: i32) -> StageObject {
    StageObject {
        id: ObjectId::from_static(id),
        position: HexCoord::new(q, r),
        object: StageObjectKind::SpawnPoint {
            profile: "player".into(),
        },
        properties: json!({}),
    }
}

fn marker(id: &'static str, q: i32, r: i32) -> StageObject {
    StageObject {
        id: ObjectId::from_static(id),
        position: HexCoord::new(q, r),
        object: StageObjectKind::Marker {
            marker_type: "goal".into(),
        },
        properties: json!({}),
    }
}

fn playable_stage() -> StageDocument {
    let mut value = stage();
    value.objects.push(spawn("spawn.player", 0, 0));
    value
}

fn blocked_spawn_stage() -> StageDocument {
    let mut value = playable_stage();
    value.navigation = StageNavigation {
        blocked: vec![HexCoord::new(0, 0)],
    };
    value
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
        actor: "eng023-test".into(),
        permissions: PermissionSet::content_author(),
    }
}

fn request(input: Value, dry_run: bool) -> CommandRequest {
    CommandRequest {
        command: ToolId::from_static(TOOL_ID),
        input,
        dry_run,
    }
}

fn default_input() -> Value {
    json!({"stage_id": STAGE_ID})
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

fn assert_locally_playable(stage: &StageDocument) {
    let diagnostics = validate_stage(stage);
    assert!(!diagnostics.has_errors(), "diagnostics: {diagnostics:?}");
    assert!(!diagnostics
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "STAGE_NO_SPAWN_POINT"));
}

fn transaction(value: &Value) -> TransactionReport {
    serde_json::from_value(value.clone()).unwrap()
}

#[test]
fn already_playable_stage_is_an_empty_no_op_transaction() {
    let mut executor = command_executor();
    let mut state = state_with(playable_stage());
    let before = state.clone();

    let report = executor
        .execute(
            request(default_input(), false),
            &author_context(),
            &mut state,
        )
        .unwrap();

    assert_eq!(report.status, TransactionStatus::Applied);
    assert!(report.document_diffs.is_empty());
    assert_eq!(report.output["playable_before"], true);
    assert_eq!(report.output["playable_after"], true);
    assert_eq!(report.output["plan"], json!([]));
    assert_eq!(state, before);
    assert_eq!(executor.audit_log().len(), 1);
}

#[test]
fn blocked_spawn_dry_run_is_reviewable_and_apply_uses_one_outer_transaction() {
    let mut executor = command_executor();
    let mut state = state_with(blocked_spawn_stage());
    let before = state.clone();

    let preview = executor
        .execute(
            request(default_input(), true),
            &author_context(),
            &mut state,
        )
        .unwrap();
    assert_eq!(preview.status, TransactionStatus::Previewed);
    assert_eq!(preview.document_diffs.len(), 1);
    assert_eq!(preview.document_diffs[0].document_id.as_str(), STAGE_ID);
    assert_eq!(preview.output["playable_before"], false);
    assert_eq!(preview.output["playable_after"], true);
    assert_eq!(preview.output["plan"].as_array().unwrap().len(), 1);
    assert_eq!(
        preview.output["plan"][0]["command"],
        "stage.set_navigation_blocked"
    );
    assert_eq!(preview.output["plan"][0]["input"]["blocked"], false);
    assert_eq!(
        preview.output["plan"][0]["reason_codes"],
        json!(["STAGE_NAV_SPAWN_BLOCKED"])
    );
    assert_eq!(state, before);

    let applied = executor
        .execute(
            request(default_input(), false),
            &author_context(),
            &mut state,
        )
        .unwrap();
    assert_eq!(applied.status, TransactionStatus::Applied);
    assert_eq!(applied.document_diffs.len(), 1);
    assert!(read_stage(&state).navigation.blocked.is_empty());
    assert_locally_playable(&read_stage(&state));
    assert_eq!(executor.audit_log().len(), 2);
}

#[test]
fn missing_spawn_uses_deterministic_fallback_and_honors_explicit_identity() {
    let mut value = stage();
    value.navigation = StageNavigation {
        blocked: vec![HexCoord::new(0, 0)],
    };
    let mut executor = command_executor();
    let mut state = state_with(value);
    let before = state.clone();
    let input = json!({
        "stage_id": STAGE_ID,
        "spawn_object_id": "spawn.entry",
        "spawn_profile": "hero"
    });

    let preview = executor
        .execute(request(input.clone(), true), &author_context(), &mut state)
        .unwrap();
    assert_eq!(preview.status, TransactionStatus::Previewed);
    assert_eq!(preview.output["plan"].as_array().unwrap().len(), 1);
    assert_eq!(preview.output["plan"][0]["command"], "stage.create_spawn");
    assert_eq!(
        preview.output["plan"][0]["input"]["object_id"],
        "spawn.entry"
    );
    assert_eq!(preview.output["plan"][0]["input"]["profile"], "hero");
    assert_eq!(preview.output["plan"][0]["input"]["q"], 0);
    assert_eq!(preview.output["plan"][0]["input"]["r"], 1);
    assert_eq!(
        preview.output["plan"][0]["reason_codes"],
        json!(["STAGE_NO_SPAWN_POINT"])
    );
    assert_eq!(state, before);

    let applied = executor
        .execute(request(input, false), &author_context(), &mut state)
        .unwrap();
    assert_eq!(applied.status, TransactionStatus::Applied);
    assert_eq!(applied.document_diffs.len(), 1);
    let repaired = read_stage(&state);
    let created = repaired
        .objects
        .iter()
        .find(|object| object.id.as_str() == "spawn.entry")
        .unwrap();
    assert_eq!(created.position, HexCoord::new(0, 1));
    assert!(matches!(
        &created.object,
        StageObjectKind::SpawnPoint { profile } if profile == "hero"
    ));
    assert_locally_playable(&repaired);
}

#[test]
fn multiple_blocked_spawns_are_planned_deterministically_and_commit_atomically() {
    let mut value = stage();
    value.objects.push(spawn("spawn.late", 3, 1));
    value.objects.push(spawn("spawn.early", 0, 2));
    value.navigation = StageNavigation {
        blocked: vec![HexCoord::new(3, 1), HexCoord::new(0, 2)],
    };
    let mut executor = command_executor();
    let mut state = state_with(value);

    let report = executor
        .execute(
            request(default_input(), false),
            &author_context(),
            &mut state,
        )
        .unwrap();

    assert_eq!(report.status, TransactionStatus::Applied);
    assert_eq!(report.document_diffs.len(), 1);
    assert_eq!(report.output["plan"].as_array().unwrap().len(), 2);
    assert_eq!(report.output["plan"][0]["input"]["q"], 0);
    assert_eq!(report.output["plan"][0]["input"]["r"], 2);
    assert_eq!(report.output["plan"][1]["input"]["q"], 3);
    assert_eq!(report.output["plan"][1]["input"]["r"], 1);
    assert!(read_stage(&state).navigation.blocked.is_empty());
    assert_locally_playable(&read_stage(&state));
    assert_eq!(executor.audit_log().len(), 1);
}

#[test]
fn unsupported_blocker_preserves_authoritative_diagnostic_and_never_partially_mutates() {
    let mut value = stage();
    value.objects.push(marker("marker.invalid", 9, 9));
    let mut executor = command_executor();
    let mut state = state_with(value);
    let before = state.clone();

    let error = executor
        .execute(
            request(default_input(), false),
            &author_context(),
            &mut state,
        )
        .unwrap_err();
    let CommandError::ValidationFailed(bag) = error else {
        panic!("expected validation failure");
    };
    assert!(bag
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "STAGE_OBJECT_OUT_OF_BOUNDS"));
    assert_eq!(state, before);
    assert!(executor.audit_log().is_empty());
}

#[test]
fn intent_result_matches_the_authoritative_primitive_blocked_spawn_mutation() {
    let original = blocked_spawn_stage();
    let mut intent_executor = command_executor();
    let mut intent_state = state_with(original.clone());
    let intent_report = intent_executor
        .execute(
            request(default_input(), false),
            &author_context(),
            &mut intent_state,
        )
        .unwrap();
    assert_eq!(
        intent_report.output["plan"][0]["command"],
        "stage.set_navigation_blocked"
    );

    let mut primitive_executor = command_executor();
    let mut primitive_state = state_with(original);
    primitive_executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("stage.set_navigation_blocked"),
                input: intent_report.output["plan"][0]["input"].clone(),
                dry_run: false,
            },
            &author_context(),
            &mut primitive_state,
        )
        .unwrap();

    assert_eq!(read_stage(&intent_state), read_stage(&primitive_state));
}

fn host_with(stage: StageDocument) -> AgentHost {
    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    let mut queries = QueryRegistry::default();
    register_tactical_queries(&mut queries).unwrap();
    let catalog = ToolCatalog::from_registries(&commands, &queries).unwrap();
    AgentHost::new(
        state_with(stage),
        CommandExecutor::new(commands),
        QueryExecutor::new(queries),
        catalog,
    )
}

fn session(id: &'static str, permissions: PermissionSet) -> AgentSession {
    AgentSession::new(AgentSessionId::from_static(id), "ENG-023", permissions)
}

fn call(id: &'static str, dry_run: bool) -> ToolCall {
    ToolCall {
        id: id.into(),
        tool: ToolId::from_static(TOOL_ID),
        input: default_input(),
        dry_run,
    }
}

#[test]
fn agent_host_enforces_permission_and_working_set_before_intent_mutation() {
    let mut host = host_with(blocked_spawn_stage());
    let before = host.state.clone();

    let mut read_only = session("session.eng023.read", PermissionSet::read_only());
    let denied = host.execute(&mut read_only, call("permission-denied", false));
    assert_eq!(denied.status, ToolResultStatus::Rejected);
    assert_eq!(denied.output["error"]["code"], AGENT_PERMISSION_DENIED);
    assert!(denied.transaction.is_none());
    assert_eq!(host.state, before);

    let mut restricted = session("session.eng023.scope", PermissionSet::content_author());
    restricted.working_set = vec![DocumentId::from_static("stage.other")];
    let scoped = host.execute(&mut restricted, call("working-set-denied", false));
    assert_eq!(scoped.status, ToolResultStatus::Rejected);
    assert_eq!(scoped.output["error"]["code"], AGENT_WORKING_SET_DENIED);
    assert!(scoped.transaction.is_none());
    assert_eq!(host.state, before);
}

#[test]
fn live_catalog_exposes_machine_visible_defaults_and_dry_run_graph_discipline() {
    let mut host = host_with(stage());
    let descriptor = host
        .catalog()
        .list()
        .find(|descriptor| descriptor.id.as_str() == TOOL_ID)
        .unwrap();
    assert_eq!(descriptor.input_schema["required"], json!(["stage_id"]));
    assert_eq!(
        descriptor.input_schema["properties"]["spawn_object_id"]["default"],
        "spawn.player"
    );
    assert_eq!(
        descriptor.input_schema["properties"]["spawn_profile"]["default"],
        "player"
    );

    let before_state = host.state.clone();
    let before_graph = serde_json::to_value(host.project_graph_snapshot()).unwrap();
    let mut author = session("session.eng023.author", PermissionSet::content_author());
    let preview = host.execute(&mut author, call("preview", true));
    assert_eq!(preview.status, ToolResultStatus::Ok);
    let preview_transaction = transaction(preview.transaction.as_ref().unwrap());
    assert_eq!(preview_transaction.status, TransactionStatus::Previewed);
    assert_eq!(host.state, before_state);
    assert_eq!(
        serde_json::to_value(host.project_graph_snapshot()).unwrap(),
        before_graph
    );

    let applied = host.execute(&mut author, call("apply", false));
    assert_eq!(applied.status, ToolResultStatus::Ok);
    let applied_transaction = transaction(applied.transaction.as_ref().unwrap());
    assert_eq!(applied_transaction.status, TransactionStatus::Applied);
    assert_eq!(applied_transaction.command.as_str(), TOOL_ID);
    assert_eq!(applied_transaction.document_diffs.len(), 1);
    assert_locally_playable(&read_stage(&host.state));
    assert_eq!(
        serde_json::to_value(host.project_graph_snapshot()).unwrap(),
        before_graph,
        "spawn repair changes no semantic references; derived graph truth remains stable"
    );
}

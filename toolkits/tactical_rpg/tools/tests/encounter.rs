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
use tactical_rpg_content::{
    EncounterDocument, EncounterParticipant, HexCoord, StageDocument, StageObject, StageObjectKind,
    ACTOR_DEFINITION_TYPE,
};
use tactical_rpg_tools::{register_tactical_commands, register_tactical_queries};
use tactical_rpg_validation::{
    ENCOUNTER_DUPLICATE_PARTICIPANT_ID, ENCOUNTER_NO_PARTICIPANTS, ENCOUNTER_SPAWN_NOT_FOUND,
};

const ENCOUNTER_ID: &str = "encounter.eng021";
const STAGE_ID: &str = "stage.eng021";
const ACTOR_ID: &str = "actor.guard";
const SPAWN_ID: &str = "spawn.enemy";
const PARTICIPANT_ID: &str = "participant.guard";

fn stage() -> StageDocument {
    let mut stage = StageDocument::new_hex(DocumentId::from_static(STAGE_ID), "ENG-021", 4, 4);
    stage.objects.push(StageObject {
        id: ObjectId::from_static(SPAWN_ID),
        position: HexCoord::new(1, 1),
        object: StageObjectKind::SpawnPoint {
            profile: "enemy".into(),
        },
        properties: json!({}),
    });
    stage
}

fn actor_definition() -> DocumentEnvelope {
    DocumentEnvelope {
        header: DocumentHeader {
            id: DocumentId::from_static(ACTOR_ID),
            type_id: EngineTypeId::from_static(ACTOR_DEFINITION_TYPE),
            schema_version: 1,
            display_name: "Guard".into(),
        },
        references: Vec::new(),
        payload: json!({"name": "Guard"}),
    }
}

fn encounter(with_participant: bool) -> EncounterDocument {
    let mut encounter = EncounterDocument::new(
        DocumentId::from_static(ENCOUNTER_ID),
        "ENG-021 Encounter",
        DocumentId::from_static(STAGE_ID),
    );
    if with_participant {
        encounter.participants.push(EncounterParticipant {
            id: ObjectId::from_static(PARTICIPANT_ID),
            actor_definition: DocumentId::from_static(ACTOR_ID),
            spawn_point: ObjectId::from_static(SPAWN_ID),
            side: "enemy".into(),
        });
    }
    encounter
}

fn state(with_encounter: bool, with_participant: bool) -> EngineState {
    let mut state = EngineState::default();
    state
        .documents
        .insert(stage().to_envelope().unwrap())
        .unwrap();
    state.documents.insert(actor_definition()).unwrap();
    if with_encounter {
        state
            .documents
            .insert(encounter(with_participant).to_envelope().unwrap())
            .unwrap();
    }
    state
}

fn executor() -> CommandExecutor {
    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    CommandExecutor::new(commands)
}

fn context() -> CommandContext {
    CommandContext {
        actor: "eng021-test".into(),
        permissions: PermissionSet::content_author(),
    }
}

fn add_request(spawn: &str, stage_id: &str, dry_run: bool) -> CommandRequest {
    CommandRequest {
        command: ToolId::from_static("encounter.add_participant"),
        input: json!({
            "encounter_id": ENCOUNTER_ID,
            "stage_id": stage_id,
            "participant_id": PARTICIPANT_ID,
            "actor_definition": ACTOR_ID,
            "spawn_point": spawn,
            "side": "enemy"
        }),
        dry_run,
    }
}

fn read_encounter(state: &EngineState) -> EncounterDocument {
    EncounterDocument::from_envelope(
        state
            .documents
            .get(&DocumentId::from_static(ENCOUNTER_ID))
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn create_encounter_dry_run_and_apply_preserve_warning_and_reviewability() {
    let mut executor = executor();
    let mut state = state(false, false);
    let before = state.clone();
    let request = |dry_run| CommandRequest {
        command: ToolId::from_static("encounter.create"),
        input: json!({
            "encounter_id": ENCOUNTER_ID,
            "name": "ENG-021 Encounter",
            "stage_id": STAGE_ID
        }),
        dry_run,
    };

    let preview = executor
        .execute(request(true), &context(), &mut state)
        .unwrap();
    assert_eq!(preview.status, TransactionStatus::Previewed);
    assert_eq!(preview.document_diffs.len(), 1);
    assert_eq!(preview.document_diffs[0].document_id.as_str(), ENCOUNTER_ID);
    assert!(preview
        .diagnostics
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_NO_PARTICIPANTS));
    assert_eq!(state, before);

    let applied = executor
        .execute(request(false), &context(), &mut state)
        .unwrap();
    assert_eq!(applied.status, TransactionStatus::Applied);
    assert_eq!(read_encounter(&state).participants.len(), 0);
    assert!(applied
        .diagnostics
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_NO_PARTICIPANTS));
    assert_eq!(executor.audit_log().len(), 2);
}

#[test]
fn add_participant_updates_document_references_and_rejects_invalid_candidates_atomically() {
    let mut executor = executor();
    let mut state = state(true, false);
    let applied = executor
        .execute(
            add_request(SPAWN_ID, STAGE_ID, false),
            &context(),
            &mut state,
        )
        .unwrap();
    assert_eq!(applied.status, TransactionStatus::Applied);
    assert_eq!(applied.document_diffs.len(), 1);
    let encounter = read_encounter(&state);
    assert_eq!(encounter.participants.len(), 1);
    let envelope = state
        .documents
        .get(&DocumentId::from_static(ENCOUNTER_ID))
        .unwrap();
    assert_eq!(envelope.references.len(), 3);

    let mut duplicate_executor = executor();
    let before = state.clone();
    let duplicate = duplicate_executor
        .execute(
            add_request(SPAWN_ID, STAGE_ID, false),
            &context(),
            &mut state,
        )
        .unwrap_err();
    let CommandError::ValidationFailed(bag) = duplicate else {
        panic!("expected duplicate participant validation failure");
    };
    assert!(bag
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_DUPLICATE_PARTICIPANT_ID));
    assert_eq!(state, before);

    let mut missing_executor = executor();
    let mut missing_state = state(true, false);
    let missing_before = missing_state.clone();
    let missing = missing_executor
        .execute(
            add_request("spawn.missing", STAGE_ID, false),
            &context(),
            &mut missing_state,
        )
        .unwrap_err();
    let CommandError::ValidationFailed(bag) = missing else {
        panic!("expected missing spawn validation failure");
    };
    assert!(bag
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_SPAWN_NOT_FOUND));
    assert_eq!(missing_state, missing_before);

    let mut mismatch_executor = executor();
    let mismatch = mismatch_executor
        .execute(
            add_request(SPAWN_ID, "stage.other", false),
            &context(),
            &mut missing_state,
        )
        .unwrap_err();
    let CommandError::ValidationFailed(bag) = mismatch else {
        panic!("expected Stage mismatch validation failure");
    };
    assert!(bag
        .diagnostics
        .iter()
        .any(|item| item.code == "ENCOUNTER_STAGE_MISMATCH"));
    assert_eq!(missing_state, missing_before);
}

#[test]
fn remove_final_participant_applies_with_warning_and_missing_id_is_atomic() {
    let mut executor = executor();
    let mut state = state(true, true);
    let removed = executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("encounter.remove_participant"),
                input: json!({
                    "encounter_id": ENCOUNTER_ID,
                    "participant_id": PARTICIPANT_ID
                }),
                dry_run: false,
            },
            &context(),
            &mut state,
        )
        .unwrap();
    assert_eq!(removed.status, TransactionStatus::Applied);
    assert!(read_encounter(&state).participants.is_empty());
    assert!(removed
        .diagnostics
        .diagnostics
        .iter()
        .any(|item| item.code == ENCOUNTER_NO_PARTICIPANTS));

    let before = state.clone();
    let missing = executor
        .execute(
            CommandRequest {
                command: ToolId::from_static("encounter.remove_participant"),
                input: json!({
                    "encounter_id": ENCOUNTER_ID,
                    "participant_id": "participant.missing"
                }),
                dry_run: false,
            },
            &context(),
            &mut state,
        )
        .unwrap_err();
    let CommandError::ValidationFailed(bag) = missing else {
        panic!("expected missing participant validation failure");
    };
    assert!(bag
        .diagnostics
        .iter()
        .any(|item| item.code == "ENCOUNTER_PARTICIPANT_NOT_FOUND"));
    assert_eq!(state, before);
}

fn host() -> AgentHost {
    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).unwrap();
    register_tactical_queries(&mut queries).unwrap();
    let catalog = ToolCatalog::from_registries(&commands, &queries).unwrap();
    AgentHost::new(
        state(true, false),
        CommandExecutor::new(commands),
        QueryExecutor::new(queries),
        catalog,
    )
}

fn session(id: &'static str, permissions: PermissionSet) -> AgentSession {
    AgentSession::new(AgentSessionId::from_static(id), "ENG-021", permissions)
}

fn add_call(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        tool: ToolId::from_static("encounter.add_participant"),
        input: add_request(SPAWN_ID, STAGE_ID, false).input,
        dry_run: false,
    }
}

#[test]
fn agent_host_refreshes_project_graph_after_participant_add_and_remove() {
    let mut host = host();
    let mut session = session("session.eng021.graph", PermissionSet::content_author());
    let refs = |id: &str| ToolCall {
        id: id.into(),
        tool: ToolId::from_static("project.references.find"),
        input: json!({"target_document": ACTOR_ID}),
        dry_run: false,
    };

    let before = host.execute(&mut session, refs("refs-before"));
    assert_eq!(before.status, ToolResultStatus::Ok);
    assert_eq!(before.output, json!([]));

    let added = host.execute(&mut session, add_call("add"));
    assert_eq!(added.status, ToolResultStatus::Ok);
    assert_eq!(added.transaction.as_ref().unwrap()["status"], "applied");

    let after_add = host.execute(&mut session, refs("refs-added"));
    assert_eq!(after_add.status, ToolResultStatus::Ok);
    assert_eq!(after_add.output.as_array().unwrap().len(), 1);
    assert_eq!(after_add.output[0]["source_document"], ENCOUNTER_ID);
    assert_eq!(after_add.output[0]["source_object"], PARTICIPANT_ID);

    let removed = host.execute(
        &mut session,
        ToolCall {
            id: "remove".into(),
            tool: ToolId::from_static("encounter.remove_participant"),
            input: json!({
                "encounter_id": ENCOUNTER_ID,
                "participant_id": PARTICIPANT_ID
            }),
            dry_run: false,
        },
    );
    assert_eq!(removed.status, ToolResultStatus::Ok);

    let after_remove = host.execute(&mut session, refs("refs-removed"));
    assert_eq!(after_remove.status, ToolResultStatus::Ok);
    assert_eq!(after_remove.output, json!([]));
}

#[test]
fn agent_host_enforces_permission_and_full_document_access_plan() {
    let mut host = host();
    let before = host.state.clone();

    let mut read_only = session("session.eng021.read", PermissionSet::read_only());
    let denied = host.execute(&mut read_only, add_call("permission-denied"));
    assert_eq!(denied.status, ToolResultStatus::Rejected);
    assert_eq!(denied.output["error"]["code"], AGENT_PERMISSION_DENIED);
    assert_eq!(host.state, before);

    let mut restricted = session("session.eng021.scope", PermissionSet::content_author());
    restricted.working_set = vec![
        DocumentId::from_static(ENCOUNTER_ID),
        DocumentId::from_static(ACTOR_ID),
    ];
    let scoped = host.execute(&mut restricted, add_call("scope-denied"));
    assert_eq!(scoped.status, ToolResultStatus::Rejected);
    assert_eq!(scoped.output["error"]["code"], AGENT_WORKING_SET_DENIED);
    assert_eq!(host.state, before);
}

#[test]
fn live_catalog_exposes_all_encounter_command_schemas() {
    let mut commands = CommandRegistry::default();
    register_tactical_commands(&mut commands).unwrap();
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).unwrap();
    register_tactical_queries(&mut queries).unwrap();
    let catalog = ToolCatalog::from_registries(&commands, &queries).unwrap();

    for (tool, required) in [
        (
            "encounter.create",
            ["encounter_id", "name", "stage_id"].as_slice(),
        ),
        (
            "encounter.add_participant",
            [
                "encounter_id",
                "stage_id",
                "participant_id",
                "actor_definition",
                "spawn_point",
                "side",
            ]
            .as_slice(),
        ),
        (
            "encounter.remove_participant",
            ["encounter_id", "participant_id"].as_slice(),
        ),
    ] {
        let descriptor = catalog
            .list()
            .find(|descriptor| descriptor.id.as_str() == tool)
            .unwrap();
        assert!(descriptor.input_schema.is_object());
        let advertised = descriptor.input_schema["required"].as_array().unwrap();
        for field in required {
            assert!(advertised.contains(&Value::String((*field).into())));
        }
    }
}

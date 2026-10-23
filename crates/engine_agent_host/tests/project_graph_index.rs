use guiyi_engine_agent_host::{AgentHost, AgentSession};
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_command::{
    register_builtin_document_commands, CommandDescriptor, CommandError, CommandExecutor,
    CommandHandler, CommandRegistry, EngineState,
};
use guiyi_engine_content::{ContentReference, DocumentEnvelope, DocumentHeader, DocumentStore};
use guiyi_engine_core::{
    AgentSessionId, DocumentAccessPlan, DocumentId, EngineTypeId, Permission, PermissionSet, ToolId,
};
use guiyi_engine_protocol::{ToolCall, ToolResult, ToolResultStatus};
use guiyi_engine_query::{register_builtin_queries, QueryExecutor, QueryRegistry};
use guiyi_engine_schema::{FieldSchema, SchemaNode};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
struct RetargetInput {
    document_id: DocumentId,
    target_document: DocumentId,
}

struct RetargetReferenceCommand;

impl CommandHandler for RetargetReferenceCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("test.reference.retarget"),
            title: "Retarget reference".into(),
            description: "Test command that replaces one document reference.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into()],
            related_tools: vec![ToolId::from_static("project.impact.analyze")],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("document_id", SchemaNode::string()),
            FieldSchema::required("target_document", SchemaNode::string()),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: RetargetInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.document_id))
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: RetargetInput = serde_json::from_value(input.clone())?;
        let document = state.documents.get_mut(&input.document_id)?;
        document.references = vec![ContentReference {
            source_object: None,
            target_document: input.target_document.clone(),
            target_object: None,
            kind: "test".into(),
        }];
        Ok(json!({
            "document_id": input.document_id,
            "target_document": input.target_document
        }))
    }
}

fn document(id: &'static str, target: Option<&'static str>) -> DocumentEnvelope {
    DocumentEnvelope {
        header: DocumentHeader {
            id: DocumentId::from_static(id),
            type_id: EngineTypeId::from_static("example.document"),
            schema_version: 1,
            display_name: id.into(),
        },
        references: target
            .map(|target| {
                vec![ContentReference {
                    source_object: None,
                    target_document: DocumentId::from_static(target),
                    target_object: None,
                    kind: "test".into(),
                }]
            })
            .unwrap_or_default(),
        payload: json!({}),
    }
}

fn state(documents: impl IntoIterator<Item = DocumentEnvelope>) -> EngineState {
    let mut store = DocumentStore::default();
    for document in documents {
        store.insert(document).unwrap();
    }
    EngineState { documents: store }
}

fn host(state: EngineState) -> AgentHost {
    let mut commands = CommandRegistry::default();
    register_builtin_document_commands(&mut commands).unwrap();
    commands.register(RetargetReferenceCommand).unwrap();
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).unwrap();
    let catalog = ToolCatalog::from_registries(&commands, &queries).unwrap();
    AgentHost::new(
        state,
        CommandExecutor::new(commands),
        QueryExecutor::new(queries),
        catalog,
    )
}

fn session(permissions: PermissionSet) -> AgentSession {
    AgentSession::new(
        AgentSessionId::from_static("session.project-graph"),
        "Project graph integration test",
        permissions,
    )
}

fn impact(host: &mut AgentHost, session: &mut AgentSession, target: &'static str) -> ToolResult {
    host.execute(
        session,
        ToolCall {
            id: format!("impact-{target}-{}", session.actions.len()),
            tool: ToolId::from_static("project.impact.analyze"),
            input: json!({"document_id": target}),
            dry_run: false,
        },
    )
}

fn retarget(
    host: &mut AgentHost,
    session: &mut AgentSession,
    target: &'static str,
    dry_run: bool,
) -> ToolResult {
    host.execute(
        session,
        ToolCall {
            id: format!("retarget-{target}-{dry_run}"),
            tool: ToolId::from_static("test.reference.retarget"),
            input: json!({
                "document_id": "doc.b",
                "target_document": target
            }),
            dry_run,
        },
    )
}

#[test]
fn applied_command_refreshes_graph_dry_run_does_not_and_state_replacement_invalidates() {
    let initial = state([
        document("doc.a", None),
        document("doc.b", Some("doc.a")),
        document("doc.c", None),
    ]);
    let mut host = host(initial.clone());
    let mut authoring_session = session(PermissionSet::content_author());

    let before = impact(&mut host, &mut authoring_session, "doc.a");
    assert_eq!(before.status, ToolResultStatus::Ok);
    assert_eq!(before.output, json!(["doc.b"]));

    let preview = retarget(&mut host, &mut authoring_session, "doc.c", true);
    assert_eq!(preview.status, ToolResultStatus::Ok);
    assert_eq!(preview.transaction.as_ref().unwrap()["status"], "previewed");
    assert_eq!(
        impact(&mut host, &mut authoring_session, "doc.a").output,
        json!(["doc.b"])
    );
    assert_eq!(
        impact(&mut host, &mut authoring_session, "doc.c").output,
        json!([])
    );

    let applied = retarget(&mut host, &mut authoring_session, "doc.c", false);
    assert_eq!(applied.status, ToolResultStatus::Ok);
    assert_eq!(applied.transaction.as_ref().unwrap()["status"], "applied");
    assert_eq!(
        impact(&mut host, &mut authoring_session, "doc.a").output,
        json!([])
    );
    assert_eq!(
        impact(&mut host, &mut authoring_session, "doc.c").output,
        json!(["doc.b"])
    );

    host.replace_state(initial);

    let mut restricted = session(PermissionSet::read_only());
    restricted.id = AgentSessionId::from_static("session.project-graph-after-rollback");
    restricted.working_set = vec![DocumentId::from_static("doc.a")];
    assert_eq!(
        impact(&mut host, &mut restricted, "doc.a").output,
        json!([])
    );

    assert_eq!(
        impact(&mut host, &mut authoring_session, "doc.a").output,
        json!(["doc.b"])
    );
    assert_eq!(
        impact(&mut host, &mut authoring_session, "doc.c").output,
        json!([])
    );
}

#[test]
fn applied_create_and_delete_refresh_nodes_and_preserve_dangling_incoming_edges() {
    let mut host = host(state([
        document("doc.a", None),
        document("doc.b", Some("doc.a")),
    ]));
    let mut session = session(PermissionSet::content_author());

    let created = host.execute(
        &mut session,
        ToolCall {
            id: "create-doc-new".into(),
            tool: ToolId::from_static("document.create"),
            input: json!({
                "id": "doc.new",
                "type_id": "example.document",
                "display_name": "New document"
            }),
            dry_run: false,
        },
    );
    assert_eq!(created.status, ToolResultStatus::Ok);
    assert!(host
        .project_graph_snapshot()
        .outgoing
        .contains_key(&DocumentId::from_static("doc.new")));

    let deleted = host.execute(
        &mut session,
        ToolCall {
            id: "delete-doc-a".into(),
            tool: ToolId::from_static("document.delete"),
            input: json!({"document_id": "doc.a"}),
            dry_run: false,
        },
    );
    assert_eq!(deleted.status, ToolResultStatus::Ok);

    let graph = host.project_graph_snapshot();
    assert!(!graph
        .outgoing
        .contains_key(&DocumentId::from_static("doc.a")));
    assert_eq!(
        graph.dependents(&DocumentId::from_static("doc.a")),
        BTreeSet::from([DocumentId::from_static("doc.b")])
    );
}

#[test]
fn restricted_working_set_neither_returns_nor_traverses_hidden_graph_nodes() {
    let mut host = host(state([
        document("doc.a", None),
        document("doc.hidden", Some("doc.a")),
        document("doc.c", Some("doc.hidden")),
    ]));
    let mut unrestricted = session(PermissionSet::read_only());
    assert_eq!(
        impact(&mut host, &mut unrestricted, "doc.a").output,
        json!(["doc.c", "doc.hidden"])
    );

    let mut restricted = session(PermissionSet::read_only());
    restricted.id = AgentSessionId::from_static("session.project-graph-restricted");
    restricted.working_set = vec![
        DocumentId::from_static("doc.a"),
        DocumentId::from_static("doc.c"),
    ];
    let restricted_impact = impact(&mut host, &mut restricted, "doc.a");
    assert_eq!(restricted_impact.status, ToolResultStatus::Ok);
    assert_eq!(restricted_impact.output, json!([]));

    let references = host.execute(
        &mut restricted,
        ToolCall {
            id: "references-doc-a".into(),
            tool: ToolId::from_static("project.references.find"),
            input: json!({"target_document": "doc.a"}),
            dry_run: false,
        },
    );
    assert_eq!(references.status, ToolResultStatus::Ok);
    assert_eq!(references.output, json!([]));

    let graph = host.project_graph_snapshot();
    assert_eq!(
        graph.dependents(&DocumentId::from_static("doc.a")),
        BTreeSet::from([DocumentId::from_static("doc.hidden")])
    );
}

#[test]
fn graph_queries_remain_live_schema_bearing_catalog_capabilities() {
    let host = host(EngineState::default());
    for tool in ["project.references.find", "project.impact.analyze"] {
        let descriptor = host.catalog().get(&ToolId::new(tool).unwrap()).unwrap();
        assert!(descriptor.input_schema.is_object());
        assert!(descriptor.output_schema.is_object() || descriptor.output_schema.is_array());
    }
}

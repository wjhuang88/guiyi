use async_trait::async_trait;
use guiyi_engine_agent_host::AgentHost;
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_command::{
    register_builtin_document_commands, CommandExecutor, CommandRegistry, EngineState,
};
use guiyi_engine_content::{DocumentEnvelope, ProjectManifest, ProjectPath, ProjectStorage};
use guiyi_engine_core::{AgentSessionId, DocumentId, PermissionSet};
use guiyi_engine_query::{register_builtin_queries, QueryExecutor, QueryRegistry};
use guiyi_talos_runtime::{GuiyiExecutionContext, GuiyiTalosAdapter, ProjectStoragePersistence};
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use talos_runtime::{
    AgentEvent, ApprovalChoice, ApprovalHandler, LanguageModel, Message, MessageToolResult,
    ProviderResult, Receiver, RuntimeTurnCompletionStatus, SessionEvent, StopReason,
    ToolCall as TalosToolCall, ToolDefinition, TurnEventPayload, Usage,
};

fn host(state: EngineState) -> AgentHost {
    let mut commands = CommandRegistry::default();
    register_builtin_document_commands(&mut commands).unwrap();
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

#[derive(Clone)]
enum ProviderStep {
    Tool {
        id: &'static str,
        name: &'static str,
        input: Value,
    },
    Text(&'static str),
}

#[derive(Default)]
struct ScriptedProviderState {
    steps: VecDeque<ProviderStep>,
    tool_definitions: Vec<Vec<ToolDefinition>>,
    tool_results: BTreeMap<String, MessageToolResult>,
}

#[derive(Default)]
struct ScriptedProvider {
    state: Mutex<ScriptedProviderState>,
}

impl ScriptedProvider {
    fn new(steps: impl IntoIterator<Item = ProviderStep>) -> Self {
        Self {
            state: Mutex::new(ScriptedProviderState {
                steps: steps.into_iter().collect(),
                ..ScriptedProviderState::default()
            }),
        }
    }

    fn observed_tools(&self) -> Vec<ToolDefinition> {
        self.state
            .lock()
            .unwrap()
            .tool_definitions
            .iter()
            .find(|tools| !tools.is_empty())
            .cloned()
            .unwrap_or_default()
    }

    fn result(&self, id: &str) -> MessageToolResult {
        self.state
            .lock()
            .unwrap()
            .tool_results
            .get(id)
            .cloned()
            .unwrap_or_else(|| panic!("missing tool result for {id}"))
    }

    async fn emit(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> ProviderResult<Receiver<AgentEvent>> {
        let step = {
            let mut state = self.state.lock().unwrap();
            state.tool_definitions.push(tools.to_vec());
            for message in messages {
                if let Message::Tool { result } = message {
                    state
                        .tool_results
                        .entry(result.tool_use_id.clone())
                        .or_insert_with(|| result.clone());
                }
            }
            state
                .steps
                .pop_front()
                .unwrap_or(ProviderStep::Text("done"))
        };
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        tokio::spawn(async move {
            let _ = tx.send(AgentEvent::TurnStart).await;
            match step {
                ProviderStep::Tool { id, name, input } => {
                    let _ = tx
                        .send(AgentEvent::ToolCall {
                            call: TalosToolCall {
                                id: id.into(),
                                name: name.into(),
                                input,
                            },
                            provenance: Default::default(),
                            summary_fields: vec![],
                        })
                        .await;
                    let _ = tx
                        .send(AgentEvent::TurnEnd {
                            stop_reason: StopReason::ToolUse,
                            usage: Usage::default(),
                        })
                        .await;
                }
                ProviderStep::Text(text) => {
                    let _ = tx.send(AgentEvent::TextDelta { delta: text.into() }).await;
                    let _ = tx
                        .send(AgentEvent::TurnEnd {
                            stop_reason: StopReason::EndTurn,
                            usage: Usage::default(),
                        })
                        .await;
                }
            }
        });
        Ok(rx)
    }
}

#[async_trait]
impl LanguageModel for ScriptedProvider {
    async fn stream(&self, messages: &[Message]) -> ProviderResult<Receiver<AgentEvent>> {
        self.emit(messages, &[]).await
    }

    async fn stream_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> ProviderResult<Receiver<AgentEvent>> {
        self.emit(messages, tools).await
    }
}

struct ApproveOnce;

#[async_trait]
impl ApprovalHandler for ApproveOnce {
    async fn request_approval(
        &self,
        _tool_name: &str,
        _arguments: &Value,
        _summary_fields: &[String],
    ) -> ApprovalChoice {
        ApprovalChoice::ApproveOnce
    }
}

async fn run_runtime(adapter: &GuiyiTalosAdapter, provider: Arc<ScriptedProvider>) {
    let mut runtime = adapter
        .register(
            talos_runtime::RuntimeBuilder::new()
                .provider(provider)
                .workspace_root(".")
                .approval_handler(Arc::new(ApproveOnce)),
        )
        .build()
        .unwrap();
    runtime
        .submit("Execute the scripted GUIYI workflow")
        .await
        .unwrap();
    let mut completed = false;
    while let Some(event) = runtime.next_event().await {
        match event {
            SessionEvent::TurnEvent {
                payload: TurnEventPayload::Completed { status },
                ..
            }
            | SessionEvent::TurnCompleted { status, .. } => {
                assert!(matches!(
                    status,
                    RuntimeTurnCompletionStatus::Success { .. }
                ));
                completed = true;
                break;
            }
            SessionEvent::Error { message } => panic!("Talos runtime error: {message}"),
            _ => {}
        }
    }
    assert!(completed, "Talos runtime did not complete the turn");
    runtime.shutdown().await.unwrap();
}

fn parsed_result(result: MessageToolResult) -> Value {
    serde_json::from_str(&result.content).unwrap()
}

#[tokio::test]
async fn live_catalog_becomes_schema_bearing_talos_tools_and_query_executes() {
    let adapter = GuiyiTalosAdapter::new(
        host(EngineState::default()),
        GuiyiExecutionContext::new(
            AgentSessionId::from_static("session.talos.discovery"),
            PermissionSet::read_only(),
        ),
    );
    let provider = Arc::new(ScriptedProvider::new([
        ProviderStep::Tool {
            id: "query",
            name: "project.documents.list",
            input: json!({}),
        },
        ProviderStep::Text("done"),
    ]));
    run_runtime(&adapter, Arc::clone(&provider)).await;

    let tools = provider.observed_tools();
    let query = tools
        .iter()
        .find(|tool| tool.name == "project.documents.list")
        .expect("live query registered with Talos");
    assert_eq!(query.parameters["type"], "object");
    assert!(tools.iter().any(|tool| tool.name == "document.create"));
    let result = parsed_result(provider.result("query"));
    assert_eq!(result["status"], "ok");
    assert!(result["output"].is_array());
}

#[tokio::test]
async fn dry_run_then_apply_persists_transaction_and_audit() {
    let root = temporary_project("mutation");
    let (persistence, state) = ProjectStoragePersistence::open(&root).unwrap();
    let adapter = GuiyiTalosAdapter::with_persistence(
        host(state),
        GuiyiExecutionContext::new(
            AgentSessionId::from_static("session.talos.mutation"),
            PermissionSet::content_author(),
        ),
        persistence,
    );
    let command_input = json!({
        "id": "doc.talos",
        "type_id": "example.document",
        "display_name": "Talos"
    });
    let provider = Arc::new(ScriptedProvider::new([
        ProviderStep::Tool {
            id: "dry",
            name: "document.create",
            input: json!({"input": command_input.clone(), "dry_run": true}),
        },
        ProviderStep::Tool {
            id: "apply",
            name: "document.create",
            input: json!({"input": command_input, "dry_run": false}),
        },
        ProviderStep::Tool {
            id: "query",
            name: "project.documents.list",
            input: json!({}),
        },
        ProviderStep::Text("done"),
    ]));
    run_runtime(&adapter, Arc::clone(&provider)).await;

    let dry = parsed_result(provider.result("dry"));
    let apply = parsed_result(provider.result("apply"));
    assert_eq!(dry["status"], "ok");
    assert_eq!(dry["transaction"]["status"], "previewed");
    assert_eq!(apply["status"], "ok");
    assert_eq!(apply["transaction"]["status"], "applied");
    assert!(!apply["transaction"]["document_diffs"]
        .as_array()
        .unwrap()
        .is_empty());

    let storage = ProjectStorage::open(&root).unwrap();
    let manifest: ProjectManifest = storage
        .load_json(&ProjectPath::new("engine-project.json").unwrap())
        .unwrap();
    assert_eq!(manifest.documents.len(), 1);
    let document: DocumentEnvelope = storage.load_json(&manifest.documents[0]).unwrap();
    assert_eq!(document.header.id.as_str(), "doc.talos");
    let audit = storage.audit_records().unwrap();
    assert_eq!(audit.len(), 1, "dry-run must not produce a storage commit");
    assert_eq!(audit[0].session_id.as_str(), "session.talos.mutation");
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn guiyi_security_rejects_talos_admitted_calls_and_runtime_continues() {
    let adapter = GuiyiTalosAdapter::new(
        host(EngineState::default()),
        GuiyiExecutionContext::new(
            AgentSessionId::from_static("session.talos.security"),
            PermissionSet::read_only(),
        )
        .with_working_set(vec![DocumentId::from_static("doc.allowed")]),
    );
    let provider = Arc::new(ScriptedProvider::new([
        ProviderStep::Tool {
            id: "permission",
            name: "document.create",
            input: json!({
                "input": {
                    "id": "doc.denied",
                    "type_id": "example.document",
                    "display_name": "Denied"
                },
                "dry_run": false
            }),
        },
        ProviderStep::Tool {
            id: "working-set",
            name: "project.document.get",
            input: json!({"id": "doc.denied"}),
        },
        ProviderStep::Tool {
            id: "after",
            name: "project.documents.list",
            input: json!({}),
        },
        ProviderStep::Text("done"),
    ]));
    run_runtime(&adapter, Arc::clone(&provider)).await;

    let permission = parsed_result(provider.result("permission"));
    assert_eq!(
        permission["output"]["error"]["code"],
        "AGENT_PERMISSION_DENIED"
    );
    let working_set = parsed_result(provider.result("working-set"));
    assert_eq!(
        working_set["output"]["error"]["code"],
        "AGENT_WORKING_SET_DENIED"
    );
    let after = parsed_result(provider.result("after"));
    assert_eq!(after["status"], "ok");
}

#[tokio::test]
async fn recoverable_guiyi_failure_is_structured_and_followed_by_valid_tool_call() {
    let adapter = GuiyiTalosAdapter::new(
        host(EngineState::default()),
        GuiyiExecutionContext::new(
            AgentSessionId::from_static("session.talos.recovery"),
            PermissionSet::read_only(),
        ),
    );
    let provider = Arc::new(ScriptedProvider::new([
        ProviderStep::Tool {
            id: "missing",
            name: "project.document.get",
            input: json!({"id": "doc.missing"}),
        },
        ProviderStep::Tool {
            id: "after",
            name: "project.documents.list",
            input: json!({}),
        },
        ProviderStep::Text("done"),
    ]));
    run_runtime(&adapter, Arc::clone(&provider)).await;

    let missing = parsed_result(provider.result("missing"));
    assert_eq!(missing["status"], "failed");
    assert_eq!(missing["output"]["error"]["code"], "AGENT_TOOL_FAILED");
    let after = parsed_result(provider.result("after"));
    assert_eq!(after["status"], "ok");
}

fn temporary_project(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("guiyi-talos-{name}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("engine-project.json"),
        r#"{
            "project_id": "project.talos-test",
            "name": "Talos Test",
            "engine_api_version": "0.1.0",
            "content_schema_version": 1,
            "documents": []
        }"#,
    )
    .unwrap();
    root
}

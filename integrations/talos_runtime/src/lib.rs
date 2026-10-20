#![forbid(unsafe_code)]

//! Thin Talos runtime integration for GUIYI's existing machine-facing Tool surface.
//!
//! Talos owns the first-party Agent loop. This crate only adapts the live GUIYI
//! [`ToolCatalog`](guiyi_engine_agent_tools::ToolCatalog) into Talos
//! [`AgentTool`] instances and delegates execution back to [`AgentHost`].

use async_trait::async_trait;
use guiyi_engine_agent_host::{AgentHost, AgentSession};
use guiyi_engine_agent_tools::{ToolDescriptor, ToolKind};
use guiyi_engine_command::EngineState;
use guiyi_engine_content::{
    DocumentEnvelope, DocumentStore, ProjectManifest, ProjectPath, ProjectStorage,
    ProjectTransaction,
};
use guiyi_engine_core::{AgentSessionId, DocumentId, PermissionSet, TransactionId};
use guiyi_engine_protocol::{
    ToolCall as GuiyiToolCall, ToolResult as GuiyiToolResult, ToolResultStatus,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use talos_runtime::{AgentTool, RuntimeBuilder, ToolResult as TalosToolResult};
use thiserror::Error;

const ADAPTER_INVALID_INVOCATION: &str = "GUIYI_TALOS_INVALID_INVOCATION";
const ADAPTER_PERSISTENCE_FAILED: &str = "GUIYI_TALOS_PERSISTENCE_FAILED";

/// GUIYI-owned domain execution state required to authorize Talos Tool calls.
///
/// Generic objective, turn, stopping, retry, and reasoning-budget state belongs
/// to Talos. The legacy `AgentSession` is used only as an internal compatibility
/// seam while direct alpha clients still depend on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiyiExecutionContext {
    pub session_id: AgentSessionId,
    pub permissions: PermissionSet,
    /// Empty means unrestricted; otherwise this is the strict project working set.
    pub working_set: Vec<DocumentId>,
}

impl GuiyiExecutionContext {
    pub fn new(session_id: AgentSessionId, permissions: PermissionSet) -> Self {
        Self {
            session_id,
            permissions,
            working_set: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_working_set(mut self, working_set: Vec<DocumentId>) -> Self {
        self.working_set = working_set;
        self
    }
}

/// Crash-safe project persistence used after an applied GUIYI Command.
///
/// This is an outer-client coordinator over the existing `ProjectStorage` /
/// `ProjectTransaction` authority; it does not replace GUIYI transaction rules.
pub struct ProjectStoragePersistence {
    storage: ProjectStorage,
    manifest: ProjectManifest,
    paths: BTreeMap<DocumentId, ProjectPath>,
}

impl ProjectStoragePersistence {
    /// Load a project through the existing sandboxed storage boundary.
    pub fn open(root: impl AsRef<Path>) -> Result<(Self, EngineState), TalosIntegrationError> {
        let storage = ProjectStorage::open(root)?;
        let manifest_path = ProjectPath::new("engine-project.json")?;
        let manifest: ProjectManifest = storage.load_json(&manifest_path)?;
        let mut documents = DocumentStore::default();
        let mut paths = BTreeMap::new();
        for relative in &manifest.documents {
            let document: DocumentEnvelope = storage.load_json(relative)?;
            paths.insert(document.header.id.clone(), relative.clone());
            documents.insert(document)?;
        }
        Ok((
            Self {
                storage,
                manifest,
                paths,
            },
            EngineState { documents },
        ))
    }

    fn persist(
        &mut self,
        session_id: &AgentSessionId,
        store: &DocumentStore,
        result: &GuiyiToolResult,
    ) -> Result<(), TalosIntegrationError> {
        let report = result
            .transaction
            .clone()
            .ok_or(TalosIntegrationError::MissingTransactionReport)?;
        let transaction_id = report
            .get("transaction_id")
            .and_then(Value::as_str)
            .ok_or(TalosIntegrationError::MissingTransactionId)?;
        let transaction_id = TransactionId::new(transaction_id)
            .map_err(|error| TalosIntegrationError::InvalidTransactionId(error.to_string()))?;
        let mut transaction = ProjectTransaction::new(
            transaction_id,
            session_id.clone(),
            session_id.to_string(),
            report,
        );
        let mut next_manifest = self.manifest.clone();
        let mut next_paths = self.paths.clone();

        let stale = next_paths
            .keys()
            .filter(|id| store.get(id).is_err())
            .cloned()
            .collect::<Vec<_>>();
        for id in stale {
            if let Some(relative) = next_paths.remove(&id) {
                transaction.delete(relative);
            }
        }
        for (id, document) in store.iter() {
            let relative = if let Some(existing) = next_paths.get(id) {
                existing.clone()
            } else {
                let path = ProjectPath::new(format!("content/generated/{}.json", id.as_str()))?;
                next_manifest.documents.push(path.clone());
                next_paths.insert(id.clone(), path.clone());
                path
            };
            transaction.write_json(relative, document)?;
        }
        next_manifest
            .documents
            .retain(|path| next_paths.values().any(|known| known == path));
        transaction
            .write_manifest_json(ProjectPath::new("engine-project.json")?, &next_manifest)?;
        self.storage.commit(transaction)?;
        self.manifest = next_manifest;
        self.paths = next_paths;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum TalosIntegrationError {
    #[error(transparent)]
    Content(#[from] guiyi_engine_content::ContentError),
    #[error(transparent)]
    ProjectPath(#[from] guiyi_engine_content::ProjectPathError),
    #[error("GUIYI transaction result is missing its transaction report")]
    MissingTransactionReport,
    #[error("GUIYI transaction report is missing transaction_id")]
    MissingTransactionId,
    #[error("invalid GUIYI transaction_id: {0}")]
    InvalidTransactionId(String),
}

struct SharedExecution {
    host: AgentHost,
    compatibility_session: AgentSession,
    persistence: Option<ProjectStoragePersistence>,
    next_call_sequence: u64,
}

/// Registers the live GUIYI Tool catalog as individual Talos `AgentTool`s.
#[derive(Clone)]
pub struct GuiyiTalosAdapter {
    execution: Arc<Mutex<SharedExecution>>,
    descriptors: Arc<Vec<ToolDescriptor>>,
}

impl GuiyiTalosAdapter {
    /// Create an in-memory adapter. Applied Commands update the supplied host
    /// state but are not persisted unless `with_persistence` is used.
    pub fn new(host: AgentHost, context: GuiyiExecutionContext) -> Self {
        Self::from_parts(host, context, None)
    }

    /// Create an adapter whose successfully applied Commands are committed via
    /// the existing crash-safe GUIYI project-storage boundary.
    pub fn with_persistence(
        host: AgentHost,
        context: GuiyiExecutionContext,
        persistence: ProjectStoragePersistence,
    ) -> Self {
        Self::from_parts(host, context, Some(persistence))
    }

    fn from_parts(
        host: AgentHost,
        context: GuiyiExecutionContext,
        persistence: Option<ProjectStoragePersistence>,
    ) -> Self {
        let descriptors = host.catalog().list().cloned().collect::<Vec<_>>();
        let mut compatibility_session = AgentSession::new(
            context.session_id,
            "Talos-owned lifecycle; GUIYI domain execution compatibility seam",
            context.permissions,
        );
        compatibility_session.working_set = context.working_set;
        // AgentBudget is a legacy generic action budget. Talos owns first-party
        // reasoning/action stopping, so the adapter intentionally does not add a
        // second practical limit here while AgentHost still requires AgentSession.
        compatibility_session.budget.max_actions = u32::MAX;
        Self {
            execution: Arc::new(Mutex::new(SharedExecution {
                host,
                compatibility_session,
                persistence,
                next_call_sequence: 0,
            })),
            descriptors: Arc::new(descriptors),
        }
    }

    /// Materialize one real Talos Tool per descriptor in the live GUIYI catalog.
    pub fn agent_tools(&self) -> Vec<Arc<dyn AgentTool>> {
        self.descriptors
            .iter()
            .cloned()
            .map(|descriptor| {
                Arc::new(GuiyiAgentTool {
                    descriptor,
                    execution: Arc::clone(&self.execution),
                }) as Arc<dyn AgentTool>
            })
            .collect()
    }

    /// Register every live GUIYI capability through Talos's public builder API.
    #[must_use]
    pub fn register(&self, mut builder: RuntimeBuilder) -> RuntimeBuilder {
        for tool in self.agent_tools() {
            builder = builder.tool(tool);
        }
        builder
    }
}

struct GuiyiAgentTool {
    descriptor: ToolDescriptor,
    execution: Arc<Mutex<SharedExecution>>,
}

impl GuiyiAgentTool {
    fn parameters_value(&self) -> Value {
        match self.descriptor.kind {
            ToolKind::Query => self.descriptor.input_schema.clone(),
            ToolKind::Command => json!({
                "type": "object",
                "properties": {
                    "input": self.descriptor.input_schema.clone(),
                    "dry_run": {
                        "type": "boolean",
                        "default": false,
                        "description": "When true, execute GUIYI's authoritative command dry-run path without persistence."
                    }
                },
                "required": ["input"],
                "additionalProperties": false,
                "x-guiyi-command-input-schema": self.descriptor.input_schema.clone()
            }),
        }
    }

    fn parse_invocation(&self, input: Value) -> Result<(Value, bool), String> {
        match self.descriptor.kind {
            ToolKind::Query => Ok((input, false)),
            ToolKind::Command => {
                let object = input.as_object().ok_or_else(|| {
                    "command invocation must be an object with `input` and optional `dry_run`"
                        .to_string()
                })?;
                let command_input = object
                    .get("input")
                    .cloned()
                    .ok_or_else(|| "command invocation is missing `input`".to_string())?;
                let dry_run = object
                    .get("dry_run")
                    .map(|value| {
                        value
                            .as_bool()
                            .ok_or_else(|| "`dry_run` must be a boolean".to_string())
                    })
                    .transpose()?
                    .unwrap_or(false);
                Ok((command_input, dry_run))
            }
        }
    }
}

#[async_trait]
impl AgentTool for GuiyiAgentTool {
    fn name(&self) -> &str {
        self.descriptor.id.as_str()
    }

    fn description(&self) -> &str {
        &self.descriptor.description
    }

    fn parameters(&self) -> Value {
        self.parameters_value()
    }

    fn is_read_only(&self) -> bool {
        self.descriptor.kind == ToolKind::Query
    }

    async fn execute(&self, input: Value) -> TalosToolResult {
        let (guiyi_input, dry_run) = match self.parse_invocation(input) {
            Ok(value) => value,
            Err(message) => {
                return TalosToolResult::error(
                    json!({
                        "call_id": null,
                        "status": "rejected",
                        "output": {
                            "error": {
                                "code": ADAPTER_INVALID_INVOCATION,
                                "message": message
                            }
                        },
                        "diagnostics": [],
                        "transaction": null
                    })
                    .to_string(),
                )
            }
        };

        let mut shared = match self.execution.lock() {
            Ok(shared) => shared,
            Err(_) => {
                return TalosToolResult::error(
                    json!({
                        "call_id": null,
                        "status": "failed",
                        "output": {
                            "error": {
                                "code": "GUIYI_TALOS_EXECUTION_LOCK_FAILED",
                                "message": "GUIYI execution state lock was poisoned"
                            }
                        },
                        "diagnostics": [],
                        "transaction": null
                    })
                    .to_string(),
                )
            }
        };
        shared.next_call_sequence += 1;
        let call = GuiyiToolCall {
            id: format!(
                "talos:{}:{}",
                self.descriptor.id.as_str(),
                shared.next_call_sequence
            ),
            tool: self.descriptor.id.clone(),
            input: guiyi_input,
            dry_run,
        };
        let rollback_state = if self.descriptor.kind == ToolKind::Command && !dry_run {
            Some(shared.host.state.clone())
        } else {
            None
        };

        let result = {
            let SharedExecution {
                host,
                compatibility_session,
                ..
            } = &mut *shared;
            host.execute(compatibility_session, call)
        };

        let should_persist = self.descriptor.kind == ToolKind::Command
            && !dry_run
            && result.status == ToolResultStatus::Ok
            && result.transaction.is_some();
        if should_persist {
            let persistence_result = {
                let SharedExecution {
                    host,
                    compatibility_session,
                    persistence,
                    ..
                } = &mut *shared;
                match persistence {
                    Some(persistence) => persistence.persist(
                        &compatibility_session.id,
                        &host.state.documents,
                        &result,
                    ),
                    None => Ok(()),
                }
            };
            if let Err(error) = persistence_result {
                if let Some(previous) = rollback_state {
                    shared.host.state = previous;
                }
                return TalosToolResult::error(
                    json!({
                        "call_id": result.call_id,
                        "status": "failed",
                        "output": {
                            "error": {
                                "code": ADAPTER_PERSISTENCE_FAILED,
                                "message": error.to_string()
                            }
                        },
                        "diagnostics": result.diagnostics,
                        "transaction": result.transaction
                    })
                    .to_string(),
                );
            }
        }

        let is_error = result.status != ToolResultStatus::Ok;
        match serde_json::to_string(&result) {
            Ok(content) if is_error => TalosToolResult::error(content),
            Ok(content) => TalosToolResult::success(content),
            Err(error) => TalosToolResult::error(
                json!({
                    "call_id": result.call_id,
                    "status": "failed",
                    "output": {
                        "error": {
                            "code": "GUIYI_TALOS_RESULT_ENCODING_FAILED",
                            "message": error.to_string()
                        }
                    },
                    "diagnostics": [],
                    "transaction": null
                })
                .to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_agent_tools::ToolCatalog;
    use guiyi_engine_command::{
        register_builtin_document_commands, CommandExecutor, CommandRegistry,
    };
    use guiyi_engine_query::{register_builtin_queries, QueryExecutor, QueryRegistry};
    use std::collections::{BTreeMap, VecDeque};
    use std::fs;
    use std::sync::Mutex as StdMutex;
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
        state: StdMutex<ScriptedProviderState>,
    }

    impl ScriptedProvider {
        fn new(steps: impl IntoIterator<Item = ProviderStep>) -> Self {
            Self {
                state: StdMutex::new(ScriptedProviderState {
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
                RuntimeBuilder::new()
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
                } => {
                    assert_eq!(status, RuntimeTurnCompletionStatus::Completed);
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
        assert_eq!(dry["transaction"]["status"], "dry_run");
        assert_eq!(apply["status"], "ok");
        assert_eq!(apply["transaction"]["status"], "applied");
        assert!(!apply["transaction"]["diff"].as_array().unwrap().is_empty());

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
                id: "invalid",
                name: "project.document.get",
                input: json!({}),
            },
            ProviderStep::Tool {
                id: "after",
                name: "project.documents.list",
                input: json!({}),
            },
            ProviderStep::Text("done"),
        ]));
        run_runtime(&adapter, Arc::clone(&provider)).await;

        let invalid = parsed_result(provider.result("invalid"));
        assert_eq!(invalid["status"], "rejected");
        assert_eq!(
            invalid["output"]["error"]["code"],
            "AGENT_ACCESS_PLAN_INVALID"
        );
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
}

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

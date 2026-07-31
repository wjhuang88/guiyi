#![forbid(unsafe_code)]

//! Agent sessions, budgets, working sets, loop adapters, and unified tool routing.

use guiyi_engine_agent_tools::{ToolCatalog, ToolKind};
use guiyi_engine_command::{
    CommandContext, CommandError, CommandExecutor, CommandRequest, EngineState, TransactionStatus,
};
use guiyi_engine_content::DocumentStore;
use guiyi_engine_core::{AgentSessionId, DocumentId, PermissionSet};
use guiyi_engine_protocol::{ToolCall, ToolResult, ToolResultStatus};
use guiyi_engine_query::{QueryContext, QueryExecutor, QueryRequest};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeSet, VecDeque};
use thiserror::Error;

pub const AGENT_SESSION_NOT_ACTIVE: &str = "AGENT_SESSION_NOT_ACTIVE";
pub const AGENT_BUDGET_EXCEEDED: &str = "AGENT_BUDGET_EXCEEDED";
pub const AGENT_PERMISSION_DENIED: &str = "AGENT_PERMISSION_DENIED";
pub const AGENT_WORKING_SET_DENIED: &str = "AGENT_WORKING_SET_DENIED";
pub const AGENT_ACCESS_PLAN_INVALID: &str = "AGENT_ACCESS_PLAN_INVALID";
pub const AGENT_TOOL_NOT_FOUND: &str = "AGENT_TOOL_NOT_FOUND";
pub const AGENT_TOOL_FAILED: &str = "AGENT_TOOL_FAILED";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Ready,
    Running,
    Completed,
    Stopped,
    BudgetExceeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBudget {
    pub max_actions: u32,
}

impl Default for AgentBudget {
    fn default() -> Self {
        Self { max_actions: 32 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentActionRecord {
    pub sequence: u32,
    pub call: ToolCall,
    pub result: ToolResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: AgentSessionId,
    pub objective: String,
    pub permissions: PermissionSet,
    /// Empty means unrestricted. A non-empty set is a strict visibility and
    /// mutation boundary for every command and query.
    #[serde(default)]
    pub working_set: Vec<DocumentId>,
    pub budget: AgentBudget,
    /// Number of tool attempts that entered permission/access/dispatch checks.
    /// Budget-denied attempts are audited but do not increment this value.
    #[serde(default)]
    pub actions_used: u32,
    pub status: SessionStatus,
    #[serde(default)]
    pub actions: Vec<AgentActionRecord>,
    pub final_summary: Option<String>,
}

impl AgentSession {
    pub fn new(
        id: AgentSessionId,
        objective: impl Into<String>,
        permissions: PermissionSet,
    ) -> Self {
        Self {
            id,
            objective: objective.into(),
            permissions,
            working_set: Vec::new(),
            budget: AgentBudget::default(),
            actions_used: 0,
            status: SessionStatus::Ready,
            actions: Vec::new(),
            final_summary: None,
        }
    }

    pub fn allowed_documents(&self) -> Option<BTreeSet<DocumentId>> {
        if self.working_set.is_empty() {
            None
        } else {
            Some(self.working_set.iter().cloned().collect())
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            SessionStatus::Completed
                | SessionStatus::Stopped
                | SessionStatus::BudgetExceeded
                | SessionStatus::Failed
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentDirective {
    Tool(ToolCall),
    Complete { summary: String },
    Stop { reason: String },
}

pub trait AgentLoopDriver {
    fn next_action(
        &mut self,
        session: &AgentSession,
        catalog: &ToolCatalog,
        last_result: Option<&ToolResult>,
    ) -> Result<AgentDirective, AgentHostError>;
}

pub struct AgentHost {
    pub state: EngineState,
    command_executor: CommandExecutor,
    query_executor: QueryExecutor,
    catalog: ToolCatalog,
}

impl AgentHost {
    pub fn new(
        state: EngineState,
        command_executor: CommandExecutor,
        query_executor: QueryExecutor,
        catalog: ToolCatalog,
    ) -> Self {
        Self {
            state,
            command_executor,
            query_executor,
            catalog,
        }
    }

    pub fn catalog(&self) -> &ToolCatalog {
        &self.catalog
    }

    /// The only public tool-execution entry point.
    ///
    /// It enforces session lifecycle, budget, permissions, working set,
    /// structured results, action history, and deterministic status updates.
    pub fn execute(&mut self, session: &mut AgentSession, call: ToolCall) -> ToolResult {
        if session.is_terminal() {
            let result = rejected(
                &call,
                AGENT_SESSION_NOT_ACTIVE,
                format!("session is not active: {:?}", session.status),
                json!({"session_status": session.status}),
            );
            record_action(session, call, result.clone());
            return result;
        }
        if session.status == SessionStatus::Ready {
            session.status = SessionStatus::Running;
        }
        if session.actions_used >= session.budget.max_actions {
            session.status = SessionStatus::BudgetExceeded;
            session.final_summary = Some(format!(
                "action budget exhausted at {} tool actions",
                session.budget.max_actions
            ));
            let result = rejected(
                &call,
                AGENT_BUDGET_EXCEEDED,
                "agent action budget exceeded",
                json!({
                    "max_actions": session.budget.max_actions,
                    "actions_used": session.actions_used
                }),
            );
            record_action(session, call, result.clone());
            return result;
        }

        session.actions_used += 1;
        let result = self.dispatch(session, &call);
        record_action(session, call, result.clone());
        result
    }

    fn dispatch(&mut self, session: &AgentSession, call: &ToolCall) -> ToolResult {
        let descriptor = match self.catalog.get(&call.tool) {
            Some(descriptor) => descriptor,
            None => {
                return rejected(
                    call,
                    AGENT_TOOL_NOT_FOUND,
                    format!("tool not found: {}", call.tool),
                    json!({"tool": call.tool}),
                )
            }
        };
        if !session
            .permissions
            .contains_all(&descriptor.required_permissions)
        {
            return rejected(
                call,
                AGENT_PERMISSION_DENIED,
                format!("permission denied for tool: {}", call.tool),
                json!({
                    "tool": call.tool,
                    "required_permissions": descriptor.required_permissions
                }),
            );
        }

        let normalized_input = if descriptor.kind == ToolKind::Command {
            match self.command_executor.prepare_input(&CommandRequest {
                command: call.tool.clone(),
                input: call.input.clone(),
                dry_run: call.dry_run,
            }) {
                Ok(normalized) => normalized,
                Err(CommandError::ValidationFailed(bag)) => {
                    return ToolResult {
                        call_id: call.id.clone(),
                        status: ToolResultStatus::Rejected,
                        output: json!({
                            "error": {
                                "code": "COMMAND_VALIDATION_FAILED",
                                "message": "command input structural validation failed"
                            }
                        }),
                        diagnostics: bag
                            .diagnostics
                            .iter()
                            .map(|item| serde_json::to_value(item).unwrap_or(Value::Null))
                            .collect(),
                        transaction: None,
                    };
                }
                Err(error) => {
                    return failed(
                        call,
                        AGENT_TOOL_FAILED,
                        error.to_string(),
                        json!({"tool": call.tool}),
                    )
                }
            }
        } else {
            call.input.clone()
        };

        let allowed = session.allowed_documents();
        let access = match descriptor.kind {
            ToolKind::Command => self.command_executor.document_access(&CommandRequest {
                command: call.tool.clone(),
                input: normalized_input.clone(),
                dry_run: call.dry_run,
            }),
            ToolKind::Query => self
                .query_executor
                .document_access(&QueryRequest {
                    query: call.tool.clone(),
                    input: call.input.clone(),
                })
                .map_err(|error| CommandError::InvalidInput(error.to_string())),
        };
        let access = match access {
            Ok(access) => access,
            Err(error) => {
                return rejected(
                    call,
                    AGENT_ACCESS_PLAN_INVALID,
                    error.to_string(),
                    json!({"tool": call.tool}),
                )
            }
        };
        if let Some(allowed) = &allowed {
            let denied = access
                .required
                .iter()
                .filter(|document| !allowed.contains(*document))
                .cloned()
                .collect::<Vec<_>>();
            if !denied.is_empty() || (descriptor.kind == ToolKind::Command && access.scans_project)
            {
                return rejected(
                    call,
                    AGENT_WORKING_SET_DENIED,
                    "tool access exceeds the session working set",
                    json!({
                        "denied_documents": denied,
                        "project_scan": access.scans_project,
                        "working_set": allowed
                    }),
                );
            }
        }

        match descriptor.kind {
            ToolKind::Command => {
                self.execute_command(session, call, &normalized_input, allowed.as_ref())
            }
            ToolKind::Query => self.execute_query(session, call, allowed.as_ref()),
        }
    }

    fn execute_command(
        &mut self,
        session: &AgentSession,
        call: &ToolCall,
        normalized_input: &Value,
        allowed: Option<&BTreeSet<DocumentId>>,
    ) -> ToolResult {
        let result = self.command_executor.execute_scoped(
            CommandRequest {
                command: call.tool.clone(),
                input: normalized_input.clone(),
                dry_run: call.dry_run,
            },
            &CommandContext {
                actor: session.id.to_string(),
                permissions: session.permissions.clone(),
            },
            &mut self.state,
            allowed,
        );
        match result {
            Ok(report) => match serde_json::to_value(&report) {
                Ok(transaction) => ToolResult {
                    call_id: call.id.clone(),
                    status: ToolResultStatus::Ok,
                    output: report.output.clone(),
                    diagnostics: report
                        .diagnostics
                        .diagnostics
                        .iter()
                        .map(|item| serde_json::to_value(item).unwrap_or(Value::Null))
                        .collect(),
                    transaction: Some(transaction),
                },
                Err(error) => failed(
                    call,
                    AGENT_TOOL_FAILED,
                    error.to_string(),
                    json!({"tool": call.tool}),
                ),
            },
            Err(CommandError::ValidationFailed(bag)) => ToolResult {
                call_id: call.id.clone(),
                status: ToolResultStatus::Rejected,
                output: json!({
                    "error": {
                        "code": "COMMAND_VALIDATION_FAILED",
                        "message": "command validation failed"
                    }
                }),
                diagnostics: bag
                    .diagnostics
                    .iter()
                    .map(|item| serde_json::to_value(item).unwrap_or(Value::Null))
                    .collect(),
                transaction: None,
            },
            Err(CommandError::PermissionDenied(_)) => rejected(
                call,
                AGENT_PERMISSION_DENIED,
                "command permission denied",
                json!({"tool": call.tool}),
            ),
            Err(CommandError::WorkingSetDenied(documents)) => rejected(
                call,
                AGENT_WORKING_SET_DENIED,
                "command modified documents outside the working set",
                json!({"denied_documents": documents}),
            ),
            Err(error) => failed(
                call,
                AGENT_TOOL_FAILED,
                error.to_string(),
                json!({"tool": call.tool}),
            ),
        }
    }

    fn execute_query(
        &self,
        session: &AgentSession,
        call: &ToolCall,
        allowed: Option<&BTreeSet<DocumentId>>,
    ) -> ToolResult {
        let visible_store = allowed.map(|allowed| filter_store(&self.state.documents, allowed));
        let store = visible_store.as_ref().unwrap_or(&self.state.documents);
        let output = self.query_executor.execute(
            QueryRequest {
                query: call.tool.clone(),
                input: call.input.clone(),
            },
            &QueryContext {
                actor: session.id.to_string(),
                permissions: session.permissions.clone(),
            },
            store,
        );
        match output {
            Ok(output) => ToolResult {
                call_id: call.id.clone(),
                status: ToolResultStatus::Ok,
                output,
                diagnostics: Vec::new(),
                transaction: None,
            },
            Err(error) => failed(
                call,
                AGENT_TOOL_FAILED,
                error.to_string(),
                json!({"tool": call.tool}),
            ),
        }
    }

    pub fn run(
        &mut self,
        driver: &mut dyn AgentLoopDriver,
        session: &mut AgentSession,
    ) -> Result<(), AgentHostError> {
        if session.is_terminal() {
            return Err(AgentHostError::SessionNotActive(session.status));
        }
        session.status = SessionStatus::Running;
        let mut last_result: Option<ToolResult> = None;
        loop {
            let directive = match driver.next_action(session, &self.catalog, last_result.as_ref()) {
                Ok(directive) => directive,
                Err(error) => {
                    session.status = SessionStatus::Failed;
                    session.final_summary = Some(error.to_string());
                    return Err(error);
                }
            };
            match directive {
                AgentDirective::Tool(call) => {
                    let result = self.execute(session, call);
                    last_result = Some(result);
                    match session.status {
                        SessionStatus::BudgetExceeded => {
                            return Err(AgentHostError::BudgetExceeded)
                        }
                        SessionStatus::Failed => {
                            return Err(AgentHostError::ToolFailed(
                                session
                                    .final_summary
                                    .clone()
                                    .unwrap_or_else(|| "tool execution failed".into()),
                            ))
                        }
                        _ => {}
                    }
                }
                AgentDirective::Complete { summary } => {
                    session.status = SessionStatus::Completed;
                    session.final_summary = Some(summary);
                    return Ok(());
                }
                AgentDirective::Stop { reason } => {
                    session.status = SessionStatus::Stopped;
                    session.final_summary = Some(reason);
                    return Ok(());
                }
            }
        }
    }
}

fn filter_store(store: &DocumentStore, allowed: &BTreeSet<DocumentId>) -> DocumentStore {
    let mut visible = DocumentStore::default();
    for document in allowed {
        if let Ok(document) = store.get(document) {
            visible.upsert(document.clone());
        }
    }
    visible
}

fn record_action(session: &mut AgentSession, call: ToolCall, result: ToolResult) {
    session.actions.push(AgentActionRecord {
        sequence: session.actions.len() as u32 + 1,
        call,
        result,
    });
}

fn rejected(call: &ToolCall, code: &str, message: impl Into<String>, details: Value) -> ToolResult {
    error_result(call, ToolResultStatus::Rejected, code, message, details)
}

fn failed(call: &ToolCall, code: &str, message: impl Into<String>, details: Value) -> ToolResult {
    error_result(call, ToolResultStatus::Failed, code, message, details)
}

fn error_result(
    call: &ToolCall,
    status: ToolResultStatus,
    code: &str,
    message: impl Into<String>,
    details: Value,
) -> ToolResult {
    ToolResult {
        call_id: call.id.clone(),
        status,
        output: json!({
            "error": {
                "code": code,
                "message": message.into(),
                "details": details
            }
        }),
        diagnostics: Vec::new(),
        transaction: None,
    }
}

#[derive(Debug, Error)]
pub enum AgentHostError {
    #[error("agent action budget exceeded")]
    BudgetExceeded,
    #[error("agent session is not active: {0:?}")]
    SessionNotActive(SessionStatus),
    #[error("agent tool failed: {0}")]
    ToolFailed(String),
    #[error("agent driver failed: {0}")]
    Driver(String),
}

pub struct ScriptedAgentDriver {
    actions: VecDeque<AgentDirective>,
}

impl ScriptedAgentDriver {
    pub fn new(actions: impl IntoIterator<Item = AgentDirective>) -> Self {
        Self {
            actions: actions.into_iter().collect(),
        }
    }
}

impl AgentLoopDriver for ScriptedAgentDriver {
    fn next_action(
        &mut self,
        _session: &AgentSession,
        _catalog: &ToolCatalog,
        _last_result: Option<&ToolResult>,
    ) -> Result<AgentDirective, AgentHostError> {
        self.actions
            .pop_front()
            .ok_or_else(|| AgentHostError::Driver("script exhausted without completion".into()))
    }
}

pub fn transaction_was_applied(result: &ToolResult) -> bool {
    result
        .transaction
        .as_ref()
        .and_then(|value| value.get("status"))
        == Some(&json!(TransactionStatus::Applied))
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_agent_tools::ToolCatalog;
    use guiyi_engine_command::{
        register_builtin_document_commands, CommandDescriptor, CommandHandler, CommandRegistry,
    };
    use guiyi_engine_content::{DocumentEnvelope, DocumentHeader};
    use guiyi_engine_core::{DocumentAccessPlan, EngineTypeId, Permission, ToolId};
    use guiyi_engine_query::{register_builtin_queries, QueryRegistry};

    fn document(id: &'static str) -> DocumentEnvelope {
        DocumentEnvelope {
            header: DocumentHeader {
                id: DocumentId::from_static(id),
                type_id: EngineTypeId::from_static("example.document"),
                schema_version: 1,
                display_name: id.into(),
            },
            references: Vec::new(),
            payload: json!({}),
        }
    }

    fn host() -> AgentHost {
        let mut command_registry = CommandRegistry::default();
        register_builtin_document_commands(&mut command_registry).unwrap();
        let mut query_registry = QueryRegistry::default();
        register_builtin_queries(&mut query_registry).unwrap();
        let catalog = ToolCatalog::from_registries(&command_registry, &query_registry)
            .expect("built-in tool IDs are unique");
        AgentHost::new(
            EngineState::default(),
            CommandExecutor::new(command_registry),
            QueryExecutor::new(query_registry),
            catalog,
        )
    }

    fn session(permissions: PermissionSet) -> AgentSession {
        AgentSession::new(
            AgentSessionId::from_static("session.test"),
            "Test session",
            permissions,
        )
    }

    fn list_call(id: &str) -> ToolCall {
        ToolCall {
            id: id.into(),
            tool: ToolId::from_static("project.documents.list"),
            input: json!({}),
            dry_run: false,
        }
    }

    fn error_code(result: &ToolResult) -> Option<&str> {
        result
            .output
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str)
    }

    #[test]
    fn permission_denial_is_structured_and_recorded() {
        let mut host = host();
        let mut session = session(PermissionSet::read_only());
        let result = host.execute(
            &mut session,
            ToolCall {
                id: "call-1".into(),
                tool: ToolId::from_static("document.create"),
                input: json!({
                    "id": "doc.denied",
                    "type_id": "example.document",
                    "display_name": "Denied"
                }),
                dry_run: false,
            },
        );
        assert_eq!(result.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&result), Some(AGENT_PERMISSION_DENIED));
        assert_eq!(session.actions_used, 1);
        assert_eq!(session.actions.len(), 1);
        assert_eq!(session.status, SessionStatus::Running);
    }

    #[test]
    fn working_set_blocks_reads_and_mutations_outside_the_set() {
        let mut host = host();
        host.state.documents.insert(document("stage.a")).unwrap();
        host.state.documents.insert(document("stage.b")).unwrap();
        let mut session = session(PermissionSet::content_author());
        session.working_set = vec![DocumentId::from_static("stage.a")];
        for (tool, input) in [
            ("project.document.get", json!({"document_id": "stage.b"})),
            (
                "document.set_field",
                json!({"document_id": "stage.b", "path": ["x"], "value": 1}),
            ),
            ("document.delete", json!({"document_id": "stage.b"})),
        ] {
            let result = host.execute(
                &mut session,
                ToolCall {
                    id: format!("call-{tool}"),
                    tool: ToolId::new(tool).unwrap(),
                    input,
                    dry_run: false,
                },
            );
            assert_eq!(result.status, ToolResultStatus::Rejected);
            assert_eq!(error_code(&result), Some(AGENT_WORKING_SET_DENIED));
        }
        assert!(host
            .state
            .documents
            .get(&DocumentId::from_static("stage.b"))
            .is_ok());
        assert_eq!(session.actions.len(), 3);
    }

    #[test]
    fn project_queries_only_see_working_set_documents() {
        let mut host = host();
        host.state.documents.insert(document("stage.a")).unwrap();
        host.state.documents.insert(document("stage.b")).unwrap();
        let mut session = session(PermissionSet::read_only());
        session.working_set = vec![DocumentId::from_static("stage.a")];
        let result = host.execute(&mut session, list_call("call-list"));
        assert_eq!(result.status, ToolResultStatus::Ok);
        let values = result.output.as_array().unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0]["id"], json!("stage.a"));
    }

    #[test]
    fn budget_is_enforced_and_budget_rejection_is_audited() {
        let mut host = host();
        let mut session = session(PermissionSet::read_only());
        session.budget.max_actions = 1;
        assert_eq!(
            host.execute(&mut session, list_call("call-1")).status,
            ToolResultStatus::Ok
        );
        let rejected = host.execute(&mut session, list_call("call-2"));
        assert_eq!(rejected.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&rejected), Some(AGENT_BUDGET_EXCEEDED));
        assert_eq!(session.actions_used, 1);
        assert_eq!(session.actions.len(), 2);
        assert_eq!(session.status, SessionStatus::BudgetExceeded);
    }

    #[test]
    fn driver_can_complete_after_exactly_maximum_actions() {
        let mut host = host();
        let mut session = session(PermissionSet::read_only());
        session.budget.max_actions = 1;
        let mut driver = ScriptedAgentDriver::new([
            AgentDirective::Tool(list_call("call-1")),
            AgentDirective::Complete {
                summary: "done".into(),
            },
        ]);
        host.run(&mut driver, &mut session).unwrap();
        assert_eq!(session.actions_used, 1);
        assert_eq!(session.status, SessionStatus::Completed);
        assert_eq!(session.final_summary.as_deref(), Some("done"));
    }

    #[test]
    fn unknown_tools_are_rejected_recorded_and_do_not_stop_the_session() {
        let mut host = host();
        let mut session = session(PermissionSet::read_only());
        let result = host.execute(
            &mut session,
            ToolCall {
                id: "call-missing".into(),
                tool: ToolId::from_static("missing.tool"),
                input: json!({}),
                dry_run: false,
            },
        );
        assert_eq!(result.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&result), Some(AGENT_TOOL_NOT_FOUND));
        assert_eq!(session.actions.len(), 1);
        assert_eq!(session.status, SessionStatus::Running);
        let following = host.execute(&mut session, list_call("call-after"));
        assert_eq!(following.status, ToolResultStatus::Ok);
        assert_eq!(session.actions.len(), 2);
    }

    #[test]
    fn multi_document_access_requires_every_document() {
        struct MultiDocumentCommand;

        impl CommandHandler for MultiDocumentCommand {
            fn descriptor(&self) -> CommandDescriptor {
                CommandDescriptor {
                    id: ToolId::from_static("test.multi"),
                    title: "Multi".into(),
                    description: "Multi-document test command".into(),
                    input_schema: json!({"type": "object"}),
                    output_schema: json!({"type": "object"}),
                    required_permissions: PermissionSet::new([Permission::EditContent]),
                    side_effects: vec!["modifies_document".into()],
                    related_tools: Vec::new(),
                }
            }

            fn input_schema(&self) -> guiyi_engine_schema::SchemaNode {
                guiyi_engine_schema::SchemaNode::object(vec![])
            }

            fn document_access(&self, _input: &Value) -> Result<DocumentAccessPlan, CommandError> {
                Ok(DocumentAccessPlan::documents([
                    DocumentId::from_static("stage.a"),
                    DocumentId::from_static("stage.b"),
                ]))
            }

            fn apply(
                &self,
                _input: &Value,
                _state: &mut EngineState,
            ) -> Result<Value, CommandError> {
                Ok(json!({}))
            }
        }

        let mut commands = CommandRegistry::default();
        commands.register(MultiDocumentCommand).unwrap();
        let queries = QueryRegistry::default();
        let catalog =
            ToolCatalog::from_registries(&commands, &queries).expect("test tool IDs are unique");
        let mut host = AgentHost::new(
            EngineState::default(),
            CommandExecutor::new(commands),
            QueryExecutor::new(queries),
            catalog,
        );
        let mut session = session(PermissionSet::new([Permission::EditContent]));
        session.working_set = vec![DocumentId::from_static("stage.a")];
        let result = host.execute(
            &mut session,
            ToolCall {
                id: "call-multi".into(),
                tool: ToolId::from_static("test.multi"),
                input: json!({}),
                dry_run: false,
            },
        );
        assert_eq!(result.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&result), Some(AGENT_WORKING_SET_DENIED));
    }

    #[test]
    fn scripted_agent_creates_and_queries_a_document() {
        let mut host = host();
        let mut session = session(PermissionSet::content_author());
        let mut driver = ScriptedAgentDriver::new([
            AgentDirective::Tool(ToolCall {
                id: "call-1".into(),
                tool: ToolId::from_static("document.create"),
                input: json!({
                    "id": "doc.agent",
                    "type_id": "example.document",
                    "display_name": "Agent document"
                }),
                dry_run: false,
            }),
            AgentDirective::Tool(list_call("call-2")),
            AgentDirective::Complete {
                summary: "Created and checked the document".into(),
            },
        ]);
        host.run(&mut driver, &mut session).unwrap();
        assert_eq!(session.status, SessionStatus::Completed);
        assert_eq!(host.state.documents.len(), 1);
        assert_eq!(session.actions.len(), 2);
    }

    #[test]
    fn malformed_command_input_rejected_before_access_planning() {
        let mut host = host();
        let mut session = session(PermissionSet::content_author());
        let result = host.execute(
            &mut session,
            ToolCall {
                id: "call-malformed".into(),
                tool: ToolId::from_static("document.create"),
                input: json!({"missing_required": true}),
                dry_run: false,
            },
        );
        assert_eq!(result.status, ToolResultStatus::Rejected);
        assert_eq!(result.output["error"]["code"], "COMMAND_VALIDATION_FAILED");
        assert!(!result.diagnostics.is_empty());
        let following = host.execute(&mut session, list_call("call-after"));
        assert_eq!(following.status, ToolResultStatus::Ok);
    }
}
